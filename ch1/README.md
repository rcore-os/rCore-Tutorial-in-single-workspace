# 第一章：应用程序与基本执行环境

本 crate 提供一个最简单的 RISC-V S 态裸机程序，展示操作系统的最小执行环境，功能与原 ch1 等价。

不使用opensbi，rustsbi，仅支持 -bios none 参数。

## 功能概述

- 使用 `_start` 裸函数汇编入口，初始化栈并跳转到 Rust
- 通过 SBI 调用打印 `Hello, world!`
- 调用 SBI 关机
- 在 `build.rs` 中生成链接脚本，将 `.text.entry` 放置在 `0x8020_0000`，确保被正确引导

## 默认 QEMU 启动参数

`-machine virt -nographic -bios none`

## 运行

请在 tg-ch1 目录下执行：

`cargo run`

## License

GPL Version 2.0.
