# 第一章：应用程序与基本执行环境

本章实现了一个最简单的 RISC-V S 态裸机程序，展示操作系统的最小执行环境。

不使用opensbi，rustsbi，仅支持 -bios none 参数。

## 功能概述

- 使用 `_start` 裸函数汇编入口，初始化栈并跳转到 Rust
- 通过 SBI 调用打印 `Hello, world!`
- 调用 SBI 关机
- 在 `build.rs` 中生成链接脚本，将 `.text.entry` 放置在 `0x8020_0000`，确保被正确引导

## 快速开始

请在 tg-ch1 目录下执行：

```bash
cargo run
```

## 默认 QEMU 启动参数

`-machine virt -nographic -bios none`

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
| `tg-sbi` | SBI 调用封装库，支持 nobios 模式 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
