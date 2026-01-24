//! 内存分配。

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

use alloc::alloc::handle_alloc_error;
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
};
use customizable_buddy::{BuddyAllocator, LinkedListBuddy, UsizeBuddy};

/// 静态单元格，用于安全地包装需要内部可变性的静态变量。
///
/// 这是对 `static mut` 的替代方案，通过 `UnsafeCell` 提供内部可变性，
/// 同时避免了 `static mut` 带来的编译警告和潜在的未定义行为。
///
/// # Safety
///
/// 此类型实现了 `Sync`，但调用者必须确保：
/// - 在单处理器环境下使用，或
/// - 通过外部同步机制保证线程安全
struct StaticCell<T> {
    inner: UnsafeCell<T>,
}

// SAFETY: StaticCell 仅在单处理器环境下使用，不存在并发访问。
unsafe impl<T> Sync for StaticCell<T> {}

impl<T> StaticCell<T> {
    /// 创建一个包含给定值的新 `StaticCell`。
    const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    /// 获取内部值的可变指针。
    ///
    /// # Safety
    ///
    /// 调用者必须确保不存在对内部值的并发访问。
    #[inline]
    fn get(&self) -> *mut T {
        self.inner.get()
    }
}

/// 初始化内存分配。
///
/// 参数 `base_address` 表示动态内存区域的起始位置。
///
/// # 注意
///
/// 此函数必须在使用任何堆分配之前调用，且只能调用一次。
#[inline]
pub fn init(base_address: usize) {
    // SAFETY: 此函数只在内核初始化时调用一次，此时没有其他代码会访问 HEAP。
    // base_address 由调用者保证是有效的堆起始地址。
    heap_mut().init(
        core::mem::size_of::<usize>().trailing_zeros() as _,
        NonNull::new(base_address as *mut u8).unwrap(),
    );
}

/// 将一个内存块托管到内存分配器。
///
/// # Safety
///
/// 调用者必须确保：
/// - `region` 内存块与已经转移到分配器的内存块都不重叠
/// - `region` 未被其他对象引用
/// - `region` 必须位于初始化时传入的起始位置之后
/// - 内存块的所有权将转移到分配器
#[inline]
pub unsafe fn transfer(region: &'static mut [u8]) {
    let ptr = NonNull::new(region.as_mut_ptr()).unwrap();
    // SAFETY: 由调用者保证内存块有效且不重叠
    heap_mut().transfer(ptr, region.len());
}

/// 堆分配器。
///
/// 最大容量：6 + 21 + 3 = 30 -> 1 GiB。
/// 不考虑并发使用，因此没有加锁。
///
/// 使用 `StaticCell` 包装以避免 `static mut` 的使用，
/// 通过 `heap_mut()` 函数获取可变引用。
static HEAP: StaticCell<BuddyAllocator<21, UsizeBuddy, LinkedListBuddy>> =
    StaticCell::new(BuddyAllocator::new());

/// 获取堆分配器的可变引用。
///
/// # Safety
///
/// 此函数内部使用 unsafe 获取可变引用，仅在单处理器环境下安全。
/// 调用者必须确保不存在对 HEAP 的并发访问。
#[inline]
fn heap_mut() -> &'static mut BuddyAllocator<21, UsizeBuddy, LinkedListBuddy> {
    // SAFETY: 仅在单处理器环境下使用，不存在并发访问。
    unsafe { &mut *HEAP.get() }
}

struct Global;

#[global_allocator]
static GLOBAL: Global = Global;

// SAFETY: GlobalAlloc 的实现必须是 unsafe 的。
// 此实现仅用于单处理器环境，不支持并发访问。
unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: 通过 heap_mut() 访问分配器，在单处理器环境下不会有并发的分配请求。
        // layout 的有效性由调用者（Rust 的 alloc 机制）保证。
        if let Ok((ptr, _)) = heap_mut().allocate_layout::<u8>(layout) {
            ptr.as_ptr()
        } else {
            handle_alloc_error(layout)
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: 通过 heap_mut() 访问分配器，在单处理器环境下不会有并发的释放请求。
        // ptr 和 layout 的有效性由调用者保证（必须是之前 alloc 返回的）。
        heap_mut().deallocate_layout(NonNull::new(ptr).unwrap(), layout)
    }
}
