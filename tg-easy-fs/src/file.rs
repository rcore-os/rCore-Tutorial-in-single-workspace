use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;

use crate::Inode;

///Array of u8 slice that user communicate with os
pub struct UserBuffer {
    ///U8 vec
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    ///Create a `UserBuffer` by parameter
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    /// 获取 `UserBuffer` 的总长度。
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }

    /// 检查 `UserBuffer` 是否为空。
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }
}

bitflags! {
  /// Open file flags
  pub struct OpenFlags: u32 {
      ///Read only
      const RDONLY = 0;
      ///Write only
      const WRONLY = 1 << 0;
      ///Read & Write
      const RDWR = 1 << 1;
      ///Allow create
      const CREATE = 1 << 9;
      ///Clear file and return an empty one
      const TRUNC = 1 << 10;
  }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

/// Cached file metadata in memory
#[derive(Clone)]
pub struct FileHandle {
    /// FileSystem Inode
    pub inode: Option<Arc<Inode>>,
    /// Open options: able to read
    pub read: bool,
    /// Open options: able to write
    pub write: bool,
    /// Current offset
    pub offset: usize,
    // TODO: CH7
    // /// Specify if this is pipe
    // pub pipe: bool,
}

impl FileHandle {
    /// 创建一个新的文件句柄。
    pub fn new(read: bool, write: bool, inode: Arc<Inode>) -> Self {
        Self {
            inode: Some(inode),
            read,
            write,
            offset: 0,
        }
    }

    /// 创建一个空的文件句柄（无 inode）。
    pub fn empty(read: bool, write: bool) -> Self {
        Self {
            inode: None,
            read,
            write,
            offset: 0,
        }
    }

    /// 是否可读。
    pub fn readable(&self) -> bool {
        self.read
    }

    /// 是否可写。
    pub fn writable(&self) -> bool {
        self.write
    }

    /// 从文件读取数据到用户缓冲区。
    pub fn read(&mut self, mut buf: UserBuffer) -> isize {
        let mut total_read_size: usize = 0;
        if let Some(inode) = &self.inode {
            for slice in buf.buffers.iter_mut() {
                let read_size = inode.read_at(self.offset, slice);
                if read_size == 0 {
                    break;
                }
                self.offset += read_size;
                total_read_size += read_size;
            }
            total_read_size as _
        } else {
            -1
        }
    }

    /// 将用户缓冲区数据写入文件。
    pub fn write(&mut self, buf: UserBuffer) -> isize {
        let mut total_write_size: usize = 0;
        if let Some(inode) = &self.inode {
            for slice in buf.buffers.iter() {
                let write_size = inode.write_at(self.offset, slice);
                assert_eq!(write_size, slice.len());
                self.offset += write_size;
                total_write_size += write_size;
            }
            total_write_size as _
        } else {
            -1
        }
    }
}

/// 文件系统管理器 trait。
pub trait FSManager {
    /// 打开文件。
    fn open(&self, path: &str, flags: OpenFlags) -> Option<Arc<FileHandle>>;

    /// 查找文件。
    fn find(&self, path: &str) -> Option<Arc<Inode>>;

    /// 创建硬链接。
    fn link(&self, src: &str, dst: &str) -> isize;

    /// 删除硬链接。
    fn unlink(&self, path: &str) -> isize;

    /// 列出目录内容。
    fn readdir(&self, path: &str) -> Option<Vec<String>>;
}
