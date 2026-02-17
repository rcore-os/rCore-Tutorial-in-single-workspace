# 第七章：进程间通信与信号

本章在第六章"文件系统"的基础上，引入了两大新机制：

1. **管道（Pipe）**：基于文件描述符的进程间通信机制，用于父子进程之间的单向数据传递
2. **信号（Signal）**：异步事件通知机制，允许一个进程向另一个进程发送事件通知

通过本章的学习和实践，你将理解：

- 管道的概念、创建和使用流程
- 管道的内部实现：环形缓冲区（Ring Buffer）
- 统一的文件描述符抽象：文件、管道、标准 I/O 共享同一套接口
- 信号的概念：信号集、信号屏蔽字、信号处理函数
- 信号的发送（kill）、注册（sigaction）、屏蔽（sigprocmask）和返回（sigreturn）
- 命令行参数的传递和 I/O 重定向的原理

> **前置知识**：建议先完成第一章至第六章的学习，理解裸机启动、Trap 处理、系统调用、多任务调度、虚拟内存、进程管理和文件系统。

## 项目结构

```
ch7/
├── .cargo/
│   └── config.toml     # Cargo 配置：交叉编译目标和 QEMU runner
├── .gitignore           # Git 忽略规则
├── build.rs            # 构建脚本：编译用户程序，打包 easy-fs 磁盘镜像
├── Cargo.toml          # 项目配置与依赖
├── LICENSE             # GPL v3 许可证
├── README.md           # 本文档
├── rust-toolchain.toml # Rust 工具链配置
├── test.sh             # 自动测试脚本
└── src/
    ├── main.rs         # 内核主体：初始化、调度循环、系统调用实现（含信号处理）
    ├── fs.rs           # 文件系统管理 + 统一的 Fd 枚举
    ├── process.rs      # 进程结构：含 fd_table 和 signal
    ├── processor.rs    # 处理器管理：进程管理器
    └── virtio_block.rs # VirtIO 块设备驱动
```

<a id="source-nav"></a>

## 源码阅读导航索引

