//! # 第三章（show 版）：多道程序与分时多任务
//!
//! 在第三章基础上增加了三个教学扩展：
//!
//! 1. **lec4 知识点展示**：在代码关键路径输出可 grep 的 `[LEC4-LAB3]` 标签
//! 2. **源码级 backtrace**：通过帧指针回溯展示内核态函数调用栈（含参数值）
//! 3. **GDB 调试支持**：配合 `gdb/` 和 `scripts/` 下的脚本观察特权级切换
//!
//! ## 核心概念
//!
//! - **多道程序**：多个用户程序同时驻留在内存中，内核在它们之间切换执行
//! - **任务控制块（TCB）**：管理每个任务的上下文、状态和资源
//! - **协作式调度**：任务通过 `yield` 系统调用主动让出 CPU
//! - **抢占式调度**：通过时钟中断强制切换任务，实现时间片轮转

#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

mod task;

#[cfg(target_arch = "riscv64")]
mod heap;
#[cfg(target_arch = "riscv64")]
mod lec4_lab3;
#[cfg(target_arch = "riscv64")]
mod stackwalk;
#[cfg(target_arch = "riscv64")]
mod symtab_resolve;

#[macro_use]
extern crate tg_console;

use impls::{Console, SyscallContext};
use riscv::register::*;
use task::TaskControlBlock;
use tg_console::log;
use tg_sbi;

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(include_str!(env!("APP_ASM")));

const APP_CAPACITY: usize = 32;

// ========== backtrace 演示函数 ==========

/// 嵌套调用以形成多帧，便于 backtrace 打印动态调用关系。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth3(msg: &str, flag: bool) {
    core::hint::black_box(msg);
    core::hint::black_box(flag);
    stackwalk::print_backtrace();
}

/// 中间层：携带不同基本类型参数展示 backtrace 中的形参名显示能力。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth2(count: u32, label: &str) {
    core::hint::black_box(count);
    bt_depth3(label, count > 0);
}

/// 外层入口：触发正常执行路径下的 backtrace。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth1(id: usize, name: &str, value: i64) {
    core::hint::black_box(id);
    core::hint::black_box(value);
    bt_depth2(id as u32, name);
}

/// 模拟数组越界访问——触发 panic 以演示异常路径 backtrace。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn buggy_access(data: &[u8], index: usize) {
    core::hint::black_box(data);
    core::hint::black_box(index);
    let _val = data[index];
}

/// 错误触发入口：构造短数组并越界访问。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn trigger_error(kind: &str, n: usize) {
    core::hint::black_box(kind);
    let arr = [10u8, 20, 30];
    buggy_access(&arr, n);
}

// ========== 内核入口 ==========

#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = (APP_CAPACITY + 2) * 8192;
    #[unsafe(link_section = ".boot.stack")]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",
        "j  {main}",
        stack = sym STACK,
        stack_size = const STACK_SIZE,
        main = sym rust_main,
    )
}

// ========== 内核主函数 ==========

