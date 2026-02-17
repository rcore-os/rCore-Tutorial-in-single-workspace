# tg-console

[![Crates.io](https://img.shields.io/crates/v/tg-console.svg)](https://crates.io/crates/tg-console)
[![Documentation](https://docs.rs/tg-console/badge.svg)](https://docs.rs/tg-console)
[![License](https://img.shields.io/crates/l/tg-console.svg)](LICENSE)

控制台输出模块，为 rCore 教学操作系统提供可定制的 `print!`、`println!` 与日志能力。

## 设计目标

- 提供 `no_std` 环境可用的统一打印接口。
- 将“字符输出设备”与“格式化/日志逻辑”解耦。
- 让章节内核无需重复实现控制台与日志框架。

## 总体架构

- `Console` trait：抽象底层字符输出设备（通常由 `tg-sbi::console_putchar` 支撑）。
- 全局控制台注册：`init_console(&'static dyn Console)`。
- 打印宏：`print!`、`println!` 最终走 `_print(...)`。
- 日志集成：导出 `log` 并提供日志级别初始化。

## 主要特征

- 提供 `print!` / `println!` 宏。
- 支持 `log` 生态并可设置日志级别。
- 支持基础彩色日志输出（按级别区分）。
- 完整 `no_std` 支持。

## 功能实现要点

- 使用全局单例保存控制台实现，避免多处重复初始化。
- 格式化输出通过 `core::fmt`，不依赖标准库 I/O。
- 日志能力通过 `log::Log` 集成，章节内核只需注册一次。

## 对外接口

- trait：
  - `Console`（核心方法 `put_char`）
- 函数：
  - `init_console(&'static dyn Console)`
  - `set_log_level(Option<&str>)`
  - `test_log()`
  - `_print(core::fmt::Arguments)`
- 宏：
  - `print!`
  - `println!`
- 模块导出：
  - `log`

## 使用示例

```rust
#[macro_use]
extern crate tg_console;

struct ConsoleImpl;
impl tg_console::Console for ConsoleImpl {
    fn put_char(&self, c: u8) {
        tg_sbi::console_putchar(c);
    }
}

tg_console::init_console(&ConsoleImpl);
tg_console::set_log_level(Some("info"));
println!("hello from kernel");
```

- 章节内真实用法：
  - `ch2/src/main.rs` 中初始化控制台与日志。
  - `ch3/src/main.rs` 及之后章节统一使用该接口输出。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch2` 到 `ch8`。
- 关键职责：提供教学内核的统一打印与日志能力。
- 关键引用文件：
  - `ch2/Cargo.toml`
  - `ch2/src/main.rs`
  - `ch8/src/main.rs`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
