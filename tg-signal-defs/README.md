# tg-signal-defs

Signal definitions for the rCore tutorial operating system.

## 设计目标

- 提供信号子系统共享的“纯定义层”，避免内核与用户侧重复定义。
- 维持与 POSIX 语义接近的信号编号与处理动作描述。
- 作为 `tg-signal` / `tg-signal-impl` 的稳定基础依赖。

## 总体架构

- `SignalNo`：信号编号枚举（标准信号 + 实时信号）。
- `SignalAction`：用户态 handler 地址与 mask 描述。
- `MAX_SIG`：信号数量上限常量。

## 主要特征

- 提供完整信号编号类型（含 `SIGRT*`）。
- 提供可序列化/可复制的基础信号动作结构。
- 实现简洁、`no_std` 兼容，适合底层公用。

## 功能实现要点

- 本 crate 不做调度或投递逻辑，只提供类型定义。
- 通过稳定编号和结构体布局，保障跨 crate 协作一致性。

## 对外接口

- 枚举：
  - `SignalNo`
- 结构体：
  - `SignalAction`
- 常量：
  - `MAX_SIG`

## 使用示例

```rust
use tg_signal_defs::{SignalAction, SignalNo};

let action = SignalAction { handler: 0, mask: 0 };
let _sig = SignalNo::SIGINT;
let _ = action;
```

- 章节内真实用法：
  - 通过 `tg-signal` / `tg-signal-impl` 间接用于 `ch7`、`ch8` 的信号处理。

## 与 ch1~ch8 的关系

- 直接依赖章节：无（章节通常通过上层 crate 间接使用）。
- 关键职责：提供信号编号与动作定义，供 `tg-signal` 体系复用。
- 关键引用链路：
  - `tg-signal` -> `tg-signal-defs`
  - `tg-signal-impl` -> `tg-signal-defs`
  - `ch7/Cargo.toml`, `ch8/Cargo.toml`（通过上层间接依赖）

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
