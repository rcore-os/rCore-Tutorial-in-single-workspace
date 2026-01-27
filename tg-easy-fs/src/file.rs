use core::cell::Cell;

use crate::pipe::{Pipe, PipeRingBuffer};
use crate::Inode;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use spin::Mutex;

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

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

/// 用户缓冲区迭代器
pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
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
    pub offset: Cell<usize>,
    /// Specify if this is pipe
    pub pipe: bool,
    /// Pipe ring buffer
    pub buffer: Option<Arc<Mutex<PipeRingBuffer>>>,
}

impl FileHandle {
    /// 创建一个新的文件句柄。
    pub fn new(read: bool, write: bool, inode: Arc<Inode>) -> Self {
        Self {
            inode: Some(inode),
            read,
            write,
            offset: Cell::new(0),
            pipe: false,
            buffer: None,
        }
    }

    /// 创建一个空的文件句柄（无 inode）。
    pub fn empty(read: bool, write: bool) -> Self {
        Self {
            inode: None,
            read,
            write,
            offset: Cell::new(0),
            pipe: false,
            buffer: None,
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

    /// 从文件/管道读取数据到用户缓冲区。
    /// 管道读取返回值：
    /// - > 0: 实际读取的字节数
    /// - 0: 写端已关闭且无数据可读（EOF）
    /// - -2: 当前无数据可读但写端未关闭（需等待）
    pub fn read(&self, mut buf: UserBuffer) -> isize {
        if self.pipe {
            assert!(self.readable() && self.buffer.is_some());
            let want_to_read = buf.len();
            let mut buf_iter = buf.into_iter();
            let mut already_read = 0usize;
            let mut ring_buffer = self.buffer.as_ref().unwrap().lock();
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                // 无数据可读
                if ring_buffer.all_write_ends_closed() {
                    return 0; // EOF
                }
                return -2; // 需等待
            }
            // 读取尽可能多的数据
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe {
                        *byte_ref = ring_buffer.read_byte();
                    }
                    already_read += 1;
                    if already_read == want_to_read {
                        return want_to_read as _;
                    }
                } else {
                    return already_read as _;
                }
            }
            // 缓冲区数据读完但还没满足需求，返回已读取的字节数
            already_read as _
        } else {
            // 文件读取
            let mut total_read_size: usize = 0;
            if let Some(inode) = &self.inode {
                for slice in buf.buffers.iter_mut() {
                    let read_size = inode.read_at(self.offset.get(), slice);
                    if read_size == 0 {
                        break;
                    }
                    self.offset.set(self.offset.get() + read_size);
                    total_read_size += read_size;
                }
                total_read_size as _
            } else {
                -1
            }
        }
    }

    /// 将用户缓冲区数据写入文件/管道
    /// 管道写入返回值：
    /// - > 0: 实际写入的字节数
    /// - -2: 当前无空间可写（需等待）
    pub fn write(&self, buf: UserBuffer) -> isize {
        if self.pipe {
            assert!(self.writable() && self.buffer.is_some());
            let want_to_write = buf.len();
            let mut buf_iter = buf.into_iter();
            let mut already_write = 0usize;
            let mut ring_buffer = self.buffer.as_ref().unwrap().lock();
            let loop_write = ring_buffer.available_write();
            if loop_write == 0 {
                return -2; // 缓冲区满，需等待
            }
            // 写入尽可能多的数据
            for _ in 0..loop_write {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    already_write += 1;
                    if already_write == want_to_write {
                        return want_to_write as _;
                    }
                } else {
                    return already_write as _;
                }
            }
            // 缓冲区写满但还没写完，返回已写入的字节数
            already_write as _
        } else {
            // 文件写入
            let mut total_write_size: usize = 0;
            if let Some(inode) = &self.inode {
                for slice in buf.buffers.iter() {
                    let write_size = inode.write_at(self.offset.get(), slice);
                    assert_eq!(write_size, slice.len());
                    self.offset.set(self.offset.get() + write_size);
                    total_write_size += write_size;
                }
                total_write_size as _
            } else {
                -1
            }
        }
    }
}

impl Pipe for FileHandle {
    /// 创建一个管道读端
    fn read_end(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            inode: None,
            read: true,
            write: false,
            offset: Cell::new(0),
            pipe: true,
            buffer: Some(buffer),
        }
    }
    /// 创建一个管道写端
    fn write_end(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            inode: None,
            read: false,
            write: true,
            offset: Cell::new(0),
            pipe: true,
            buffer: Some(buffer),
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
