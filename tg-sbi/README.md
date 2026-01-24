# tg-sbi

[![Crates.io](https://img.shields.io/crates/v/tg-sbi.svg)](https://crates.io/crates/tg-sbi)
[![Documentation](https://docs.rs/tg-sbi/badge.svg)](https://docs.rs/tg-sbi)
[![License](https://img.shields.io/crates/l/tg-sbi.svg)](LICENSE)

SBI (Supervisor Binary Interface) 调用封装模块，为 rCore 教学操作系统提供 SBI 接口。

## 功能特性

- 提供 RISC-V SBI 调用的 Rust 封装
- 可选的内置 M-Mode SBI 实现（用于 `-bios none` 启动）
- `no_std` 环境支持

## 支持的 SBI 扩展

- Legacy 控制台 I/O（EID 0x01, 0x02）
- Timer 扩展（EID 0x54494D45）
- System Reset 扩展（EID 0x53525354）

## 使用方法

```rust
use tg_sbi::{console_putchar, set_timer, shutdown};

// 输出字符
console_putchar(b'H');

// 设置定时器中断
set_timer(1000000);

// 关闭系统
shutdown(false);
```

## Features

- `nobios`: 启用内置的 M-Mode SBI 实现。当使用 QEMU 的 `-bios none` 选项启动时，
  此 feature 提供基本的 SBI 服务，包括 UART 控制台、定时器和系统重置功能。

## 硬件要求

`nobios` feature 专为 QEMU virt 机器设计，假设以下 MMIO 地址：

- UART: `0x1000_0000`
- CLINT mtimecmp: `0x200_4000`
- QEMU virt test: `0x10_0000`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
