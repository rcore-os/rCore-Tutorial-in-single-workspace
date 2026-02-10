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

### 2.4 运行测试

```bash
./test.sh
```

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

## License

Licensed under GNU GENERAL PUBLIC LICENSE, Version 3.0.
