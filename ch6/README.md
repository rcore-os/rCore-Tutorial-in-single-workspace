# 第六章：文件系统

本章实现了文件系统支持，使用 easy-fs 文件系统和 virtio 块设备驱动，用户程序从磁盘镜像加载。

## 功能概述

- easy-fs 简单文件系统
- virtio-blk 块设备驱动
- 进程文件描述符表管理，支持标准输入输出和普通文件
- 标准文件操作接口

## 快速开始

在 tg-ch6 目录下执行：

```bash
cargo run                      # 基础模式
cargo run --features exercise  # 练习模式
```

> 默认会在 tg-ch6 目录下创建 tg-user 源码目录（通过 `cargo clone`）。
> 默认拉取版本为 `0.2.0-preview.1`，可通过环境变量 `TG_USER_VERSION` 覆盖。
> 若已有本地 tg-user，可通过 `TG_USER_DIR` 指定路径。

### 测试

```bash
./test.sh  # 全部测试，等价于 ./test.sh all
./test.sh base  # 基础测试
./test.sh exercise  # 练习测试
```

## 用户程序加载

tg-ch6 在构建阶段会拉取 tg-user 并编译用户程序，然后将编译产物打包到 easy-fs 磁盘镜像 `fs.img` 中。运行时 QEMU 挂载该磁盘镜像，内核通过 virtio-blk 驱动访问文件系统，按文件名加载并执行用户程序。

## 默认 QEMU 启动参数

```text
-machine virt -nographic -bios none\
-drive file=target/riscv64gc-unknown-none-elf/debug/fs.img,if=none,format=raw,id=x0\
-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
```

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

## 新增或更新的系统调用

| 系统调用 | 功能 |
|----------|------|
| `open` | 打开文件 |
| `close` | 关闭文件 |
| `read` | 读取文件或标准输入 |
| `write` | 写入文件或标准输出 |

## 依赖与配置

### Features

| Feature | 说明 |
|---------|------|
| `exercise` | 练习模式测例 |

### Dependencies

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

## 练习

见 [Exercise](./exercise.md)

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
