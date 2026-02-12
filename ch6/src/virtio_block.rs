//! VirtIO 块设备驱动模块
//!
//! 本模块实现了 VirtIO 块设备驱动，连接 QEMU 的虚拟块设备与 easy-fs 文件系统。
//!
//! ## 架构
//!
//! ```text
//! easy-fs 文件系统
//!       │
//!       ▼
//! BlockDevice trait（read_block / write_block）
//!       │
//!       ▼
//! VirtIOBlock（本模块实现）
//!       │
//!       ▼
//! virtio-drivers 库（VirtIOBlk）
//!       │
//!       ▼
//! QEMU VirtIO MMIO 设备（0x10001000）
//!       │
//!       ▼
//! fs.img 磁盘镜像文件
//! ```
//!
//! ## VirtioHal
//!
//! `virtio-drivers` 库需要一个 `Hal` 实现来处理 DMA 内存分配和地址转换。
//! 由于内核使用恒等映射，物理地址 == 虚拟地址，因此转换非常简单。
//!
//! 教程阅读建议：
//!
//! - 先看 `BLOCK_DEVICE`：理解驱动实例如何被文件系统全局复用；
//! - 再看 `BlockDevice` trait 实现：理解文件系统读写如何下沉到块设备；
//! - 最后看 `VirtioHal`：理解 DMA 分配与地址转换为何能“近似直通”。

use crate::{build_flags, Sv39, KERNEL_SPACE};
use alloc::{
    alloc::{alloc_zeroed, dealloc},
    sync::Arc,
};
use core::{alloc::Layout, ptr::NonNull};
use spin::{Lazy, Mutex};
use tg_easy_fs::BlockDevice;
use tg_kernel_vm::page_table::{MmuMeta, VAddr, VmFlags};
use virtio_drivers::{Hal, MmioTransport, VirtIOBlk, VirtIOHeader};

/// VirtIO 块设备的 MMIO 基地址（QEMU virt 平台）
const VIRTIO0: usize = 0x10001000;

/// 全局块设备实例（延迟初始化）
///
/// 通过 MMIO 地址创建 VirtIO 块设备驱动实例。
/// 被 easy-fs 文件系统用于读写磁盘块。
pub static BLOCK_DEVICE: Lazy<Arc<dyn BlockDevice>> = Lazy::new(|| {
    Arc::new(unsafe {
        VirtIOBlock(Mutex::new(
            VirtIOBlk::new(
                MmioTransport::new(NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap())
                    .expect("Error when creating MmioTransport"),
            )
            .expect("Error when creating VirtIOBlk"),
        ))
    })
});

/// VirtIO 块设备封装
///
/// 使用 Mutex 保护内部的 VirtIOBlk，确保线程安全访问。
struct VirtIOBlock(Mutex<VirtIOBlk<VirtioHal, MmioTransport>>);

// Safety: VirtIOBlock 内部使用 Mutex 保护，确保线程安全访问
unsafe impl Send for VirtIOBlock {}
unsafe impl Sync for VirtIOBlock {}

/// 实现 easy-fs 的 BlockDevice trait
///
/// 将文件系统的块读写请求转发给 VirtIO 驱动。
impl BlockDevice for VirtIOBlock {
    /// 读取一个磁盘块（512 字节）
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    /// 写入一个磁盘块（512 字节）
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

/// VirtIO HAL 实现
///
/// virtio-drivers 库需要的硬件抽象层，负责 DMA 内存分配和地址转换。
struct VirtioHal;

impl Hal for VirtioHal {
    /// 分配 DMA 内存（物理连续、页对齐、已清零）
    fn dma_alloc(pages: usize) -> usize {
        unsafe {
            alloc_zeroed(Layout::from_size_align_unchecked(
                pages << Sv39::PAGE_BITS,
                1 << Sv39::PAGE_BITS,
            )) as _
        }
    }

    /// 释放 DMA 内存
    fn dma_dealloc(paddr: usize, pages: usize) -> i32 {
        unsafe {
            dealloc(
                paddr as _,
                Layout::from_size_align_unchecked(pages << Sv39::PAGE_BITS, 1 << Sv39::PAGE_BITS),
            )
        }
        0
    }

    /// 物理地址转虚拟地址（恒等映射下直接返回）
    fn phys_to_virt(paddr: usize) -> usize {
        paddr
    }

    /// 虚拟地址转物理地址（通过内核页表查询）
    fn virt_to_phys(vaddr: usize) -> usize {
        const VALID: VmFlags<Sv39> = build_flags("__V");
        let ptr: NonNull<u8> = unsafe {
            KERNEL_SPACE
                .assume_init_ref()
                .translate(VAddr::new(vaddr), VALID)
                .unwrap()
        };
        ptr.as_ptr() as usize
    }
}
