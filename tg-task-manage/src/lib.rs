//! 任务管理 lib
//!
//! 教程阅读建议：
//!
//! - 从 `Manage`/`Schedule` 两个 trait 入手，理解“对象存储”和“调度策略”解耦；
//! - 再看 `PManager`/`PThreadManager`，理解父子进程、线程归属与 wait 语义。

#![no_std]
#![deny(warnings, missing_docs)]

extern crate alloc;

mod id;
mod manager;
mod scheduler;

pub use id::*;
pub use manager::Manage;
pub use scheduler::Schedule;

#[cfg(feature = "proc")]
mod proc_manage;
#[cfg(feature = "proc")]
mod proc_rel;
#[cfg(feature = "proc")]
pub use proc_manage::PManager;
#[cfg(feature = "proc")]
pub use proc_rel::ProcRel;

#[cfg(feature = "thread")]
mod proc_thread_rel;
#[cfg(feature = "thread")]
mod thread_manager;
#[cfg(feature = "thread")]
pub use proc_thread_rel::ProcThreadRel;
#[cfg(feature = "thread")]
pub use thread_manager::PThreadManager;
