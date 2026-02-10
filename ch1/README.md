# 第一章：应用程序与基本执行环境

本章实现了一个最简单的 RISC-V S 态裸机程序（tg-ch1），展示操作系统的最小执行环境。程序在 QEMU 模拟的 RISC-V 64 硬件上运行，不依赖 OpenSBI 或 RustSBI，通过 `-bios none` 模式直接启动，打印 `Hello, world!` 后关机。

通过本章的学习和实践，你将理解：

- 应用程序的执行环境是什么，为什么 `Hello, world!` 并不简单
- 如何让 Rust 程序脱离标准库，在裸机上运行
- RISC-V 的启动流程和特权级机制
- SBI 的作用以及操作系统如何与硬件交互

## 项目结构

```
ch1/
├── .cargo/
│   └── config.toml     # Cargo 配置：指定交叉编译目标和 QEMU runner
├── build.rs            # 构建脚本：自动生成链接脚本
├── Cargo.toml          # 项目配置与依赖
├── README.md           # 本文档
└── src/
    └── main.rs         # 程序源码：入口、主函数、panic 处理
```

## 一、环境准备

### 1.1 安装 Rust 工具链

本项目使用 Rust 语言编写，需要通过 rustup 安装 Rust 工具链。

**Linux / macOS / WSL：**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

**Windows：**

