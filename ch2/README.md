# 第二章：批处理系统

本章在第一章"最小执行环境"的基础上，实现了一个**批处理操作系统**（tg-ch2）。它能够依次加载并运行多个用户程序，支持特权级切换和 Trap 处理，并实现了 `write` 和 `exit` 两个系统调用。

通过本章的学习和实践，你将理解：

- 什么是批处理系统，为什么需要特权级机制
- RISC-V 的 U-mode / S-mode 特权级切换过程
- Trap 的触发、上下文保存/恢复和处理流程
- 系统调用的实现原理：从用户态 `ecall` 到内核态处理
- 用户程序如何被打包进内核并依次执行

> **前置知识**：建议先完成第一章（tg-ch1）的学习，理解 `#![no_std]`、裸机启动、SBI 等基础概念。

## 项目结构

```
ch2/
├── .cargo/
│   └── config.toml     # Cargo 配置：交叉编译目标和 QEMU runner
├── build.rs            # 构建脚本：下载编译用户程序，生成链接脚本和 APP_ASM
├── Cargo.toml          # 项目配置与依赖
├── README.md           # 本文档
├── test.sh             # 自动测试脚本
└── src/
    └── main.rs         # 内核源码：批处理主循环、Trap 处理、系统调用
```

<a id="source-nav"></a>

## 源码阅读导航索引

