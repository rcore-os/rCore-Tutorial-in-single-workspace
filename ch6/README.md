# 第六章：文件系统

本章在第五章"进程管理"的基础上，引入了 **文件系统** 支持。用户程序不再嵌入内核镜像，而是存放在 **磁盘镜像**（fs.img）中，内核通过 **VirtIO 块设备驱动** 和 **easy-fs 文件系统** 按名称加载和执行程序。同时，进程拥有了**文件描述符表**，可以通过 `open`/`close`/`read`/`write` 等标准接口操作文件。

通过本章的学习和实践，你将理解：

- 什么是文件系统，为什么需要文件系统
- easy-fs 的五层架构（块设备 → 块缓存 → 磁盘数据结构 → 磁盘管理器 → Inode）
- 磁盘布局：SuperBlock、Inode Bitmap、Inode Area、Data Bitmap、Data Area
- 文件描述符表和文件句柄的设计
- VirtIO 块设备驱动的工作原理
- open/close/read/write 系统调用的实现
- 硬链接的概念和实现（练习题）

> **前置知识**：建议先完成第一章至第五章的学习，理解裸机启动、Trap 处理、系统调用、多任务调度、虚拟内存和进程管理。

## 项目结构

```
ch6/
├── .cargo/
│   └── config.toml     # Cargo 配置：交叉编译目标和 QEMU runner（含块设备参数）
├── .gitignore           # Git 忽略规则
├── build.rs            # 构建脚本：编译用户程序，打包 easy-fs 磁盘镜像
├── Cargo.toml          # 项目配置与依赖
├── LICENSE             # GPL v3 许可证
├── README.md           # 本文档
├── rust-toolchain.toml # Rust 工具链配置
├── test.sh             # 自动测试脚本
└── src/
    ├── main.rs         # 内核主体：初始化、调度循环、系统调用实现
    ├── fs.rs           # 文件系统管理：easy-fs 封装
    ├── process.rs      # 进程结构：含文件描述符表
    ├── processor.rs    # 处理器管理：进程管理器
    └── virtio_block.rs # VirtIO 块设备驱动
```

## 一、环境准备

### 1.1 安装 Rust 工具链

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

验证：

```bash
rustc --version    # 要求 >= 1.85.0（支持 edition 2024）
cargo --version
```

### 1.2 添加 RISC-V 64 编译目标

```bash
rustup target add riscv64gc-unknown-none-elf
```

### 1.3 安装 QEMU 模拟器

**Ubuntu / Debian：**

```bash
sudo apt update
sudo apt install qemu-system-misc
```

**macOS（Homebrew）：**

```bash
brew install qemu
```

验证：

```bash
qemu-system-riscv64 --version    # 建议 >= 7.0
```

### 1.4 安装额外工具

```bash
cargo install cargo-clone
cargo install cargo-binutils
rustup component add llvm-tools
```

### 1.5 获取源代码

**方式一：只获取本实验**

```bash
cargo clone tg-ch6
cd tg-ch6
```

**方式二：获取所有实验**

```bash
git clone https://github.com/rcore-os/rCore-Tutorial-in-single-workspace.git
cd rCore-Tutorial-in-single-workspace/ch6
```

## 二、编译与运行

### 2.1 编译

```bash
cargo build
```

编译过程与前几章类似，但 `build.rs` 有重要变化：
1. 下载并编译 `tg-user` 用户程序
2. **不再**将用户程序嵌入内核镜像，而是打包到 **easy-fs 磁盘镜像** `fs.img` 中

> 环境变量说明：
> - `TG_USER_DIR`：指定本地 tg-user 源码路径
> - `TG_USER_VERSION`：指定 tg-user 版本（默认 `0.2.0-preview.1`）
> - `TG_SKIP_USER_APPS`：跳过用户程序编译
> - `LOG`：设置日志级别

### 2.2 运行

**基础模式：**

```bash
cargo run
```

**练习模式：**

```bash
cargo run --features exercise
```

