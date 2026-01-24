#![no_std]
#![no_main]
#![deny(warnings)]

use tg_sbi;

/// Supervisor 汇编入口。
///
/// 设置栈并跳转到 Rust。
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
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    tg_sbi::shutdown(true)
}
