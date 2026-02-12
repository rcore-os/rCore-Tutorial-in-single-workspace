#![no_std]
#![deny(warnings)]
//!
//! 教程阅读建议：
//!
//! - 用户态读 `user.rs`：看 syscall 封装如何把参数放入 a0-a5/a7；
//! - 内核态读 `kernel/mod.rs`：看 syscall 号如何分发到各子系统 trait。

#[cfg(all(feature = "kernel", feature = "user"))]
compile_error!("You can only use one of `supervisor` or `user` features at a time");

mod fs;
mod io;
mod time;

include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));
// 由构建脚本生成的 syscall 编号常量（与课程章节保持同步）。

pub use fs::*;
pub use io::*;
pub use tg_signal_defs::{SignalAction, SignalNo, MAX_SIG};
pub use time::*;

#[cfg(feature = "user")]
mod user;

#[cfg(feature = "user")]
pub use user::*;

#[cfg(feature = "kernel")]
mod kernel;

#[cfg(feature = "kernel")]
pub use kernel::*;

/// 系统调用号。
///
/// 实现为包装类型，在不损失扩展性的情况下实现类型安全性。
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(transparent)]
pub struct SyscallId(pub usize);

impl From<usize> for SyscallId {
    #[inline]
    fn from(val: usize) -> Self {
        Self(val)
    }
}
