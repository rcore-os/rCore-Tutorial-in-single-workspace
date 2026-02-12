# tg-kernel-context

Kernel context management for the rCore tutorial operating system.

## 设计目标

- 提供 RISC-V 上下文切换的统一抽象，简化章节内核对 trap/调度的实现。
- 支持用户态与内核态切换（`LocalContext`）。
- 在需要跨地址空间切换时提供 `foreign` 扩展能力。

## 总体架构

- `LocalContext`：封装通用寄存器、`sepc`、`sstatus` 等上下文状态。
- 裸汇编执行路径：负责寄存器保存/恢复与 `sret` 相关切换流程。
- `feature = "foreign"`：
  - `ForeignContext`
  - `ForeignPortal` / `PortalCache`
  - `MultislotPortal`

## 主要特征

- `LocalContext::user(pc)`：创建用户态执行上下文。
- `LocalContext::thread(pc, interrupt)`：创建内核线程上下文。
- `execute()`：执行一次上下文切换（unsafe）。
- `move_next()`：系统调用返回时推进 `sepc`。
- `foreign`：支持跨地址空间切换相关工具。

## 功能实现要点

- 切换过程中直接操作关键 CSR（如 `sscratch`、`sepc`、`sstatus`、`stvec`）。
- `execute()` 为 unsafe：调用方必须保证上下文内容、栈、入口地址与页表状态有效。
- 面向教学场景提供较清晰的寄存器访问接口（`a(i)`、`sp_mut()` 等）。

## 对外接口

- 结构体：
  - `LocalContext`
- 关键方法：
  - `empty()`
  - `user(pc)`
  - `thread(pc, interrupt)`
  - `a(i)`, `a_mut(i)`
  - `sp()`, `sp_mut()`
  - `pc()`, `pc_mut()`
  - `move_next()`
  - `execute()`
- `foreign` 模块（按 feature）：
  - `ForeignContext`
  - `ForeignPortal`
  - `PortalCache`
  - `MultislotPortal`

## 使用示例

```rust
use tg_kernel_context::LocalContext;

let mut ctx = LocalContext::user(entry_point);
*ctx.sp_mut() = user_stack_top;

unsafe { ctx.execute() };
```

- 章节内真实用法：
  - `ch2/src/main.rs` 使用 `LocalContext::user` 与 `execute` 跑用户程序。
  - `ch4/src/process.rs` 在 `foreign` 场景下进行地址空间相关切换。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch2` 到 `ch8`。
- 关键职责：承接 trap 返回、系统调用返回与任务切换的上下文管理。
- 关键引用文件：
  - `ch2/src/main.rs`
  - `ch3/src/task.rs`
  - `ch4/src/process.rs`
  - `ch8/src/process.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
