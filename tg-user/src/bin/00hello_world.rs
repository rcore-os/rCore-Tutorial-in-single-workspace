#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

// 教学目标：
// 验证最小用户程序是否能被内核正确装载并输出到控制台。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("Hello, world from user mode program!");
    0
}
