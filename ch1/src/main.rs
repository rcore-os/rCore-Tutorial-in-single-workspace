//! 第一章：应用程序与基本执行环境
//!
//! 本章实现了一个最简单的 RISC-V S 态裸机程序，展示操作系统的最小执行环境。
#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

use tg_sbi;

/// Supervisor 汇编入口。
///
/// 设置栈并跳转到 Rust。
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 4096;

    #[link_section = ".bss.uninit"]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",
        "j  {main}",
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        main       =   sym rust_main,
    )
}

/// 非常简单的 Supervisor 裸机程序。
///
/// 打印 `Hello, World!`，然后关机。
extern "C" fn rust_main() -> ! {
    for c in b"Hello, world!\n" {
        tg_sbi::console_putchar(*c);
    }
    tg_sbi::shutdown(false)
}

/// Rust 异常处理函数，以异常方式关机。
#[cfg(target_arch = "riscv64")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    tg_sbi::shutdown(true)
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
