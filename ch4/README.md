# 第四章：地址空间

本章在第三章"多道程序与分时多任务"的基础上，引入了 **RISC-V Sv39 虚拟内存机制**，为每个用户进程提供**独立的地址空间**（tg-ch4）。这是操作系统实现**进程隔离**和**内存保护**的关键一步。

通过本章的学习和实践，你将理解：

- 什么是虚拟内存，为什么需要地址空间隔离
- RISC-V Sv39 三级页表的结构和地址翻译过程
- 内核地址空间与用户地址空间的布局
- 异界传送门（MultislotPortal）如何解决跨地址空间切换问题
- ELF 文件如何被解析并加载到独立地址空间
- 系统调用中如何进行用户地址翻译和权限检查
- 堆内存管理（sbrk 系统调用）

> **前置知识**：建议先完成第一章至第三章的学习，理解裸机启动、Trap 处理、系统调用和多任务调度。

## 项目结构

```
ch4/
├── .cargo/
│   └── config.toml     # Cargo 配置：交叉编译目标和 QEMU runner
├── .gitignore           # Git 忽略规则
├── build.rs            # 构建脚本：下载编译用户程序，生成链接脚本和 APP_ASM
├── Cargo.toml          # 项目配置与依赖
├── LICENSE             # GPL v3 许可证
├── README.md           # 本文档
├── rust-toolchain.toml # Rust 工具链配置
├── test.sh             # 自动测试脚本
└── src/
    ├── main.rs         # 内核源码：初始化、调度、系统调用、页表管理器
    └── process.rs      # 进程结构：地址空间、ELF 加载、堆管理
```

<a id="source-nav"></a>

## 源码阅读导航索引

