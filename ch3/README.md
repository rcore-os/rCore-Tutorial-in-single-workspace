# 第三章：多道程序与分时多任务

本章在第二章"批处理系统"的基础上，实现了一个**多道程序操作系统**（tg-ch3）。它支持多个用户程序同时驻留在内存中并发执行，通过时钟中断实现**抢占式调度**，通过 `yield` 系统调用支持**协作式调度**，并引入了时间管理功能。

通过本章的学习和实践，你将理解：

- 什么是多道程序系统，它与批处理系统有何区别
- 任务控制块（TCB）如何管理任务的状态和上下文
- 协作式调度与抢占式调度的原理和实现
- 时钟中断的工作机制和时间片轮转算法
- 新增的系统调用：`yield`（让出 CPU）和 `clock_gettime`（获取时间）

> **前置知识**：建议先完成第一章（tg-ch1）和第二章（tg-ch2）的学习，理解裸机启动、Trap 处理、系统调用等基础概念。

## 项目结构

```
ch3/
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
    ├── main.rs         # 内核源码：多道程序主循环、Trap 处理、系统调用
    └── task.rs         # 任务控制块（TCB）和调度事件定义
```

<a id="source-nav"></a>

## 源码阅读导航索引

[返回根文档导航总表](../README.md#chapters-source-nav-map)

本章建议按“任务模型 -> 调度循环 -> 时钟中断/系统调用”顺序阅读。

| 阅读顺序 | 文件 | 重点问题 |
|---|---|---|
| 1 | `src/task.rs` | `TaskControlBlock` 如何封装上下文、栈和任务状态？ |
| 2 | `src/main.rs` 的主循环 | 轮转调度如何在多任务之间切换？ |
| 3 | 时钟中断分支 | 抢占式调度中，时间片到期后发生了什么？ |
| 4 | `yield` 与 syscall 分支 | 协作式让出与普通 syscall 返回路径有何区别？ |

配套建议：结合 `tg-sbi::set_timer` 与 `clock_gettime` 实现，串起“硬件时钟 -> 内核调度 -> 用户可见时间”的链路。

## DoD 验收标准（本章完成判据）

- [ ] 能运行 `cargo run` 并说明抢占式调度（时钟中断）发生的证据
- [ ] 能运行 `cargo run --features coop` 并说明协作式调度与抢占式差异
- [ ] 能解释 `TaskControlBlock` 中“上下文/栈/完成状态”的作用
- [ ] 能从 Trap 分支区分 `SupervisorTimer` 与 `UserEnvCall` 两类事件
- [ ] 能完成 `./test.sh base`（以及练习时 `./test.sh exercise`）

## 概念-源码-测试三联表

| 核心概念 | 源码入口 | 自测方式（命令/现象） |
|---|---|---|
| 任务控制块（TCB） | `ch3/src/task.rs` | 能说清 `init/execute/handle_syscall` 的职责 |
| 抢占式调度 | `ch3/src/main.rs` 的时钟中断分支 | 日志出现 timeout/轮转切换行为 |
| 协作式调度 | `ch3/src/main.rs` 的 `Event::Yield` 分支 | `--features coop` 下由用户主动让出 CPU |
| 时间系统调用 | `ch3/src/main.rs` 的 `Clock` 实现 | 用户态 `clock_gettime` 返回时间单调递增 |

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

tg-ch3 的构建脚本需要 `cargo-clone`（用于自动下载用户程序 crate）和 `rust-objcopy`（用于将 ELF 转为二进制）：

```bash
cargo install cargo-clone
# rust-objcopy 由 cargo-binutils 提供
cargo install cargo-binutils
rustup component add llvm-tools
```

### 1.5 获取源代码

**方式一：只获取本实验**

```bash
cargo clone tg-ch3
cd tg-ch3
```

**方式二：获取所有实验**

```bash
git clone https://github.com/rcore-os/tg-rcore-tutorial.git
cd tg-rcore-tutorial/ch3
```

## 二、编译与运行

### 2.1 编译

在 `ch3`（或 `tg-ch3`）目录下执行：

```bash
cargo build
```

编译过程与第二章类似，`build.rs` 会自动完成以下工作：

1. **生成链接脚本**：使用 `tg_linker::NOBIOS_SCRIPT` 生成内核的内存布局
2. **下载用户程序**：自动通过 `cargo clone` 获取 `tg-user` crate（包含用户测试程序）
3. **编译用户程序**：根据 `cases.toml` 中的 `ch3` 或 `ch3_exercise` 配置，为每个用户程序交叉编译
4. **生成 APP_ASM**：生成汇编文件，将所有用户程序的二进制数据内联到内核镜像中

> 环境变量说明：
> - `TG_USER_DIR`：指定本地 tg-user 源码路径（跳过自动下载）
> - `TG_USER_VERSION`：指定 tg-user 版本（默认 `0.2.0-preview.1`）
> - `TG_SKIP_USER_APPS`：设置后跳过用户程序编译（生成空的占位 APP_ASM）
> - `LOG`：设置日志级别（如 `LOG=INFO`、`LOG=TRACE`，默认为 `info`）

### 2.2 运行

**默认模式（抢占式调度）：**

```bash
cargo run
```

**协作式调度模式：**

```bash
cargo run --features coop
```

启用 `coop` feature 后，禁用时钟中断抢占，任务只能通过 `yield` 主动让出 CPU。

**练习模式：**

```bash
cargo run --features exercise
```

加载练习专用的测试用例（包含 `sys_trace` 相关测试）。

实际执行的 QEMU 命令等价于：

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch3
```

### 2.3 预期输出

```
[tg-ch3 0.3.0-preview.1] Hello, world!
[ INFO] .data [0x802xxxxx, 0x802xxxxx)
[ WARN] boot_stack top=bottom=0x802xxxxx, lower_bound=0x802xxxxx
[ERROR] .bss [0x802xxxxx, 0x802xxxxx)
[ INFO] load app0 to 0x802xxxxx
[ INFO] load app1 to 0x802xxxxx
[ INFO] load app2 to 0x802xxxxx
...

power_3 [10000/200000]
power_3 [20000/200000]
...
power_3 [200000/200000]
3^200000 = 871008973(MOD 998244353)
Test power_3 OK!
...
AAAAAAAAAA [1/5]
BBBBBBBBBB [1/5]
CCCCCCCCCC [1/5]
...（交替输出，体现时间片轮转）
Test write A OK!
Test write B OK!
Test write C OK!
...
Test sleep OK!
```

与第二章的串行输出不同，你会观察到多个用户程序的输出交替出现（如 power_3、power_5、write_a、write_b 交错），这就是抢占式调度的效果——时钟中断强制切换任务，实现了时间片轮转。

### 2.4 运行测试

```bash
./test.sh           # 运行全部测试（基础 + 练习）
./test.sh base      # 仅运行基础测试
./test.sh exercise  # 仅运行练习测试
```

测试脚本会同时在终端显示 `cargo run` 的完整输出，并通过 `tg-checker` 自动验证输出是否符合预期。

---

## 三、操作系统核心概念

### 3.1 从批处理到多道程序

**第二章的批处理系统**串行执行用户程序：一个程序运行完毕后才加载下一个。这种方式的缺点是：当一个程序等待 I/O 或主动暂停时，CPU 处于空闲状态，造成资源浪费。

**多道程序系统**（Multiprogramming）解决了这个问题：

```
批处理系统                          多道程序系统
┌──────────────────┐               ┌──────────────────┐
│ App0 ██████████  │               │ App0 ██  ██  ██  │
│ App1     ████████│               │ App1   ██  ██  ██│
│ App2         ████│               │ App2  ██  ██  ██ │
│      ──────→ 时间│               │      ──────→ 时间 │
│  串行执行，CPU   │               │  交替执行，CPU     │
│  利用率低        │               │  利用率高         │
└──────────────────┘               └──────────────────┘
```

核心改进：
- **一次性加载**：所有用户程序在启动时同时加载到内存，减少切换开销
- **任务切换**：内核可以在多个任务之间快速切换，保证每个任务都能得到执行
- **调度算法**：决定何时切换、切换到哪个任务

### 3.2 任务控制块（TCB）

**任务控制块**（Task Control Block, TCB）是内核管理任务的核心数据结构。在 tg-ch3 中，每个 TCB 包含：

| 字段 | 类型 | 说明 |
|------|------|------|
| `ctx` | `LocalContext` | 用户态上下文（所有通用寄存器 + CSR） |
| `finish` | `bool` | 任务是否已完成 |
| `stack` | `[usize; 1024]` | 独立的用户栈（8 KiB） |

与第二章相比，本章将用户上下文和栈空间封装到 TCB 中，使得多个任务可以独立管理，互不干扰。

**任务状态变化：**

```
          init()
  [未初始化] ──→ [就绪]
                   │
           execute()│
                   ▼
                [运行中]
               ╱   │    ╲
         yield/  exit/   异常/
         超时   退出   被杀死
             ╱     │      ╲
           ▼      ▼      ▼
        [就绪] [已完成] [已完成]
```

### 3.3 任务切换机制

任务切换是操作系统的核心机制。tg-ch3 使用 `tg-kernel-context` 库中的 `LocalContext` 实现：

1. **保存当前任务上下文**：将所有用户寄存器保存到当前 TCB 的 `ctx` 中
2. **恢复目标任务上下文**：从目标 TCB 的 `ctx` 中恢复用户寄存器
3. **切换执行**：通过 `sret` 指令返回到目标任务的用户态

```
当前任务（App A）                    下一个任务（App B）
     │                                    ▲
     ▼                                    │
  触发 Trap                           sret 返回
     │                                    ▲
     ▼                                    │
  保存 A 的上下文到 TCB[A]     恢复 B 的上下文从 TCB[B]
     │                                    ▲
     └──────── 内核调度决策 ───────────────┘
```

与第二章的 Trap 处理相比，本章增加了"不结束当前任务但切换到下一个"的逻辑。

### 3.4 协作式调度（yield）

**协作式调度**依赖任务主动让出 CPU。用户程序调用 `yield` 系统调用，告诉内核"我暂时不需要 CPU 了，可以去执行别的任务"。

**典型使用场景**：当程序需要等待外设完成 I/O 操作时，与其忙等浪费 CPU 时间，不如 yield 让出 CPU 给其他任务。

```
App A 发起 I/O 请求                App B 在运行
     │                                 │
     ├─ 调用 yield                     │
     │   （ecall，a7=124）             │
     │                                 │
     ▼                                 ▼
  内核处理：                        继续执行
  标记 A 为"就绪"                       │
  切换到 B                             │
     │                                 │
     ...（一段时间后轮转回 A）...       │
     │                                 │
     ▼                                 │
  A 继续执行                           │
  检查 I/O 是否完成                    │
```

在 tg-ch3 中，启用 `coop` feature 可以体验纯协作式调度——时钟中断被禁用，任务只能通过 yield 主动让出 CPU。

### 3.5 抢占式调度（时钟中断）

**协作式调度的问题**：如果一个任务永远不调用 yield（例如进入死循环），其他任务就永远得不到执行。

**抢占式调度**通过时钟中断解决这个问题：

```
App A 正在执行（可能是死循环）
     │
     │  ← 12500 个时钟周期后
     │     时钟中断触发！
     ▼
  硬件自动陷入 S-mode
     │
     ▼
  scause = Interrupt::SupervisorTimer
     │
     ▼
  内核处理：
  1. 清除时钟中断（set_timer(u64::MAX)）
  2. 切换到下一个任务
     │
     ▼
  App B 开始执行
```

**关键代码逻辑：**

```rust
// 每次切换到用户程序前，设置时钟中断
tg_sbi::set_timer(time::read64() + 12500);
unsafe { tcb.execute() };

// Trap 返回后判断原因
match scause::read().cause() {
    Trap::Interrupt(Interrupt::SupervisorTimer) => {
        tg_sbi::set_timer(u64::MAX);  // 清除中断
        false  // 不结束任务，切换到下一个
    }
    // ...
}
```

**时间片轮转算法（Round-Robin）：**

tg-ch3 使用最简单的轮转算法：维护一个任务索引 `i`，每次时钟中断后 `i = (i + 1) % n`，循环执行各任务。每个任务获得相等的时间片（12500 个时钟周期 ≈ 1ms，在 QEMU 12.5MHz 时钟下）。

### 3.6 时钟中断的实现

RISC-V 的时钟中断机制：

| 组件 | 说明 |
|------|------|
| `mtime` 寄存器 | 硬件计数器，持续递增 |
| `mtimecmp` 寄存器 | 比较值，当 `mtime >= mtimecmp` 时触发中断 |
| `sie.stie` | S 特权级时钟中断使能位 |
| `set_timer()` | 通过 SBI 调用设置 `mtimecmp` |

初始化步骤：
1. `unsafe { sie::set_stimer() }` —— 开启 S 特权级时钟中断
2. 每次执行用户程序前：`set_timer(time::read64() + interval)` —— 设置下次中断时间

时钟中断到达后：
1. 硬件自动陷入 S-mode
2. `scause = Interrupt::SupervisorTimer`
3. 内核清除中断并切换任务

### 3.7 系统调用

tg-ch3 在第二章的基础上新增了 `yield` 和 `clock_gettime` 两个系统调用：

| syscall ID | 名称 | 功能 |
|-----------|------|------|
| 64 | `write` | 将缓冲区数据写入文件描述符（fd=1 为标准输出） |
| 93 | `exit` | 退出当前任务 |
| 124 | `sched_yield` | 主动让出 CPU，切换到下一个任务 |
| 113 | `clock_gettime` | 获取当前时间（纳秒精度） |
| 410 | `trace` | 追踪系统调用信息（**练习题**，需自行实现） |

**clock_gettime 的实现原理：**

```
用户程序调用 clock_gettime(CLOCK_MONOTONIC, &ts)
       │
       ▼
内核读取 RISC-V time 寄存器
       │
       ▼
将 tick 数转换为纳秒：time * 10000 / 125 = time * 80 ns
       │
       ▼
填充 TimeSpec { tv_sec, tv_nsec } 写回用户空间
```

### 3.8 调度事件机制

tg-ch3 引入了 `SchedulingEvent` 枚举来统一描述系统调用的调度效果：

| 事件 | 含义 | 触发条件 |
|------|------|---------|
| `None` | 继续执行当前任务 | write、clock_gettime 等普通系统调用 |
| `Yield` | 切换到下一个任务 | yield 系统调用 |
| `Exit(code)` | 任务退出 | exit 系统调用 |
| `UnsupportedSyscall(id)` | 杀死任务 | 不支持的系统调用 |

这种设计将系统调用的处理逻辑（在 `handle_syscall` 中）与调度决策（在主循环中）清晰分离。

---

## 四、代码解读

### 4.1 `src/main.rs` —— 内核主体

程序结构分为五个部分：

**模块文档与属性（第 1-22 行）：**
与第二章相同的 `#![no_std]`、`#![no_main]` 和条件编译属性。新增了 `mod task` 引入任务管理模块。

**外部依赖引入（第 30-41 行）：**
与第二章类似，但增加了 `task::TaskControlBlock` 的引用。

**启动与初始化（第 45-56 行）：**
- `global_asm!(include_str!(env!("APP_ASM")))`：嵌入用户程序
- `APP_CAPACITY = 32`：最大支持 32 个应用
- `boot0!(rust_main; stack = (APP_CAPACITY + 2) * 8192)`：分配 272 KiB 内核栈（因为 TCB 中包含用户栈）

**内核主函数 `rust_main`（第 60-153 行）：**

核心的多道程序循环：

```rust
// 初始化 → 加载所有应用到 TCB 数组
// → 开启时钟中断
// → 轮转执行：
while remain > 0 {
    if !tcb.finish {
        set_timer(...);        // 设置时间片
        tcb.execute();         // 切换到 U-mode
        match scause {
            Timer     → 切换到下一个任务
            UserEnvCall → 处理系统调用
            Exception → 杀死任务
        }
    }
    i = (i + 1) % n;         // 轮转到下一个
}
shutdown()
```

**接口实现模块 `impls`（第 164-237 行）：**
在第二章的 `Console`、`IO`、`Process` 基础上，新增了：
- `Scheduling`：处理 yield 系统调用
- `Clock`：处理 clock_gettime 系统调用
- `Trace`：练习题的占位实现

### 4.2 `src/task.rs` —— 任务管理

定义了两个核心类型：

**`TaskControlBlock`**：任务控制块
- `init(entry)` —— 创建用户态上下文，分配独立用户栈
- `execute()` —— 切换到 U-mode 执行
- `handle_syscall()` —— 处理系统调用并返回调度事件

**`SchedulingEvent`**：调度事件枚举
- `None` / `Yield` / `Exit(code)` / `UnsupportedSyscall(id)`

### 4.3 `build.rs` —— 构建脚本

与第二章结构相同，但根据 `exercise` feature 选择不同的测试用例集：

```rust
let case_key = if env::var("CARGO_FEATURE_EXERCISE").is_ok() {
    "ch3_exercise"   // 练习模式测例
} else {
    "ch3"            // 基础模式测例
};
```

| 函数 | 功能 |
|------|------|
| `write_linker()` | 生成链接脚本 |
| `ensure_tg_user()` | 确保 tg-user 源码可用（本地或 cargo clone） |
| `build_apps()` | 读取 cases.toml 配置，编译所有用户程序 |
| `build_user_app()` | 编译单个用户程序 |
| `objcopy_to_bin()` | 将 ELF 转为纯二进制 |
| `write_app_asm()` | 生成汇编文件，嵌入用户程序二进制 |
| `write_dummy_app_asm()` | 生成空的占位汇编（用于 publish --dry-run） |

### 4.4 `Cargo.toml` —— 配置与依赖

**Features：**

| Feature | 说明 |
|---------|------|
| `coop` | 协作式调度：禁用时钟中断，任务需主动 yield |
| `exercise` | 练习模式：加载练习测例 |

**Dependencies：**

| 依赖 | 说明 |
|------|------|
| `riscv` | RISC-V CSR 寄存器访问（`sie`、`scause`、`time`） |
| `tg-sbi` | SBI 调用封装，包括 `set_timer` 设置时钟中断 |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出（`print!` / `println!`）和日志 |
| `tg-kernel-context` | 用户上下文 `LocalContext`，实现特权级切换 |
| `tg-syscall` | 系统调用定义与分发（含 `Scheduling`、`Clock`、`Trace` trait） |

---

## 五、编程练习：实现 `sys_trace`

### 5.1 题目描述

在 tg-ch3 中，我们的系统已经能够支持多个任务分时轮流运行。我们希望引入一个新的系统调用 `sys_trace`（ID 为 410）用来追踪当前任务系统调用的历史信息，并做对应的修改。定义如下：

```rust
fn trace(&self, _caller: tg_syscall::Caller, _trace_request: usize, _id: usize, _data: usize) -> isize
```

**调用规范：**

这个系统调用有三种功能，根据 `trace_request` 的值不同，执行不同的操作：

| trace_request | 功能 | 参数说明 | 返回值 |
|--------------|------|---------|--------|
| 0 | 读取用户内存 | `id` 视为 `*const u8`，读取该地址处 1 字节 | 该地址处的值（无符号） |
| 1 | 写入用户内存 | `id` 视为 `*mut u8`，写入 `data` 的最低字节 | 0 |
| 2 | 查询系统调用计数 | `id` 为系统调用编号 | 该系统调用的调用次数（**本次调用也计入统计**） |
| 其他 | 无效 | 忽略其他参数 | -1 |

**说明：**
- 读写操作在未实现地址空间前并不安全，使用不当可能导致崩溃。**不要求实现安全检查机制，只需通过测试用例即可**。
- 本章只要求追踪自身任务的信息，在后续章节引入进程、线程等概念后才会扩展到追踪其他任务。

### 5.2 实现提示

- **大胆修改已有框架！** 除了配置文件，你几乎可以随意修改已有框架的内容。
- **系统调用次数**可以考虑在 `TaskControlBlock::handle_syscall()` 中统计。
- 可以**扩展 `TaskControlBlock` 结构**来维护系统调用计数信息。
- 不要害怕使用 `unsafe` 做类型转换，这在内核处理用户调用时是不可避免的。
- 在实现时，可以把系统调用参数中前缀的下划线去掉，这样更清晰。

### 5.3 实验要求

**目录结构：**

```
tg-ch3/
├── Cargo.toml          # 内核配置文件
├── src/                # 内核源代码（需要修改）
│   ├── main.rs         # 内核主函数，包括系统调用接口实现
│   └── task.rs         # 任务控制块（需要扩展）
└── tg-user/            # 用户程序（运行时自动拉取，无需修改）
    └── src/bin         # 测试用例
```

> `tg-user` 会在运行时自动拉取到 `tg-ch3/tg-user` 目录下，只需修改 `tg-ch3/src/` 目录下的内核代码。

**运行练习测例：**

```bash
cargo run --features exercise
```

**测试练习测例：**

```bash
./test.sh exercise
```

---

## 六、本章小结

通过本章的学习和实践，你在第二章的基础上实现了重要的进化：

1. **从串行到并发**：批处理系统一次只运行一个程序，多道程序系统让多个程序交替执行，大幅提高 CPU 利用率
2. **任务控制块（TCB）**：将任务的上下文、状态和栈空间封装到统一的数据结构中，是后续章节进程管理的基础
3. **协作式调度**：任务通过 `yield` 主动让出 CPU，适用于 I/O 密集型场景
4. **抢占式调度**：时钟中断强制切换任务，保证公平性，防止任务独占 CPU
5. **时间片轮转**：最基本的调度算法，每个任务获得相等的时间片
6. **时间管理**：通过 `clock_gettime` 让用户程序获取系统时间

在后续章节中，我们将引入**地址空间**，为每个任务提供独立的虚拟内存，进一步增强隔离性和安全性。

## 七、思考题

1. **协作式 vs 抢占式调度的权衡？** 协作式调度的优点和缺点分别是什么？在什么场景下协作式调度更合适？可以用 `cargo run --features coop` 体验协作式调度。

2. **时间片大小的影响？** tg-ch3 使用 12500 个时钟周期作为时间片。如果把时间片设得非常大（如 1 秒），系统行为会如何变化？如果设得非常小（如 10 个时钟周期），又会有什么问题？

3. **为什么需要 `SchedulingEvent` 枚举？** 如果不用枚举，直接在 `handle_syscall` 中决定是否切换任务，会有什么设计上的问题？

4. **时钟中断和 `sstatus.sie` 的关系？** 在 Trap 处理过程中，时钟中断会被屏蔽吗？为什么 RISC-V 默认这样设计？这与嵌套中断有什么关系？

5. **如果一个用户程序的用户栈溢出，会发生什么？** 在当前 tg-ch3 的设计中，栈溢出可能覆盖哪些数据？如何改进设计来检测栈溢出？

## 参考资料

- [rCore-Tutorial-Guide 第三章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第三章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter3/index.html)
- [RISC-V Privileged Specification](https://riscv.org/specifications/privileged-isa/)
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
