#![allow(unused_variables)]

use crate::FileHandle;
use alloc::sync::{Arc, Weak};
use spin::Mutex;

const RING_BUFFER_SIZE: usize = 32;

/// 管道环形缓冲区状态
#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    /// 满
    Full,
    /// 空
    Empty,
    /// 正常
    Normal,
}

/// 管道环形缓冲区
pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<FileHandle>>,
}

impl PipeRingBuffer {
    /// 创建一个管道环形缓冲区
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }

    /// 设置写端
    pub fn set_write_end(&mut self, write_end: &Arc<FileHandle>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    /// 写入一个字节
    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    /// 读取一个字节
    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        c
    }

    /// 可读取的字节数
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }

    /// 可写入的字节数
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }

    /// 所有写端是否都已关闭
    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

pub trait Pipe {
    /// 创建一个管道读端
    fn read_end(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self;
    /// 创建一个管道写端
    fn write_end(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self;
}

/// 创建一个管道，返回读端和写端
pub fn make_pipe() -> (Arc<FileHandle>, Arc<FileHandle>) {
    let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
    let read_end = Arc::new(FileHandle::read_end(buffer.clone()));
    let write_end = Arc::new(FileHandle::write_end(buffer.clone()));
    buffer.lock().set_write_end(&write_end);
    (read_end, write_end)
}
