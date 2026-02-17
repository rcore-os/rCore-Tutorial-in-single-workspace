# tg-kernel-vm

Kernel virtual memory management for the rCore tutorial operating system.

## 设计目标

- 提供教学内核可复用的虚拟内存管理抽象（以 Sv39 为主）。
- 将“页表遍历/映射细节”与“章节策略逻辑”解耦。
- 支持按章节逐步扩展：从基础映射到进程地址空间复制。

## 总体架构

- `PageManager<Meta>` trait：
  - 约束底层物理页管理能力（页分配、根页表访问、地址转换等）。
- `AddressSpace<Meta, M>`：
  - 在 `PageManager` 之上提供高层地址空间操作。
- `page_table` 导出：
  - 复用底层页表结构、标志与地址类型。

## 主要特征

- `AddressSpace` 统一管理映射/解除映射/翻译。
- 可按标志控制读写执行与用户态访问权限。
- 提供地址空间复制能力，支持 `fork` 等场景。
- 适配 `no_std`、裸机内核。

## 功能实现要点

- 将不同章节中的物理页来源统一抽象为 `PageManager`。
- `AddressSpace` 处理页面粒度映射与权限标志组合。
- 通过类型参数 `Meta` 保留架构相关元数据扩展点。

## 对外接口

- trait：
  - `PageManager<Meta>`
- 结构体：
  - `AddressSpace<Meta, M>`
- 常见方法（`AddressSpace`）：
  - `new()`
  - `root_ppn()`
  - `map(...)`
  - `unmap(...)`
  - `translate(...)`
  - `cloneself(...)`
- 模块导出：
  - `page_table`

## 使用示例

```rust
use tg_kernel_vm::{AddressSpace, page_table::Sv39};

let _space = AddressSpace::<Sv39, MyPageManager>::new();
```

- 章节内真实用法：
  - `ch4/src/main.rs` 构建内核地址空间与映射。
  - `ch4/src/process.rs`、`ch5/src/process.rs` 管理进程用户地址空间。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch4` 到 `ch8`。
- 关键职责：承接页表管理、地址翻译、进程地址空间隔离。
- 关键引用文件：
  - `ch4/Cargo.toml`
  - `ch4/src/main.rs`
  - `ch5/src/process.rs`
  - `ch8/src/process.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
