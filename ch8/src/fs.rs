//! 文件系统管理模块
//!
//! 本模块与第七章相同，提供：
//! - `FS`：全局文件系统实例（easy-fs 根 Inode）
//! - `Fd`：统一文件描述符枚举（File / PipeRead / PipeWrite / Empty）
//! - `read_all`：读取文件全部内容的辅助函数
//!
//! 在第八章中，文件描述符表 `fd_table` 属于 `Process`（进程），
//! 同一进程的所有线程共享同一个 `fd_table`。
//!
//! 教程阅读建议：
//!
//! - 先把 `Fd` 当成“线程共享资源的统一句柄”来理解；
//! - 再看 `Fd::{readable, writable, read, write}`：理解多线程下 I/O 行为复用的边界；
//! - 最后结合 `ch8/src/main.rs` 的系统调用实现，观察线程与共享 fd_table 的互动。

use crate::virtio_block::BLOCK_DEVICE;
use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Lazy;
use tg_easy_fs::{
    EasyFileSystem, FSManager, FileHandle, Inode, OpenFlags, PipeReader, PipeWriter, UserBuffer,
};

/// 全局文件系统实例（延迟初始化）
pub static FS: Lazy<FileSystem> = Lazy::new(|| FileSystem {
    root: EasyFileSystem::root_inode(&EasyFileSystem::open(BLOCK_DEVICE.clone())),
});

/// easy-fs 文件系统封装
pub struct FileSystem {
    /// 根 Inode
    root: Inode,
}

impl FSManager for FileSystem {
    /// 打开文件
    fn open(&self, path: &str, flags: OpenFlags) -> Option<Arc<FileHandle>> {
        let (readable, writable) = flags.read_write();
        if flags.contains(OpenFlags::CREATE) {
            if let Some(inode) = self.find(path) {
                inode.clear();
                Some(Arc::new(FileHandle::new(readable, writable, inode)))
            } else {
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

    /// 查找文件
    fn find(&self, path: &str) -> Option<Arc<Inode>> {
        self.root.find(path)
    }

    /// 列出目录
    fn readdir(&self, _path: &str) -> Option<alloc::vec::Vec<String>> {
        Some(self.root.readdir())
    }

    fn link(&self, _src: &str, _dst: &str) -> isize { unimplemented!() }
    fn unlink(&self, _path: &str) -> isize { unimplemented!() }
}

/// 读取文件全部内容到 Vec<u8>
pub fn read_all(fd: Arc<FileHandle>) -> Vec<u8> {
    let mut offset = 0usize;
    let mut buffer = [0u8; 512];
    let mut v: Vec<u8> = Vec::new();
    if let Some(inode) = &fd.inode {
        loop {
            let len = inode.read_at(offset, &mut buffer);
            if len == 0 { break; }
            offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
    }
    v
}

/// 统一的文件描述符类型
///
/// 将普通文件、管道读端、管道写端和空描述符统一为一个枚举，
/// 简化 `fd_table` 中的类型管理。
#[derive(Clone)]
pub enum Fd {
    /// 普通文件（来自 easy-fs）
    File(FileHandle),
    /// 管道读端（只读）
    PipeRead(PipeReader),
    /// 管道写端（只写）
    PipeWrite(Arc<PipeWriter>),
    /// 空描述符（用于 stdin/stdout/stderr）
    Empty {
        /// 是否可读
        read: bool,
        /// 是否可写
        write: bool,
    },
}

impl Fd {
    /// 该描述符是否可读
    pub fn readable(&self) -> bool {
        match self {
            Fd::File(f) => f.readable(),
            Fd::PipeRead(_) => true,
            Fd::PipeWrite(_) => false,
            Fd::Empty { read, .. } => *read,
        }
    }

    /// 该描述符是否可写
    pub fn writable(&self) -> bool {
        match self {
            Fd::File(f) => f.writable(),
            Fd::PipeRead(_) => false,
            Fd::PipeWrite(_) => true,
            Fd::Empty { write, .. } => *write,
        }
    }

    /// 从描述符读取数据
    pub fn read(&self, buf: UserBuffer) -> isize {
        match self {
            Fd::File(f) => f.read(buf),
            Fd::PipeRead(p) => p.read(buf),
            _ => -1,
        }
    }

    /// 向描述符写入数据
    pub fn write(&self, buf: UserBuffer) -> isize {
        match self {
            Fd::File(f) => f.write(buf),
            Fd::PipeWrite(p) => p.write(buf),
            _ => -1,
        }
    }
}
