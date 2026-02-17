use core::any::Any;
///
/// 教程说明：
/// 这是 EasyFS 与具体硬件/驱动之间的最小抽象边界。
/// 文件系统只依赖“按块读写”，不关心块设备底层是 virtio、内存盘还是其他介质。

/// Trait for block devices
/// which reads and writes data in the unit of blocks
pub trait BlockDevice: Send + Sync + Any {
    ///Read data form block to buffer
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    ///Write data from buffer to block
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