实际执行的 QEMU 命令等价于：

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -drive file=target/riscv64gc-unknown-none-elf/debug/fs.img,if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch6
```

注意与第五章不同：QEMU 命令中多了 `-drive` 和 `-device` 参数，用于挂载 `fs.img` 磁盘镜像作为 VirtIO 块设备。

### 2.3 预期输出

```
[tg-ch6 ...] Hello, world!
[ INFO] .text    ---> 0x80200000..0x8023xxxx
[ INFO] .rodata  ---> 0x8023xxxx..0x8024xxxx
[ INFO] .data    ---> 0x8024xxxx..0x81exxxxx
[ INFO] .boot    ---> 0x81exxxxx..0x81exxxxx
[ INFO] (heap)   ---> 0x81exxxxx..0x83200000
[ INFO] MMIO range -> 0x10001000..0x10002000

Rust user shell
>> ch5b_forktest_simple
...
Shell: Process 2 exited with code 0
>> 
```

与第五章不同，你会看到：
- MMIO 地址范围的映射信息（VirtIO 块设备）
- 用户程序从磁盘镜像（而非内核内嵌）加载和执行
- Shell 交互功能与第五章相同

### 2.4 运行测试

```bash
./test.sh           # 运行全部测试（基础 + 练习）
./test.sh base      # 仅运行基础测试
./test.sh exercise  # 仅运行练习测试
```

---

## 三、操作系统核心概念

### 3.1 为什么需要文件系统？

在前几章中，用户程序直接嵌入内核镜像（通过 `APP_ASM` 或 `APPS` 表）。这存在明显的局限性：

| 问题 | 说明 |
|------|------|
| **耦合性** | 程序与内核绑定，修改用户程序需要重新编译内核 |
| **灵活性** | 无法在运行时动态创建、修改、删除文件 |
| **持久性** | 数据仅存在于内存中，关机后丢失 |
| **标准化** | 没有统一的文件操作接口（open/read/write/close） |

**文件系统** 通过在磁盘上组织数据，解决了这些问题：
- 程序和数据以文件形式存储在磁盘上
- 内核通过文件系统接口访问磁盘
- 提供标准的文件操作 API
- 数据在重启后仍然存在

### 3.2 easy-fs 文件系统架构

easy-fs 是一个简化的类 UNIX inode 文件系统，采用五层架构：

```
┌─────────────────────────────────┐
│  第 5 层：Inode（虚拟文件系统）   │  文件/目录操作接口
│  find / create / read / write    │
├─────────────────────────────────┤
│  第 4 层：磁盘管理器              │  文件系统全局管理
│  EasyFileSystem                  │  inode/数据块分配
├─────────────────────────────────┤
│  第 3 层：磁盘数据结构            │  SuperBlock / DiskInode
│  Bitmap / DirEntry               │  DiskInode 索引结构
├─────────────────────────────────┤
│  第 2 层：块缓存                  │  BlockCache + CacheManager
│  缓存磁盘块到内存                 │  自动回写脏块
├─────────────────────────────────┤
│  第 1 层：块设备接口              │  BlockDevice trait
│  read_block / write_block        │  由 VirtIO 驱动实现
└─────────────────────────────────┘
```

### 3.3 磁盘布局

easy-fs 将磁盘划分为五个区域：

```
+------------+--------------+------------+-------------+-----------+
| SuperBlock | Inode Bitmap | Inode Area | Data Bitmap | Data Area |
+------------+--------------+------------+-------------+-----------+
   1 块        若干块          若干块        若干块         若干块
```

| 区域 | 作用 |
|------|------|
| **SuperBlock** | 文件系统元信息（魔数、总块数、各区域大小） |
| **Inode Bitmap** | inode 分配位图，每 bit 对应一个 inode |
| **Inode Area** | 存储 DiskInode（文件元数据：大小、类型、数据块索引） |
| **Data Bitmap** | 数据块分配位图 |
| **Data Area** | 存储文件实际数据和目录项 |

**DiskInode 索引结构（支持大文件）：**

```
DiskInode（128 字节）
├── 28 个直接索引块（每块 512 字节 → 14 KiB）
├── 1 个一级间接索引（128 个块 → 64 KiB）
└── 1 个二级间接索引（128 × 128 个块 → 8 MiB）
最大文件大小 ≈ 8 MiB + 64 KiB + 14 KiB
```

**目录项（DirEntry，32 字节）：**

```
┌──────────────────┬──────────┐
│  文件名（28 字节） │ inode 号  │
│  含 '\0' 终止符    │  4 字节   │
└──────────────────┴──────────┘
每个 512 字节磁盘块可存储 16 个目录项
```

### 3.4 块缓存层

为了减少磁盘 I/O，easy-fs 使用**块缓存**（BlockCache）：

```
程序读写请求
     │
     ▼
