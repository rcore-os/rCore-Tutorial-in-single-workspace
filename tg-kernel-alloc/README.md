# tg-kernel-alloc

[![Crates.io](https://img.shields.io/crates/v/tg-kernel-alloc.svg)](https://crates.io/crates/tg-kernel-alloc)
[![Documentation](https://docs.rs/tg-kernel-alloc/badge.svg)](https://docs.rs/tg-kernel-alloc)
[![License](https://img.shields.io/crates/l/tg-kernel-alloc.svg)](LICENSE)

内核内存分配器模块，为 rCore 教学操作系统提供基于 buddy 算法的 `#[global_allocator]` 实现。

## 功能特性

- 提供 `#[global_allocator]` 实现
- 使用 buddy 算法进行内存分配
- 支持 `no_std` 环境

## 设计说明

内核不必区分虚存分配和物理页分配的条件是**虚地址空间覆盖物理地址空间**，换句话说，内核能直接访问到所有物理内存而无需执行修改页表之类其他操作。

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
