# rCore-Tutorial-in-single-workspace（组件化教学版）

面向操作系统课程教学与自学的组件化 rCore Tutorial 操作系统内核实验仓库。  
仓库同时包含：

- `ch1~ch8`：8 个渐进章节（每章是一个可独立运行的内核 crate + 指导文档）
- `tg-*`：可复用内核组件 crate（内存、虚存、上下文、同步、信号、文件系统等）
- `tg-user`：用户态测试程序集合
- `tg-checker`：测试输出检测工具
- `tg-linker`：为 ch1~ch8的rCore Tutorial教学操作系统内核提供链接脚本生成功能

目标是让你既能按章节学习内核演进，也能按组件视角开发和复用内核模块。

## 1. 先看这段：如何开始

### 1.1 环境要求

- Rust toolchain：本仓库使用 `stable`（见 `rust-toolchain.toml`）
- 目标架构：`riscv64gc-unknown-none-elf`
- 组件：`rust-src`、`llvm-tools-preview`（`rust-toolchain.toml` 已声明）
- QEMU：`qemu-system-riscv64`（建议 >= 7.0）
- 推荐工具：`cargo-binutils`、`cargo-clone`

### 1.2 获取代码

#### 获取整个组件化 rCore Tutorial 操作系统内核实验仓库
```bash
git clone https://github.com/rcore-os/rCore-Tutorial-in-single-workspace.git
cd rCore-Tutorial-in-single-workspace
```
#### 获取某个操作系统内核或内核功能组件
```bash
cargo clone tg-ch3  #tg-chX 是发布到 crates.io上的组件化内核， X=1..8 代表8个内核 
cd tg-ch3  # 进入 tg-ch1内核
```

### 1.3 最短上手路径（建议）

```bash
cd ch3   # 或 cd tg-ch3
cargo run
```

如果你想直接做基本功能测试（例如 ch3）：
```bash
cd ch3  # 或 tg-ch3  cargo clone tg-ch3 ; cd tg-ch3
cargo build   
./test.sh  base
```

如果你想直接做练习章（例如 ch3）：

```bash
cd ch3  # 或 tg-ch3  cargo clone tg-ch3 ; cd tg-ch3
cargo build --features exercise
./test.sh exercise
```

## 2. 仓库结构总览

| 路径 | 作用 | 你通常在什么时候用 |
|---|---|---|
| `ch1` ~ `ch8` | 章节内核 + 实验指导 | 按课程顺序学习、做章节实验 |
| `ch1-lab` | ch1 参考实验 | 做第一章补充练习时 |
| `tg-console` | 控制台输出与日志 | 需要统一日志/输出接口 |
| `tg-linker` | 链接脚本生成工具 | 构建内核镜像、管理链接符号 |
| `tg-sbi` | SBI 封装（含 `nobios` 支持） | 与固件/定时器/关机交互 |
| `tg-syscall` | syscall 编号与 trait 框架 | 定义/实现系统调用 |
| `tg-kernel-context` | 上下文切换与执行上下文 | Trap、任务/线程切换 |
| `tg-kernel-alloc` | 内核内存分配器 | 需要 `#[global_allocator]` 时 |
| `tg-kernel-vm` | 地址空间与页表管理 | ch4+ 虚存、映射、权限检查 |
| `tg-task-manage` | 任务/进程/线程管理抽象 | 调度器与任务关系管理 |
| `tg-easy-fs` | 教学文件系统实现 | ch6+ 文件、目录、pipe |
| `tg-sync` | 同步原语（mutex/semaphore/condvar） | ch8 并发同步 |
| `tg-signal-defs` | 信号号与结构定义 | ch7+ 信号语义定义 |
| `tg-signal` | 信号处理 trait 抽象 | 信号框架扩展点 |
| `tg-signal-impl` | 信号处理具体实现 | 直接复用信号实现 |
| `tg-user` | 用户程序与测试用例 | 内核构建期打包/拉取用户态测例 |
| `tg-checker` | 输出匹配检测工具 | 自动判定章节测试是否通过 |
| `docs/design` | 设计文档 | 需要理解架构演进时 |

## 3. 章节与练习地图

