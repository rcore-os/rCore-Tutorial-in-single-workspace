//! 一个简单的文件系统实现。
//!
//! 本模块提供了一个独立于内核的简易文件系统（EasyFS），
//! 用于 rCore 教学操作系统。

#![no_std]
#![deny(warnings, missing_docs)]
extern crate alloc;
mod bitmap;
mod block_cache;
mod block_dev;
mod efs;
mod file;
mod layout;
mod pipe;
mod vfs;
/// Use a block size of 512 bytes
pub const BLOCK_SZ: usize = 512;
use bitmap::Bitmap;
use block_cache::{block_cache_sync_all, get_block_cache};
pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
pub use file::*;
use layout::*;
pub use pipe::make_pipe;
pub use vfs::Inode;
