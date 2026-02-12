# 第五章：进程

本章在第四章"地址空间"的基础上，引入了完整的 **进程管理** 机制，实现了 `fork`、`exec`、`waitpid` 等核心系统调用。进程是操作系统中最重要的抽象之一——它将"运行中的程序"封装为一个可管理的实体，使得用户可以动态创建、终止、等待进程，并通过 Shell 与操作系统交互。

通过本章的学习和实践，你将理解：

- 什么是进程，进程与任务的区别
- `fork` 如何复制父进程的地址空间创建子进程
- `exec` 如何用新程序替换当前进程的地址空间
- `waitpid` 如何等待子进程退出并回收资源
- 进程树结构和父子关系的维护
- 初始进程（initproc）和 Shell 的运行机制
- 进程调度（FIFO/RR → stride 调度算法）

> **前置知识**：建议先完成第一章至第四章的学习，理解裸机启动、Trap 处理、系统调用、多任务调度和虚拟内存机制。

## 项目结构

```
ch5/
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
    ├── main.rs         # 内核主体：初始化、调度循环、系统调用实现
    ├── process.rs      # 进程结构：ELF 加载、fork、exec、堆管理
    └── processor.rs    # 处理器管理：进程管理器、调度队列
```

<a id="source-nav"></a>

## 源码阅读导航索引

