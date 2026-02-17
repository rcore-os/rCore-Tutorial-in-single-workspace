//! 同步互斥模块
//!
//! 教程阅读建议：
//!
//! - `up.rs`：先理解“单核下通过关中断保护临界区”；
//! - `mutex/semaphore/condvar`：再看经典同步原语如何基于等待队列实现。

#![no_std]
#![deny(warnings, missing_docs)]

mod condvar;
mod mutex;
mod semaphore;
mod up;

extern crate alloc;

pub use condvar::Condvar;
pub use mutex::{Mutex, MutexBlocking};
pub use semaphore::Semaphore;
pub use up::{UPIntrFreeCell, UPIntrRefMut};