查找块缓存 ──命中──→ 直接读写内存缓冲区
     │
   未命中
     │
     ▼
从磁盘读取块到缓存 ──→ 读写内存缓冲区
     │
  缓存满时
     │
     ▼
淘汰最早的缓存块（若脏则回写磁盘）
```

块缓存的关键设计：
- 每个缓存项包含 512 字节缓冲区 + 修改标记
- 全局缓存管理器限制最大缓存数量
- FIFO 淘汰策略
- Drop 时自动回写脏块

### 3.5 VirtIO 块设备驱动

VirtIO 是 QEMU 使用的虚拟化 I/O 标准。在 tg-ch6 中：

```
QEMU 宿主机
┌──────────────────────────────────┐
│  fs.img ──→ VirtIO 后端          │
│              ▲                    │
│              │ MMIO（0x10001000） │
│              ▼                    │
│  VirtIO 前端（Guest 内核驱动）     │
└──────────────────────────────────┘
```

内核需要：
1. 在地址空间中映射 MMIO 地址 `0x10001000`
2. 实现 `Hal` trait（DMA 内存分配、地址转换）
3. 实现 `BlockDevice` trait，将 read_block/write_block 转发给 VirtIO 驱动

### 3.6 文件描述符表

本章为进程引入了**文件描述符表**（fd_table）：

```rust
pub struct Process {
    pub fd_table: Vec<Option<Mutex<FileHandle>>>,
    // ... 其他字段
}
```

预留的标准文件描述符：

| fd | 名称 | 说明 |
|----|------|------|
| 0 | stdin | 标准输入（SBI console_getchar） |
| 1 | stdout | 标准输出（SBI console_putchar） |
| 2 | stderr | 标准错误（同 stdout） |
| 3+ | 普通文件 | 通过 open 系统调用分配 |

**文件操作流程：**

```
用户程序                    内核
   │                         │
   │── open("test.txt") ───→ │ 1. 地址翻译读取文件名
   │                         │ 2. easy-fs 查找/创建文件
   │                         │ 3. 分配 fd，插入 fd_table
   │←── 返回 fd = 3 ────────│
   │                         │
   │── write(3, buf, len) ──→│ 1. 查找 fd_table[3]
   │                         │ 2. 地址翻译获取用户缓冲区
   │                         │ 3. 通过 FileHandle 写入文件
   │←── 返回写入字节数 ──────│
   │                         │
   │── close(3) ────────────→│ 1. fd_table[3] = None
   │←── 返回 0 ─────────────│
```

### 3.7 系统调用

| syscall ID | 名称 | 功能 |
|-----------|------|------|
| 56 | `open` | 打开文件（**新增**） |
| 57 | `close` | 关闭文件描述符（**新增**） |
| 63 | `read` | 读取文件或标准输入（**扩展**：支持文件 fd） |
| 64 | `write` | 写入文件或标准输出（**扩展**：支持文件 fd） |
| 93 | `exit` | 退出进程 |
| 124 | `sched_yield` | 让出 CPU |
| 113 | `clock_gettime` | 获取时间 |
| 172 | `getpid` | 获取 PID |
| 214 | `sbrk` | 调整堆 |
| 220 | `fork` | 创建子进程（**扩展**：复制 fd_table） |
| 221 | `exec` | 替换程序（**变化**：从文件系统加载） |
| 260 | `wait` | 等待子进程 |
| 37 | `linkat` | 创建硬链接（**练习题**） |
| 35 | `unlinkat` | 删除链接（**练习题**） |
| 80 | `fstat` | 获取文件状态（**练习题**） |

### 3.8 从内嵌程序到文件系统加载

对比第五章和第六章的程序加载方式：

```
第五章：程序嵌入内核
────────────────────
build.rs 编译用户程序 → 生成 APP_ASM → 嵌入内核镜像
exec 时：APPS.get(name) → 内存中的 ELF 数据

