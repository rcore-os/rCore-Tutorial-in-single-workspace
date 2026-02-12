#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::mmap;

// 教学目标：
// 覆盖非法保护位组合（R=0,W=1）并验证访问时异常路径。
// 理想结果：程序触发访存异常，被杀死。不输出 error 就算过。
// 注意：在 RISC-V 中，R == 0 && W == 1 是非法的。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let start: usize = 0x10000000;
    let len: usize = 4096;
    let prot: usize = 2; // 只写（在 RISC-V 中非法）
    assert_eq!(0, mmap(start, len, prot));
    let addr: *mut u8 = start as *mut u8;
    unsafe {
        assert!(*addr != 0); // 尝试读取，应该触发异常
    }
    println!("Should cause error, Test 04_2 fail!");
    0
}
