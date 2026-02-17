# tg-task-manage

[![Crates.io](https://img.shields.io/crates/v/tg-task-manage.svg)](https://crates.io/crates/tg-task-manage)
[![Documentation](https://docs.rs/tg-task-manage/badge.svg)](https://docs.rs/tg-task-manage)
[![License](https://img.shields.io/crates/l/tg-task-manage.svg)](LICENSE)

任务管理模块，为 rCore 教学操作系统提供任务和进程管理功能，包括调度和关系管理。

## 设计目标

- 提供章节内核可复用的任务管理抽象，避免将调度和关系逻辑散落在各章节。
- 用 trait 统一“任务容器管理”和“调度队列策略”。
- 用 feature 分层支持“仅进程”与“进程 + 线程”两种阶段。

## 总体架构

- ID 层：`ProcId`、`ThreadId`、`CoroId`。
- 抽象层：
  - `Manage<T, I>`：对象增删查改接口。
  - `Schedule<I>`：调度队列接口。
- 关系层：
  - `ProcRel`（`proc`）
  - `ProcThreadRel`（`thread`）
- 管理器实现：
  - `PManager`（进程）
  - `PThreadManager`（线程）

## 主要特征

- 任务对象通过 ID 关联，降低结构体互相持有复杂度。
- 调度策略通过 `Schedule` 可替换。
- 关系管理支持 `wait/waitpid/waittid` 等语义需要。
- feature：
  - `proc`：进程管理
  - `thread`：线程管理

## 功能实现要点

- 关系容器维护“运行中”和“已结束”子任务列表，便于实现等待语义。
- 管理器负责当前任务状态迁移与调度队列交互。
- 尽量保持与章节内 PCB/TCB 结构解耦，便于教学增量演进。

## 对外接口

- ID 类型：
  - `ProcId`
  - `ThreadId`
  - `CoroId`
- trait：
  - `Manage<T, I>`
  - `Schedule<I>`
- 结构体：
  - `PManager`, `ProcRel`（`proc`）
  - `PThreadManager`, `ProcThreadRel`（`thread`）

## 使用示例

```rust
use tg_task_manage::{Manage, Schedule, ProcId};

fn _touch_id(id: ProcId) -> ProcId {
    id
}
```

- 章节内真实用法：
  - `ch5/src/processor.rs` 使用 `PManager` 管理进程调度。
  - `ch8/src/processor.rs` 使用 `PThreadManager` 管理线程调度。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch5` 到 `ch8`。
- 关键职责：提供进程/线程 ID、关系维护与调度抽象。
- 关键引用文件：
  - `ch5/Cargo.toml`
  - `ch5/src/processor.rs`
  - `ch8/src/processor.rs`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