第六章：程序存储在文件系统
────────────────────────
build.rs 编译用户程序 → 打包到 fs.img → QEMU 挂载为块设备
exec 时：FS.open(name) → read_all() → 从磁盘读取 ELF 数据
```

---

## 四、代码解读

### 4.1 `src/main.rs` —— 内核主体

**启动流程（与第五章类似，新增 MMIO 映射）：**
1. 清零 BSS 段 → 初始化控制台 → 初始化堆
2. 创建异界传送门 → 建立内核地址空间
3. **新增**：映射 VirtIO MMIO 地址 `0x10001000`
4. 初始化系统调用 → 从**文件系统**加载 initproc → 进入调度循环

**IO 系统调用的变化：**
- `write`/`read`：先检查是否为标准 I/O fd，否则通过 fd_table 查找文件句柄读写
- `open`：从用户空间读取文件路径字符串 → easy-fs 打开文件 → 分配 fd
- `close`：将 fd_table 对应项设为 None
- `exec`：从 `FS.open(name)` + `read_all()` 加载 ELF，而非 `APPS.get(name)`

### 4.2 `src/fs.rs` —— 文件系统管理

**`FS`**：全局文件系统实例，通过 `BLOCK_DEVICE`（VirtIO）打开 easy-fs

**`FileSystem` 实现 `FSManager` trait：**
- `open()`：支持 CREATE（创建）、TRUNC（清空）等标志
- `find()`：在根目录中查找文件
- `readdir()`：列出所有文件名
- `read_all()`：辅助函数，读取整个文件内容

### 4.3 `src/process.rs` —— 进程管理

与第五章相比新增 `fd_table` 字段：
- `from_elf()`：初始化时预留 fd 0/1/2
- `fork()`：深拷贝父进程的 fd_table（子进程继承已打开文件）
- `exec()`：保留 fd_table 不变

### 4.4 `src/virtio_block.rs` —— VirtIO 驱动

- `BLOCK_DEVICE`：全局块设备实例
- `VirtIOBlock`：封装 virtio-drivers 库的 VirtIOBlk
- `VirtioHal`：DMA 内存分配和地址转换（恒等映射下很简单）

### 4.5 `Cargo.toml` —— 依赖说明

| 依赖 | 说明 |
|------|------|
| `virtio-drivers` | VirtIO 设备驱动库（**本章新增**） |
| `tg-easy-fs` | easy-fs 文件系统实现（**本章新增**） |
| `xmas-elf` | ELF 文件解析 |
| `riscv` | RISC-V CSR 寄存器访问 |
| `spin` | 自旋锁（Lazy、Mutex） |
| `tg-sbi` | SBI 调用封装 |
| `tg-linker` | 链接脚本和内核布局 |
| `tg-console` | 控制台输出和日志 |
| `tg-kernel-context` | 用户上下文及异界传送门 |
| `tg-kernel-alloc` | 内核堆分配器 |
| `tg-kernel-vm` | 虚拟内存管理 |
| `tg-syscall` | 系统调用定义与分发 |
| `tg-task-manage` | 进程管理框架 |

---

## 五、编程练习

### 5.1 硬链接

硬链接要求两个不同的目录项指向同一个文件，在我们的文件系统中也就是两个不同名称目录项指向同一个磁盘块。

本节要求实现三个系统调用 `linkat`、`unlinkat`、`fstat`。

#### linkat

- syscall ID: 37
- 功能：创建一个文件的硬链接（[linkat 标准接口](https://linux.die.net/man/2/linkat)）

```rust
fn linkat(&self, _caller: Caller, _olddirfd: i32, oldpath: usize,
          _newdirfd: i32, newpath: usize, _flags: u32) -> isize
