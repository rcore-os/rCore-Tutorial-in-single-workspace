#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::sched_yield;

const WIDTH: usize = 10;
const HEIGHT: usize = 2;

// 教学目标：
// 与 write_a/write_c 协同运行，观察多任务交替打印行为。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    for i in 0..HEIGHT {
        for _ in 0..WIDTH {
            print!("B");
        }
        println!(" [{}/{}]", i + 1, HEIGHT);
        sched_yield();
    }
    println!("Test write B OK!");
    0
}
