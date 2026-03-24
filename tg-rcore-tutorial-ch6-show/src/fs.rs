//! 文件系统管理模块
//!
//! 本模块封装了 easy-fs 文件系统的初始化和操作接口。
//!
//! ## 核心组件
//!
//! - `FS`：全局文件系统实例（延迟初始化），基于 VirtIO 块设备
//! - `FileSystem`：实现 `FSManager` trait，提供文件的打开、查找、目录列表等操作
//! - `read_all()`：辅助函数，读取文件的全部内容到内存
//!
//! ## 与第五章的区别
//!
//! 第五章的程序通过 `APPS` 内存表加载，而本章通过文件系统从磁盘读取。
//! `exec` 系统调用的实现从 `APPS.get(name)` 变为 `FS.open(name) + read_all()`。
//!
//! 教程阅读建议：
//!
//! - 先看 `FS` 的初始化：理解块设备与文件系统是如何绑定的；
//! - 再看 `open`：理解 CREATE/TRUNC/RDONLY 等标志的行为；
//! - 最后看 `read_all`：把握“按块读取 -> 拼接 ELF 数据”的加载路径。

use crate::virtio_block::BLOCK_DEVICE;
use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Lazy;
use tg_easy_fs::{EasyFileSystem, FSManager, FileHandle, Inode, OpenFlags};

/// 全局文件系统实例
///
/// 在首次访问时初始化：
/// 1. 通过 `BLOCK_DEVICE`（VirtIO 块设备）打开 easy-fs 文件系统
/// 2. 获取根目录 inode
pub static FS: Lazy<FileSystem> = Lazy::new(|| FileSystem {
    root: EasyFileSystem::root_inode(&EasyFileSystem::open(BLOCK_DEVICE.clone())),
});

/// 文件系统管理器
///
/// 封装 easy-fs 的根目录 inode，提供文件操作接口。
/// 当前仅支持**单级目录**（所有文件在根目录下）。
pub struct FileSystem {
    /// 根目录 inode
    root: Inode,
}

impl FSManager for FileSystem {
    /// 打开文件
    ///
    /// 根据 `OpenFlags` 处理不同的打开模式：
    /// - `CREATE`：文件存在则清空，不存在则创建
    /// - `TRUNC`：清空文件内容
    /// - `RDONLY`/`WRONLY`/`RDWR`：设置读写权限
    fn open(&self, path: &str, flags: OpenFlags) -> Option<Arc<FileHandle>> {
        let (readable, writable) = flags.read_write();
        if flags.contains(OpenFlags::CREATE) {
            if let Some(inode) = self.find(path) {
                // 文件已存在，清空内容
                inode.clear();
                Some(Arc::new(FileHandle::new(readable, writable, inode)))
            } else {
                // 文件不存在，创建新文件
                self.root
                    .create(path)
                    .map(|new_inode| Arc::new(FileHandle::new(readable, writable, new_inode)))
            }
        } else {
            self.find(path).map(|inode| {
                if flags.contains(OpenFlags::TRUNC) {
                    inode.clear();
                }
                Arc::new(FileHandle::new(readable, writable, inode))
            })
        }
    }

    /// 在根目录中查找文件
    fn find(&self, path: &str) -> Option<Arc<Inode>> {
        self.root.find(path)
    }

    /// 列出根目录下所有文件名
    fn readdir(&self, _path: &str) -> Option<alloc::vec::Vec<String>> {
        Some(self.root.readdir())
    }

    /// 创建硬链接（TODO 练习题）
    fn link(&self, _src: &str, _dst: &str) -> isize {
        unimplemented!()
    }

    /// 删除硬链接（TODO 练习题）
    fn unlink(&self, _path: &str) -> isize {
        unimplemented!()
    }
}

/// 读取文件的全部内容到 Vec<u8>
///
/// 通过文件句柄的 inode，从偏移 0 开始逐块读取，
/// 直到读取长度为 0（表示文件结束）。
pub fn read_all(fd: Arc<FileHandle>) -> Vec<u8> {
    let mut offset = 0usize;
    let mut buffer = [0u8; 512];
    let mut v: Vec<u8> = Vec::new();
    if let Some(inode) = &fd.inode {
        loop {
            let len = inode.read_at(offset, &mut buffer);
            if len == 0 {
                break;
            }
            offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
    }
    v
}