```

- 参数：
  - olddirfd, newdirfd: 仅为兼容性考虑，始终为 AT_FDCWD (-100)，可忽略
  - flags: 仅为兼容性考虑，始终为 0，可忽略
  - oldpath：原有文件路径
  - newpath: 新的链接文件路径
- 说明：
  - 不考虑新文件路径已存在的情况（属于未定义行为）
  - 新旧名字一致时返回 -1
- 返回值：成功 0，错误 -1

#### unlinkat

- syscall ID: 35
- 功能：取消一个文件路径到文件的链接（[unlinkat 标准接口](https://linux.die.net/man/2/unlinkat)）

```rust
fn unlinkat(&self, _caller: Caller, _dirfd: i32, path: usize, _flags: u32) -> isize
```

- 参数：
  - dirfd: 始终为 AT_FDCWD (-100)，可忽略
  - flags: 始终为 0，可忽略
  - path：文件路径
- 说明：使用 unlink 彻底删除文件时，需要回收 inode 及其数据块
- 返回值：成功 0，错误 -1
- 可能的错误：文件不存在

#### fstat

- syscall ID: 80
- 功能：获取文件状态

```rust
fn fstat(&self, _caller: Caller, fd: usize, st: usize) -> isize
```

- 参数：
  - fd: 文件描述符
  - st: 文件状态结构体指针

```rust
#[repr(C)]
pub struct Stat {
    pub dev: u64,        // 磁盘驱动器号（写死为 0）
    pub ino: u64,        // inode 编号
    pub mode: StatMode,  // 文件类型（FILE 或 DIR）
    pub nlink: u32,      // 硬链接数量（初始为 1）
    pad: [u64; 7],
}

