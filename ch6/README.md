# 第六章：文件系统

本章实现了文件系统支持，使用 easy-fs 文件系统和 virtio 块设备驱动，用户程序从磁盘镜像加载。

## 功能概述

- easy-fs 简单文件系统
- virtio-blk 块设备驱动
- 进程文件描述符表管理，支持标准输入输出和普通文件
- 标准文件操作接口

## 用户程序加载

用户程序存储在 easy-fs 磁盘镜像中，内核启动时挂载文件系统，通过文件名从文件系统加载。

## 新增或更新的系统调用

| 系统调用 | 功能 |
|----------|------|
| `open` | 打开文件 |
| `close` | 关闭文件 |
| `read` | 读取文件或标准输入 |
| `write` | 写入文件或标准输出 |

## 文件描述符表

每个进程维护一个文件描述符表 `fd_table`，统一管理标准输入输出和普通文件：

```rust
pub struct Process {
    pub fd_table: Vec<Option<Mutex<FileHandle>>>,
    // ...
}
```

打开文件时分配新的文件描述符，读写时通过 fd 查找对应的 `FileHandle`：

```rust
fn open(&self, path: usize, flags: usize) -> isize {
    if let Some(file) = FS.open(path_str, flags) {
        let new_fd = current.fd_table.len();
        current.fd_table.push(Some(Mutex::new(file)));
        new_fd as isize
    } else { -1 }
}
```

标准输入(0)、标准输出(1)、标准错误(2) 是预留的特殊 fd，直接通过 SBI 控制台操作。

## 关键依赖：tg-easy-fs

`tg-easy-fs` 是一个简单的文件系统实现。

- **磁盘布局**：

    ```text
    +------------+--------------+------------+-------------+-----------+
    | SuperBlock | Inode Bitmap | Inode Area | Data Bitmap | Data Area |
    +------------+--------------+------------+-------------+-----------+
    ```
- **FSManager trait**：文件系统管理接口
  ```rust
  pub trait FSManager {
      fn open(&self, path: &str, flags: OpenFlags) -> Option<Arc<FileHandle>>;
      fn find(&self, path: &str) -> Option<Arc<Inode>>;
      fn link(&self, src: &str, dst: &str) -> isize;
      fn unlink(&self, path: &str) -> isize;
      fn readdir(&self, path: &str) -> Option<Vec<String>>;
  }
  ```
- **FileHandle**：文件句柄，封装 `Inode` 和读写偏移，支持 `read`/`write` 操作
- **UserBuffer**：用户缓冲区抽象，处理跨页的用户空间数据

内核通过实现 `BlockDevice` trait 将 virtio-blk 与文件系统对接：

```rust
pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
```

## Exercise

见 [Exercise](./exercise.md)

## Dependencies

| 依赖 | 说明 |
|------|------|
| `virtio-drivers` | virtio 块设备驱动 |
| `xmas-elf` | ELF 文件解析 |
| `riscv` | RISC-V CSR 寄存器访问 |
| `tg-sbi` | SBI 调用封装库 |
| `tg-linker` | 链接脚本生成、内核布局定位 |
| `tg-console` | 控制台输出 (`print!`/`println!`) 和日志 |
| `tg-kernel-context` | 用户上下文及异界传送门（启用 `foreign` feature） |
| `tg-kernel-alloc` | 内核内存分配器 |
| `tg-kernel-vm` | 虚拟内存管理 |
| `tg-syscall` | 系统调用定义与分发 |
| `tg-task-manage` | 进程管理框架（启用 `proc` feature） |
| `tg-easy-fs` | 简单文件系统 |

## Features

| Feature | 说明 |
|---------|------|
| `nobios` | 无需外部 SBI 实现，直接从 QEMU `-bios none` 模式启动 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
