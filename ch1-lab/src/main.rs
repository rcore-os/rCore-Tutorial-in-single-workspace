#![no_std]
#![no_main]
#![deny(warnings)]

#[macro_use]
extern crate tg_console;

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

/// 使用 `console` 输出的 Supervisor 裸机程序。
///
/// 测试各种日志和输出后关机。
extern "C" fn rust_main() -> ! {
    // 初始化 `console`
    tg_console::init_console(&Console);
    // 设置日志级别
    tg_console::set_log_level(option_env!("LOG"));
    // 测试各种打印
    tg_console::test_log();

    tg_sbi::shutdown(false)
}

/// 将传给 `console` 的控制台对象。
///
/// 这是一个 Unit struct，它不需要空间。否则需要传一个 static 对象。
struct Console;

/// 为 `Console` 实现 `console::Console` trait。
impl tg_console::Console for Console {
    fn put_char(&self, c: u8) {
        tg_sbi::console_putchar(c);
    }
}

/// Rust 异常处理函数，以异常方式关机。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    tg_sbi::shutdown(true)
}