bitflags! {
    pub struct StatMode: u32 {
        const NULL  = 0;
        const DIR   = 0o040000;   // 目录
        const FILE  = 0o100000;   // 普通文件
    }
}
```

### 5.2 实现提示

- `linkat` 和 `unlinkat` 的文件路径读取可参考 `src/main.rs` 中 `open` 系统调用的实现
- `fstat` 的 Stat 结构体写入可参考 `clock_gettime` 对 TimeSpec 的写入方式（地址翻译后写入）
- 需要拉取 `tg-easy-fs` 到本地并修改以支持硬链接：
  ```bash
  cd tg-ch6
  cargo clone tg-easy-fs
  ```
  然后修改 `Cargo.toml`：
  ```toml
  [dependencies]
  tg-easy-fs = { path = "./tg-easy-fs" }
  ```

### 5.3 实验要求

**目录结构：**

```
tg-ch6/
├── Cargo.toml（需要修改依赖配置）
├── src/（需要修改）
│   ├── main.rs
│   ├── fs.rs
│   ├── process.rs
│   ├── processor.rs
│   └── virtio_block.rs
├── tg-easy-fs/（需拉取到本地并修改以支持硬链接）
│   └── src/
└── tg-user/（自动拉取，无需修改）
```

**运行和测试：**

```bash
cargo run --features exercise    # 运行练习测例
./test.sh exercise               # 测试练习测例
```

然后在终端中输入 `ch6_usertest` 运行所有练习测例。

> **前向兼容**：你的内核必须前向兼容，需要能通过前一章的所有测例。

---

## 六、本章小结

通过本章的学习和实践，你完成了操作系统中的重要基础设施——文件系统：

1. **文件系统概念**：通过 easy-fs 理解了 inode 文件系统的基本原理
2. **五层架构**：块设备接口 → 块缓存 → 磁盘数据结构 → 磁盘管理器 → Inode
3. **磁盘布局**：SuperBlock、Bitmap、Inode Area、Data Area 的组织方式
4. **VirtIO 驱动**：通过 MMIO 访问虚拟块设备，连接文件系统与磁盘
5. **文件描述符表**：统一管理标准 I/O 和普通文件的抽象
6. **文件操作接口**：open/close/read/write 系统调用的实现
7. **程序加载方式的变化**：从内核内嵌到文件系统动态加载

在后续章节中，我们将在文件系统的基础上引入**进程间通信**（管道等）机制。

## 七、思考题

1. **为什么需要块缓存？** 如果每次读写都直接访问磁盘，性能会怎样？块缓存的淘汰策略（FIFO vs LRU）对性能有什么影响？

2. **DiskInode 的索引设计？** 为什么 easy-fs 的 DiskInode 使用直接 + 一级间接 + 二级间接的三级索引结构？如果只有直接索引，最大文件大小是多少？

3. **文件描述符表的继承？** fork 时子进程复制了父进程的 fd_table。如果父进程打开了一个文件然后 fork，父子进程写入同一个文件会发生什么？

4. **硬链接 vs 软链接？** 硬链接和软链接有什么区别？为什么硬链接不能跨文件系统？删除一个硬链接后，文件何时真正被删除？

5. **exec 后的 fd_table？** 本实现中 exec 不清除 fd_table。这意味着什么？UNIX 系统中 exec 如何处理文件描述符（提示：close-on-exec 标志）？

## 参考资料

- [rCore-Tutorial-Guide 第六章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第六章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter6/index.html)
- [VirtIO 规范](https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.html)
- [UNIX 文件系统设计](https://en.wikipedia.org/wiki/Unix_File_System)
- [Linux VFS 层](https://www.kernel.org/doc/html/latest/filesystems/vfs.html)

---

## 附录：rCore-Tutorial 组件分析表

### 表 1：tg-ch1 ~ tg-ch8 操作系统内核总体情况描述表

| 操作系统内核 | 所涉及核心知识点 | 主要完成功能 | 所依赖的组件 |
|:-----|:------------|:---------|:---------------|
| **tg-ch1** | 应用程序执行环境<br>裸机编程（Bare-metal）<br>SBI（Supervisor Binary Interface）<br>RISC-V 特权级（M/S-mode）<br>链接脚本（Linker Script）<br>内存布局（Memory Layout）<br>Panic 处理 | 最小 S-mode 裸机程序<br>QEMU 直接启动（无 OpenSBI）<br>打印 "Hello, world!" 并关机<br>演示最基本的 OS 执行环境 | tg-sbi |
| **tg-ch2** | 批处理系统（Batch Processing）<br>特权级切换（U-mode ↔ S-mode）<br>Trap 处理（ecall / 异常）<br>上下文保存与恢复<br>系统调用（write / exit）<br>用户态 / 内核态<br>`sret` 返回指令 | 批处理操作系统<br>顺序加载运行多个用户程序<br>特权级切换和 Trap 处理框架<br>实现 write / exit 系统调用 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-syscall |
| **tg-ch3** | 多道程序（Multiprogramming）<br>任务控制块（TCB）<br>协作式调度（yield）<br>抢占式调度（Preemptive）<br>时钟中断（Clock Interrupt）<br>时间片轮转（Time Slice）<br>任务切换（Task Switch）<br>任务状态（Ready/Running/Finished）<br>clock_gettime 系统调用 | 多道程序与分时多任务<br>多程序同时驻留内存<br>协作式 + 抢占式调度<br>时钟中断与时间管理 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-syscall |
| **tg-ch4** | 虚拟内存（Virtual Memory）<br>Sv39 三级页表（Page Table）<br>地址空间隔离（Address Space）<br>页表项（PTE）与标志位<br>地址转换（VA → PA）<br>异界传送门（MultislotPortal）<br>ELF 加载与解析<br>堆管理（sbrk）<br>恒等映射（Identity Mapping）<br>内存保护（Memory Protection）<br>satp CSR | 引入 Sv39 虚拟内存<br>每个用户进程独立地址空间<br>跨地址空间上下文切换<br>进程隔离和内存保护 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall |
| **tg-ch5** | 进程（Process）<br>进程控制块（PCB）<br>进程标识符（PID）<br>fork（地址空间深拷贝）<br>exec（程序替换）<br>waitpid（等待子进程）<br>进程树 / 父子关系<br>初始进程（initproc）<br>Shell 交互式命令行<br>进程生命周期（Ready/Running/Zombie）<br>步幅调度（Stride Scheduling） | 引入进程管理<br>fork / exec / waitpid 系统调用<br>动态创建、替换、等待进程<br>Shell 交互式命令行 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage |
| **tg-ch6** | 文件系统（File System）<br>easy-fs 五层架构<br>SuperBlock / Inode / 位图<br>DiskInode（直接+间接索引）<br>目录项（DirEntry）<br>文件描述符表（fd_table）<br>文件句柄（FileHandle）<br>VirtIO 块设备驱动<br>MMIO（Memory-Mapped I/O）<br>块缓存（Block Cache）<br>硬链接（Hard Link）<br>open / close / read / write 系统调用 | 引入文件系统与 I/O<br>用户程序存储在磁盘镜像（fs.img）<br>VirtIO 块设备驱动<br>easy-fs 文件系统实现<br>文件打开 / 关闭 / 读写 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage<br>tg-easy-fs |
| **tg-ch7** | 进程间通信（IPC）<br>管道（Pipe）<br>环形缓冲区（Ring Buffer）<br>统一文件描述符（Fd 枚举）<br>信号（Signal）<br>信号集（SignalSet）<br>信号屏蔽字（Signal Mask）<br>信号处理函数（Signal Handler）<br>kill / sigaction / sigprocmask / sigreturn<br>命令行参数（argc / argv）<br>I/O 重定向（dup） | 进程间通信-管道 <br>异步事件通知（信号）<br>统一文件描述符抽象<br>信号发送 / 注册 / 屏蔽 / 返回 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage<br>tg-easy-fs<br>tg-signal<br>tg-signal-impl |
| **tg-ch8** | 同步互斥（Sync&Mutex）<br>线程（Thread）/ 线程标识符（TID）<br>进程-线程分离<br>竞态条件（Race Condition）<br>临界区（Critical Section）<br>互斥（Mutual Exclusion）<br>互斥锁（Mutex：自旋锁 vs 阻塞锁）<br>信号量（Semaphore：P/V 操作）<br>条件变量（Condvar）<br>管程（Monitor：Mesa 语义）<br>线程阻塞与唤醒（wait queue）<br>死锁（Deadlock）/ 死锁四条件<br>银行家算法（Banker's Algorithm）<br>双层管理器（PThreadManager） | 进程-线程分离<br>同一进程内多线程并发<br>互斥锁（MutexBlocking）<br>信号量（Semaphore）<br>条件变量（Condvar）<br>线程阻塞与唤醒机制<br>死锁检测（练习） | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage<br>tg-easy-fs<br>tg-signal<br>tg-signal-impl<br>tg-sync |

### 表 2：tg-ch1 ~ tg-ch8 操作系统内核所依赖组件总体情况描述表

| 功能组件 | 所涉及核心知识点 | 主要完成功能 | 所依赖的组件 |
|:-----|:------------|:---------|:----------------------|
| **tg-sbi** | SBI（Supervisor Binary Interface）<br>console_putchar / console_getchar<br>系统关机（shutdown）<br>RISC-V 特权级（M/S-mode）<br>ecall 指令 | S→M 模式的 SBI 调用封装<br>字符输出 / 字符读取<br>系统关机<br>支持 nobios 直接操作 UART | 无 |
| **tg-console** | 控制台 I/O<br>格式化输出（print! / println!）<br>日志系统（Log Level）<br>自旋锁保护的全局控制台 | 可定制 print! / println! 宏<br>log::Log 日志实现<br>Console trait 抽象底层输出 | 无 |
| **tg-kernel-context** | 上下文（Context）<br>Trap 帧（Trap Frame）<br>寄存器保存与恢复<br>特权级切换<br>stvec / sepc / scause CSR<br>LocalContext（本地上下文）<br>ForeignContext（跨地址空间上下文）<br>异界传送门（MultislotPortal） | 用户/内核态切换上下文管理<br>LocalContext 结构<br>ForeignContext（含 satp）<br>MultislotPortal 跨地址空间执行 | 无 |
| **tg-kernel-alloc** | 内核堆分配器<br>伙伴系统（Buddy Allocation）<br>动态内存管理<br>#[global_allocator] | 基于伙伴算法的 GlobalAlloc<br>堆初始化（init）<br>物理内存转移（transfer） | 无 |
| **tg-kernel-vm** | 虚拟内存管理<br>页表（Page Table）<br>Sv39 分页（三级页表）<br>虚拟地址（VAddr）/ 物理地址（PAddr）<br>虚拟页号（VPN）/ 物理页号（PPN）<br>页表项（PTE）/ 页表标志位（VmFlags）<br>地址空间（AddressSpace）<br>PageManager trait<br>地址翻译（translate） | Sv39 页表管理<br>AddressSpace 地址空间抽象<br>虚实地址转换<br>页面映射（map / map_extern）<br>页表项操作 | 无 |
| **tg-syscall** | 系统调用（System Call）<br>系统调用号（SyscallId）<br>系统调用分发（handle）<br>系统调用结果（Done / Unsupported）<br>Caller 抽象<br>IO / Process / Scheduling / Clock /<br>Signal / Thread / SyncMutex trait 接口 | 系统调用 ID 与参数定义<br>trait 接口供内核实现<br>init_io / init_process / init_scheduling /<br>init_clock / init_signal /<br>init_thread / init_sync_mutex<br>支持 kernel / user feature | tg-signal-defs |
| **tg-task-manage** | 任务管理（Task Management）<br>调度（Scheduling）<br>进程管理器（PManager, proc feature）<br>双层管理器（PThreadManager, thread feature）<br>ProcId / ThreadId<br>就绪队列（Ready Queue）<br>Manage trait / Schedule trait<br>进程等待（wait / waitpid）<br>线程等待（waittid）<br>阻塞与唤醒（blocked / re_enque） | Manage 和 Schedule trait 抽象<br>proc feature：单层进程管理器（PManager）<br>thread feature：双层管理器（PThreadManager）<br>进程树 / 父子关系<br>线程阻塞 / 唤醒 | 无 |
| **tg-easy-fs** | 文件系统（File System）<br>SuperBlock / Inode / 位图（Bitmap）<br>DiskInode（直接+间接索引）<br>块缓存（Block Cache）<br>BlockDevice trait<br>文件句柄（FileHandle）<br>打开标志（OpenFlags）<br>管道（Pipe）/ 环形缓冲区<br>用户缓冲区（UserBuffer）<br>FSManager trait | easy-fs 五层架构实现<br>文件创建 / 读写 / 目录操作<br>块缓存管理<br>管道环形缓冲区实现<br>FSManager trait 抽象 | 无 |
| **tg-signal-defs** | 信号编号（SignalNo）<br>SIGKILL / SIGINT / SIGUSR1 等<br>信号动作（SignalAction）<br>信号集（SignalSet）<br>最大信号数（MAX_SIG） | 信号编号枚举定义<br>信号动作结构定义<br>信号集类型定义<br>为 tg-signal 和 tg-syscall 提供共用类型 | 无 |
| **tg-signal** | 信号处理（Signal Handling）<br>Signal trait 接口<br>add_signal / handle_signals<br>get_action_ref / set_action<br>update_mask / sig_return / from_fork<br>SignalResult（Handled / ProcessKilled） | Signal trait 接口定义<br>信号添加 / 处理 / 动作设置<br>屏蔽字更新 / 信号返回<br>fork 继承 | tg-kernel-context<br>tg-signal-defs |
| **tg-signal-impl** | SignalImpl 结构<br>已接收信号位图（received）<br>信号屏蔽字（mask）<br>信号处理中状态（handling）<br>信号动作表（actions）<br>信号处理函数调用<br>上下文保存与恢复 | Signal trait 的参考实现<br>信号接收位图管理<br>屏蔽字逻辑<br>处理状态和动作表 | tg-kernel-context<br>tg-signal |
| **tg-sync** | 互斥锁（Mutex trait: lock / unlock）<br>阻塞互斥锁（MutexBlocking）<br>信号量（Semaphore: up / down）<br>条件变量（Condvar: signal / wait_with_mutex）<br>等待队列（VecDeque\<ThreadId\>）<br>UPIntrFreeCell | MutexBlocking 阻塞互斥锁<br>Semaphore 信号量<br>Condvar 条件变量<br>通过 ThreadId 与调度器交互 | tg-task-manage |
| **tg-user** | 用户态程序（User-space App）<br>用户库（User Library）<br>系统调用封装（syscall wrapper）<br>用户堆分配器<br>用户态 print! / println! | 用户测试程序运行时库<br>系统调用封装<br>用户堆分配器<br>各章节测试用例（ch2~ch8） | tg-console<br>tg-syscall |
| **tg-checker** | 测试验证<br>输出模式匹配<br>正则表达式（Regex）<br>测试用例判定 | rCore-Tutorial CLI 测试输出检查工具<br>验证内核输出匹配预期模式<br>支持 --ch N 和 --exercise 模式 | 无 |
| **tg-linker** | 链接脚本（Linker Script）<br>内核内存布局（KernelLayout）<br>.text / .rodata / .data / .bss / .boot 段<br>入口点（boot0! 宏）<br>BSS 段清零 | 形成内核空间布局的链接脚本模板<br>用于 build.rs 工具构建 linker.ld<br>内核布局定位（KernelLayout::locate）<br>入口宏（boot0!）<br>段信息迭代 | 无 |
## License

Licensed under GNU GENERAL PUBLIC LICENSE, Version 3.0.