[返回根文档导航总表](../README.md#chapters-source-nav-map)

本章建议按“统一 fd 抽象 -> 管道数据流 -> 信号状态机”三条线并行阅读。

| 阅读顺序 | 文件 | 重点问题 |
|---|---|---|
| 1 | `src/fs.rs` | 为什么要用 `Fd` 枚举统一文件/管道/标准 I/O？ |
| 2 | `src/main.rs` 的 `impls::IO` | `pipe`、`read`、`write` 如何共享同一套 fd_table 入口？ |
| 3 | `src/process.rs` | 进程信号状态在 `from_elf`、`fork`、`exec` 中如何继承/重置？ |
| 4 | `src/main.rs` Trap 主循环 | 为什么在 syscall 返回前处理信号，处理结果如何影响进程存活？ |
| 5 | `src/main.rs` 的 `impls::Signal` | `kill/sigaction/sigprocmask/sigreturn` 的内核语义是什么？ |

配套建议：结合 `tg-signal` / `tg-signal-impl` 注释阅读，重点关注 `handle_signals` 的决策流程。

## DoD 验收标准（本章完成判据）

- [ ] 能解释为什么要用统一 `Fd` 枚举承载“文件/管道/标准IO”
- [ ] 能从代码说明 `pipe` 建立后父子进程如何通过读写端通信
- [ ] 能说明信号的发送、注册、屏蔽、返回四个关键 syscall 语义
- [ ] 能解释“syscall 返回前检查信号”对进程退出路径的影响
- [ ] 能执行 `./test.sh base` 并通过本章基础测试

## 概念-源码-测试三联表

| 核心概念 | 源码入口 | 自测方式（命令/现象） |
|---|---|---|
| 统一 fd 抽象 | `ch7/src/fs.rs` 的 `Fd` 枚举 | 同一 `read/write` 入口可操作文件与管道 |
| 管道通信 | `ch7/src/main.rs` 的 `impls::IO::pipe` | 运行 `ch7b_pipetest` 可观察父子通信成功 |
| 信号状态管理 | `ch7/src/process.rs` 的 `signal` 字段与 `fork` 继承 | 子进程信号处理配置符合预期 |
| 信号 syscall | `ch7/src/main.rs` 的 `impls::Signal` | `kill/sigaction/sigprocmask/sigreturn` 行为正确 |

遇到构建/运行异常可先查看根文档的“高频错误速查表”。

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
cargo clone tg-ch7
cd tg-ch7
```

**方式二：获取所有实验**

```bash
git clone https://github.com/rcore-os/tg-rcore-tutorial.git
cd tg-rcore-tutorial/ch7
```

## 二、编译与运行

### 2.1 编译

```bash
cargo build
```

构建过程与第六章相同：`build.rs` 会自动下载编译 `tg-user` 用户程序，打包到 `fs.img` 磁盘镜像中。

> 环境变量说明：
> - `TG_USER_DIR`：指定本地 tg-user 源码路径
> - `TG_USER_VERSION`：指定 tg-user 版本（默认 `0.2.0-preview.1`）
> - `TG_SKIP_USER_APPS`：跳过用户程序编译
> - `LOG`：设置日志级别

### 2.2 运行

```bash
cargo run
```

QEMU 命令与第六章相同（挂载 fs.img 块设备）：

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -drive file=target/riscv64gc-unknown-none-elf/debug/fs.img,if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch7
```

### 2.3 预期输出

```
[tg-ch7 ...] Hello, world!
[ INFO] .text    ---> 0x80200000..0x8023xxxx
[ INFO] .rodata  ---> 0x8023xxxx..0x8024xxxx
[ INFO] .data    ---> 0x8024xxxx..0x81exxxxx
[ INFO] .boot    ---> 0x81exxxxx..0x81exxxxx
[ INFO] (heap)   ---> 0x81exxxxx..0x83200000
[ INFO] MMIO range -> 0x10001000..0x10002000

Rust user shell
>> ch7b_pipetest
Read OK, child process exited!
pipetest passed!
Shell: Process 2 exited with code 0
>>
```

你可以在 Shell 中运行管道测试程序：
- `ch7b_pipetest`：父进程通过管道向子进程传递字符串
- `ch7b_pipe_large_test`：通过两个管道实现父子进程双向通信

### 2.4 运行测试

```bash
./test.sh           # 运行基础测试
./test.sh base      # 等价于上面
./test.sh all       # 运行全部测试（ch7 仅有基础测试）
```

---

## 三、操作系统核心概念

### 3.1 为什么需要进程间通信？

在前几章中，各个进程是完全独立的，它们无法直接交换数据。但在实际操作系统中，进程协作是非常常见的需求：

| 场景 | 说明 |
|------|------|
| 管道命令 | `cat file.txt \| grep "hello"` —— 前一个进程的输出作为后一个进程的输入 |
| 服务通信 | Web 服务器将请求转发给后端处理进程 |
| 父子协作 | 父进程 fork 子进程后通过管道交换数据 |

**管道** 是最基本的 IPC 机制之一，它提供了一种简单的单向数据通道。

### 3.2 管道（Pipe）

#### 管道的基本概念

管道是一对文件描述符：**读端**（read end）和**写端**（write end），数据从写端流向读端。

```text
┌──────────┐     ┌──────────────────────────┐     ┌──────────┐
│ 父进程    │     │         管道              │     │ 子进程    │
│          │     │  ┌──────────────────────┐ │     │          │
│ write(fd) ──→  │  │   环形缓冲区          │ │  ──→ read(fd) │
│          │     │  │  [H|e|l|l|o|_|_|_|_] │ │     │          │
│          │     │  └──────────────────────┘ │     │          │
└──────────┘     └──────────────────────────┘     └──────────┘
    写端                                               读端
```

#### 管道的创建和使用

典型的管道使用流程：

```text
1. 父进程调用 pipe() 创建管道
   ┌─────────────────────┐
   │ fd_table:            │
   │   ...                │
   │   [3] = PipeRead     │  ← 读端
   │   [4] = PipeWrite    │  ← 写端
   └─────────────────────┘

2. 父进程调用 fork() 创建子进程
   父进程和子进程都拥有 fd[3]（读端）和 fd[4]（写端）

3. 子进程关闭写端 close(4)，父进程关闭读端 close(3)
   父进程：write(4, "Hello")  →  管道  →  子进程：read(3, buf)

4. 通信完毕后，各自关闭剩余的 fd
```

#### 管道的内部实现：环形缓冲区

管道内部使用**环形缓冲区**（Ring Buffer）存储数据：

```text
PipeRingBuffer:
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ A │ B │ C │   │   │   │   │   │
└───┴───┴───┴───┴───┴───┴───┴───┘
  ↑head          ↑tail
  读取位置       写入位置

状态：
- EMPTY：head == tail 且已知为空
- FULL：head == tail 且已知为满
- NORMAL：其他情况
```

关键行为：
- **读取（read）**：从 head 位置读取，head 向前移动。如果缓冲区为空，暂停当前进程等待数据。
- **写入（write）**：向 tail 位置写入，tail 向前移动。如果缓冲区满，暂停当前进程等待读取。
- **所有写端关闭**：缓冲区中残余数据读完后，read 返回 0 表示 EOF。

#### pipe 系统调用

```rust
/// 创建管道
/// pipe: 用户空间的 usize[2] 数组地址
/// 返回：pipe[0] = 读端 fd，pipe[1] = 写端 fd
fn pipe(&self, pipe: usize) -> isize
```

### 3.3 统一的文件描述符类型（Fd 枚举）

本章引入了 `Fd` 枚举，将所有文件描述符类型统一管理：

```rust
pub enum Fd {
    File(FileHandle),           // 普通磁盘文件
    PipeRead(PipeReader),       // 管道读端
    PipeWrite(Arc<PipeWriter>), // 管道写端
    Empty { read, write },      // 空描述符（stdin/stdout/stderr）
}
```

统一接口的好处：
- read/write 系统调用不需要知道 fd 的具体类型
- 文件和管道可以混合使用（如 I/O 重定向）
- fork 时子进程自动继承所有类型的 fd

### 3.4 信号（Signal）

#### 信号的基本概念

信号是**异步事件通知**机制。进程可以在任意时刻收到信号，类似于硬件中断对 CPU 的打断。

常见信号：

| 信号 | 编号 | 默认行为 | 说明 |
|------|------|----------|------|
| SIGKILL | 9 | 终止进程 | 不可捕获、不可忽略 |
| SIGINT | 2 | 终止进程 | Ctrl+C |
| SIGTERM | 15 | 终止进程 | 请求终止 |
| SIGCHLD | 17 | 忽略 | 子进程状态变化 |
| SIGUSR1/2 | 10/12 | 终止进程 | 用户自定义 |

#### 信号处理流程

```text
进程 A                          进程 B
  │                               │
  │── kill(pid_B, SIGUSR1) ──→    │
  │                               │ （信号加入 received 位图）
  │                               │
  │                               │ ← syscall 返回前检查信号
  │                               │
  │                               │ 调用注册的信号处理函数
  │                               │ （或执行默认行为）
  │                               │
  │                               │ ← sigreturn 恢复原执行流
```

#### 信号相关数据结构

每个进程的信号处理器（`SignalImpl`）维护：

```rust
pub struct SignalImpl {
    received: SignalSet,                        // 已接收的信号（位图）
    mask: SignalSet,                            // 信号屏蔽字（被屏蔽的信号不处理）
    handling: Option<HandlingSignal>,           // 正在处理的信号状态
    actions: [Option<SignalAction>; MAX_SIG+1], // 信号处理函数表
}
```

#### 信号相关系统调用

| 系统调用 | 功能 |
|----------|------|
| `kill(pid, signum)` | 向指定进程发送信号 |
| `sigaction(signum, action, old_action)` | 设置/获取信号处理函数 |
| `sigprocmask(mask)` | 更新信号屏蔽字 |
| `sigreturn()` | 从信号处理函数返回，恢复原上下文 |

#### 信号处理时机

在本实现中，信号处理发生在**系统调用返回用户态之前**：

```rust
// 系统调用执行完毕后
match task.signal.handle_signals(ctx) {
    SignalResult::ProcessKilled(exit_code) => {
        // 收到终止信号，进程退出
    },
    _ => {
        // 正常返回系统调用结果
    }
}
```

### 3.5 命令行参数与 I/O 重定向

虽然本 tg-ch7 实现未完整支持命令行参数传递，但这是第七章教学内容的重要部分。

#### 命令行参数

```text
exec("program", ["arg1", "arg2"])

用户栈布局：
┌──────────────┐ 高地址
│ argv[0] 指针  │ → "program\0"
│ argv[1] 指针  │ → "arg1\0"
│ argv[2] 指针  │ → "arg2\0"
│ 0 (终止符)    │
├──────────────┤
│ "program\0"  │
│ "arg1\0"     │
│ "arg2\0"     │
├──────────────┤ ← user_sp（对齐到 8 字节）
│   ...        │
└──────────────┘ 低地址
```

进入用户态时，`a0 = argc`（参数个数），`a1 = argv`（参数数组指针）。

#### I/O 重定向

I/O 重定向通过文件描述符的"关闭-复制"技巧实现：

```text
输出重定向：program > output.txt

1. fork() 创建子进程
2. 子进程中：
   fd = open("output.txt", CREATE|WRONLY)  // 假设 fd = 3
   close(1)                                 // 关闭 stdout
   dup(3)                                   // 复制 fd 3 → 得到 fd 1
   close(3)                                 // 关闭原始 fd
   exec("program")                          // 程序的 stdout 现在指向 output.txt
```

### 3.6 系统调用汇总

| syscall ID | 名称 | 功能 | 状态 |
|-----------|------|------|------|
| 59 | `pipe` | 创建管道 | **新增** |
| 129 | `kill` | 发送信号 | **新增** |
| 134 | `sigaction` | 设置信号处理 | **新增** |
| 135 | `sigprocmask` | 设置屏蔽字 | **新增** |
| 139 | `sigreturn` | 信号返回 | **新增** |
| 56 | `open` | 打开文件 | 继承 |
| 57 | `close` | 关闭 fd | 继承 |
| 63 | `read` | 读取（**扩展**：支持管道） | 扩展 |
| 64 | `write` | 写入（**扩展**：支持管道） | 扩展 |
| 93 | `exit` | 退出进程 | 继承 |
| 220 | `fork` | 创建子进程（**扩展**：继承信号配置） | 扩展 |
| 221 | `exec` | 替换程序 | 继承 |
| 260 | `wait` | 等待子进程 | 继承 |

---

## 四、代码解读

### 4.1 `src/main.rs` —— 内核主体

**与第六章的区别：**
- 新增 `tg_syscall::init_signal()` 初始化信号系统调用
- 主调度循环中，系统调用返回前新增**信号处理**：
  - `SignalResult::ProcessKilled`：进程被终止信号杀死
  - 其他情况：正常处理系统调用返回值
- `impls` 模块新增 `Signal` trait 实现（kill/sigaction/sigprocmask/sigreturn）
- `impls` 模块新增 `pipe` 系统调用实现

### 4.2 `src/fs.rs` —— 文件系统管理 + Fd 枚举

**核心变化：统一的 `Fd` 枚举**
- `Fd::File(FileHandle)`：普通磁盘文件
- `Fd::PipeRead(PipeReader)`：管道读端
- `Fd::PipeWrite(Arc<PipeWriter>)`：管道写端
- `Fd::Empty { read, write }`：空描述符（stdin/stdout/stderr）

所有类型都实现了 `readable()`、`writable()`、`read()`、`write()` 方法。

### 4.3 `src/process.rs` —— 进程管理

**与第六章的区别：**
- `fd_table` 类型从 `Vec<Option<Mutex<FileHandle>>>` 变为 `Vec<Option<Mutex<Fd>>>`
- 新增 `signal: Box<dyn Signal>` 字段
- `fork()` 通过 `self.signal.from_fork()` 让子进程继承信号配置
- `from_elf()` 使用 `Fd::Empty` 初始化 stdin/stdout/stderr

### 4.4 `Cargo.toml` —— 依赖说明

与第六章相比新增的依赖：

| 依赖 | 说明 |
|------|------|
| `tg-signal` | 信号模块 trait 定义（Signal、SignalResult、SignalNo） |
| `tg-signal-impl` | 信号模块参考实现（SignalImpl） |

**信号 crate 依赖关系：**

```text
tg-ch7
  ├── tg-signal-impl → tg-signal → tg-signal-defs → numeric-enum-macro
  ├── tg-signal
  └── tg-syscall → tg-signal-defs
```

---

## 五、本章小结

通过本章的学习和实践，你完成了操作系统中进程间通信的核心机制：

1. **管道机制**：基于环形缓冲区的单向数据通道，通过 pipe 系统调用创建
2. **统一文件描述符**：Fd 枚举将文件、管道、标准 I/O 统一为一套接口
3. **信号机制**：异步事件通知，支持信号发送、处理函数注册、屏蔽和返回
4. **命令行参数**：通过用户栈传递参数给程序
5. **I/O 重定向**：通过文件描述符的关闭-复制技巧实现

## 六、思考题

1. **管道 vs 共享内存**：管道通过内核缓冲区传递数据，而共享内存允许进程直接访问同一块物理内存。各有什么优缺点？什么场景下应该使用哪种？

2. **管道的阻塞语义**：当管道缓冲区为空时，读取操作应该阻塞等待。在本实现中，这种"阻塞"是如何实现的？（提示：任务切换）

3. **信号 vs 管道**：信号和管道都是 IPC 机制，但它们的适用场景有何不同？信号能传递数据吗？

4. **信号处理时机**：本实现中信号在系统调用返回前处理。如果进程正在执行用户态代码（没有系统调用），信号什么时候被处理？理想的实现应该是什么样的？

5. **fork 后的管道共享**：fork 后父子进程共享管道的读端和写端。为什么通常需要在子进程中关闭不需要的一端？如果不关闭会有什么问题？

6. **环形缓冲区的大小**：管道的环形缓冲区大小是固定的。如果写入的数据量超过缓冲区大小会发生什么？Linux 中管道的默认缓冲区大小是多少？

## 参考资料

- [rCore-Tutorial-Guide 第七章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第七章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter7/index.html)
- [Linux pipe(2) man page](https://man7.org/linux/man-pages/man2/pipe.2.html)
- [Linux signal(7) man page](https://man7.org/linux/man-pages/man7/signal.7.html)
- [UNIX 进程间通信](https://en.wikipedia.org/wiki/Inter-process_communication)

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
