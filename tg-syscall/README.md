# tg-syscall

[![Crates.io](https://img.shields.io/crates/v/tg-syscall.svg)](https://crates.io/crates/tg-syscall)
[![Documentation](https://docs.rs/tg-syscall/badge.svg)](https://docs.rs/tg-syscall)
[![License](https://img.shields.io/crates/l/tg-syscall.svg)](LICENSE)

System call definitions and interfaces for the rCore tutorial operating system.

## 设计目标

- 统一管理教学内核的系统调用号、参数约定和返回约定。
- 提供内核侧“可插拔 trait 注册 + 分发器”机制。
- 提供用户侧 syscall 封装函数，减少内联汇编重复代码。

## 总体架构

- 通用定义：
  - `SyscallId`、`Caller`、`SyscallResult`、`STDOUT/STDIN` 等常量与类型。
- `feature = "kernel"`：
  - 各能力 trait（`IO`、`Process`、`Scheduling`、`Clock`、`Memory`、`Signal`、`Thread`、`SyncMutex`、`Trace`）。
  - `init_*` 注册函数与 `handle(...)` 分发函数。
- `feature = "user"`：
  - `write/read/exit/fork/...` 等用户态包装。
  - `native::syscall0..syscall6` 底层调用入口。

## 主要特征

- 支持 `kernel` 与 `user` 双模式。
- trait 驱动的系统调用实现，便于按章节逐步增量接入。
- 覆盖教学中常用 syscall 族：I/O、进程、调度、时间、信号、线程、同步等。
- `no_std` 友好，适配裸机内核与用户程序运行时。

## 功能实现要点

- 内核侧通过 `init_*` 注册实现，再由 `handle(caller, id, args)` 统一分发。
- 用户侧封装遵循 RISC-V 调用约定，将参数落在 `a0-a5/a7`。
- 分发结果以 `SyscallResult` 表达“完成/不支持”等状态，便于上层处理。

## 对外接口

- 类型/常量：
  - `SyscallId`, `Caller`, `SyscallResult`
  - `ClockId`, `TimeSpec`, `Stat`, `OpenFlags`
  - `STDIN`, `STDOUT`, `STDDEBUG`
- 内核侧（`kernel`）：
  - trait：`IO`, `Process`, `Scheduling`, `Clock`, `Memory`, `Signal`, `Thread`, `SyncMutex`, `Trace`
  - 函数：`init_io`, `init_process`, `init_scheduling`, `init_clock`, `init_memory`, `init_signal`, `init_thread`, `init_sync_mutex`, `init_trace`
  - 分发：`handle(caller, id, args)`
- 用户侧（`user`）：
  - `write`, `read`, `open`, `close`, `exit`, `fork`, `exec`, `wait`, `getpid`, `clock_gettime`, `sched_yield` 等

## 使用示例

### Kernel side (with `kernel` feature)

```rust
use tg_syscall::{Caller, SyscallId, SyscallResult};

tg_syscall::init_io(&my_io_impl);
tg_syscall::init_process(&my_process_impl);
tg_syscall::init_scheduling(&my_sched_impl);

let result = tg_syscall::handle(Caller { entity: 0, flow: 0 }, SyscallId::WRITE, [0; 6]);
let _ = result;
```

### User side (with `user` feature)

```rust
use tg_syscall::{write, exit, STDOUT};

write(STDOUT, b"hello\n");
exit(0);
```

- 章节内真实用法：
  - `ch2/src/main.rs` 注册 `IO` 和 `Process` 并调用 `handle`。
  - `ch3/src/main.rs` 增加 `Scheduling`、`Clock`。
  - `ch8/src/main.rs` 增加 `Thread`、`SyncMutex`。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch2` 到 `ch8`。
- 关键职责：作为“用户态请求 -> 内核实现”的中间协议层与分发层。
- 关键引用文件：
  - `ch2/src/main.rs`
  - `ch3/src/main.rs`
  - `ch5/src/main.rs`
  - `ch8/src/main.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
