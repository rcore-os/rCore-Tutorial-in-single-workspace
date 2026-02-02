# 第三章：多道程序与分时多任务

本章实现了多道程序系统，支持协作式和抢占式调度，多个用户程序可以并发执行。

## 功能概述

- 任务控制块 (TCB) 管理任务状态和上下文
- 时钟中断驱动的抢占式调度（默认模式）
- 协作式调度（通过 `yield` 系统调用主动让出 CPU）
- 轮转调度算法，依次执行各任务

## 快速开始

在 tg-ch3 目录下执行：

```bash
cargo run                      # 基础模式，抢占式调度
cargo run --features exercise  # 练习模式
cargo run --features coop      # 协作式调度（需主动 yield）
```

> 默认会在 tg-ch3 目录下创建 tg-user 源码目录（通过 `cargo clone`）。
> 默认拉取版本为 `0.2.0-preview.1`，可通过环境变量 `TG_USER_VERSION` 覆盖。
> 若已有本地 tg-user，可通过 `TG_USER_DIR` 指定路径。

### 测试

```bash
./test.sh  # 全部测试，等价于 ./test.sh all
./test.sh base  # 基础测试
./test.sh exercise  # 练习测试
```

## 用户程序加载

tg-ch3 在构建阶段会拉取 tg-user 并编译用户程序，生成 `APP_ASM` 内联到内核镜像中，运行时依次加载执行。

## 默认 QEMU 启动参数

`-machine virt -nographic -bios none`

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

## 系统调用

| 系统调用 | 功能 |
|----------|------|
| `write` | 向标准输出写入数据 |
| `exit` | 退出当前任务 |
| `sched_yield` | 主动让出 CPU |
| `clock_gettime` | 获取当前时间 |

## 依赖与配置

### Features

| Feature | 说明 |
|---------|------|
| `coop` | 协作式调度，禁用时钟中断抢占 |
| `exercise` | 练习模式测例 |

### Dependencies

| 依赖 | 说明 |
|------|------|
| `riscv` | RISC-V CSR 寄存器访问（`sie`, `scause`, `time`） |
| `tg-sbi` | SBI 调用封装库，包括 `set_timer` 设置时钟中断 |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出 (`print!`/`println!`) 和日志 |
| `tg-kernel-context` | 用户上下文 `LocalContext` 及特权级切换 |
| `tg-syscall` | 系统调用定义与分发 |

## 练习

见 [Exercise](./exercise.md)

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
