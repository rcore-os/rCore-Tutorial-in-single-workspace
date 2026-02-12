#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::ptr::slice_from_raw_parts_mut;
use user_lib::sbrk;

// 教学目标：
// 系统化验证 `sbrk` 的扩容/缩容语义，并最终触发越界访问检查。

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    println!("Test sbrk start.");
    const PAGE_SIZE: usize = 0x1000;
    let origin_brk = sbrk(0);
    println!("origin break point = {:x}", origin_brk);
    let brk = sbrk(PAGE_SIZE as i32);
    if brk != origin_brk {
        return -1;
    }
    let brk = sbrk(0);
    println!("one page allocated,  break point = {:x}", brk);
    println!("try write to allocated page");
    // 将新分配页映射成切片，验证其可读写。
    let new_page = unsafe {
        &mut *slice_from_raw_parts_mut(origin_brk as usize as *const u8 as *mut u8, PAGE_SIZE)
    };
    for pos in 0..PAGE_SIZE {
        new_page[pos] = 1;
    }
    println!("write ok");
    sbrk(PAGE_SIZE as i32 * 10);
    let brk = sbrk(0);
    println!("10 page allocated,  break point = {:x}", brk);
    sbrk(PAGE_SIZE as i32 * -11);
    let brk = sbrk(0);
    println!("11 page DEALLOCATED,  break point = {:x}", brk);
    println!("try DEALLOCATED more one page, should be failed.");
    let ret = sbrk(PAGE_SIZE as i32 * -1);
    if ret != -1 {
        println!("Test sbrk failed!");
        return -1;
    }
    println!("Test sbrk almost OK!");
    println!("now write to deallocated page, should cause page fault.");
    // 故意写回收后的地址，期望触发 page fault。
    for pos in 0..PAGE_SIZE {
        new_page[pos] = 2;
    }
    println!("Test sbrk failed!");
    0
}
