//! # 第一章：应用程序与基本执行环境
//!
//! 本章实现了一个最简单的 RISC-V S 态裸机程序，展示操作系统的最小执行环境。
//!
//! ## 关键概念
//!
//! - `#![no_std]`：不使用 Rust 标准库，改用不依赖操作系统的核心库 `core`
//! - `#![no_main]`：不使用标准的 `main` 入口，自定义裸函数 `_start` 作为入口
//! - 裸函数（naked function）：不生成函数序言/尾声，可在无栈环境下执行
//! - SBI（Supervisor Binary Interface）：S 态软件向 M 态固件请求服务的标准接口
//!
//! 教程阅读建议：
//!
//! - 先看 `_start`：理解无运行时情况下的最小启动流程；
//! - 再看 `rust_main`：理解最小 I/O 路径（SBI 输出 + 关机）；
//! - 最后看 `panic_handler`：理解 no_std 程序的异常收口方式。

// 不使用标准库，因为裸机环境没有操作系统提供系统调用支持
#![no_std]
// 不使用标准入口，因为裸机环境没有 C runtime 进行初始化
#![no_main]
// RISC-V64 架构下启用严格警告和文档检查
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
// 非 RISC-V64 架构允许死代码（用于 cargo publish --dry-run 在主机上通过编译）
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

// 引入 SBI 调用库，提供 console_putchar（输出字符）和 shutdown（关机）功能
// 启用 nobios 特性后，tg_sbi 内建了 M-mode 启动代码，无需外部 SBI 固件
use tg_sbi::{console_putchar, shutdown};

#[cfg(target_arch = "riscv64")]
mod heap;
#[cfg(target_arch = "riscv64")]
mod lec2_lab1;
#[cfg(target_arch = "riscv64")]
mod stackwalk;
#[cfg(target_arch = "riscv64")]
mod symtab_resolve;

/// 嵌套调用以形成多帧，便于 `stackwalk` 打印动态调用关系（教学演示）。
/// 各层携带不同基本类型的参数，展示 backtrace 中的形参名显示能力。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth3(msg: &str, flag: bool) {
    // 防止参数被优化掉
    core::hint::black_box(msg);
    core::hint::black_box(flag);
    stackwalk::print_backtrace();
}

#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth2(count: u32, label: &str) {
    core::hint::black_box(count);
    bt_depth3(label, count > 0);
}

#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn bt_depth1(id: usize, name: &str, value: i64) {
    core::hint::black_box(id);
    core::hint::black_box(value);
    bt_depth2(id as u32, name);
}

/// 模拟数组越界访问——Rust 自动插入的边界检查会触发 panic。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn buggy_access(data: &[u8], index: usize) {
    core::hint::black_box(data);
    core::hint::black_box(index);
    let _val = data[index];
}

/// 错误触发入口：构造一个短数组，用越界下标访问以触发 panic。
#[cfg(target_arch = "riscv64")]
#[inline(never)]
fn trigger_error(kind: &str, n: usize) {
    core::hint::black_box(kind);
    let arr = [10u8, 20, 30];
    buggy_access(&arr, n);
}

/// S 态程序入口点。
///
/// 这是一个裸函数（naked function），放置在 `.text.entry` 段，
/// 链接脚本将其安排在地址 `0x80200000`。
///
/// 裸函数不生成函数序言和尾声，因此可以在没有栈的情况下执行。
/// 它完成两件事：
/// 1. 设置栈指针 `sp`，指向栈顶（栈从高地址向低地址增长）
/// 2. 跳转到 Rust 主函数 `rust_main`
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    // 栈大小：4 KiB
    const STACK_SIZE: usize = 4096;

    // 在 .bss.uninit 段中分配栈空间
    #[unsafe(link_section = ".bss.uninit")]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}", // 将 sp 设置为栈顶地址
        "j  {main}",                      // 跳转到 rust_main
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        main       =   sym rust_main,
    )
}

/// S 态主函数：打印 "Hello, world!" 并关机。
///
/// 通过 SBI 的 `console_putchar` 逐字节输出字符串，
/// 然后调用 `shutdown` 正常关机退出 QEMU。
extern "C" fn rust_main() -> ! {
    #[cfg(target_arch = "riscv64")]
    heap::init();
    #[cfg(target_arch = "riscv64")]
    symtab_resolve::init();

    for c in b"Hello, world!\n" {
        console_putchar(*c);
    }
    #[cfg(target_arch = "riscv64")]
    lec2_lab1::emit_all_observables();
    #[cfg(target_arch = "riscv64")]
    bt_depth1(42, "hello_os", -1);

    #[cfg(target_arch = "riscv64")]
    trigger_error("oob", 10);

    shutdown(false)
}

/// panic 处理函数。
///
/// `#![no_std]` 环境下必须自行实现。先打印 `PanicInfo`（消息 + 源码位置），
/// 再通过 backtrace 展示完整调用链，最后以异常状态关机。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
    struct W;
    impl core::fmt::Write for W {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for b in s.bytes() {
                console_putchar(b);
            }
            Ok(())
        }
    }
    let _ = write!(W, "\n[PANIC] {info}\n");
    #[cfg(target_arch = "riscv64")]
    stackwalk::print_backtrace();
    shutdown(true)
}

/// 非 RISC-V64 架构的占位模块。
///
/// 提供 `main` 等符号，使得在主机平台（如 x86_64）上也能通过编译，
/// 满足 `cargo publish --dry-run` 和 `cargo test` 的需求。
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
