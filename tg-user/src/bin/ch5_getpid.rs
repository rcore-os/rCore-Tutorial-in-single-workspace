#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::getpid;

// 教学目标：
// 验证 getpid 系统调用基本功能，常配合 spawn/fork 测例使用。
// 辅助测例，打印子进程 pid。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let pid = getpid();
    println!("Test getpid OK! pid = {}", pid);
    0
}