从 [https://rustup.rs](https://rustup.rs) 下载并运行 `rustup-init.exe`。

验证安装：

```bash
rustc --version    # 应显示 rustc 1.xx.x
cargo --version    # 应显示 cargo 1.xx.x
```

### 1.2 添加 RISC-V 64 编译目标

由于 tg-ch1 是面向 RISC-V 64 裸机平台的程序，需要添加对应的编译目标：

```bash
rustup target add riscv64gc-unknown-none-elf
```

这个目标三元组的含义是：
- **riscv64gc**：RISC-V 64 位，支持 G（通用）和 C（压缩）指令集扩展
- **unknown**：没有特定的 CPU 厂商
- **none**：没有操作系统
- **elf**：生成 ELF 格式的可执行文件，无标准运行时库

### 1.3 安装 QEMU 模拟器

tg-ch1 在 QEMU 模拟的 RISC-V 64 虚拟机上运行，需要安装 `qemu-system-riscv64`（建议版本 >= 7.0）。

**Ubuntu / Debian：**

```bash
sudo apt update
sudo apt install qemu-system-misc
```

**macOS（Homebrew）：**

```bash
brew install qemu
```

**验证安装：**

```bash
qemu-system-riscv64 --version
```

### 1.4 获取源代码
**方式一**
只获取本实验
```bash
cargo clone tg-ch1
cd tg-ch1
```
获取所有8个实验和所依赖的tg-* crates.
**方式二**
```bash
git clone https://github.com/rcore-os/rCore-Tutorial-in-single-workspace.git
cd rCore-Tutorial-in-single-workspace/ch1
```

## 二、编译与运行

### 2.1 编译

在 `tg-ch1` 或 `ch1` 目录下执行：

```bash
cargo build
```

这条命令实际上执行的是**交叉编译**——编译器在你的主机（如 x86_64）上运行，但生成的可执行文件是针对 `riscv64gc-unknown-none-elf` 平台的。这个目标平台由 `.cargo/config.toml` 中的配置自动指定：

```toml
[build]
target = "riscv64gc-unknown-none-elf"
```

编译过程中，`build.rs` 构建脚本会自动检测目标架构，为 RISC-V 64 生成链接脚本（linker.ld），控制程序的内存布局。

编译成功后，可执行文件位于 `target/riscv64gc-unknown-none-elf/debug/tg-ch1`。

### 2.2 运行

```bash
cargo run
```

`cargo run` 在编译成功后会自动调用 `.cargo/config.toml` 中配置的 runner 来执行程序。实际执行的命令等价于：

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch1
```

**QEMU 参数说明：**

| 参数 | 说明 |
|------|------|
| `-machine virt` | 使用 QEMU 的 `virt` 虚拟平台，这是一个通用的 RISC-V 虚拟机 |
| `-nographic` | 无图形界面，所有输出通过串口重定向到终端 |
| `-bios none` | 不加载任何 BIOS/SBI 固件，tg-ch1 自带 M-mode 启动代码 |
| `-kernel <文件>` | 将 ELF 可执行文件加载到内存中作为内核启动 |

### 2.3 预期输出

```
Hello, world!
```

输出一行 `Hello, world!` 后，QEMU 自动退出。这是因为程序通过 SBI 调用执行了关机操作。

---

## 三、操作系统核心概念

以下内容帮助你理解 tg-ch1 代码背后的操作系统原理。

### 3.1 应用程序执行环境

大多数程序员的职业生涯都从 `Hello, world!` 开始。然而，要在屏幕上打印一行字，并不像表面上那么简单。

在日常开发中，我们编写的应用程序运行在一个多层次的**执行环境栈**之上：

```
  ┌─────────────────────────┐
  │      应用程序            │  ← 你写的代码
  ├─────────────────────────┤
  │   标准库 (std / libc)    │  ← println! 等函数的实现
  ├─────────────────────────┤
  │     操作系统内核         │  ← 系统调用：write, exit 等
  ├─────────────────────────┤
  │   硬件抽象层 (SBI/BIOS)  │  ← 固件，为内核提供基础服务
  ├─────────────────────────┤
  │       硬件 (CPU/内存)    │  ← 物理硬件
  └─────────────────────────┘
```

每一层为上一层提供服务，层与层之间通过明确定义的接口交互：
- 应用程序通过**系统调用**（如 `ecall`）请求操作系统服务
- 操作系统通过 **SBI 调用**请求固件服务
- 固件直接操作硬件

当我们在 Linux 上执行 `println!("Hello, world!")` 时，实际经历了：`println!` → Rust 标准库 → libc 的 `write()` → Linux 内核 `sys_write` 系统调用 → 串口/终端驱动 → 硬件显示。

**tg-ch1 做了什么？** 它跳过了标准库和操作系统内核，直接在裸机上通过 SBI 接口输出字符。这就是"最小执行环境"的含义。

### 3.2 移除标准库依赖

要让程序在裸机上运行，首先需要摆脱对操作系统的依赖。Rust 标准库 `std` 依赖操作系统提供的系统调用（如文件 I/O、内存分配、线程等），在没有操作系统的裸机上无法使用。

tg-ch1 在 `src/main.rs` 的开头使用了两个关键的属性标记：

**`#![no_std]` —— 不使用标准库**

告诉 Rust 编译器不链接标准库 `std`，改用核心库 `core`。核心库 `core` 是 Rust 语言的子集实现，不依赖任何操作系统功能，包含了基本类型、迭代器、Option/Result 等核心机制。

**`#![no_main]` —— 不使用标准入口**

标准的 `main()` 函数入口需要运行时环境（如 C runtime）进行初始化。在裸机环境中没有这些支持，所以我们告诉编译器不使用标准入口，自己定义程序的入口点 `_start`。

**`#[panic_handler]` —— 自定义 panic 处理**

标准库提供了 panic 时打印错误信息并终止程序的功能。使用 `#![no_std]` 后，需要自己实现 panic 处理函数。tg-ch1 中的实现是直接调用 SBI 关机：

```rust
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    shutdown(true)  // 以异常状态关机
}
```

**什么是交叉编译？**

编译器运行在主机平台（如 `x86_64-unknown-linux-gnu`）上，但生成的可执行文件需要在目标平台（`riscv64gc-unknown-none-elf`）上运行，这种情况称为**交叉编译**（Cross Compile）。`.cargo/config.toml` 中的 `target = "riscv64gc-unknown-none-elf"` 配置使 cargo 自动进行交叉编译。

### 3.3 裸机启动流程

理解程序如何在裸机上启动，是操作系统学习的重要一步。

tg-ch1 采用 **nobios 模式**（`-bios none`），不依赖外部 SBI 固件，而是在 `tg-sbi` 库中自带了一个最小的 M-mode 启动代码。启动流程如下：

```
QEMU 加电
    │
    ▼
PC = 0x1000（QEMU 内置引导代码）
    │
    ▼
跳转到 0x80000000（M-mode 入口，tg-sbi 的 _m_start）
    │  ── 在 M-mode 下初始化硬件环境
    │  ── 设置中断委托、PMP 等
    ▼
跳转到 0x80200000（S-mode 入口，tg-ch1 的 _start）
    │  ── 设置栈指针 sp
    ▼
跳转到 rust_main()
    │  ── 打印 "Hello, world!"
    │  ── 调用 SBI shutdown 关机
    ▼
QEMU 退出
```

**关键地址：**
- `0x80000000`：M-mode 代码的起始地址，由链接脚本中的 `M_BASE_ADDRESS` 指定
- `0x80200000`：S-mode 代码的起始地址，由链接脚本中的 `S_BASE_ADDRESS` 指定，这是 `_start` 函数所在的位置

**链接脚本的作用**

链接脚本控制程序各段在内存中的布局。`build.rs` 在编译时自动生成链接脚本，将程序组织为：

```
地址空间布局：

0x80000000  ┌────────────────────┐
            │  .text.m_entry     │  M-mode 入口代码（tg-sbi）
            │  .text.m_trap      │  M-mode 中断处理
            │  .bss.m_stack      │  M-mode 栈空间
            │  .bss.m_data       │  M-mode 数据
            │        ...         │
0x80200000  ├────────────────────┤
            │  .text             │  S-mode 代码段（含 .text.entry）
            │  .rodata           │  只读数据段
            │  .data             │  可读写数据段
            │  .bss              │  未初始化数据段（含栈）
            └────────────────────┘
```

**栈空间初始化**

在裸机环境中，没有操作系统帮我们设置栈。`_start` 是一个**裸函数**（`#[unsafe(naked)]`），它不会生成函数序言（prologue）和尾声（epilogue），可以在没有栈的情况下执行。它做的第一件事就是设置栈指针 `sp`，然后跳转到 Rust 函数 `rust_main`：

```rust
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 4096;

    #[unsafe(link_section = ".bss.uninit")]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",  // 将 sp 设置为栈顶地址
        "j  {main}",                       // 跳转到 rust_main
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        main       =   sym rust_main,
    )
}
```

> **注意**：Rust edition 2024 要求 `no_mangle`、`link_section` 等 unsafe 属性必须用 `unsafe(...)` 包装，这与 edition 2021 的写法不同。

栈大小为 4096 字节（4 KiB），放置在 `.bss.uninit` 段中。`la sp, STACK + 4096` 将 `sp` 设置为栈顶地址（栈从高地址向低地址增长）。

### 3.4 SBI 与特权级

**RISC-V 特权级**

RISC-V 定义了三个特权级（Privilege Level），从高到低：

| 特权级 | 缩写 | 说明 |
|--------|------|------|
| Machine Mode | M-mode | 最高特权级，直接访问所有硬件资源 |
| Supervisor Mode | S-mode | 操作系统内核运行的特权级 |
| User Mode | U-mode | 应用程序运行的特权级 |

不同特权级之间通过 `ecall`（Environment Call）指令切换：
- 应用程序（U-mode）执行 `ecall` → 陷入操作系统（S-mode）：这是**系统调用**
- 操作系统（S-mode）执行 `ecall` → 陷入固件（M-mode）：这是 **SBI 调用**

虽然都是 `ecall` 指令，但因为所在特权级不同，产生的效果也不同。

**SBI（Supervisor Binary Interface）**

SBI 是 RISC-V 的标准规范，定义了 S-mode 软件（操作系统）向 M-mode 固件请求服务的接口。可以把 SBI 理解为"操作系统的操作系统"——它为操作系统提供最基本的硬件抽象服务。

tg-ch1 通过 `use tg_sbi::{console_putchar, shutdown}` 引入了两个 SBI 服务：

| 函数 | 说明 |
|------|------|
| `console_putchar(c)` | 向控制台输出一个字符（通过串口） |
| `shutdown(fail)` | 关闭虚拟机（`fail=false` 正常关机，`fail=true` 异常关机） |

`rust_main` 的实现非常简洁——逐字符输出 "Hello, world!\n"，然后关机：

```rust
extern "C" fn rust_main() -> ! {
    for c in b"Hello, world!\n" {
        console_putchar(*c);
    }
    shutdown(false) // false 表示正常关机
}
```

**nobios 模式的特殊之处**

传统方案（如 rCore-Tutorial 旧版）使用外部 SBI 固件（如 RustSBI），需要将 SBI 固件和内核分别加载。tg-ch1 采用 `tg-sbi` 的 `nobios` 特性，将 M-mode 启动代码直接编译进同一个 ELF 文件中，因此可以用 `-bios none -kernel` 的方式一步加载，简化了启动流程。

---

## 四、代码解读

### 4.1 `.cargo/config.toml` —— 交叉编译与运行配置

```toml
[build]
target = "riscv64gc-unknown-none-elf"

[target.riscv64gc-unknown-none-elf]
runner = [
    "qemu-system-riscv64",
    "-machine", "virt",
    "-nographic",
    "-bios", "none",
    "-kernel",
]
```

- `[build] target`：设置默认编译目标为 RISC-V 64 裸机平台，每次 `cargo build` 自动交叉编译
- `[target...] runner`：设置运行器为 QEMU，`cargo run` 时自动在 QEMU 中执行编译产物

### 4.2 `Cargo.toml` —— 项目配置

```toml
[package]
name = "tg-ch1"
edition = "2024"
# ...

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[dependencies]
tg-sbi = { version = "0.1.0-preview.1", features = ["nobios"] }
```

关键配置：
- `edition = "2024"`：使用 Rust 2024 edition，要求 unsafe 属性使用 `unsafe(...)` 包装
- `panic = "abort"`：panic 时直接终止，不进行栈展开（unwinding），减少裸机程序的复杂度
- `tg-sbi` 依赖启用了 `nobios` 特性，使其内建 M-mode 启动代码

### 4.3 `build.rs` —— 构建脚本

```rust
fn main() {
    use std::{env, fs, path::PathBuf};

    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "riscv64" {
        let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
        fs::write(&ld, LINKER_SCRIPT).unwrap();
        println!("cargo:rustc-link-arg=-T{}", ld.display());
    }
}
```

构建脚本在**编译之前**自动执行：
1. 检测目标架构是否为 `riscv64`
2. 如果是，将内嵌的链接脚本写入 `OUT_DIR/linker.ld`
3. 通过 `cargo:rustc-link-arg` 指示链接器使用该脚本

链接脚本定义了两个关键地址：
- `M_BASE_ADDRESS = 0x80000000`：M-mode 代码起始地址
- `S_BASE_ADDRESS = 0x80200000`：S-mode 代码起始地址（`_start` 所在位置）

### 4.4 `src/main.rs` —— 程序源码

整个程序由五部分组成：

**模块文档与属性标记（第 1-19 行）：**
- 模块级文档注释（`//!`）概述本章关键概念
- `#![no_std]`：不使用标准库
- `#![no_main]`：不使用标准入口
- `cfg_attr`：在 RISC-V 64 上启用严格警告，其他架构允许死代码（用于 `cargo publish --dry-run` 在主机上通过编译）

**SBI 引入（第 21-23 行）：**
- `use tg_sbi::{console_putchar, shutdown}` 明确引入所需的两个 SBI 函数

**入口函数 `_start`（第 34-53 行）：**
- 仅在 `riscv64` 架构下编译（`#[cfg(target_arch = "riscv64")]`）
- 裸函数，放置在 `.text.entry` 段，链接脚本将其安排在 `0x80200000`
- edition 2024 要求使用 `#[unsafe(no_mangle)]`、`#[unsafe(link_section = "...")]` 语法
- 分配 4 KiB 栈空间，设置 `sp` 后跳转到 `rust_main`

**主函数 `rust_main`（第 59-64 行）：**
- 逐字节调用 `console_putchar` 输出 "Hello, world!\n"
- 调用 `shutdown(false)` 正常关机

**panic 处理（第 69-72 行）：**
- 发生 panic 时调用 `shutdown(true)` 以异常方式关机

**非 RISC-V 占位模块 `stub`（第 78-95 行）：**
- 提供 `main`、`__libc_start_main` 等符号，使得在非 RISC-V 平台上也能通过编译（用于 `cargo publish --dry-run` 验证）

---

## 五、本章小结

通过本章的学习和实践，你完成了从普通应用程序到裸机程序的蜕变过程：

1. **理解了执行环境**：应用程序依赖多层执行环境（标准库 → 操作系统 → 硬件），`Hello, world!` 的背后并不简单
2. **摆脱了标准库**：通过 `#![no_std]` 和 `#![no_main]`，让 Rust 程序不再依赖操作系统
3. **掌握了裸机启动流程**：从 QEMU 加电到 M-mode 初始化，再到 S-mode 的 `_start` 入口
4. **认识了 RISC-V 特权级和 SBI**：M-mode / S-mode / U-mode 的层次关系，以及 `ecall` 指令如何跨越特权级

这是操作系统内核开发的第一步——在后续章节中，我们将在这个最小执行环境的基础上，逐步添加批处理、多道程序、内存管理、进程调度等操作系统核心功能。

## 六、思考题

1. **为什么 `_start` 函数必须是裸函数（`#[naked]`）？** 如果不是裸函数会发生什么问题？提示：思考函数序言（prologue）需要什么前提条件。

2. **`ecall` 指令在不同特权级中的效果有何不同？** 为什么应用程序和操作系统都使用 `ecall`，却能产生不同的行为？

3. **如果把链接脚本中的 `S_BASE_ADDRESS` 从 `0x80200000` 改为其他值（如 `0x80100000`），程序还能正常运行吗？** 需要做哪些相应的修改？

## 参考资料

- [rCore-Tutorial-Guide 第一章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第一章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter1/index.html)
- [BlogOS: A Freestanding Rust Binary](https://os.phil-opp.com/freestanding-rust-binary/)
- [RISC-V Privileged Specification](https://riscv.org/specifications/privileged-isa/)
- [RISC-V SBI Specification](https://github.com/riscv-non-isa/riscv-sbi-doc)
- [RISC-V Reader 中文版](http://riscvbook.com/chinese/RISC-V-Reader-Chinese-v2p1.pdf)

## Dependencies

| 依赖 | 说明 |
|------|------|
| `tg-sbi` | SBI 调用封装库，支持 nobios 模式，内建 M-mode 启动代码 |

## License

Licensed under GNU GENERAL PUBLIC LICENSE, Version 3.0.
