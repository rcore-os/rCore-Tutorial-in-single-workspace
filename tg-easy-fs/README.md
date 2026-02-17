# tg-easy-fs

A simple filesystem implementation for the rCore tutorial operating system.

## 设计目标

- 提供教学友好的简化文件系统实现（EasyFS）。
- 让章节内核在 `no_std` 环境中具备“文件 + 目录 + 管道”基础能力。
- 统一块设备抽象，便于接入 virtio-block 或镜像构建工具。

## 总体架构

- 设备层：`BlockDevice` trait。
- 缓存层：块缓存与同步机制。
- 布局层：位图、inode、目录项等磁盘格式定义。
- 接口层：
  - `EasyFileSystem`
  - `Inode`
  - `FileHandle`
  - `PipeReader` / `PipeWriter`

## 主要特征

- 块设备抽象（默认 512B block）。
- inode 风格文件系统结构。
- 块缓存 + 位图分配。
- 支持 pipe IPC。
- 可用于内核运行期与构建期镜像准备（`build.rs`）。

## 功能实现要点

- 文件系统元数据与数据块均通过块缓存统一读写。
- inode 提供目录查找、文件读写、清理等高层接口。
- 管道使用独立读写端对象，服务进程间流式通信。

## 对外接口

- trait：
  - `BlockDevice`
- 常量：
  - `BLOCK_SZ`
- 核心类型：
  - `EasyFileSystem`
  - `Inode`
  - `FileHandle`
  - `PipeReader`, `PipeWriter`
- 函数：
  - `make_pipe()`
  - `get_block_cache(...)`
  - `block_cache_sync_all()`

## 使用示例

```rust
use tg_easy_fs::{EasyFileSystem, BlockDevice};

fn open_fs(dev: alloc::sync::Arc<dyn BlockDevice>) {
    let _efs = EasyFileSystem::open(dev);
}
```

- 章节内真实用法：
  - `ch6/src/fs.rs` 中进行文件系统与文件接口调用。
  - `ch6/build.rs`、`ch7/build.rs`、`ch8/build.rs` 用于准备镜像内容。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch6` 到 `ch8`（含 `build-dependencies`）。
- 关键职责：提供文件系统、文件描述符与管道能力。
- 关键引用文件：
  - `ch6/Cargo.toml`
  - `ch6/src/fs.rs`
  - `ch6/build.rs`
  - `ch8/src/main.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
