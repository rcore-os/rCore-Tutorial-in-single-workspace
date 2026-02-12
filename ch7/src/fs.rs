//! 文件系统管理模块
//!
//! 与第六章相比，本章引入了**统一的文件描述符类型 `Fd`**。
//!
//! ## 第六章 vs 第七章
//!
//! | 特性 | 第六章 | 第七章 |
//! |------|--------|--------|
//! | fd_table 类型 | `Vec<Option<Mutex<FileHandle>>>` | `Vec<Option<Mutex<Fd>>>` |
//! | 支持的 fd 种类 | 普通文件 + 空 fd（stdin/stdout） | 普通文件 + 管道读端 + 管道写端 + 空 fd |
//! | IPC | 无 | 管道（PipeReader/PipeWriter） |
//!
//! `Fd` 枚举统一了文件描述符表中的所有类型，使 read/write 系统调用可以
//! 通过相同的接口操作普通文件和管道。
//!
//! 教程阅读建议：
//!
//! - 先看 `Fd` 枚举：把“文件/管道/标准IO”统一抽象的设计意图先看懂；
//! - 再看 `Fd::{read, write}`：理解系统调用层如何避免分支扩散；
//! - 最后看 `FS` 与 `read_all`：区分“程序加载路径”和“运行时 I/O 路径”。

use crate::virtio_block::BLOCK_DEVICE;
use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Lazy;
use tg_easy_fs::{
    EasyFileSystem, FSManager, FileHandle, Inode, OpenFlags, PipeReader, PipeWriter, UserBuffer,
};

/// 全局文件系统实例（与第六章相同）
pub static FS: Lazy<FileSystem> = Lazy::new(|| FileSystem {
    root: EasyFileSystem::root_inode(&EasyFileSystem::open(BLOCK_DEVICE.clone())),
});

/// 文件系统管理器
pub struct FileSystem {
    /// 根目录 inode
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

    /// 列出目录内容
    fn readdir(&self, _path: &str) -> Option<alloc::vec::Vec<String>> {
        Some(self.root.readdir())
    }

    /// 创建硬链接（未实现）
    fn link(&self, _src: &str, _dst: &str) -> isize {
        unimplemented!()
    }

    /// 删除硬链接（未实现）
    fn unlink(&self, _path: &str) -> isize {
        unimplemented!()
    }
}

/// 读取文件全部内容到 Vec<u8>
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

/// 统一的文件描述符类型（**本章新增**）
///
/// 将普通文件、管道读端、管道写端和空描述符统一为一个枚举类型，
/// 使 fd_table 可以同时管理所有种类的文件描述符。
///
/// ```text
/// fd_table[0] = Fd::Empty { read: true, .. }    // stdin
/// fd_table[1] = Fd::Empty { write: true, .. }   // stdout
/// fd_table[2] = Fd::Empty { write: true, .. }   // stderr
/// fd_table[3] = Fd::File(FileHandle)             // 普通文件（open 分配）
/// fd_table[4] = Fd::PipeRead(PipeReader)         // 管道读端（pipe 分配）
/// fd_table[5] = Fd::PipeWrite(PipeWriter)        // 管道写端（pipe 分配）
/// ```
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
    /// 判断是否可读
    pub fn readable(&self) -> bool {
        match self {
            Fd::File(f) => f.readable(),
            Fd::PipeRead(_) => true,
            Fd::PipeWrite(_) => false,
            Fd::Empty { read, .. } => *read,
        }
    }

    /// 判断是否可写
    pub fn writable(&self) -> bool {
        match self {
            Fd::File(f) => f.writable(),
            Fd::PipeRead(_) => false,
            Fd::PipeWrite(_) => true,
            Fd::Empty { write, .. } => *write,
        }
    }

    /// 从 fd 读取数据（文件或管道读端）
    pub fn read(&self, buf: UserBuffer) -> isize {
        match self {
            Fd::File(f) => f.read(buf),
            Fd::PipeRead(p) => p.read(buf),
            _ => -1,
        }
    }

    /// 向 fd 写入数据（文件或管道写端）
    pub fn write(&self, buf: UserBuffer) -> isize {
        match self {
            Fd::File(f) => f.write(buf),
            Fd::PipeWrite(p) => p.write(buf),
            _ => -1,
        }
    }
}
