use bitflags::bitflags;

// 教程说明：
// 本文件定义用户/内核共享的文件元数据结构（尤其是 `Stat`）。
// 这些类型通常作为 syscall 参数在用户态与内核态之间传递。

bitflags! {
    /// 文件类型标志
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}

/// 文件状态信息
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Stat {
    /// 文件所在磁盘驱动器号
    pub dev: u64,
    /// inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量
    pub nlink: u32,
    /// 填充字段
    pad: [u64; 7],
}

impl Stat {
    /// 构造一个全零初始化的 `Stat`。
    ///
    /// 常用于用户态先分配结构体，再通过 `fstat` 系统调用由内核回填。
    pub fn new() -> Self {
        Self {
            dev: 0,
            ino: 0,
            mode: StatMode::NULL,
            nlink: 0,
            pad: [0; 7],
        }
    }
}
