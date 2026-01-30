# 第一章：应用程序与基本执行环境

本章实现了一个最简单的 RISC-V S 态裸机程序，展示操作系统的最小执行环境。

## 功能概述

- 使用 `_start` 裸函数汇编入口，初始化栈并跳转到 Rust
- 通过 SBI 调用打印 `Hello, world!`
- 调用 SBI 关机
- 在 `build.rs` 中生成链接脚本，将 `.text.entry` 放置在 `0x8020_0000`，确保被 SBI 正确引导

## 裸函数入口

本章使用 `#[naked]` 属性定义裸函数 `_start` 作为程序入口。裸函数不会生成函数序言和尾声，可以在没有栈的情况下执行：

```rust
#[unsafe(naked)]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",
        "j  {main}",
        // ...
    )
}
```

入口函数完成两件事：设置栈指针 `sp`，然后跳转到 Rust 主函数。链接脚本确保 `.text.entry` 位于 `0x8020_0000`，这是 SBI 引导后的跳转地址。

## Dependencies

| 依赖 | 说明 |
|------|------|
| `tg-sbi` | SBI 调用封装库，支持标准 RustSBI 模式和 nobios 模式 |

## Features

| Feature | 说明 |
|---------|------|
| `nobios` | 无需外部 SBI 实现，直接从 QEMU `-bios none` 模式启动 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.