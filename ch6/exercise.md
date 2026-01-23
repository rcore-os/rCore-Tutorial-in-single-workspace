# chapter6 练习

## 编程作业

### 硬链接

硬链接要求两个不同的目录项指向同一个文件，在我们的文件系统中也就是两个不同名称目录项指向同一个磁盘块。

本节要求实现三个系统调用 `linkat`、`unlinkat`、`fstat`。

**linkat**：

- syscall ID: 37
- 功能：创建一个文件的一个硬链接，[linkat 标准接口](https://linux.die.net/man/2/linkat)。

```rust
fn linkat(&self, _caller: Caller, _olddirfd: i32, oldpath: usize, _newdirfd: i32, newpath: usize, _flags: u32) -> isize
```

- 参数：
  - olddirfd, newdirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。
  - flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。
  - oldpath：原有文件路径
  - newpath: 新的链接文件路径。
- 说明：
  - 为了方便，不考虑新文件路径已经存在的情况（属于未定义行为）。除非出现新旧名字一致的情况，此时需要返回 -1。
  - 返回值：如果出现了错误则返回 -1，否则返回 0。
- 可能的错误：
  - 链接同名文件。

**unlinkat**:

- syscall ID: 35
- 功能：取消一个文件路径到文件的链接，[unlinkat 标准接口](https://linux.die.net/man/2/unlinkat)。

```rust
fn unlinkat(&self, _caller: Caller, _dirfd: i32, path: usize, _flags: u32) -> isize
```

- 参数：
  - dirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。
  - flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。
  - path：文件路径。
- 说明：
  - 注意考虑使用 unlink 彻底删除文件的情况，此时需要回收 inode 以及它对应的数据块。
- 返回值：如果出现了错误则返回 -1，否则返回 0。
- 可能的错误：
  - 文件不存在。

**fstat**:

- syscall ID: 80
- 功能：获取文件状态。

```rust
fn fstat(&self, _caller: Caller, fd: usize, st: usize) -> isize
```

- 参数：
  - fd: 文件描述符
  - st: 文件状态结构体指针

```rust
// Stat 结构体和 StatMode 结构体已在 syscall/src/fs.rs 中定义
#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
    pub dev: u64,
    /// inode 文件所在 inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为 1
    pub nlink: u32,
    /// 无需考虑，为了兼容性设计
    pad: [u64; 7],
}

bitflags! {
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}
```

### HINT

- `linkat` 和 `unlinkat` 的文件路径 path 的读取可参考 `ch6/src/main.rs` 中的 `open` 系统调用实现。
- `fstat` 的文件状态结构体 `Stat` 指针 st 的写入可参考 `ch6/src/main.rs` 中的 `clock_gettime` 系统调用对 `TimeSpec` 的写入实现。

### 实验要求

- 在 ch6 目录下完成实验。
- 目录结构说明：

```
├── ch6（内核实现）
│   ├── Cargo.toml（配置文件）
│   └── src（内核源代码）
│       ├── main.rs（内核主函数，包括系统调用接口实现）
│       ├── fs.rs（文件系统相关）
│       ├── process.rs（进程结构）
│       ├── processor.rs（进程管理器）
│       └── virtio_block.rs（VirtIO 块设备实现）
├── easy-fs（文件系统实现，需要修改以支持硬链接）
├── syscall（系统调用模块）
│   └── src
│       ├── kernel/mod.rs（IO trait 定义，无需修改）
│       └── fs.rs（Stat 结构体和 StatMode 结构体定义，无需修改）
├── user（用户程序）
│   └── src/bin（测试用例，无需修改）
├── ...
```

- 测试方法：
```bash
cargo qemu --ch 6 --exercise
```
然后在终端中输入 `ch6_usertest` 运行，这个测例打包了所有你需要通过的测例。

### 测试说明

- 你的内核必须前向兼容，需要能通过前一章的所有测例
