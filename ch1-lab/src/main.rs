#![no_std]
#![no_main]
#![deny(warnings)]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

#[cfg(target_arch = "riscv64")]
#[macro_use]
extern crate tg_console;

#[cfg(target_arch = "riscv64")]
use tg_sbi;

// 教程阅读建议：
// 1) 先看 `_start`：理解“无运行时环境”下如何手动设栈并跳转；
// 2) 再看 `rust_main`：理解控制台初始化与日志输出链路；
// 3) 最后看 `panic`：理解 no_std 程序异常时如何终止系统。

/// Supervisor 汇编入口。
///
/// 设置栈并跳转到 Rust。
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 4096;

    #[unsafe(link_section = ".bss.uninit")]
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
#[cfg(target_arch = "riscv64")]
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
#[cfg(target_arch = "riscv64")]
struct Console;

/// 为 `Console` 实现 `console::Console` trait。
#[cfg(target_arch = "riscv64")]
impl tg_console::Console for Console {
    fn put_char(&self, c: u8) {
        tg_sbi::console_putchar(c);
    }
}

/// Rust 异常处理函数，以异常方式关机。
#[cfg(target_arch = "riscv64")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    tg_sbi::shutdown(true)
}

#[cfg(not(target_arch = "riscv64"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// 非 RISC-V64 架构的占位模块，用于 `cargo publish --dry-run` 在主机上通过编译。
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
