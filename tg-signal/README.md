# tg-signal

Signal handling abstractions for the rCore tutorial operating system.

## 设计目标

- 定义“信号子系统的接口层”，把实现细节与内核调用方解耦。
- 抽象统一的信号处理行为，支持不同实现（教学扩展或实验变体）。
- 以 trait 形式约束 `fork`、mask、投递、处理、返回等关键语义。

## 总体架构

- `Signal` trait：信号管理核心接口。
- `SignalResult`：信号处理结果枚举，指导上层调度决策。
- 复用 `tg-signal-defs` 导出的 `SignalNo`、`SignalAction`。

## 主要特征

- trait 驱动，便于替换具体实现。
- 结果类型覆盖继续执行、忽略、终止、挂起等路径。
- `no_std` 兼容，适合内核态。

## 功能实现要点

- `handle_signals(&mut LocalContext)` 将信号处理与上下文切换关联。
- `sig_return` 提供用户 handler 返回后恢复现场的统一入口。
- `from_fork` 支撑子进程继承/重建信号状态。

## 对外接口

- trait：
  - `Signal`
- 类型：
  - `SignalResult`
  - `SignalNo`（re-export）
  - `SignalAction`（re-export）

## 使用示例

```rust
use tg_signal::{Signal, SignalResult};
use tg_kernel_context::LocalContext;

fn drive_signal(sig: &mut dyn Signal, ctx: &mut LocalContext) -> SignalResult {
    sig.handle_signals(ctx)
}
```

- 章节内真实用法：
  - `ch7/src/main.rs` 根据 `SignalResult` 决定进程后续状态。
  - `ch8/src/main.rs` 延续并扩展同一信号处理流程。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch7`、`ch8`。
- 关键职责：定义信号处理抽象接口，供内核主循环统一驱动。
- 关键引用文件：
  - `ch7/Cargo.toml`
  - `ch7/src/main.rs`
  - `ch8/src/main.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
