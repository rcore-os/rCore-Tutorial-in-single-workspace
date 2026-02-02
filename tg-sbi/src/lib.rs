//! # tg-sbi
//!
//! 为 rCore 教学操作系统提供的 SBI (Supervisor Binary Interface) 调用封装。
//!
//! ## Features
//!
//! - `nobios`: 启用内置的 M-Mode SBI 实现，用于 `-bios none` 启动模式。
//!   启用此 feature 后，crate 将提供自己的 M-Mode 陷阱处理程序和基本 SBI 服务。
//!
//! ## 支持的 SBI 扩展
//!
//! - Legacy 控制台 I/O (EID 0x01, 0x02)
//! - Timer 扩展 (EID 0x54494D45)
//! - System Reset 扩展 (EID 0x53525354)
//!
//! ## Example
//!
//! ```ignore
//! use tg_sbi::{console_putchar, set_timer, shutdown};
//!
//! // 输出字符
//! console_putchar(b'H');
//!
//! // 设置定时器中断
//! set_timer(1000000);
//!
//! // 关闭系统
//! shutdown(false);
//! ```

#![no_std]
#![deny(warnings, missing_docs)]

// M-Mode SBI 实现（用于 -bios none 启动）
#[cfg(all(feature = "nobios", target_arch = "riscv64"))]
pub mod msbi;
// M-Mode SBI 入口点（用于 -bios none 启动）
#[cfg(all(feature = "nobios", target_arch = "riscv64"))]
core::arch::global_asm!(include_str!("m_entry.asm"));

// Legacy SBI 调用号（用于兼容性）
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;

// SBI 扩展 ID
const SBI_EXT_TIMER: usize = 0x54494D45;
const SBI_EXT_SRST: usize = 0x53525354;

/// 通用 SBI 调用。
#[cfg(all(target_arch = "riscv64", not(feature = "nobios")))]
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    // SAFETY: 执行 SBI ecall，这是 S-mode 软件向 SBI 固件请求服务的标准接口。
    // ecall 指令定义明确，寄存器约定遵循 RISC-V SBI 规范。
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") fid,
            in("x17") eid,
        );
    }
    ret
}

/// 通用 SBI 调用（nobios 模式，使用自定义 M-mode 处理程序）。
#[cfg(all(target_arch = "riscv64", feature = "nobios"))]
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret1: isize;
    let ret2: usize;
    // SAFETY: 执行 ecall 调用自定义的 M-mode SBI 处理程序。
    // 处理程序在 m_entry.asm 中设置，在 msbi.rs 中实现。
    // 寄存器约定遵循 RISC-V SBI 规范。
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") arg0 => ret1,
            inlateout("x11") arg1 => ret2,
            in("x12") arg2,
            in("x16") fid,
            in("x17") eid
        );
    }
    if ret1 < 0 {
        panic!("SBI call failed: {}", ret1);
    }
    ret2
}

/// 非 riscv64 架构处理。
#[cfg(not(target_arch = "riscv64"))]
#[inline(always)]
fn sbi_call(_eid: usize, _fid: usize, _arg0: usize, _arg1: usize, _arg2: usize) -> usize {
    unimplemented!("SBI calls are only supported on riscv64")
}

/// 设置下一次定时器中断的时间。
pub fn set_timer(timer: u64) {
    sbi_call(SBI_EXT_TIMER, 0, timer as usize, 0, 0);
}

/// 向调试控制台输出一个字符。
pub fn console_putchar(c: u8) {
    sbi_call(SBI_CONSOLE_PUTCHAR, 0, c as usize, 0, 0);
}

/// 从调试控制台读取一个字符。
pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0, 0)
}

/// 关闭系统。
pub fn shutdown(failure: bool) -> ! {
    if failure {
        sbi_call(SBI_EXT_SRST, 0, 1, 0, 0);
    } else {
        sbi_call(SBI_EXT_SRST, 0, 0, 0, 0);
    }
    panic!("It should shutdown!");
}