| 章节 | 主题 | 默认运行 | 练习模式 |
|---|---|---|---|
| `ch1` | 裸机与最小执行环境 | `cargo run` | 无独立 exercise |
| `ch2` | Batch OS、Trap、基本 syscall | `cargo run` | 无独立 exercise |
| `ch3` | 多道程序与分时 | `cargo run` | `cargo run --features exercise` |
| `ch4` | 地址空间与页表 | `cargo run` | `cargo run --features exercise` |
| `ch5` | 进程与调度 | `cargo run` | `cargo run --features exercise` |
| `ch6` | 文件系统 | `cargo run` | `cargo run --features exercise` |
| `ch7` | IPC（pipe/signal） | `cargo run` | 基础测试为主 |
| `ch8` | 线程与并发同步 | `cargo run` | `cargo run --features exercise` |

5 个常见练习章：`ch3`、`ch4`、`ch5`、`ch6`、`ch8`。

## 4. 常用开发与测试流程

### 4.1 进入某章开发

```bash
cd ch<N>
cargo build
cargo run
```

### 4.2 运行章节测试脚本

```bash
./test.sh          # 默认（通常等价于 all 或 base）
./test.sh base     # 基础测试
./test.sh exercise # 练习测试（若该章支持）
./test.sh all      # 全量测试（若该章支持）
```

### 4.3 使用 `tg-checker` 做输出检测

先安装（本地路径）：

```bash
cargo install --path tg-checker
```

基础测试示例（以 ch2 为例）：

```bash
cargo run 2>&1 | tg-checker --ch 2
```

练习测试示例（以 ch3 为例）：

```bash
cargo run --features exercise 2>&1 | tg-checker --ch 3 --exercise
```

## 5. `tg-*` 内核功能组件开发工作流（开发者重点）

### 5.1 先理解 workspace 边界

根目录 `Cargo.toml` 的 workspace **主要管理内核功能组件 crate**（如 `tg-*`），并显式排除了 `ch1~ch8`、`tg-user`、`tg-checker`。  
这意味着：

- 在根目录执行 `cargo check --workspace`，重点检查内核功能组件层
- 章节内核需要进入各章节目录单独构建和测试

### 5.2 修改组件后如何验证

建议流程：

1. 在根目录先验证组件本身：
   ```bash
   cargo check -p tg-kernel-vm
   ```
2. 切到依赖该组件的章节（例如 `ch4`）做集成验证：
   ```bash
   cd ch4
   cargo run --features exercise
   ./test.sh exercise
   ```

### 5.3 何时改成本地路径依赖

某些章节练习要求你直接改组件实现（例如 `ch4` 对 `tg-kernel-vm`、`ch6` 对 `tg-easy-fs`）。  
此时按章节文档把依赖改为本地 `path`，再做章节测试。

## 6. 推荐学习/开发顺序

1. `ch1 -> ch2`：跑通启动、Trap、基础 syscall
2. `ch3 -> ch4`：完成任务调度到地址空间
3. `ch5 -> ch6`：完成进程与文件系统
4. `ch7 -> ch8`：完成 IPC、线程与并发同步
5. 回到 `tg-*`：按组件抽象复盘与重构

## 7. 常见问题（FAQ）

### Q1：为什么我在根目录 `cargo run` 不会直接跑某个章节？

因为章节 crate 不在根 workspace 默认成员里。请进入 `ch<N>` 目录运行。

### Q2：为什么 exercise 测试和 base 测试结果不同？

`exercise` 会启用章节额外需求（例如新增 syscall 或扩展行为），与基础模式测例不同是正常的。

### Q3：如何快速定位“实现错了还是输出格式错了”？

先使用章节 `./test.sh`，再用 `tg-checker` 管线检测输出，可快速区分行为错误与输出不匹配。

## 8. 相关文档入口

- 章节文档：`ch1/README.md` ~ `ch8/README.md`
- 练习说明：`ch3/exercise.md`、`ch4/exercise.md`、`ch5/exercise.md`、`ch6/exercise.md`、`ch8/exercise.md`
- 设计文档：`docs/design/20220814-crate-types.md`、`docs/design/20220823-kpti.md`

---

如果你是课程开发者，建议先读完本 README，把`ch1`~`ch8`的README.md看看，并运行一下，再从 `ch3` 或 `ch4` 开始做一个完整练习闭环（实现 -> 测试 -> 回归 -> 文档化），可以最快理解本仓库的“章节驱动 + 组件复用”开发模式。

## License: GNU GENERAL PUBLIC LICENSE v3