[返回根文档导航总表](../README.md#chapters-source-nav-map)

本章建议围绕 `src/main.rs` 建立“批处理 + Trap + 系统调用”主线。

| 阅读顺序 | 位置 | 重点问题 |
|---|---|---|
| 1 | `rust_main` | 批处理循环如何逐个装载并执行用户程序？ |
| 2 | Trap 分支（`scause` 匹配） | 用户态 `ecall` 与异常进入内核后，分支逻辑如何区分？ |
| 3 | `handle_syscall` | `a7`/`a0~a5`/`a0` 的系统调用寄存器约定如何落到代码中？ |
| 4 | `impls` 模块 | `IO` / `Process` trait 如何与 syscall 分发层对接？ |

配套建议：结合 `tg-kernel-context` 和 `tg-syscall` 的注释阅读，理解上下文切换与 syscall 分发的职责边界。

## DoD 验收标准（本章完成判据）

- [ ] 能在 `ch2` 目录运行 `cargo run`，观察多个用户程序被依次装载与执行
- [ ] 能解释 U/S 特权级切换与 `ecall` 触发 Trap 的基本路径
- [ ] 能从代码定位 syscall 参数来源（`a0~a5`）与 syscall 号来源（`a7`）
- [ ] 能说明为什么 syscall 返回前需要 `sepc += 4`（跳过 `ecall` 指令）
- [ ] 能执行 `./test.sh base` 并通过基础测试

## 概念-源码-测试三联表

| 核心概念 | 源码入口 | 自测方式（命令/现象） |
|---|---|---|
| 批处理主循环 | `ch2/src/main.rs` 的 `rust_main` | 日志中按顺序出现 app 装载与退出信息 |
| Trap 分发 | `ch2/src/main.rs` 中 `scause::read().cause()` 匹配分支 | 非法行为可被识别并输出错误日志 |
| 系统调用参数约定 | `ch2/src/main.rs` 的 `handle_syscall` | `write/exit` 行为与预期一致 |
| syscall trait 对接 | `ch2/src/main.rs` 的 `impls` 模块 | `STDOUT` 可输出，非法 fd 被拒绝 |

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

tg-ch2 的构建脚本需要 `cargo-clone`（用于自动下载用户程序 crate）和 `rust-objcopy`（用于将 ELF 转为二进制）：

```bash
cargo install cargo-clone
# rust-objcopy 由 cargo-binutils 提供
cargo install cargo-binutils
rustup component add llvm-tools
```

### 1.5 获取源代码

**方式一：只获取本实验**

```bash
cargo clone tg-ch2
cd tg-ch2
```

**方式二：获取所有实验**

```bash
git clone https://github.com/rcore-os/rCore-Tutorial-in-single-workspace.git
cd rCore-Tutorial-in-single-workspace/ch2
```

## 二、编译与运行

### 2.1 编译

在 `ch2`（或 `tg-ch2`）目录下执行：

```bash
cargo build
```

编译过程比第一章复杂，`build.rs` 会自动完成以下工作：

1. **生成链接脚本**：使用 `tg_linker::NOBIOS_SCRIPT` 生成内核的内存布局
2. **下载用户程序**：自动通过 `cargo clone` 获取 `tg-user` crate（包含用户测试程序）
3. **编译用户程序**：为每个用户程序交叉编译到 RISC-V 64 目标
4. **生成 APP_ASM**：生成汇编文件，将所有用户程序的二进制数据内联到内核镜像中

> 环境变量说明：
> - `TG_USER_DIR`：指定本地 tg-user 源码路径（跳过自动下载）
> - `TG_USER_VERSION`：指定 tg-user 版本（默认 `0.2.0-preview.1`）
> - `TG_SKIP_USER_APPS`：设置后跳过用户程序编译（生成空的占位 APP_ASM）
> - `LOG`：设置日志级别（如 `LOG=INFO`、`LOG=TRACE`）

### 2.2 运行

```bash
cargo run
```

实际执行的 QEMU 命令等价于：

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch2
```

### 2.3 预期输出

```
[tg-ch2 0.3.1-preview.1] Hello, world!
[ INFO] .data [0x802xxxxx, 0x802xxxxx)
[ WARN] boot_stack top=bottom=0x802xxxxx, lower_bound=0x802xxxxx
[ERROR] .bss [0x802xxxxx, 0x802xxxxx)
[ INFO] load app0 to 0x802xxxxx
Hello world from user mode program!
[ INFO] app0 exit with code 0

[ INFO] load app1 to 0x802xxxxx
...（更多用户程序输出）...
```

批处理系统依次加载并运行每个用户程序：
- 正常的用户程序会打印输出，然后通过 `exit` 系统调用退出
- 出错的用户程序（如非法指令、访存错误）会被内核杀死，然后继续运行下一个

### 2.4 检查tg-ch2内核是否通过基础测试

```bash
./test.sh
```
结果
```
运行 ch2 基础测试...
========== Testing ch2 base ==========
Expected patterns: 4, Not expected: 1

[PASS] found <Hello, world from user mode program!>
[PASS] found <Test power_3 OK!>
[PASS] found <Test power_5 OK!>
[PASS] found <Test power_7 OK!>
[PASS] not found <FAIL: T.T>
```

Test PASSED: 5/5
✓ ch2 基础测试通过

---

## 三、操作系统核心概念

### 3.1 批处理系统

**批处理系统**（Batch System）是最早期的操作系统形态，出现于计算资源匮乏的年代。其核心思想是：将多个程序打包到一起输入计算机，当一个程序运行结束后，计算机自动执行下一个程序。

tg-ch2 实现的批处理系统工作流程：

```
内核启动
    │
    ▼
初始化（清零 BSS、初始化控制台和系统调用）
    │
    ▼
┌─→ 加载第 i 个用户程序
│       │
│       ▼
│   创建用户上下文（设置入口地址、用户栈、U-mode）
│       │
│       ▼
│   execute() → sret 切换到 U-mode 运行用户程序
│       │
│       ▼
│   用户程序触发 Trap（ecall 或异常）
│       │
│       ▼
│   内核处理 Trap（系统调用 / 杀死出错程序）
│       │
│       ├─ 系统调用 write → 输出数据，继续运行
│       ├─ 系统调用 exit  → 程序退出
│       └─ 异常            → 杀死程序
│       │
│       ▼
└── 加载下一个用户程序（i++）
        │
        ▼
    所有程序完成 → 关机
```

**为什么需要特权级？** 如果用户程序的错误（如访问非法地址、执行特权指令）能够影响内核的运行，那整个系统就不可靠了。特权级机制将用户程序和内核隔离，确保出错的用户程序只会被杀死，而不会破坏内核。

### 3.2 RISC-V 特权级机制

RISC-V 定义了三个特权级，本章重点关注 U-mode 和 S-mode 之间的切换：

| 特权级 | 缩写 | 运行的软件 | 能做什么 |
|--------|------|-----------|---------|
| Machine Mode | M-mode | SBI 固件 | 访问所有硬件资源 |
| Supervisor Mode | S-mode | 操作系统内核 | 管理内存、处理 Trap |
| User Mode | U-mode | 用户程序 | 仅能执行普通指令 |

**特权级切换的方向**：
- **U → S**（Trap）：用户程序执行 `ecall` 或发生异常时，CPU 自动陷入 S-mode
- **S → U**（sret）：内核执行 `sret` 指令返回 U-mode 继续运行用户程序

### 3.3 Trap 处理

**Trap** 是 CPU 从低特权级陷入高特权级的机制，触发原因包括：
- **系统调用**：用户程序执行 `ecall` 指令
- **异常**：非法指令、访存错误、页错误等
- **中断**：时钟中断、外部中断等（本章暂不涉及）

**Trap 相关的 CSR（控制状态寄存器）：**

| CSR | 功能 |
|-----|------|
| `stvec` | Trap 处理入口地址 |
| `sepc` | Trap 发生前最后一条指令的地址（异常）或下一条指令地址（中断） |
| `scause` | Trap 原因（系统调用、非法指令、页错误等） |
| `stval` | Trap 附加信息（如出错的地址） |
| `sstatus` | SPP 字段记录 Trap 前的特权级 |

**Trap 处理流程：**

```
用户程序执行 ecall
       │
       ▼
  ┌── 硬件自动完成 ──┐
  │ 1. sstatus.SPP ← U  │  （记录 Trap 前的特权级）
  │ 2. sepc ← ecall 地址  │  （记录 Trap 前的 PC）
  │ 3. scause ← 原因      │  （如 UserEnvCall）
  │ 4. PC ← stvec         │  （跳转到 Trap 入口）
  │ 5. 特权级 ← S-mode    │  （切换到内核态）
  └──────────────────────┘
       │
       ▼
  Trap 入口（__alltraps）
  ── 保存所有用户寄存器到内核栈（Trap 上下文）
  ── 跳转到 Rust 的 trap_handler
       │
       ▼
  trap_handler 处理
  ── 读取 scause 判断 Trap 类型
  ── 系统调用：处理后 sepc += 4（跳过 ecall 指令）
  ── 异常：杀死程序
       │
       ▼
  __restore
  ── 从内核栈恢复用户寄存器
  ── 执行 sret 返回 U-mode
       │
       ▼
  用户程序从 ecall 的下一条指令继续执行
```

**为什么 sepc 要加 4？** 因为 `ecall` 指令本身占 4 字节。硬件将 `sepc` 设为 `ecall` 的地址，如果不加 4，`sret` 后会再次执行 `ecall`，陷入无限循环。

**上下文保存与恢复**

进入 Trap 时必须保存用户态的全部寄存器状态（称为 Trap 上下文），否则内核代码的执行会破坏用户寄存器的值。tg-ch2 使用 `tg-kernel-context` 库中的 `LocalContext` 结构体来管理上下文：

- `LocalContext::user(entry)` —— 创建一个用户态上下文，设置入口地址和 `sstatus.SPP = User`
- `ctx.execute()` —— 恢复寄存器并执行 `sret`，切换到 U-mode
- Trap 发生后自动返回到 `execute()` 的下一行

### 3.4 系统调用

系统调用是用户程序请求内核服务的唯一合法途径。用户程序将参数放入寄存器，执行 `ecall`，内核读取参数并处理。

**RISC-V 系统调用约定：**

| 寄存器 | 用途 |
|--------|------|
| `a7` | syscall ID |
| `a0` - `a5` | 参数 |
| `a0` | 返回值 |

**tg-ch2 支持的系统调用：**

| syscall ID | 名称 | 功能 |
|-----------|------|------|
| 64 | `write` | 将缓冲区数据写入文件描述符（fd=1 为标准输出） |
| 93 | `exit` | 退出当前用户程序 |

用户程序中的系统调用过程（以 `write` 为例）：

```
用户程序调用 println!("Hello")
       │
       ▼
用户库将其转为 sys_write(fd=1, buf, len)
       │
       ▼
内嵌汇编：a7=64, a0=1, a1=buf, a2=len, ecall
       │
       ▼
Trap 进入内核 → handle_syscall
       │
       ▼
内核读取 a7=64 → 调用 write 处理函数
       │
       ▼
将 buf 指向的数据通过 SBI 输出到控制台
       │
       ▼
返回值写入 a0，sepc += 4，sret 回到用户态
```

### 3.5 用户程序的打包与加载

与第一章不同，本章需要将多个用户程序嵌入到内核中。`build.rs` 在编译时完成以下工作：

1. 自动下载 `tg-user` crate（包含用户测试程序的源码）
2. 逐个编译用户程序为 RISC-V 64 的 ELF 文件
3. 使用 `rust-objcopy` 将 ELF 转为纯二进制格式（.bin）
4. 生成汇编文件 `app.asm`，用 `.incbin` 指令将所有 .bin 文件嵌入到内核的 `.data` 段

运行时，内核通过 `tg_linker::AppMeta::locate()` 获取用户程序的元数据（数量、位置、大小），然后依次加载到内存中执行。

---

## 四、代码解读

### 4.1 `src/main.rs` —— 内核主体

程序结构分为六个部分：

**模块文档与属性（第 1-21 行）：**
与第一章相同的 `#![no_std]`、`#![no_main]` 和条件编译属性。

**外部依赖引入（第 23-38 行）：**
- `tg_console`：`print!` / `println!` 宏和日志功能
- `riscv::register::*`：访问 CSR 寄存器（如 `scause`）
- `tg_kernel_context::LocalContext`：用户上下文管理
- `tg_syscall`：系统调用分发框架

**启动与数据嵌入（第 42-47 行）：**
- `global_asm!(include_str!(env!("APP_ASM")))`：将用户程序二进制数据嵌入内核
- `tg_linker::boot0!(rust_main; stack = 8 * 4096)`：定义入口，分配 32 KiB 内核栈

**内核主函数 `rust_main`（第 51-107 行）：**
核心的批处理循环：初始化 → 遍历用户程序 → 创建上下文 → execute → 处理 Trap → 下一个

**系统调用处理 `handle_syscall`（第 121-142 行）：**
从上下文提取 syscall ID 和参数，分发到 `tg_syscall::handle`，将返回值写回 `a0`

**接口实现模块 `impls`（第 146-194 行）：**
- `Console`：通过 SBI 实现字符输出
- `SyscallContext`：实现 `write` 和 `exit` 系统调用

### 4.2 `build.rs` —— 构建脚本

这是本章最复杂的文件，负责在编译期完成用户程序的获取、编译和打包。关键函数：

| 函数 | 功能 |
|------|------|
| `write_linker()` | 生成链接脚本 |
| `ensure_tg_user()` | 确保 tg-user 源码可用（本地或 cargo clone） |
| `build_apps()` | 读取 cases.toml 配置，编译所有用户程序 |
| `build_user_app()` | 编译单个用户程序 |
| `objcopy_to_bin()` | 将 ELF 转为纯二进制 |
| `write_app_asm()` | 生成汇编文件，嵌入用户程序二进制 |
| `write_dummy_app_asm()` | 生成空的占位汇编（用于 publish --dry-run） |

### 4.3 `Cargo.toml` —— 依赖说明

| 依赖 | 说明 |
|------|------|
| `riscv` | RISC-V CSR 寄存器访问库 |
| `tg-sbi` | SBI 调用封装，提供 nobios 模式启动 |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出（`print!` / `println!`）和日志 |
| `tg-kernel-context` | 用户上下文 `LocalContext`，实现特权级切换 |
| `tg-syscall` | 系统调用定义与分发框架 |

---

## 五、本章小结

通过本章的学习和实践，你在第一章的基础上迈出了重要的一步：

1. **理解了批处理系统**：操作系统自动依次加载和运行多个用户程序，是 OS 的最早期形态
2. **掌握了特权级机制**：U-mode / S-mode 的隔离保护了内核不受用户程序错误的影响
3. **理解了 Trap 处理流程**：从 `ecall` 触发到硬件自动保存 CSR，再到软件保存/恢复上下文
4. **实现了系统调用**：`write` 和 `exit` 是用户程序与内核交互的最基本接口
5. **了解了用户程序的打包**：在编译期将用户程序嵌入内核镜像

在后续章节中，我们将从批处理系统演进为**多道程序系统**和**分时共享系统**，实现多任务切换和时间片调度。

## 六、思考题

1. **为什么需要内核栈和用户栈分离？** 如果 Trap 处理时仍然使用用户栈，会有什么安全问题？

2. **`sepc` 在系统调用和异常时的值有何不同？** 为什么处理系统调用时需要将 `sepc` 加 4，而处理异常时不需要？

3. **`fence.i` 指令的作用是什么？** 在批处理系统中，为什么在加载下一个用户程序前需要执行这条指令？提示：思考指令缓存（i-cache）和数据缓存（d-cache）的区别。

4. **如果用户程序执行了 S-mode 的特权指令（如 `sret`），会发生什么？** 从特权级机制的角度解释这个行为。

## 参考资料

- [rCore-Tutorial-Guide 第二章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第二章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter2/index.html)
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
| **tg-linker** | 链接脚本（Linker Script）<br>内核内存布局（KernelLayout）<br>.text / .rodata / .data / .bss / .boot 段<br>入口点（boot0! 宏）<br>BSS 段清零 | 形成内核空间布局的链接脚本模板<br>用于 build.rs 工具构建 linker.ld<br>内核布局定位（KernelLayout::locate）<br>入口宏（boot0!）<br>段信息迭代 | 无 |
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
