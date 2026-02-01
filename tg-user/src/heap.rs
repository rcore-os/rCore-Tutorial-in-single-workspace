use alloc::alloc::handle_alloc_error;
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
};
use customizable_buddy::{BuddyAllocator, LinkedListBuddy, UsizeBuddy};

/// 初始化全局分配器和内核堆分配器。
struct StaticCell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for StaticCell<T> {}

impl<T> StaticCell<T> {
    const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    #[inline]
    fn get(&self) -> *mut T {
        self.inner.get()
    }
}

pub fn init() {
    // 托管空间 16 KiB
    const MEMORY_SIZE: usize = 16 << 10;
    static MEMORY: StaticCell<[u8; MEMORY_SIZE]> = StaticCell::new([0u8; MEMORY_SIZE]);
    unsafe {
        heap_mut().init(
            core::mem::size_of::<usize>().trailing_zeros() as _,
            NonNull::new((*MEMORY.get()).as_mut_ptr()).unwrap(),
        );
        heap_mut().transfer(
            NonNull::new_unchecked((*MEMORY.get()).as_mut_ptr()),
            MEMORY_SIZE,
        );
    }
}

type MutAllocator<const N: usize> = BuddyAllocator<N, UsizeBuddy, LinkedListBuddy>;
static HEAP: StaticCell<MutAllocator<32>> = StaticCell::new(MutAllocator::new());

#[inline]
fn heap_mut() -> &'static mut MutAllocator<32> {
    unsafe { &mut *HEAP.get() }
}

struct Global;

#[global_allocator]
static GLOBAL: Global = Global;

unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Ok((ptr, _)) = heap_mut().allocate_layout::<u8>(layout) {
            ptr.as_ptr()
        } else {
            handle_alloc_error(layout)
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        heap_mut().deallocate_layout(NonNull::new(ptr).unwrap(), layout)
    }
}
