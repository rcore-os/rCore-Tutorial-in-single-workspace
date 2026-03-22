//! # 第二章：批处理系统（教学增强版 ch2-show）
//!
//! 在 ch2 基础上增加三类教学扩展：
//!
//! 1. **lec3 知识点可观测标签**：以 `[LEC3-LAB2]` 为前缀输出可 grep 的标签行，
//!    覆盖特权级、Trap 机制、系统调用 ABI、`sepc` 不变量、批处理顺序等核心概念。
//! 2. **源码级 backtrace**：基于帧指针（s0）栈回溯 + build.rs 提取的 DWARF 符号，
//!    在正常执行和 panic/异常路径下均展示 `fn=name(params) at file:line`。
//! 3. **GDB 调试脚本**：配合 `scripts/` 和 `gdb/` 目录下的脚本，
//!    使用 `riscv-none-elf-gdb-py3` 观察特权级切换过程。
//!
//! ## 核心概念
//!
//! - **批处理系统**：将多个用户程序打包，自动依次执行
//! - **特权级切换**：U-mode（用户态）与 S-mode（内核态）之间的切换
//! - **Trap 处理**：用户程序通过 `ecall` 触发系统调用，或因异常陷入内核
//! - **上下文保存与恢复**：进入/退出 Trap 时保存/恢复用户寄存器状态
//! - **系统调用**：`write`（输出）和 `exit`（退出）

#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

#[macro_use]
extern crate tg_console;

use impls::{Console, SyscallContext};
use riscv::register::*;
use tg_console::log;
use tg_kernel_context::LocalContext;
use tg_sbi;
use tg_syscall::{Caller, SyscallId};

#[cfg(target_arch = "riscv64")]
mod heap;
#[cfg(target_arch = "riscv64")]
mod lec3_lab2;
#[cfg(target_arch = "riscv64")]
mod stackwalk;
#[cfg(target_arch = "riscv64")]
mod symtab_resolve;

// ========== backtrace 演示用嵌套调用 ==========

/// 嵌套调用链最深层：展示 `&str` 和 `bool` 参数在 backtrace 中的显示。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth3(msg: &str, flag: bool) {
    core::hint::black_box(msg);
    core::hint::black_box(flag);
    stackwalk::print_backtrace();
}

/// 嵌套调用链中层：展示 `u32` 和 `&str` 参数。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth2(count: u32, label: &str) {
    core::hint::black_box(count);
    bt_depth3(label, count > 0);
}

/// 嵌套调用链入口：展示 `usize`、`&str`、`i64` 参数。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth1(id: usize, name: &str, value: i64) {
    core::hint::black_box(id);
    core::hint::black_box(value);
    bt_depth2(id as u32, name);
}

/// 模拟数组越界访问——Rust 边界检查触发 panic。
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

// ========== 启动相关 ==========

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(include_str!(env!("APP_ASM")));

/// 内核入口点：设置 8 页（32 KiB）的内核栈，然后跳转到 rust_main。
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 8 * 4096;
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

/// 内核主函数：初始化 → lec3 知识点展示 → backtrace 演示 → 批处理执行。
extern "C" fn rust_main() -> ! {
    // 清零 BSS 段
    unsafe { tg_linker::KernelLayout::locate().zero_bss() };

    // 初始化堆分配器（symtab_resolve 使用 rustc_demangle 需要 alloc）
    #[cfg(target_arch = "riscv64")]
    heap::init();
    #[cfg(target_arch = "riscv64")]
    symtab_resolve::init();

    // 初始化控制台输出
    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG"));
    tg_console::test_log();

    // 初始化系统调用处理
    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);

    println!("Hello, world from ch2-show!");

    // ===== 扩展一：lec3 知识点展示 =====
    #[cfg(target_arch = "riscv64")]
    lec3_lab2::emit_pre_batch_observables();

    // ===== 扩展二：正常执行 backtrace 演示 =====
    #[cfg(target_arch = "riscv64")]
    {
        println!("\n--- Normal backtrace demo ---");
        bt_depth1(42, "batch_os", -1);
    }

    // ===== 扩展二：panic 路径 backtrace 演示 =====
    #[cfg(target_arch = "riscv64")]
    {
        println!("\n--- Error/panic backtrace demo ---");
        trigger_error("oob", 10);
    }

    // 上面 trigger_error 会 panic，以下代码仅在未启用 backtrace 时到达

    // ===== 批处理——依次加载并运行每个用户程序 =====
    run_batch();

    tg_sbi::shutdown(false)
}

