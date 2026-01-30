use crate::file::UserBuffer;
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
    write_end: Option<Weak<PipeWriter>>,
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
    fn set_write_end(&mut self, write_end: &Arc<PipeWriter>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    /// 写入一个字节
    fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    /// 读取一个字节
    fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        c
    }

    /// 可读取的字节数
    fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }

    /// 可写入的字节数
    fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }

    /// 所有写端是否都已关闭
    fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

/// 管道读端
#[derive(Clone)]
pub struct PipeReader {
    buffer: Arc<Mutex<PipeRingBuffer>>,
}

/// 管道写端
pub struct PipeWriter {
    buffer: Arc<Mutex<PipeRingBuffer>>,
}

impl PipeReader {
    /// 从管道读取数据到用户缓冲区。
    ///
    /// 返回值：
    /// - `> 0`: 实际读取的字节数
    /// - `0`: 写端已关闭且无数据可读（EOF）
    /// - `-2`: 当前无数据可读但写端未关闭（需等待）
    pub fn read(&self, buf: UserBuffer) -> isize {
        let want_to_read = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_read = 0usize;
        let mut ring_buffer = self.buffer.lock();
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
    }
}

impl PipeWriter {
    /// 将用户缓冲区数据写入管道。
    ///
    /// 返回值：
    /// - `> 0`: 实际写入的字节数
    /// - `-2`: 当前无空间可写（需等待）
    pub fn write(&self, buf: UserBuffer) -> isize {
        let want_to_write = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_write = 0usize;
        let mut ring_buffer = self.buffer.lock();
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
    }
}

/// 创建一个管道，返回读端和写端
pub fn make_pipe() -> (PipeReader, Arc<PipeWriter>) {
    let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
    let read_end = PipeReader {
        buffer: buffer.clone(),
    };
    let write_end = Arc::new(PipeWriter {
        buffer: buffer.clone(),
    });
    buffer.lock().set_write_end(&write_end);
    (read_end, write_end)
}
