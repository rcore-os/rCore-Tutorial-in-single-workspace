# tg-sync

Synchronization primitives for the rCore tutorial operating system.

## 设计目标

- 为教学内核提供线程同步原语：互斥、信号量、条件变量。
- 在单核/教学环境下通过关中断保护关键区，降低实现复杂度。
- 与任务管理层协同，实现阻塞与唤醒语义。

## 总体架构

- `up.rs`：单核关中断保护与可变借用封装。
- `mutex.rs`：互斥接口与阻塞互斥实现。
- `semaphore.rs`：计数信号量。
- `condvar.rs`：条件变量。
- 与 `tg-task-manage::ThreadId` 协作记录等待队列。

## 主要特征

- `UPIntrFreeCell`：关中断临界区保护。
- `MutexBlocking`：支持阻塞与唤醒的互斥锁。
- `Semaphore`：`down/up` 资源计数控制。
- `Condvar`：条件等待与通知。
- `no_std` 兼容。

## 功能实现要点

- 单核假设下使用关中断避免抢占导致的数据竞争。
- 同步原语与调度器协作，阻塞线程通过 ID 进等待队列。
- 接口设计兼顾“教学可读性”和“内核运行需求”。

## 对外接口

- 类型：
  - `UPIntrFreeCell<T>`
  - `UPIntrRefMut<'_, T>`
  - `Mutex` trait
  - `MutexBlocking`
  - `Semaphore`
  - `Condvar`
- 常用方法：
  - `MutexBlocking::new()`, `lock(tid)`, `unlock()`
  - `Semaphore::new(count)`, `down(tid)`, `up()`
  - `Condvar::new()`, `wait(tid)`, `signal()`

## 使用示例

```rust
use tg_sync::{MutexBlocking, Semaphore};

let _mutex = MutexBlocking::new();
let _sem = Semaphore::new(1);
```

- 章节内真实用法：
  - `ch8/src/main.rs` 注册并处理同步相关系统调用。
  - `ch8/src/process.rs` 管理互斥锁、信号量、条件变量对象。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch8`。
- 关键职责：为并发章节提供内核同步原语实现。
- 关键引用文件：
  - `ch8/Cargo.toml`
  - `ch8/src/main.rs`
  - `ch8/src/process.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
