# tg-kernel-alloc

[![Crates.io](https://img.shields.io/crates/v/tg-kernel-alloc.svg)](https://crates.io/crates/tg-kernel-alloc)
[![Documentation](https://docs.rs/tg-kernel-alloc/badge.svg)](https://docs.rs/tg-kernel-alloc)
[![License](https://img.shields.io/crates/l/tg-kernel-alloc.svg)](LICENSE)

内核内存分配器模块，为 rCore 教学操作系统提供基于 buddy 算法的 `#[global_allocator]` 实现。

## 设计目标

- 在 `no_std` 内核下提供可用的动态内存分配能力。
- 与章节内核启动流程解耦：先初始化，再按区域逐步“喂内存”。
- 保持实现简洁，适合教学中讲解内核堆分配基本机制。

## 总体架构

- 全局分配器：对接 Rust `GlobalAlloc`。
- buddy 分配内核：负责块拆分/合并与分配策略。
- 初始化接口：
  - `init(base_address)`：设置堆管理器初始基址。
  - `transfer(region)`：向分配器移交可管理内存区域。

## 主要特征

- 提供 `#[global_allocator]` 支撑 `alloc` 生态。
- 使用 buddy 算法，兼顾实现复杂度与碎片控制。
- 可多次 `transfer` 增量扩展可用堆区。
- 适配 `no_std` / 裸机内核环境。

## 功能实现要点

- 启动阶段常由内核先调用 `init`，再移交连续可用区域。
- 与页分配器职责分离：此 crate 关注通用堆对象分配。
- 依赖内核地址空间可直接访问被移交的内存范围。

## 对外接口

- 函数：
  - `init(base_address: usize)`
  - `unsafe transfer(region: &'static mut [u8])`
- 全局行为：
  - 提供 `GlobalAlloc` 实现（供 `Vec`/`Box` 等使用）

## 使用示例

```rust
// kernel init
tg_kernel_alloc::init(heap_base);
unsafe {
    tg_kernel_alloc::transfer(heap_region);
}
```

- 章节内真实用法：
  - `ch4/src/main.rs` 初始化内核堆。
  - `ch5/src/main.rs` 到 `ch8/src/main.rs` 持续复用该分配能力。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch4` 到 `ch8`。
- 关键职责：支撑进程控制块、文件系统对象、同步对象等堆上结构。
- 关键引用文件：
  - `ch4/Cargo.toml`
  - `ch4/src/main.rs`
  - `ch8/src/main.rs`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