[返回根文档导航总表](../README.md#chapters-source-nav-map)

本章建议按“进程数据结构 -> 管理器 -> syscall 语义”阅读，重点把 `fork/exec/wait` 串起来。

| 阅读顺序 | 文件 | 重点问题 |
|---|---|---|
| 1 | `src/process.rs` | `from_elf`、`fork`、`exec` 分别如何改变进程执行映像？ |
| 2 | `src/processor.rs` | `ProcManager` 如何维护就绪队列与实体映射？ |
| 3 | `src/main.rs` 初始化路径 | `initproc` 如何被加载并进入调度体系？ |
| 4 | `src/main.rs` Trap + syscall 分支 | `exit`/`wait`/`exec` 在内核中的状态迁移如何发生？ |

配套建议：结合 `tg-task-manage` 的 `PManager`/`ProcRel` 注释阅读，可快速厘清父子进程关系与回收机制。

## DoD 验收标准（本章完成判据）

- [ ] 能描述 `fork -> exec -> wait` 的完整语义链路
- [ ] 能从源码解释父子进程关系如何被建立、等待与回收
- [ ] 能解释 `initproc` 与 `user_shell` 在系统启动后的角色
- [ ] 能在 Shell 中运行至少一个 fork/wait 相关用户程序并解释输出
- [ ] 能执行 `./test.sh base`（练习时补充 `./test.sh exercise`）

## 概念-源码-测试三联表

| 核心概念 | 源码入口 | 自测方式（命令/现象） |
|---|---|---|
| 进程创建与替换 | `ch5/src/process.rs` 的 `fork/exec/from_elf` | 子进程 PID、父子返回值与预期一致 |
| 进程调度与实体管理 | `ch5/src/processor.rs` | 能解释 `ready_queue` 如何决定下一运行进程 |
| 退出与回收 | `ch5/src/main.rs` 的 syscall 分支（`EXIT/WAIT`） | `waitpid` 能拿到子进程退出码 |
| 启动进程链 | `ch5/src/main.rs` 初始化 `initproc` | 进入 shell 并可执行命令 |

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
cargo clone tg-ch5
cd tg-ch5
```

**方式二：获取所有实验**

```bash
git clone https://github.com/rcore-os/rCore-Tutorial-in-single-workspace.git
cd rCore-Tutorial-in-single-workspace/ch5
```

## 二、编译与运行

### 2.1 编译

```bash
cargo build
```

编译过程与前几章类似，`build.rs` 会自动下载 `tg-user`、编译用户程序并嵌入内核。

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
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch5
```

### 2.3 预期输出

```
[tg-ch5 ...] Hello, world!
[ INFO] .text    ---> 0x80200000..0x8020xxxx
[ INFO] .rodata  ---> 0x8020xxxx..0x8020xxxx
[ INFO] .data    ---> 0x8020xxxx..0x8020xxxx
[ INFO] (heap)   ---> 0x8020xxxx..0x81a00000

Rust user shell
>> ch5b_forktest_simple

sys_wait without child process test passed!
parent start, pid = 2!
ready waiting on parent process!
hello child process!
child process pid = 3, exit code = 100
Shell: Process 2 exited with code 0
>> ch5b_forktree
...
```

与第四章不同，你会看到：
- 出现了 **Shell 交互界面**，用户可以通过输入命令名来执行程序
- `initproc` 进程启动后 `fork` 出 `user_shell` 子进程
- 用户程序通过 `fork`/`exec` 组合动态创建和执行
- `waitpid` 回收子进程资源，Shell 打印退出码

### 2.4 运行测试

```bash
./test.sh           # 运行全部测试（基础 + 练习）
./test.sh base      # 仅运行基础测试
./test.sh exercise  # 仅运行练习测试
```

---

## 三、操作系统核心概念

### 3.1 从任务到进程

在前几章中，我们管理的是"任务"（Task）：内核预先加载所有用户程序，按调度策略切换执行。但任务有明显的局限性：

| 特性 | 任务（第三、四章） | 进程（第五章） |
|------|-------------------|----------------|
| 创建方式 | 内核启动时全部加载 | 运行时动态创建（fork） |
| 程序替换 | 不支持 | exec 加载新程序 |
| 父子关系 | 无 | 完整的进程树 |
| 资源回收 | 内核自动回收 | 父进程通过 wait 回收 |
| 用户交互 | 无 | Shell 命令行 |
| 进程标识 | 无/内部编号 | PID（进程标识符） |

**进程**（Process）是操作系统对"运行中的程序"的抽象。每个进程拥有：
- 独立的地址空间（页表）
- 唯一的进程标识符（PID）
- 执行上下文（寄存器状态）
- 父子关系

### 3.2 核心系统调用

#### fork：创建子进程

```
fork() 系统调用
─────────────────────────────────────
syscall ID: 220
功能：由当前进程复制出一个子进程
返回值：
  - 对于父进程：返回子进程的 PID
  - 对于子进程：返回 0
```

`fork` 的核心操作是**深拷贝父进程的地址空间**：

```
父进程地址空间                 子进程地址空间（fork 后）
┌──────────────┐              ┌──────────────┐
│   .text      │  ──复制──→   │   .text      │
│   .data      │              │   .data      │
│   堆空间      │              │   堆空间      │
│   用户栈      │              │   用户栈      │
│   传送门      │ ──共享──→   │   传送门      │
└──────────────┘              └──────────────┘
  独立页表                       独立页表
  （不同物理页面）                 （不同物理页面）
```

fork 返回后，父子进程拥有相同的代码和数据，但在不同的地址空间中独立运行。区分父子进程的方式是 fork 的返回值：

```rust
let pid = fork();
if pid == 0 {
    // 子进程分支
} else {
    // 父进程分支，pid 是子进程的 PID
}
```

#### exec：替换程序

```
exec(path) 系统调用
─────────────────────────────────────
syscall ID: 221
功能：将当前进程的地址空间清空，加载并执行指定的 ELF 程序
参数：path 为程序名字符串
返回值：成功不返回（开始执行新程序），失败返回 -1
```

exec 的核心操作是**替换地址空间**：

```
exec 前（旧程序）              exec 后（新程序）
┌──────────────┐              ┌──────────────┐
│  旧 .text    │              │  新 .text    │
│  旧 .data    │  ──替换──→   │  新 .data    │
│  旧堆空间    │              │  新堆空间    │
│  旧用户栈    │              │  新用户栈    │
└──────────────┘              └──────────────┘
  PID 不变                       PID 不变
```

exec 保留 PID 和父子关系，但完全替换了代码和数据。

#### waitpid：等待子进程

```
waitpid(pid, exit_code) 系统调用
─────────────────────────────────────
syscall ID: 260
功能：等待子进程退出，回收资源，收集退出码
参数：
  - pid == -1：等待任意子进程
  - pid > 0：等待指定 PID 的子进程
  - exit_code：存放子进程退出码的用户空间指针
返回值：
  - 成功：返回退出的子进程 PID
  - 无符合条件的子进程：返回 -1
  - 子进程尚未退出：返回 -2（由用户库循环等待）
```

#### 其他系统调用

| syscall ID | 名称 | 功能 |
|-----------|------|------|
| 63 | `read` | 从标准输入读取（需地址翻译） |
| 64 | `write` | 写入标准输出（需地址翻译） |
| 93 | `exit` | 退出当前进程，保存退出码 |
| 124 | `sched_yield` | 主动让出 CPU |
| 113 | `clock_gettime` | 获取系统时间（需地址翻译） |
| 172 | `getpid` | 获取当前进程 PID |
| 214 | `sbrk` | 调整堆大小 |
| 220 | `fork` | 创建子进程 |
| 221 | `exec` | 替换当前程序 |
| 260 | `wait` / `waitpid` | 等待子进程退出 |
| 400 | `spawn` | 创建新进程（**练习题**） |
| 140 | `set_priority` | 设置进程优先级（**练习题**） |
| 222 | `mmap` | 映射匿名内存（**练习题**） |
| 215 | `munmap` | 取消内存映射（**练习题**） |

### 3.3 进程生命周期

```
                 fork
  父进程 ──────────────→ 子进程（就绪态）
                              │
                         exec（可选）
                              │
                              ▼
                         运行中 ←─── sched_yield / 时间片用完
                              │               ↑
                              │        调度器选中
                              ▼               │
                         就绪态 ──────────────┘
                              │
                         exit / 异常
                              │
                              ▼
                         僵尸态（Zombie）
                              │
                    父进程 waitpid 回收
                              │
                              ▼
                         资源释放，进程消亡
```

**僵尸进程**：进程退出后，其 PCB 和退出码仍然保留，等待父进程通过 `waitpid` 回收。如果父进程先退出，子进程会被挂到 `initproc` 下面，由 initproc 负责回收。

### 3.4 进程控制块（PCB）

在 tg-ch5 中，进程控制块由 `Process` 结构体表示：

```rust
pub struct Process {
    pub pid: ProcId,                                    // 进程标识符
    pub context: ForeignContext,                          // 用户态上下文 + satp
    pub address_space: AddressSpace<Sv39, Sv39Manager>,  // 独立地址空间
    pub heap_bottom: usize,                              // 堆底
    pub program_brk: usize,                              // 堆顶（sbrk）
}
```

与教科书中的 PCB 对比：

| 教科书 PCB 字段 | tg-ch5 对应 |
|----------------|-------------|
| PID | `pid: ProcId` |
| 寄存器状态 | `context.context: LocalContext` |
| 页表基地址 | `context.satp` |
| 地址空间 | `address_space: AddressSpace` |
| 父进程 / 子进程 | 由 `ProcManager` 维护 |
| 进程状态 | 由 `PManager` 管理 |
| 退出码 | 由 `PManager` 管理 |

### 3.5 进程管理器与调度

进程管理分为两层：

1. **ProcManager**：负责进程实体的存储和调度队列管理
   - `tasks: BTreeMap<ProcId, Process>`：所有进程实体
   - `ready_queue: VecDeque<ProcId>`：就绪队列（FIFO）

2. **PManager**（来自 `tg-task-manage` 库）：高层进程管理接口
   - `add()`：添加进程
   - `find_next()`：取出下一个就绪进程
   - `make_current_exited()`：标记当前进程退出
   - `make_current_suspend()`：暂停当前进程
   - `wait()`：等待子进程

当前调度算法是简单的 **FIFO / 时间片轮转**。练习题要求实现 **stride 调度算法**。

### 3.6 初始进程 initproc 和 Shell

**initproc** 是内核创建的第一个用户进程：

```
内核 rust_main
    │
    ▼
加载 initproc（ELF）
    │
    ▼
initproc 启动
    │
    ├── fork 子进程
    │      │
    │      ▼
    │   exec("user_shell")  →  Shell 启动
    │                              │
    │                         用户输入命令
    │                              │
    │                         fork + exec 执行命令
    │                              │
    │                         waitpid 等待命令完成
    │
    ▼
  loop { wait() }  // 回收僵尸进程
```

**Shell（user_shell）** 的工作流程：
1. 打印提示符 `>> `
2. 逐字符读取用户输入（通过 `read` 系统调用）
3. 用户按回车后，`fork` 出子进程
4. 子进程调用 `exec` 执行输入的程序名
5. 父进程调用 `waitpid` 等待子进程结束
6. 打印子进程的退出码，回到步骤 1

### 3.7 fork 的实现细节

fork 的核心是**深拷贝地址空间**。在 tg-ch5 中：

```rust
pub fn fork(&mut self) -> Option<Process> {
    let pid = ProcId::new();
    // 1. 复制父进程的完整地址空间
    let mut address_space = AddressSpace::new();
    self.address_space.cloneself(&mut address_space);
    // 2. 映射异界传送门
    map_portal(&address_space);
    // 3. 复制上下文（寄存器状态）
    let context = self.context.context.clone();
    let satp = (8 << 60) | address_space.root_ppn().val();
    // 4. 子进程的 a0 = 0（fork 返回值）
    // （由调用者设置）
    Some(Self { pid, context: ForeignContext { context, satp }, address_space, ... })
}
```

`cloneself` 方法会：
1. 遍历父进程地址空间的所有映射区域
2. 为子进程分配新的物理页面
3. 将父进程的页面数据逐页复制到子进程

### 3.8 exec 的实现细节

exec 替换当前进程的地址空间：

```rust
pub fn exec(&mut self, elf: ElfFile) {
    let proc = Process::from_elf(elf).unwrap();
    self.address_space = proc.address_space;  // 旧地址空间被释放
    self.context = proc.context;
    self.heap_bottom = proc.heap_bottom;
    self.program_brk = proc.program_brk;
}
```

关键点：
- PID 保持不变
- 旧地址空间的生命周期结束，所有物理页面被回收
- 从新的 ELF 创建全新的地址空间
- Trap 上下文重新初始化（入口地址、栈指针等）

### 3.9 waitpid 与资源回收

当进程调用 `exit` 退出时：
1. 标记为**僵尸态**（Zombie）
2. 将所有子进程挂到 initproc 下
3. 回收用户地址空间（物理页面）
4. 保留 PCB 和退出码（等待父进程回收）

父进程调用 `waitpid` 时：
1. 查找符合条件的僵尸子进程
2. 收集退出码（通过地址翻译写入用户空间）
3. 从进程表中删除子进程 PCB
4. 返回子进程 PID

---

## 四、代码解读

### 4.1 `src/main.rs` —— 内核主体

**启动流程 `rust_main`：**
1. 清零 BSS 段
2. 初始化控制台和日志
3. 初始化内核堆分配器（`tg_kernel_alloc`）
4. 分配并创建异界传送门
5. 建立内核地址空间（恒等映射 + 传送门映射），激活 Sv39 分页
6. 初始化系统调用处理器
7. 加载初始进程 `initproc`
8. 进入主调度循环

**主调度循环：**
- 不断从进程管理器取出就绪进程
- 通过异界传送门切换到用户地址空间执行
- Trap 返回后处理系统调用或异常
- 所有进程完成后关机

**系统调用实现（`impls` 模块）：**
- `IO`：write（地址翻译后输出）、read（SBI 读字符）
- `Process`：fork（深拷贝地址空间）、exec（替换地址空间）、wait（回收子进程）、getpid、spawn（TODO）、sbrk
- `Scheduling`：sched_yield、set_priority（TODO）
- `Clock`：clock_gettime（地址翻译后写入）
- `Memory`：mmap（TODO）、munmap（TODO）

### 4.2 `src/process.rs` —— 进程管理

**`Process::from_elf(elf)`**：从 ELF 创建进程
- 验证 ELF 头 → 创建地址空间 → 映射 LOAD 段 → 分配用户栈 → 创建 ForeignContext

**`Process::fork()`**：复制进程
- 深拷贝地址空间 → 映射传送门 → 复制上下文 → 分配新 PID

**`Process::exec(elf)`**：替换程序
- 从 ELF 创建新进程 → 替换地址空间和上下文 → 保留 PID

**`Process::change_program_brk(size)`**：实现 sbrk
- size > 0：扩展堆，映射新页面
- size < 0：收缩堆，取消映射
- 返回旧的 break 地址

### 4.3 `src/processor.rs` —— 处理器管理

**`Processor`**：全局处理器管理器
- 封装 `PManager`，提供 `get_mut()` 访问接口

**`ProcManager`**：进程管理器
- `tasks: BTreeMap`：进程实体存储
- `ready_queue: VecDeque`：FIFO 就绪队列
- 实现 `Manage` trait（insert/get_mut/delete）
- 实现 `Schedule` trait（add/fetch）

### 4.4 `Cargo.toml` —— 依赖说明

| 依赖 | 说明 |
|------|------|
| `xmas-elf` | ELF 文件格式解析库 |
| `riscv` | RISC-V CSR 寄存器访问（`satp`、`scause`） |
| `spin` | 自旋锁（`Lazy` 延迟初始化） |
| `tg-sbi` | SBI 调用封装（console、shutdown） |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出（`print!`/`println!`）和日志 |
| `tg-kernel-context` | 用户上下文及异界传送门（`foreign` feature） |
| `tg-kernel-alloc` | 内核堆分配器 |
| `tg-kernel-vm` | 虚拟内存管理（地址空间、页表） |
| `tg-syscall` | 系统调用定义与分发 |
| `tg-task-manage` | 进程管理框架（`proc` feature，支持进程树） |

---

## 五、编程练习

### 5.1 迁移 mmap 和 munmap

你仍需要迁移上一章的 `mmap` / `munmap` 以适应新的进程结构。

> **注意**：从本章节开始，不再要求维护 `trace` 这一系统调用。

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

### 5.2 进程创建：spawn 系统调用

大家一定好奇过为啥进程创建要用 fork + exec 这么一个奇怪的系统调用，就不能直接搞一个新进程吗？思而不学则殆，我们就来试一试！请实现一个完全 DIY 的系统调用 spawn，用以创建一个新进程。

spawn 系统调用定义（[标准 spawn 看这里](https://man7.org/linux/man-pages/man3/posix_spawn.3.html)）：

```rust
fn spawn(&self, _caller: Caller, path: usize, count: usize) -> isize
```

- syscall ID: 400
- 功能：新建子进程，使其执行目标程序。
- 参数：path 目标程序路径，count 路径长度
- 说明：成功返回子进程 id，否则返回 -1。
- 可能的错误：
  - 无效的文件名。

> **注意**：虽然测例很简单，但提醒读者 spawn **不必**像 fork 一样复制父进程的地址空间。spawn 直接从 ELF 创建新进程即可。

### 5.3 stride 调度算法

ch3 中我们实现的调度算法十分简单。现在我们要为我们的 OS 实现一种带优先级的调度算法：stride 调度算法。

**算法描述：**

1. 为每个进程设置一个当前 `stride`，表示该进程当前已经运行的"长度"。另外设置其对应的 `pass` 值（只与进程的优先权有关系），表示对应进程在调度后，stride 需要进行的累加值。

2. 每次需要调度时，从当前 runnable 态的进程中选择 stride 最小的进程调度。对于获得调度的进程 P，将对应的 stride 加上其对应的步长 pass。

3. 一个时间片后，回到上一步骤，重新调度当前 stride 最小的进程。

可以证明，如果令 `P.pass = BigStride / P.priority`，其中 `P.priority` 表示进程的优先权（大于 1），而 BigStride 表示一个预先定义的大常数，则该调度方案为每个进程分配的时间将与其优先级成正比。

**其他实验细节：**

- stride 调度要求进程优先级 >= 2，所以设定进程优先级 <= 1 会导致错误。
- 进程初始 stride 设置为 0 即可。
- 进程初始优先级设置为 16。

**set_priority 系统调用：**

```rust
fn set_priority(&self, _caller: Caller, prio: isize) -> isize
```

- syscall ID：140
- 设置当前进程优先级为 prio
- 参数：prio 进程优先级，要求 prio >= 2
- 返回值：如果输入合法则返回 prio，否则返回 -1

### 5.4 实现提示

- 你可以在 `Process` 中添加新字段（如 `stride`、`priority`）来支持优先级调度
- 为了减少整数除的误差，BigStride 一般需要很大，但为了不至于发生溢出反转现象，或许选择一个适中的数即可，当然能进行溢出处理就更好了
- stride 算法要找到 stride 最小的进程，使用优先级队列是效率不错的办法，但是我们的实验测例很简单，所以效率完全不是问题。事实上，很推荐使用暴力扫一遍的办法找最小值
- 注意设置进程的初始优先级
- spawn 不必像 fork 一样复制地址空间，可以直接调用 `Process::from_elf` 创建新进程

### 5.5 实验要求

**目录结构说明：**

```
tg-ch5/
├── Cargo.toml（内核配置文件）
├── src/（内核源代码，需要修改）
│   ├── main.rs（内核主函数，包括系统调用接口实现）
│   ├── process.rs（进程结构）
│   └── processor.rs（进程管理器和调度器）
└── tg-user/（用户程序，运行时自动拉取，无需修改）
    └── src/bin（测试用例）
```

> **说明**：
> - `tg-user` 会在运行时自动拉取到 `tg-ch5/tg-user` 目录下
> - 只需修改 `tg-ch5/src/` 目录下的内核代码

**运行和测试：**

运行练习测例：

```bash
cargo run --features exercise
```

然后在终端中输入 `ch5_usertest` 运行，这个测例打包了所有你需要通过的测例。你也可以通过修改这个文件调整本地测试的内容，或者单独运行某测例来纠正特定的错误。

测试练习测例：

```bash
./test.sh exercise
```

> **前向兼容**：从本章开始，你的内核必须前向兼容，需要能通过前一章的所有测例（除了 `ch3_trace` 和 `ch4_trace`）。

---

## 六、本章小结

通过本章的学习和实践，你完成了操作系统中最核心的抽象——进程：

1. **进程概念**：将"运行中的程序"封装为拥有独立资源的实体，通过 PID 标识
2. **fork 系统调用**：深拷贝父进程的地址空间创建子进程，父子通过返回值区分
3. **exec 系统调用**：替换当前进程的地址空间，加载新的 ELF 程序执行
4. **waitpid 系统调用**：等待子进程退出，回收资源，收集退出码
5. **进程树**：通过父子关系形成树状结构，initproc 负责回收孤儿进程
6. **Shell**：用户通过命令行界面与操作系统交互，动态创建和管理进程
7. **进程调度**：从简单的 FIFO/RR 到 stride 优先级调度

在后续章节中，我们将在进程的基础上引入**文件系统**，实现持久化存储和文件 I/O。

## 七、思考题

1. **fork 的效率问题？** fork 需要复制整个地址空间，如果进程占用大量内存，开销很大。现代操作系统如何优化这个问题？（提示：Copy-on-Write）

2. **为什么 fork + exec？** UNIX 为什么选择 fork + exec 的组合而不是直接 spawn？这种设计有什么优缺点？Windows 的 CreateProcess 与之有何不同？

3. **僵尸进程的问题？** 如果父进程不调用 waitpid，子进程退出后会一直是僵尸态。这会导致什么问题？initproc 如何解决孤儿进程问题？

4. **stride 调度的公平性？** 为什么 stride 调度能保证与优先级成正比的时间分配？如果 BigStride 太小会怎样？太大会怎样？

5. **spawn vs fork+exec？** spawn 相比 fork+exec 有什么优势？在什么场景下 fork+exec 更灵活？

## 参考资料

- [rCore-Tutorial-Guide 第五章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第五章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter5/index.html)
- [RISC-V Privileged Specification](https://riscv.org/specifications/privileged-isa/)
- [xv6-riscv: UNIX V6 教学操作系统](https://github.com/mit-pdos/xv6-riscv)
- [POSIX spawn(3)](https://man7.org/linux/man-pages/man3/posix_spawn.3.html)

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
