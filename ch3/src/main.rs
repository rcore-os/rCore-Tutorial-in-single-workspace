//! # 第三章：多道程序与分时多任务
//!
//! 本章在第二章"批处理系统"的基础上，实现了一个**多道程序操作系统**，
//! 支持多个用户程序并发执行，并通过时钟中断实现抢占式调度。
//!
//! ## 核心概念
//!
//! - **多道程序**：多个用户程序同时驻留在内存中，内核在它们之间切换执行
//! - **任务控制块（TCB）**：管理每个任务的上下文、状态和资源
//! - **协作式调度**：任务通过 `yield` 系统调用主动让出 CPU
//! - **抢占式调度**：通过时钟中断强制切换任务，实现时间片轮转
//! - **系统调用**：`write`、`exit`、`yield`、`clock_gettime`

// 不使用标准库，裸机环境没有操作系统提供系统调用支持
#![no_std]
// 不使用标准入口，裸机环境没有 C runtime 进行初始化
#![no_main]
// RISC-V64 架构下启用严格警告和文档检查
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
// 非 RISC-V64 架构允许死代码（用于 cargo publish --dry-run 在主机上通过编译）
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

// 任务管理模块：定义任务控制块（TCB）和调度事件
mod task;

// 引入控制台输出宏（print! / println!），由 tg_console 库提供
#[macro_use]
extern crate tg_console;

// 本地模块：Console 和 SyscallContext 的实现
use impls::{Console, SyscallContext};
// riscv 库：访问 RISC-V 控制状态寄存器（CSR），如 scause、sie、time
use riscv::register::*;
// 任务控制块
use task::TaskControlBlock;
// 日志模块
use tg_console::log;
// SBI 调用：set_timer、console_putchar、shutdown 等
use tg_sbi;

// ========== 启动相关 ==========

// 将用户程序的二进制数据内联到内核镜像的 .data 段中
// APP_ASM 由 build.rs 在编译时生成，包含所有用户程序的二进制数据
#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(include_str!(env!("APP_ASM")));

// 最大支持的应用程序数量
const APP_CAPACITY: usize = 32;

// 定义内核入口点：分配 (APP_CAPACITY + 2) * 8 KiB = 272 KiB 的内核栈
// 比第二章更大，因为需要同时容纳多个任务的内核上下文
#[cfg(target_arch = "riscv64")]
tg_linker::boot0!(rust_main; stack = (APP_CAPACITY + 2) * 8192);

// ========== 内核主函数 ==========

