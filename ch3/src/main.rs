//! 第三章：多道程序与分时多任务
//!
//! 本章实现了一个多道程序操作系统，支持多道程序并发执行，能够依次加载并运行多个用户程序。
#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

mod task;

#[macro_use]
extern crate tg_console;

use impls::{Console, SyscallContext};
use riscv::register::*;
use task::TaskControlBlock;
use tg_console::log;
use tg_sbi;

// 应用程序内联进来。
#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(include_str!(env!("APP_ASM")));
// 应用程序数量。
const APP_CAPACITY: usize = 32;
// 定义内核入口。
#[cfg(target_arch = "riscv64")]
tg_linker::boot0!(rust_main; stack = (APP_CAPACITY + 2) * 8192);

extern "C" fn rust_main() -> ! {
    // bss 段清零
    unsafe { tg_linker::KernelLayout::locate().zero_bss() };
    // 初始化 `console`
    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG"));
    tg_console::test_log();
    // 初始化 syscall
    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);
    tg_syscall::init_scheduling(&SyscallContext);
    tg_syscall::init_clock(&SyscallContext);
    tg_syscall::init_trace(&SyscallContext);
    // 任务控制块
    let mut tcbs = [TaskControlBlock::ZERO; APP_CAPACITY];
    let mut index_mod = 0;
    // 初始化
    for (i, app) in tg_linker::AppMeta::locate().iter().enumerate() {
        let entry = app.as_ptr() as usize;
        log::info!("load app{i} to {entry:#x}");
        tcbs[i].init(entry);
        index_mod += 1;
    }
    println!();
    // 打开中断
    unsafe { sie::set_stimer() };
    // 多道执行
    let mut remain = index_mod;
    let mut i = 0usize;
    while remain > 0 {
        let tcb = &mut tcbs[i];
        if !tcb.finish {
            loop {
                #[cfg(not(feature = "coop"))]
                tg_sbi::set_timer(time::read64() + 12500);
                unsafe { tcb.execute() };

                use scause::*;
                let finish = match scause::read().cause() {
                    Trap::Interrupt(Interrupt::SupervisorTimer) => {
                        tg_sbi::set_timer(u64::MAX);
                        log::trace!("app{i} timeout");
                        false
                    }
                    Trap::Exception(Exception::UserEnvCall) => {
                        use task::SchedulingEvent as Event;
                        match tcb.handle_syscall() {
                            Event::None => continue,
                            Event::Exit(code) => {
                                log::info!("app{i} exit with code {code}");
                                true
                            }
                            Event::Yield => {
                                log::debug!("app{i} yield");
                                false
                            }
                            Event::UnsupportedSyscall(id) => {
                                log::error!("app{i} call an unsupported syscall {}", id.0);
                                true
                            }
                        }
                    }
                    Trap::Exception(e) => {
                        log::error!("app{i} was killed by {e:?}");
                        true
                    }
                    Trap::Interrupt(ir) => {
                        log::error!("app{i} was killed by an unexpected interrupt {ir:?}");
                        true
                    }
                };
                if finish {
                    tcb.finish = true;
                    remain -= 1;
                }
                break;
            }
        }
        i = (i + 1) % index_mod;
    }
    tg_sbi::shutdown(false)
}

/// Rust 异常处理函数，以异常方式关机。
#[cfg(target_arch = "riscv64")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    tg_sbi::shutdown(true)
}

/// 各种接口库的实现
mod impls {
    use tg_syscall::*;

    pub struct Console;

    impl tg_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            tg_sbi::console_putchar(c);
        }
    }

    pub struct SyscallContext;

    impl IO for SyscallContext {
        #[inline]
        fn write(&self, _caller: tg_syscall::Caller, fd: usize, buf: usize, count: usize) -> isize {
            match fd {
                STDOUT | STDDEBUG => {
                    print!("{}", unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            buf as *const u8,
                            count,
                        ))
                    });
                    count as _
                }
                _ => {
                    tg_console::log::error!("unsupported fd: {fd}");
                    -1
                }
            }
        }
    }

    impl Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: tg_syscall::Caller, _status: usize) -> isize {
            0
        }
    }

    impl Scheduling for SyscallContext {
        #[inline]
        fn sched_yield(&self, _caller: tg_syscall::Caller) -> isize {
            0
        }
    }

    impl Clock for SyscallContext {
        #[inline]
        fn clock_gettime(
            &self,
            _caller: tg_syscall::Caller,
            clock_id: ClockId,
            tp: usize,
        ) -> isize {
            match clock_id {
                ClockId::CLOCK_MONOTONIC => {
                    let time = riscv::register::time::read() * 10000 / 125;
                    *unsafe { &mut *(tp as *mut TimeSpec) } = TimeSpec {
                        tv_sec: time / 1_000_000_000,
                        tv_nsec: time % 1_000_000_000,
                    };
                    0
                }
                _ => -1,
            }
        }
    }

    impl Trace for SyscallContext {
        // TODO: 实现 trace 系统调用
        #[inline]
        fn trace(
            &self,
            _caller: tg_syscall::Caller,
            _trace_request: usize,
            _id: usize,
            _data: usize,
        ) -> isize {
            tg_console::log::info!("trace: not implemented");
            -1
        }
    }
}

/// 非 RISC-V64 架构的占位实现
#[cfg(not(target_arch = "riscv64"))]
mod stub {
    #[panic_handler]
    fn panic(_: &core::panic::PanicInfo) -> ! {
        loop {}
    }

    #[no_mangle]
    pub extern "C" fn main() -> i32 {
        0
    }

    #[no_mangle]
    pub extern "C" fn __libc_start_main() -> i32 {
        0
    }
}
