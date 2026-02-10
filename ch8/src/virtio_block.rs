//! VirtIO 块设备驱动模块
//!
//! 通过 MMIO 方式访问 QEMU virt 平台的 VirtIO 块设备，
//! 实现 `BlockDevice` trait 以供 easy-fs 使用。
//!
//! 本模块与第六/七章相同。

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

/// VirtIO 设备 MMIO 基地址
const VIRTIO0: usize = 0x10001000;

/// 全局块设备实例（延迟初始化）
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
struct VirtIOBlock(Mutex<VirtIOBlk<VirtioHal, MmioTransport>>);

// Safety: 内部使用 Mutex 保护，确保线程安全
unsafe impl Send for VirtIOBlock {}
unsafe impl Sync for VirtIOBlock {}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0.lock().read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0.lock().write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

/// VirtIO HAL（硬件抽象层）实现
struct VirtioHal;

impl Hal for VirtioHal {
    /// DMA 内存分配
    fn dma_alloc(pages: usize) -> usize {
        unsafe {
            alloc_zeroed(Layout::from_size_align_unchecked(
                pages << Sv39::PAGE_BITS, 1 << Sv39::PAGE_BITS,
            )) as _
        }
    }

    /// DMA 内存释放
    fn dma_dealloc(paddr: usize, pages: usize) -> i32 {
        unsafe {
            dealloc(
                paddr as _,
                Layout::from_size_align_unchecked(pages << Sv39::PAGE_BITS, 1 << Sv39::PAGE_BITS),
            )
        }
        0
    }

    /// 物理地址转虚拟地址（恒等映射）
    fn phys_to_virt(paddr: usize) -> usize { paddr }

    /// 虚拟地址转物理地址
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