/// 内核主函数：初始化各子系统，然后以多道方式并发运行所有用户程序。
///
/// 与第二章的串行批处理不同，本章的多道程序系统支持：
/// - 多个任务同时驻留在内存中，每个任务拥有独立的 TCB 和用户栈
/// - 任务之间通过时间片轮转切换（抢占式调度，默认模式）
/// - 任务可以主动让出 CPU（协作式调度，通过 yield，需启用 `coop` feature）
extern "C" fn rust_main() -> ! {
    // 第一步：清零 BSS 段（未初始化的全局变量区域）
    unsafe { tg_linker::KernelLayout::locate().zero_bss() };

    // 第二步：初始化控制台输出（使 print!/println! 可用）
    // 默认日志级别为 info（可通过 LOG 环境变量覆盖）
    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG").or(Some("info")));
    tg_console::test_log();

    // 第三步：初始化系统调用处理
    // 比第二章多了 scheduling（yield）、clock（获取时间）和 trace（追踪，练习题）
    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);
    tg_syscall::init_scheduling(&SyscallContext);
    tg_syscall::init_clock(&SyscallContext);
    tg_syscall::init_trace(&SyscallContext);

    // 第四步：初始化任务控制块数组，加载所有用户程序
    let mut tcbs = [TaskControlBlock::ZERO; APP_CAPACITY];
    let mut index_mod = 0;
    for (i, app) in tg_linker::AppMeta::locate().iter().enumerate() {
        let entry = app.as_ptr() as usize;
        log::info!("load app{i} to {entry:#x}");
        tcbs[i].init(entry);
        index_mod += 1;
    }
    println!();

    // 第五步：开启 S 特权级时钟中断
    // 这是实现抢占式调度的关键：允许时钟中断打断用户程序的执行
    unsafe { sie::set_stimer() };

    // ========== 多道程序主循环 ==========
    // 使用轮转调度算法（Round-Robin），依次执行各任务
    let mut remain = index_mod; // 剩余未完成的任务数
    let mut i = 0usize; // 当前任务索引
    while remain > 0 {
        let tcb = &mut tcbs[i];
        if !tcb.finish {
            loop {
                // 【抢占式调度】设置时钟中断：12500 个时钟周期后触发
                // 当 coop feature 启用时，跳过此步（协作式调度，不使用时钟中断）
                #[cfg(not(feature = "coop"))]
                tg_sbi::set_timer(time::read64() + 12500);

                // 切换到 U-mode 执行用户程序
                // execute() 会恢复用户寄存器并执行 sret
                // 当用户程序触发 Trap 后返回到这里
                unsafe { tcb.execute() };

                // 读取 scause 寄存器判断 Trap 原因
                use scause::*;
                let finish = match scause::read().cause() {
                    // ─── 时钟中断：时间片用完，切换到下一个任务 ───
                    Trap::Interrupt(Interrupt::SupervisorTimer) => {
                        // 清除时钟中断（设置为最大值，避免立即再次触发）
                        tg_sbi::set_timer(u64::MAX);
                        log::trace!("app{i} timeout");
                        false // 不结束任务，切换到下一个
                    }
                    // ─── 系统调用：用户程序执行了 ecall 指令 ───
                    Trap::Exception(Exception::UserEnvCall) => {
                        use task::SchedulingEvent as Event;
                        match tcb.handle_syscall() {
                            // 普通系统调用（如 write）：处理完成后继续运行当前任务
                            Event::None => continue,
                            // exit 系统调用：任务主动退出
                            Event::Exit(code) => {
                                log::info!("app{i} exit with code {code}");
                                true
                            }
                            // yield 系统调用：任务主动让出 CPU
                            Event::Yield => {
                                log::debug!("app{i} yield");
                                false // 不结束任务，切换到下一个
                            }
                            // 不支持的系统调用：杀死任务
                            Event::UnsupportedSyscall(id) => {
                                log::error!("app{i} call an unsupported syscall {}", id.0);
                                true
                            }
                        }
                    }
                    // ─── 其他异常（如非法指令、页错误等）：杀死应用 ───
                    Trap::Exception(e) => {
                        log::error!("app{i} was killed by {e:?}");
                        true
                    }
                    // ─── 未预期的中断：杀死应用 ───
                    Trap::Interrupt(ir) => {
                        log::error!("app{i} was killed by an unexpected interrupt {ir:?}");
                        true
                    }
                };

                // 如果任务结束（退出或被杀死），标记为已完成
                if finish {
                    tcb.finish = true;
                    remain -= 1;
                }
                break;
            }
        }
        // 轮转到下一个任务（循环取模）
        i = (i + 1) % index_mod;
    }

    // 所有用户程序执行完毕，关机
    tg_sbi::shutdown(false)
}

// ========== panic 处理 ==========

/// panic 处理函数：打印错误信息后以异常状态关机。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
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

    /// IO 系统调用实现：处理 write 系统调用
    impl IO for SyscallContext {
        #[inline]
        fn write(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
            match fd {
                // 标准输出和调试输出：将缓冲区内容打印到控制台
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

    /// Process 系统调用实现：处理 exit 系统调用
    impl Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: Caller, _status: usize) -> isize {
            0
        }
    }

    /// Scheduling 系统调用实现：处理 yield 系统调用
    ///
    /// `sched_yield` 允许任务主动让出 CPU，是协作式调度的基础。
    /// 内核在收到 yield 后会切换到下一个就绪任务。
    impl Scheduling for SyscallContext {
        #[inline]
        fn sched_yield(&self, _caller: Caller) -> isize {
            0
        }
    }

    /// Clock 系统调用实现：处理 clock_gettime 系统调用
    ///
    /// 将 RISC-V 硬件计时器的值转换为纳秒精度的时间。
    /// QEMU virt 平台的时钟频率为 12.5 MHz（10000/125 = 80 ns/tick）。
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
                    // 将 RISC-V time 寄存器的值转换为纳秒
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

    /// Trace 系统调用实现（练习题需要完成的部分）
    ///
    /// 当前为占位实现，返回 -1 表示未实现。
    /// 学生需要在练习中实现 trace 功能，支持：
    /// - 读取用户内存（trace_request=0）
    /// - 写入用户内存（trace_request=1）
    /// - 查询系统调用计数（trace_request=2）
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
///
/// 提供编译所需的符号，使得 `cargo publish --dry-run` 在主机平台上能通过编译。
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
