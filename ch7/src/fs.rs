use crate::virtio_block::BLOCK_DEVICE;
use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Lazy;
use tg_easy_fs::{
    EasyFileSystem, FSManager, FileHandle, Inode, OpenFlags, PipeReader, PipeWriter, UserBuffer,
};

pub static FS: Lazy<FileSystem> = Lazy::new(|| FileSystem {
    root: EasyFileSystem::root_inode(&EasyFileSystem::open(BLOCK_DEVICE.clone())),
});

pub struct FileSystem {
    root: Inode,
}

impl FSManager for FileSystem {
    fn open(&self, path: &str, flags: OpenFlags) -> Option<Arc<FileHandle>> {
        let (readable, writable) = flags.read_write();
        if flags.contains(OpenFlags::CREATE) {
            if let Some(inode) = self.find(path) {
                // Clear size
                inode.clear();
                Some(Arc::new(FileHandle::new(readable, writable, inode)))
            } else {
                // Create new file
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

    fn find(&self, path: &str) -> Option<Arc<Inode>> {
        self.root.find(path)
    }

    fn readdir(&self, _path: &str) -> Option<alloc::vec::Vec<String>> {
        Some(self.root.readdir())
    }

    fn link(&self, _src: &str, _dst: &str) -> isize {
        unimplemented!()
    }

    fn unlink(&self, _path: &str) -> isize {
        unimplemented!()
    }
}

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

/// 统一的文件描述符类型
#[derive(Clone)]
pub enum Fd {
    /// 普通文件
    File(FileHandle),
    /// 管道读端
    PipeRead(PipeReader),
    /// 管道写端
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
    /// 是否可读
    pub fn readable(&self) -> bool {
        match self {
            Fd::File(f) => f.readable(),
            Fd::PipeRead(_) => true,
            Fd::PipeWrite(_) => false,
            Fd::Empty { read, .. } => *read,
        }
    }

    /// 是否可写
    pub fn writable(&self) -> bool {
        match self {
            Fd::File(f) => f.writable(),
            Fd::PipeRead(_) => false,
            Fd::PipeWrite(_) => true,
            Fd::Empty { write, .. } => *write,
        }
    }

    /// 读取数据
    pub fn read(&self, buf: UserBuffer) -> isize {
        match self {
            Fd::File(f) => f.read(buf),
            Fd::PipeRead(p) => p.read(buf),
            _ => -1,
        }
    }

    /// 写入数据
    pub fn write(&self, buf: UserBuffer) -> isize {
        match self {
            Fd::File(f) => f.write(buf),
            Fd::PipeWrite(p) => p.write(buf),
            _ => -1,
        }
    }
}