/// 批处理主循环：独立函数便于在 panic-backtrace 演示之后由 panic_handler
/// 判断是否跳过（教学场景中先展示 backtrace 再进入批处理）。
fn run_batch() {
    #[cfg(target_arch = "riscv64")]
    let mut first_syscall = true;

    for (i, app) in tg_linker::AppMeta::locate().iter().enumerate() {
        let app_base = app.as_ptr() as usize;
        log::info!("load app{i} to {app_base:#x}");

        #[cfg(target_arch = "riscv64")]
        lec3_lab2::emit_app_load_info(i, app_base);

        let mut ctx = LocalContext::user(app_base);

        let mut user_stack: core::mem::MaybeUninit<[usize; 512]> =
            core::mem::MaybeUninit::uninit();
        let user_stack_ptr = user_stack.as_mut_ptr() as *mut usize;
        *ctx.sp_mut() = unsafe { user_stack_ptr.add(512) } as usize;

        loop {
            unsafe { ctx.execute() };

            use scause::{Exception, Trap};
            match scause::read().cause() {
                Trap::Exception(Exception::UserEnvCall) => {
                    #[cfg(target_arch = "riscv64")]
                    {
                        let scause_val: usize;
                        let sepc_val: usize;
                        let stval_val: usize;
                        unsafe {
                            core::arch::asm!("csrr {}, scause", out(reg) scause_val);
                            core::arch::asm!("csrr {}, sepc", out(reg) sepc_val);
                            core::arch::asm!("csrr {}, stval", out(reg) stval_val);
                        }
                        if first_syscall {
                            lec3_lab2::emit_trap_info(i, scause_val, stval_val, sepc_val);
                            lec3_lab2::emit_syscall_info(
                                ctx.a(7),
                                ctx.a(0),
                                ctx.a(1),
                                ctx.a(2),
                            );
                            first_syscall = false;
                        }
                    }

                    use SyscallResult::*;
                    match handle_syscall(&mut ctx) {
                        Done => continue,
                        Exit(code) => log::info!("app{i} exit with code {code}"),
                        Error(id) => {
                            log::error!("app{i} call an unsupported syscall {}", id.0)
                        }
                    }
                }
                trap => {
                    #[cfg(target_arch = "riscv64")]
                    {
                        let scause_val: usize;
                        let sepc_val: usize;
                        let stval_val: usize;
                        unsafe {
                            core::arch::asm!("csrr {}, scause", out(reg) scause_val);
                            core::arch::asm!("csrr {}, sepc", out(reg) sepc_val);
                            core::arch::asm!("csrr {}, stval", out(reg) stval_val);
                        }
                        lec3_lab2::emit_trap_info(i, scause_val, stval_val, sepc_val);
                    }

                    log::error!("app{i} was killed because of {trap:?}");

                    #[cfg(target_arch = "riscv64")]
                    {
                        println!("[TRAP-BACKTRACE] kernel call stack when handling user trap:");
                        stackwalk::print_backtrace();
                    }
                }
            }
            unsafe { core::arch::asm!("fence.i") };
            break;
        }
        let _ = core::hint::black_box(&user_stack);
        println!();
    }
}

// ========== panic 处理 ==========

/// panic 处理函数：打印错误信息 + backtrace 后以异常状态关机。
/// 如果是 backtrace 演示触发的 panic（trigger_error），则在 panic 处理后
/// 继续运行批处理循环。
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

    // 在 backtrace 演示后继续执行批处理
    #[cfg(target_arch = "riscv64")]
    {
        let _ = write!(W, "\n--- Continuing to batch execution after panic demo ---\n\n");
        run_batch();
        tg_sbi::shutdown(false);
    }

    #[cfg(not(target_arch = "riscv64"))]
    tg_sbi::shutdown(true)
}

// ========== 系统调用处理 ==========

/// 系统调用处理结果
enum SyscallResult {
    /// 系统调用完成，继续执行用户程序
    Done,
    /// 用户程序请求退出，附带退出码
    Exit(usize),
    /// 不支持的系统调用
    Error(SyscallId),
}

/// 处理系统调用：提取 a7（id）和 a0-a5（参数），分发并写回返回值。
fn handle_syscall(ctx: &mut LocalContext) -> SyscallResult {
    use tg_syscall::{SyscallId as Id, SyscallResult as Ret};

    let id = ctx.a(7).into();
    let args = [ctx.a(0), ctx.a(1), ctx.a(2), ctx.a(3), ctx.a(4), ctx.a(5)];

    match tg_syscall::handle(Caller { entity: 0, flow: 0 }, id, args) {
        Ret::Done(ret) => match id {
            Id::EXIT => SyscallResult::Exit(ctx.a(0)),
            _ => {
                #[cfg(target_arch = "riscv64")]
                let sepc_before = ctx.pc();

                *ctx.a_mut(0) = ret as _;
                ctx.move_next();

                #[cfg(target_arch = "riscv64")]
                {
                    static SEPC_DEMO_DONE: core::sync::atomic::AtomicBool =
                        core::sync::atomic::AtomicBool::new(false);
                    if !SEPC_DEMO_DONE.load(core::sync::atomic::Ordering::Relaxed) {
                        SEPC_DEMO_DONE.store(true, core::sync::atomic::Ordering::Relaxed);
                        lec3_lab2::emit_sepc_advance(sepc_before, ctx.pc());
                    }
                }

                SyscallResult::Done
            }
        },
        Ret::Unsupported(id) => SyscallResult::Error(id),
    }
}

// ========== 接口实现 ==========

/// 各依赖库所需接口的具体实现
mod impls {
    use tg_syscall::{STDDEBUG, STDOUT};

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

    impl tg_syscall::IO for SyscallContext {
        fn write(
            &self,
            _caller: tg_syscall::Caller,
            fd: usize,
            buf: usize,
            count: usize,
        ) -> isize {
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

    impl tg_syscall::Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: tg_syscall::Caller, _status: usize) -> isize {
            0
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
