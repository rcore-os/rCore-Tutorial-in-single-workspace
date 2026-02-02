# tg-linker

[![Crates.io](https://img.shields.io/crates/v/tg-linker.svg)](https://crates.io/crates/tg-linker)
[![Documentation](https://docs.rs/tg-linker/badge.svg)](https://docs.rs/tg-linker)
[![License](https://img.shields.io/crates/l/tg-linker.svg)](LICENSE)

链接脚本生成工具模块，为 rCore 教学操作系统内核提供链接脚本生成功能。

## 功能特性

- 完全控制内核链接脚本的结构
- 所有链接脚本上定义的符号都封装在此模块内
- 内核二进制模块可以基于标准纯 Rust 语法来使用模块
- 无需手写链接脚本或记住 `extern "C"` 声明

## 设计说明

此模块将链接脚本的复杂性封装起来，提供干净的 Rust API，使得内核开发者无需直接处理链接器细节。

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
