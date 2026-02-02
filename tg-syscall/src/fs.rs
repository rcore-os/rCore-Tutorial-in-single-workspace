use bitflags::bitflags;

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
