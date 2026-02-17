# tg-signal-impl

A concrete signal handling implementation for the rCore tutorial operating system.

## 设计目标

- 提供 `tg_signal::Signal` 的默认实现，直接服务章节内核。
- 将信号队列、mask、默认行为、用户态 handler 跳转整合为可复用组件。
- 在教学代码中保持“行为正确 + 结构可读”的平衡。

## 总体架构

- `SignalImpl`：信号状态与行为核心实现。
- `SignalSet`：位图化信号集合（pending/mask 等）。
- `HandlingSignal`：当前处理态（冻结/用户处理态）。
- `default_action`：未注册用户 handler 时的默认语义。

## 主要特征

- 支持信号排队与屏蔽。
- 支持用户自定义 handler 与上下文保存恢复。
- 支持进程控制类信号（如停止/继续）。
- 与 `LocalContext` 协作完成信号处理现场切换。

## 功能实现要点

- `add_signal` 只负责投递，不立即执行处理逻辑。
- `handle_signals` 选择可处理信号并执行“默认动作或用户 handler 跳转”。
- `sig_return` 在用户 handler 完成后恢复原上下文。

## 对外接口

- 结构体：
  - `SignalImpl`
- 枚举：
  - `HandlingSignal`
- 关键函数：
  - `SignalImpl::new()`
- trait 实现：
  - `impl tg_signal::Signal for SignalImpl`

## 使用示例

```rust
use tg_signal::Signal;
use tg_signal_impl::SignalImpl;

let mut sig = SignalImpl::new();
sig.add_signal(tg_signal::SignalNo::SIGINT);
```

- 章节内真实用法：
  - `ch7/src/process.rs` 中将 `SignalImpl` 挂到进程结构。
  - `ch8/src/process.rs` 在进程/线程模型中继续复用。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch7`、`ch8`。
- 关键职责：提供可直接落地的信号处理逻辑实现。
- 关键引用文件：
  - `ch7/Cargo.toml`
  - `ch7/src/process.rs`
  - `ch8/src/process.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
