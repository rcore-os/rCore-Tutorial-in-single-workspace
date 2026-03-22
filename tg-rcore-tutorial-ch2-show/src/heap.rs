//! 裸机堆：固定区域 bump 分配器，供 `alloc` / `rustc_demangle` 使用。
//!
//! 堆大小与 `build.rs` 生成的 `linker.ld` 中 `HEAP_SIZE` 必须一致。

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;

pub const HEAP_SIZE: usize = 0x1000000;

struct BumpAllocator {
    next: UnsafeCell<usize>,
    heap_start: UnsafeCell<usize>,
    heap_end: UnsafeCell<usize>,
}

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            next: UnsafeCell::new(0),
            heap_start: UnsafeCell::new(0),
            heap_end: UnsafeCell::new(0),
        }
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

unsafe extern "C" {
    safe static __heap_start: u8;
    safe static __heap_end: u8;
}

/// 必须在任何堆分配之前调用一次。
pub fn init() {
    let start = core::ptr::addr_of!(__heap_start) as usize;
    let end = core::ptr::addr_of!(__heap_end) as usize;
    debug_assert!(end > start);
    debug_assert_eq!(end - start, HEAP_SIZE);
    unsafe {
        *ALLOCATOR.heap_start.get() = start;
        *ALLOCATOR.heap_end.get() = end;
        *ALLOCATOR.next.get() = start;
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(core::mem::align_of::<usize>());
        let size = layout.size();

        let heap_start = unsafe { *self.heap_start.get() };
        let heap_end = unsafe { *self.heap_end.get() };
        if heap_start == 0 || heap_end == 0 {
            return core::ptr::null_mut();
        }

        let next = unsafe { *self.next.get() };
        let aligned = align_up(next, align);
        let new_next = match aligned.checked_add(size) {
            Some(n) if n <= heap_end => n,
            _ => return core::ptr::null_mut(),
        };

        unsafe {
            *self.next.get() = new_next;
        }
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        /* bump: no free */
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}
