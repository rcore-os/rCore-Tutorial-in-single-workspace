#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::set_priority;

// 教学目标：
// 验证 set_priority 的合法输入与非法输入返回值约定。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    assert_eq!(set_priority(10), 10);
    assert_eq!(set_priority(isize::MAX), isize::MAX);
    assert_eq!(set_priority(0), -1);
    assert_eq!(set_priority(1), -1);
    assert_eq!(set_priority(-10), -1);
    println!("Test set_priority OK!");
    0
}