[返回根文档导航总表](../README.md#chapters-source-nav-map)

本章建议按“地址空间建立 -> 进程装载 -> 跨地址空间执行 -> 用户指针翻译”阅读。

| 阅读顺序 | 文件 | 重点问题 |
|---|---|---|
| 1 | `src/main.rs` 的 `kernel_space` | 内核恒等映射、堆映射、传送门映射分别解决什么问题？ |
| 2 | `src/process.rs` 的 `new` | ELF 如何被映射到用户地址空间，用户栈与 `satp` 如何初始化？ |
| 3 | `src/main.rs` 的 `schedule` | 异界传送门如何支撑跨地址空间执行与返回？ |
| 4 | `src/main.rs` 的 `impls` | 系统调用里 `translate()` 如何做权限检查与地址翻译？ |
| 5 | `src/process.rs` 的 `change_program_brk` | `sbrk` 如何驱动堆页映射的扩张与回收？ |

配套建议：先读本章再回看 `tg-kernel-vm` 与 `tg-kernel-context/foreign`，会更容易理解抽象设计。

## DoD 验收标准（本章完成判据）

- [ ] 能说明本章为何必须引入 Sv39 与每进程独立地址空间
- [ ] 能从代码解释“内核恒等映射 + 用户地址空间映射 + 传送门映射”三者关系
- [ ] 能说明 `translate()` 在 syscall 中如何完成权限检查与地址翻译
- [ ] 能解释 `sbrk` 扩容/缩容时页映射范围如何变化
- [ ] 能执行 `./test.sh base`（练习时补充 `./test.sh exercise`）

## 概念-源码-测试三联表

| 核心概念 | 源码入口 | 自测方式（命令/现象） |
|---|---|---|
| 内核地址空间建立 | `ch4/src/main.rs` 的 `kernel_space` | 启动日志出现 `.text/.rodata/.data/(heap)` 映射信息 |
| ELF 装载到用户空间 | `ch4/src/process.rs` 的 `Process::new` | 用户程序入口为虚拟地址（如 `0x10000`）并可运行 |
| 跨地址空间执行 | `ch4/src/main.rs` 的 `schedule` + `MultislotPortal` | 用户态与内核态可正常往返，无地址空间切换崩溃 |
| 用户指针翻译与检查 | `ch4/src/main.rs` 的 `impls`（`translate`） | 非法用户地址会被拒绝而不是直接越界访问 |

遇到构建/运行异常可先查看根文档的“高频错误速查表”。

## 一、环境准备

### 1.1 安装 Rust 工具链

**Linux / macOS / WSL：**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

验证安装：

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
cargo clone tg-ch4
cd tg-ch4
```

**方式二：获取所有实验**

```bash
git clone https://github.com/rcore-os/tg-rcore-tutorial.git
cd tg-rcore-tutorial/ch4
```

## 二、编译与运行

### 2.1 编译

```bash
cargo build
```

编译过程与第三章类似，`build.rs` 会自动下载 `tg-user`、编译用户程序并嵌入内核。

> 环境变量说明：
> - `TG_USER_DIR`：指定本地 tg-user 源码路径（跳过自动下载）
> - `TG_USER_VERSION`：指定 tg-user 版本（默认 `0.2.0-preview.1`）
> - `TG_SKIP_USER_APPS`：设置后跳过用户程序编译
> - `LOG`：设置日志级别（如 `LOG=INFO`、`LOG=TRACE`）

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
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch4
```

### 2.3 预期输出

```
[tg-ch4 ...] Hello, world!
[ INFO] .text    ---> 0x80200000..0x8020xxxx
[ INFO] .rodata  ---> 0x8020xxxx..0x8020xxxx
[ INFO] .data    ---> 0x8020xxxx..0x8020xxxx
[ INFO] (heap)   ---> 0x8020xxxx..0x81a00000

[ INFO] detect app[0]: 0x8020xxxx..0x8020xxxx
[ INFO] process entry = 0x10000, heap_bottom = 0xxxxx
[ INFO] detect app[1]: ...
...

Hello, world from user mode program!
Test power OK!
...
Test sbrk OK!
```

与前几章不同，你会看到：
- 内核地址空间的段映射信息（恒等映射）
- 每个进程的入口地址是 ELF 中的虚拟地址（如 `0x10000`），而非物理地址
- 用户程序在独立地址空间中运行，堆管理（sbrk）正常工作

### 2.4 运行测试

```bash
./test.sh           # 运行全部测试（基础 + 练习）
./test.sh base      # 仅运行基础测试
./test.sh exercise  # 仅运行练习测试
```

---

## 三、操作系统核心概念

### 3.1 为什么需要虚拟内存？

在前几章中，所有用户程序直接使用物理地址，存在严重问题：

| 问题 | 说明 |
|------|------|
| **安全性** | 用户程序可以读写任意物理地址，包括内核数据 |
| **隔离性** | 一个程序的 bug 可能破坏其他程序的内存 |
| **灵活性** | 程序必须加载到特定的物理地址，无法重定位 |

**虚拟内存**通过在 CPU 和物理内存之间加入一层**地址翻译**解决这些问题：

```
用户程序                MMU（地址翻译）              物理内存
┌──────────┐           ┌──────────┐           ┌──────────┐
│ 虚拟地址  │ ───────→ │  页表查找  │ ───────→ │ 物理地址  │
│ 0x10000  │           │ Sv39 三级 │           │ 0x80400000│
└──────────┘           └──────────┘           └──────────┘
```

每个进程拥有独立的页表，看到的虚拟地址空间完全相同（都从 `0x10000` 开始），但映射到不同的物理页面。

### 3.2 RISC-V Sv39 页表

**Sv39** 是 RISC-V 定义的 39 位虚拟地址的分页方案：

| 参数 | 值 |
|------|-----|
| 虚拟地址宽度 | 39 位（512 GiB 地址空间） |
| 物理地址宽度 | 56 位 |
| 页大小 | 4 KiB（12 位页内偏移） |
| 页表级数 | 3 级（每级 9 位，共 27 位 VPN） |
| 页表项大小 | 8 字节（64 位） |
| 每页页表项数 | 512 个 |

**虚拟地址结构：**

```
 38       30 29       21 20       12 11        0
┌──────────┬──────────┬──────────┬──────────┐
│  VPN[2]  │  VPN[1]  │  VPN[0]  │  Offset  │
│  9 bits  │  9 bits  │  9 bits  │  12 bits │
└──────────┴──────────┴──────────┴──────────┘
```

**三级页表查找过程：**

```
satp 寄存器 → 根页表物理地址
        │
        ▼
  ┌─ 根页表 ─┐
  │ VPN[2] 索引│ ──→ 得到二级页表地址
  └──────────┘
        │
        ▼
  ┌─ 二级页表 ─┐
  │ VPN[1] 索引│ ──→ 得到三级页表地址
  └──────────┘
        │
        ▼
  ┌─ 三级页表 ─┐
  │ VPN[0] 索引│ ──→ 得到物理页号 PPN
  └──────────┘
        │
        ▼
  物理地址 = PPN × 4096 + Offset
```

**页表项（PTE）标志位：**

| 标志 | 含义 |
|------|------|
| V (Valid) | 有效位，必须为 1 |
| R (Read) | 可读 |
| W (Write) | 可写 |
| X (Execute) | 可执行 |
| U (User) | 用户态可访问 |
| G (Global) | 全局映射（不随 TLB 刷新） |
| A (Accessed) | 已访问 |
| D (Dirty) | 已修改 |

**satp 寄存器：**

```
 63  60 59          44 43                    0
┌──────┬──────────────┬──────────────────────┐
│ MODE │    ASID      │      PPN             │
│ 4bit │   16 bit     │     44 bit           │
└──────┴──────────────┴──────────────────────┘
  MODE=8 表示 Sv39        根页表的物理页号
```

### 3.3 内核地址空间

tg-ch4 的内核地址空间使用**恒等映射**（Identity Mapping）：虚拟地址 == 物理地址。

```
内核地址空间
┌────────────────────────────────────┐ 高地址
│ 传送门（PORTAL_TRANSIT = VPN::MAX）│ ← 虚拟地址空间最高页
├────────────────────────────────────┤
│ 调度栈（2 页）                      │
├────────────────────────────────────┤
│              ...                    │
├────────────────────────────────────┤
│ 堆区域（恒等映射）                   │ ← layout.end() ~ start+MEMORY
├────────────────────────────────────┤
│ .bss（恒等映射）                     │
│ .data（恒等映射）                    │
│ .rodata（恒等映射）                  │
│ .text（恒等映射）                    │
└────────────────────────────────────┘ 0x80200000
```

恒等映射的优势：内核可以直接通过虚拟地址访问任意物理内存，简化页表管理代码。

### 3.4 用户地址空间

每个用户进程拥有独立的地址空间，通过 ELF 解析创建：

```
用户地址空间
┌────────────────────────────────────┐ 高地址
│ 传送门（与内核共享同一物理页）        │ ← VPN::MAX
├────────────────────────────────────┤
│              ...                    │
├────────────────────────────────────┤
│ 用户栈（2 页 = 8 KiB）             │ ← VPN[(1<<26)-2, 1<<26)
├────────────────────────────────────┤
│              ...                    │
├────────────────────────────────────┤
│ 堆区域（通过 sbrk 动态扩展）        │ ← heap_bottom ~ program_brk
├────────────────────────────────────┤
│ .bss / .data / .rodata / .text     │ ← 从 ELF LOAD 段映射
└────────────────────────────────────┘ 低地址（如 0x10000）
```

**关键特性：**
- 所有映射都带有 **U（User）标志**，允许用户态访问
- 传送门页面在内核和用户地址空间**映射到相同虚拟地址**
- 每个进程有独立的堆（通过 `sbrk` 管理）

### 3.5 异界传送门（MultislotPortal）

**核心问题**：当内核和用户程序使用不同的页表时，切换 `satp` 后当前指令所在的虚拟地址可能变为无效，导致 CPU 无法继续执行。

**解决方案**：异界传送门——一个特殊的代码页面，同时映射到内核和所有用户地址空间的**相同虚拟地址**。

```
内核地址空间                     用户地址空间
┌──────────────┐               ┌──────────────┐
│              │               │              │
│   内核代码    │               │   用户代码    │
│              │               │              │
├──────────────┤               ├──────────────┤
│   传送门页面  │ ──── 同一 ──→ │   传送门页面  │
│ (VPN::MAX)   │   物理页面    │ (VPN::MAX)   │
└──────────────┘               └──────────────┘
```

**切换流程：**

```
内核态（内核地址空间）
    │
    ▼
跳转到传送门虚拟地址
    │
    ▼
在传送门内：切换 satp → 用户地址空间
    │  （传送门在两个地址空间的虚拟地址相同，所以不会崩溃）
    ▼
恢复用户寄存器，sret → U-mode
    │
    ▼
用户程序执行...
    │
    ▼
Trap → 传送门入口
    │
    ▼
在传送门内：切换 satp → 内核地址空间
    │
    ▼
跳转到内核 Trap 处理代码
```

### 3.6 ELF 加载

本章不再将用户程序作为原始二进制加载，而是解析 **ELF 格式**：

```
ELF 文件
├── ELF Header（入口地址、程序头表位置）
├── Program Header 1（LOAD: .text, 虚拟地址 0x10000, RX）
├── Program Header 2（LOAD: .data, 虚拟地址 0x20000, RW）
└── ...
```

加载过程：
1. 解析 ELF 头，验证是 RISC-V 64 位可执行文件
2. 遍历 LOAD 类型的程序头
3. 为每个段分配物理页面，设置对应的权限标志（R/W/X + U）
4. 将段数据从 ELF 复制到新分配的页面
5. 记录最高虚拟地址作为堆底

### 3.7 地址翻译与系统调用

引入地址空间后，系统调用的实现发生了根本变化：**用户传入的指针是虚拟地址，内核无法直接访问**。

以 `write` 系统调用为例：

```
第三章（无虚拟内存）              第四章（有虚拟内存）
─────────────────               ─────────────────
用户传入 buf = 0x80400000        用户传入 buf = 0x10200
    │                               │
    ▼                               ▼
内核直接读取 buf 地址           内核通过 translate() 查页表
    │                               │
    ▼                               ▼
输出数据                        得到物理地址 0x80500200
                                    │
                                    ▼
                                输出数据
```

`translate()` 方法会：
1. 通过进程的页表将虚拟地址翻译为物理地址
2. 检查页表项的权限标志（如可读、可写）
3. 权限不足时返回 `None`，系统调用返回 -1

### 3.8 堆内存管理（sbrk）

本章新增了 `sbrk` 系统调用，允许用户程序动态调整堆大小：

```
堆底（heap_bottom）            堆顶（program_brk）
     │                              │
     ▼                              ▼
     ┌──────────────────────────────┐
     │      已分配的堆空间           │
     └──────────────────────────────┘

sbrk(+4096)  →  扩展堆，映射新的物理页面
sbrk(-4096)  →  收缩堆，取消映射物理页面
sbrk(0)      →  返回当前堆顶地址
```

### 3.9 系统调用

| syscall ID | 名称 | 功能 |
|-----------|------|------|
| 64 | `write` | 写入数据（需地址翻译） |
| 93 | `exit` | 退出当前进程 |
| 124 | `sched_yield` | 让出 CPU |
| 113 | `clock_gettime` | 获取时间（需地址翻译） |
| 214 | `sbrk` | 调整堆大小 |
| 410 | `trace` | 追踪系统调用（**练习题**） |
| 222 | `mmap` | 映射内存（**练习题**） |
| 215 | `munmap` | 取消映射（**练习题**） |

---

## 四、代码解读

### 4.1 `src/main.rs` —— 内核主体

**启动流程 `rust_main`：**
1. 清零 BSS 段
2. 初始化控制台和日志
3. 初始化内核堆分配器（`tg_kernel_alloc`）
4. 分配并创建异界传送门
5. 建立内核地址空间（恒等映射 + 传送门映射），激活 Sv39 分页
6. 解析 ELF 加载用户进程，共享传送门页表项
7. 创建调度线程，进入 `schedule()` 函数

**调度函数 `schedule`：**
- 初始化传送门和系统调用
- 循环执行：通过传送门切换到用户进程 → Trap 返回 → 处理系统调用/异常
- 所有进程完成后关机

**页表管理器 `Sv39Manager`：**
- 实现 `PageManager<Sv39>` trait
- 提供物理页面的分配、映射和地址转换
- 使用 `OWNED` 标志位标记内核分配的页面

**地址翻译在系统调用中的应用（如 `write`、`clock_gettime`）：**
- 构建权限标志（如 `READABLE`、`WRITABLE`）
- 调用 `address_space.translate()` 翻译用户地址并检查权限
- 翻译失败时返回 -1

### 4.2 `src/process.rs` —— 进程管理

**`Process::new(elf)`**：从 ELF 创建进程
- 验证 ELF 头 → 创建地址空间 → 映射 LOAD 段 → 分配用户栈 → 创建 ForeignContext

**`change_program_brk(size)`**：实现 sbrk
- size > 0：扩展堆，映射新页面
- size < 0：收缩堆，取消映射
- 返回旧的 break 地址

### 4.3 `Cargo.toml` —— 依赖说明

| 依赖 | 说明 |
|------|------|
| `xmas-elf` | ELF 文件格式解析库 |
| `riscv` | RISC-V CSR 寄存器访问（`satp`、`scause`） |
| `tg-sbi` | SBI 调用封装 |
| `tg-linker` | 链接脚本生成、内核布局定位 |
| `tg-console` | 控制台输出和日志 |
| `tg-kernel-context` | 用户上下文及异界传送门 `MultislotPortal`（`foreign` feature） |
| `tg-kernel-alloc` | 内核堆分配器 |
| `tg-kernel-vm` | 虚拟内存管理（地址空间、页表、页面管理） |
| `tg-syscall` | 系统调用定义与分发 |

---

## 五、编程练习

### 5.1 重写 trace 系统调用

引入虚存机制后，原来内核的 `trace` 函数实现就无效了。**请你重写这个系统调用的代码**，恢复其正常功能。

由于本章有了地址空间作为隔离机制，`trace` **需要考虑额外的情况**：

- 在读取（`trace_request` 为 0）时，如果对应地址用户不可见或不可读，则返回值应为 -1（`isize` 格式的 -1，而非 `u8`）。
- 在写入（`trace_request` 为 1）时，如果对应地址用户不可见或不可写，则返回值应为 -1。

### 5.2 实现 mmap 和 munmap 匿名映射

[mmap](https://man7.org/linux/man-pages/man2/mmap.2.html) 在 Linux 中主要用于在内存中映射文件，本次实验简化它的功能，仅用于申请内存。

**mmap 定义：**

```rust
fn mmap(&self, _caller: Caller, addr: usize, len: usize, prot: i32,
        _flags: i32, _fd: i32, _offset: usize) -> isize
```

- syscall ID：222
- 申请 `len` 字节物理内存，映射到 `addr` 开始的虚存，属性为 `prot`
- 参数：
  - `addr`：虚存起始地址（**必须按页对齐**）
  - `len`：字节长度（可为 0，按页向上取整）
  - `prot`：bit 0=可读，bit 1=可写，bit 2=可执行。其他位必须为 0
- 返回：成功 0，错误 -1
- 可能的错误：addr 未对齐、prot 无效、地址已映射、物理内存不足

**munmap 定义：**

```rust
fn munmap(&self, _caller: Caller, addr: usize, len: usize) -> isize
```

- syscall ID：215
- 取消 `[addr, addr + len)` 的映射
- 错误：存在未被映射的虚存

### 5.3 实现提示

- 页表项权限使用 `VmFlags::build_from_str()` 构建，如 `"U_WRV"` 表示用户态可写可读有效
- 注意 RISC-V 页表项格式与 `prot` 参数的区别
- **别忘了添加 `U`（用户态可访问）标志**
- 实现 `trace` 时，可参考 `main.rs` 中 `clock_gettime` 的实现，使用 `translate` 方法进行地址翻译和权限检查

### 5.4 实验要求

```
tg-ch4/
├── Cargo.toml          # 内核配置（需修改依赖配置）
├── src/                # 内核源代码（需修改）
│   ├── main.rs
│   └── process.rs
├── tg-kernel-vm/       # 虚拟内存模块（需拉取到本地并修改）
│   └── src/
│       ├── lib.rs
│       └── space/mod.rs
└── tg-user/            # 用户程序（自动拉取，无需修改）
```

> **注意**：`tg-kernel-vm` 需要拉取到本地才能修改：
> ```bash
> cd tg-ch4
> cargo clone tg-kernel-vm
> ```
> 然后修改 `Cargo.toml`：
> ```toml
> [dependencies]
> tg-kernel-vm = { path = "./tg-kernel-vm" }
> ```

**运行和测试：**

```bash
cargo run --features exercise    # 运行练习测例
./test.sh exercise               # 测试练习测例
```

---

## 六、本章小结

通过本章的学习和实践，你完成了操作系统中最重要的抽象之一——地址空间：

1. **虚拟内存**：通过 Sv39 三级页表将虚拟地址映射到物理地址，CPU 的每次内存访问都经过地址翻译
2. **进程隔离**：每个进程拥有独立的页表，无法访问其他进程或内核的内存
3. **异界传送门**：巧妙地解决了跨地址空间切换时代码无法执行的问题
4. **ELF 加载**：不再使用原始二进制，而是解析标准的 ELF 格式，按段设置权限
5. **地址翻译**：系统调用必须将用户虚拟地址翻译为物理地址才能访问
6. **堆管理**：通过 `sbrk` 实现动态堆扩展/收缩

在后续章节中，我们将在地址空间的基础上引入**进程**概念，实现 `fork`/`exec`/`waitpid` 等系统调用。

## 七、思考题

1. **恒等映射 vs 非恒等映射？** 内核使用恒等映射（VPN == PPN），用户使用非恒等映射。这样设计的好处是什么？如果内核也使用非恒等映射，会带来什么复杂性？

2. **为什么需要异界传送门？** 如果不使用传送门，直接在切换 `satp` 后执行下一条指令，会发生什么？能否用其他方式解决这个问题？

3. **页表项的 U 标志的作用？** 如果一个页面没有设置 U 标志，用户态程序访问它会发生什么？内核态呢？

4. **`translate()` 在系统调用中的必要性？** 为什么 `write` 系统调用不能直接用用户传入的指针？如果省略 translate 检查，可能导致什么安全问题？

5. **sbrk 与 mmap 的关系？** `sbrk` 只能线性扩展堆，而 `mmap` 可以在任意地址映射内存。现代操作系统中，`malloc` 通常同时使用两者。为什么？

## 参考资料

- [rCore-Tutorial-Guide 第四章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第四章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter4/index.html)
- [RISC-V Privileged Specification - Sv39](https://riscv.org/specifications/privileged-isa/)
- [RISC-V Reader 中文版](http://riscvbook.com/chinese/RISC-V-Reader-Chinese-v2p1.pdf)

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
