# 第三章：多道程序与分时多任务

本章实现了多道程序系统，支持协作式和抢占式调度，多个用户程序可以并发执行。

## 功能概述

- 任务控制块 (TCB) 管理任务状态和上下文
- 时钟中断驱动的抢占式调度（默认模式）
- 协作式调度（通过 `yield` 系统调用主动让出 CPU）
- 轮转调度算法，依次执行各任务

## 用户程序加载

用户程序在编译时通过 `APP_ASM` 环境变量内联到内核镜像中，运行时依次加载执行。

## 系统调用

| 系统调用 | 功能 |
|----------|------|
| `write` | 向标准输出写入数据 |
| `exit` | 退出当前任务 |
| `sched_yield` | 主动让出 CPU |
| `clock_gettime` | 获取当前时间 |

## 时钟中断与抢占式调度

本章通过 SBI 的 `set_timer` 设置时钟中断，实现抢占式调度。每次切换到用户程序前设置下一次中断时间：

```rust
// 设置 12500 个时钟周期后触发中断
tg_sbi::set_timer(time::read64() + 12500);
unsafe { tcb.execute() };
```

当时钟中断到达时，`scause` 为 `Interrupt::SupervisorTimer`，内核保存当前任务状态并切换到下一个任务，实现时间片轮转：

```rust
Trap::Interrupt(Interrupt::SupervisorTimer) => {
    tg_sbi::set_timer(u64::MAX);  // 清除中断
    false  // 不结束任务，切换到下一个
}
```

启用 `coop` feature 可禁用时钟中断，任务需主动调用 `yield` 让出 CPU。

## Exercise

见 [Exercise](./exercise.md)

## Dependencies

| 依赖 | 说明 |
|------|------|
| `riscv` | RISC-V CSR 寄存器访问（`sie`, `scause`, `time`） |
| `tg-sbi` | SBI 调用封装库，包括 `set_timer` 设置时钟中断 |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出 (`print!`/`println!`) 和日志 |
| `tg-kernel-context` | 用户上下文 `LocalContext` 及特权级切换 |
| `tg-syscall` | 系统调用定义与分发 |

## Features

| Feature | 说明 |
|---------|------|
| `coop` | 协作式调度模式，禁用时钟中断抢占，任务需主动 `yield` |
| `nobios` | 无需外部 SBI 实现，直接从 QEMU `-bios none` 模式启动 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
