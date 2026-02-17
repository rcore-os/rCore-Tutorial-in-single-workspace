#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

// 教学目标：
// 主动触发用户态非法访存（向空指针写），验证内核异常处理与进程终止路径。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("Into Test store_fault, we will insert an invalid store operation...");
    println!("Kernel should kill this application!");
    // 故意制造 store page fault。
    unsafe { core::ptr::null_mut::<u8>().write_volatile(0) };
    0
}
