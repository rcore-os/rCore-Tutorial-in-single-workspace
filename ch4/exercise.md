# chapter4 练习

## 编程作业

### 重写 trace 系统调用

引入虚存机制后，原来内核的 `trace` 函数实现就无效了。**请你重写这个系统调用的代码**，恢复其正常功能。

由于本章我们有了地址空间作为隔离机制，所以 `trace` **需要考虑一些额外的情况**：

- 在读取（`trace_request` 为 0）时，如果对应地址用户不可见或不可读，则返回值应为 -1（`isize` 格式的 -1，而非 `u8`）。
- 在写入（`trace_request` 为 1）时，如果对应地址用户不可见或不可写，则返回值应为 -1。

### mmap 和 munmap 匿名映射

[mmap](https://man7.org/linux/man-pages/man2/mmap.2.html) 在 Linux 中主要用于在内存中映射文件，本次实验简化它的功能，仅用于申请内存。

请实现 mmap 和 munmap 系统调用，mmap 定义如下：

```rust
fn mmap(&self, _caller: Caller, addr: usize, len: usize, prot: i32, _flags: i32, _fd: i32, _offset: usize) -> isize
```

- syscall ID：222
- 申请长度为 len 字节的物理内存（不要求实际物理内存位置，可以随便找一块），将其映射到 addr 开始的虚存，内存页属性为 prot
- 参数：
  - addr 需要映射的虚存起始地址，要求按页对齐
  - len 映射字节长度，可以为 0
  - prot：第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
  - _flags、_fd、_offset 在本实验中忽略
- 返回值：执行成功则返回 0，错误返回 -1
- 说明：
  - 为了简单，目标虚存区间要求按页对齐，len 可直接按页向上取整，不考虑分配失败时的页回收。
- 可能的错误：
  - addr 没有按页大小对齐
  - prot & !0x7 != 0 (prot 其余位必须为 0)
  - prot & 0x7 = 0 (这样的内存无意义)
  - [addr, addr + len) 中存在已经被映射的页
  - 物理内存不足

munmap 定义如下：

```rust
fn munmap(&self, _caller: Caller, addr: usize, len: usize) -> isize
```

- syscall ID：215
- 取消 [addr, addr + len) 虚存的映射
- 参数和返回值请参考 mmap
- 说明：
  - 为了简单，参数错误时不考虑内存的恢复和回收。
- 可能的错误：
  - [addr, addr + len) 中存在未被映射的虚存。

### HINT

- 页表项权限标志使用 `VmFlags::build_from_str()` 构建，格式如 `"U_WRV"` 表示用户态可写可读有效
- 一定要注意 mmap 的页表项权限，注意 RISC-V 页表项的格式与 prot 参数的区别
- 你添加 `U`（用户态可访问）标志了吗？
- 实现 `trace` 时，可参考 `ch4/src/main.rs` 中 `clock_gettime` 的实现方式，使用 `translate` 方法进行地址转换和权限检查

### 实验要求

- 在 ch4 目录下完成实验。
- 目录结构说明：

```
├── ch4（内核实现）
│   ├── Cargo.toml（配置文件）
│   └── src（内核源代码）
│       ├── main.rs（内核主函数，包括系统调用接口实现）
│       └── process.rs（进程结构）
├── tg-kernel-vm（虚拟内存模块）
│   └── src
│       ├── lib.rs（PageManager trait 定义）
│       └── space/mod.rs（AddressSpace 实现）
├── tg-syscall（系统调用模块）
│   └── src
│       └── kernel/mod.rs（Memory trait 定义 mmap/munmap 接口，无需修改）
├── user（用户程序）
│   └── src/bin（测试用例，无需修改）
├── ...
```

- 测试方法：
```bash
cargo qemu --ch 4 --exercise
```