/// 内核主函数：初始化、展示 lec4 知识点、运行多道程序、演示 backtrace。
extern "C" fn rust_main() -> ! {
    unsafe { tg_linker::KernelLayout::locate().zero_bss() };

    #[cfg(target_arch = "riscv64")]
    heap::init();
    #[cfg(target_arch = "riscv64")]
    symtab_resolve::init();

    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG").or(Some("info")));
    tg_console::test_log();

    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);
    tg_syscall::init_scheduling(&SyscallContext);
    tg_syscall::init_clock(&SyscallContext);
    tg_syscall::init_trace(&SyscallContext);

    // ========== 加载应用程序 ==========
    let mut tcbs = [TaskControlBlock::ZERO; APP_CAPACITY];
    let mut index_mod = 0;
    let app_meta = tg_linker::AppMeta::locate();

    for (i, app) in app_meta.iter().enumerate() {
        let entry = app.as_ptr() as usize;
        log::info!("load app{i} to {entry:#x}");
        tcbs[i].init(entry);
        #[cfg(target_arch = "riscv64")]
        lec4_lab3::emit_app_load(i, entry, tcbs[i].stack_top());
        index_mod += 1;
    }
    println!();

    // ========== lec4 静态知识点输出 ==========
    #[cfg(target_arch = "riscv64")]
    {
        // AppMeta 的 base/step 字段是私有的，直接从 apps 符号读取
        unsafe extern "C" {
            static apps: [u64; 4];
        }
        let (base, step) = unsafe { (apps[0], apps[1]) };
        lec4_lab3::emit_init_observables(index_mod, base, step);
    }

    // ========== 正常执行 backtrace 演示 ==========
    #[cfg(target_arch = "riscv64")]
    bt_depth1(42, "multitask_os", -1);

    // ========== 开启时钟中断 ==========
    unsafe { sie::set_stimer() };

    // ========== 多道程序主循环 ==========
    let mut remain = index_mod;
    let mut i = 0usize;
    let mut first_run = [true; APP_CAPACITY];
    while remain > 0 {
        let tcb = &mut tcbs[i];
        if !tcb.finish {
            loop {
                #[cfg(not(feature = "coop"))]
                tg_sbi::set_timer(time::read64() + 12500);

                #[cfg(target_arch = "riscv64")]
                if first_run[i] {
                    lec4_lab3::emit_first_enter_user(i, tcb.sepc());
                    first_run[i] = false;
                }

                unsafe { tcb.execute() };

                use scause::*;
                let finish = match scause::read().cause() {
                    // ---- 时钟中断 ----
                    Trap::Interrupt(Interrupt::SupervisorTimer) => {
                        tg_sbi::set_timer(u64::MAX);
                        log::trace!("app{i} timeout");
                        #[cfg(target_arch = "riscv64")]
                        lec4_lab3::emit_timer_interrupt(i);
                        false
                    }
                    // ---- 系统调用 ----
                    Trap::Exception(Exception::UserEnvCall) => {
                        #[cfg(target_arch = "riscv64")]
                        lec4_lab3::emit_syscall_trap(
                            i,
                            tcb.syscall_id_raw(),
                            tcb.a0(),
                            tcb.sepc(),
                        );

                        use task::SchedulingEvent as Event;
                        match tcb.handle_syscall() {
                            Event::None => continue,
                            Event::Exit(code) => {
                                log::info!("app{i} exit with code {code}");
                                #[cfg(target_arch = "riscv64")]
                                lec4_lab3::emit_task_exit(i, code, remain - 1);
                                true
                            }
                            Event::Yield => {
                                log::debug!("app{i} yield");
                                let next = (i + 1) % index_mod;
                                #[cfg(target_arch = "riscv64")]
                                lec4_lab3::emit_yield_switch(i, next);
                                false
                            }
                            Event::UnsupportedSyscall(id) => {
                                log::error!(
                                    "app{i} call an unsupported syscall {}",
                                    id.0
                                );
                                true
                            }
                        }
                    }
                    // ---- 其他异常 ----
                    Trap::Exception(e) => {
                        log::error!("app{i} was killed by {e:?}");
                        #[cfg(target_arch = "riscv64")]
                        {
                            let stval = stval::read();
                            let mut cause_buf = [0u8; 128];
                            let len = format_exception_str(&e, &mut cause_buf);
                            let cause_str =
                                core::str::from_utf8(&cause_buf[..len]).unwrap_or("Unknown");
                            lec4_lab3::emit_exception_kill(i, cause_str, stval);
                        }
                        true
                    }
                    // ---- 未预期中断 ----
                    Trap::Interrupt(ir) => {
                        log::error!(
                            "app{i} was killed by an unexpected interrupt {ir:?}"
                        );
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

        let prev_i = i;
        i = (i + 1) % index_mod;
        if prev_i != i && !tcbs[prev_i].finish {
            #[cfg(target_arch = "riscv64")]
            lec4_lab3::emit_task_switch(prev_i, i);
        }
    }

    // ========== 异常路径 backtrace 演示 ==========
    #[cfg(target_arch = "riscv64")]
    trigger_error("oob", 10);

    tg_sbi::shutdown(false)
}

/// 将异常类型格式化为字符串（使用 Debug trait）。
#[cfg(target_arch = "riscv64")]
fn format_exception_str(e: &scause::Exception, buf: &mut [u8; 128]) -> usize {
    use core::fmt::Write;
    struct BufWriter<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }
    impl core::fmt::Write for BufWriter<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for &b in s.as_bytes() {
                if self.pos < self.buf.len() {
                    self.buf[self.pos] = b;
                    self.pos += 1;
                }
            }
            Ok(())
        }
    }
    let mut w = BufWriter { buf: buf, pos: 0 };
    let _ = write!(w, "{e:?}");
    w.pos
}

// ========== panic 处理 ==========

/// panic 处理函数：打印错误信息和 backtrace 后以异常状态关机。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
    struct W;
    impl core::fmt::Write for W {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for b in s.bytes() {
                tg_sbi::console_putchar(b);
            }
            Ok(())
        }
    }
    let _ = write!(W, "\n[PANIC] {info}\n");
    #[cfg(target_arch = "riscv64")]
    stackwalk::print_backtrace();
    tg_sbi::shutdown(true)
}

// ========== 接口实现 ==========

/// 各依赖库所需接口的具体实现
mod impls {
    use tg_syscall::*;

    /// 控制台实现：通过 SBI 逐字符输出
    pub struct Console;

    impl tg_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            tg_sbi::console_putchar(c);
        }
    }

    /// 系统调用上下文实现
    pub struct SyscallContext;

    /// IO 系统调用实现
    impl IO for SyscallContext {
        #[inline]
        fn write(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
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

    /// Process 系统调用实现
    impl Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: Caller, _status: usize) -> isize {
            0
        }
    }

    /// Scheduling 系统调用实现
    impl Scheduling for SyscallContext {
        #[inline]
        fn sched_yield(&self, _caller: Caller) -> isize {
            0
        }
    }

    /// Clock 系统调用实现
    impl Clock for SyscallContext {
        #[inline]
        fn clock_gettime(
            &self,
            _caller: Caller,
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

    /// Trace 系统调用实现（练习题）
    impl Trace for SyscallContext {
        #[inline]
        fn trace(
            &self,
            _caller: Caller,
            _trace_request: usize,
            _id: usize,
            _data: usize,
        ) -> isize {
            tg_console::log::info!("trace: not implemented");
            -1
        }
    }
}

/// 非 RISC-V64 架构的占位模块。
#[cfg(not(target_arch = "riscv64"))]
mod stub {
    /// 主机平台占位入口
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        0
    }

    /// C 运行时占位
    #[unsafe(no_mangle)]
    pub extern "C" fn __libc_start_main() -> i32 {
        0
    }

    /// Rust 异常处理人格占位
    #[unsafe(no_mangle)]
    pub extern "C" fn rust_eh_personality() {}
}
