# tg-console

[![Crates.io](https://img.shields.io/crates/v/tg-console.svg)](https://crates.io/crates/tg-console)
[![Documentation](https://docs.rs/tg-console/badge.svg)](https://docs.rs/tg-console)
[![License](https://img.shields.io/crates/l/tg-console.svg)](LICENSE)

控制台输出模块，为 rCore 教学操作系统提供可定制实现的 `print!`、`println!` 和 `log::Log`。

## 功能特性

- 提供 `print!` 和 `println!` 宏
- 实现 `log::Log` trait，支持日志功能
- 支持基本的彩色输出
- `no_std` 环境支持

## 使用方法

```rust
use tg_console::{print, println};

println!("Hello, rCore!");
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
