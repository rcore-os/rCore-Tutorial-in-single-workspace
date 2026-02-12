#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::sched_yield;

const WIDTH: usize = 10;
const HEIGHT: usize = 5;

// 教学目标：
// 与 write_b/write_c 配合，验证 `sched_yield` 下的协作式调度与交错输出。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    for i in 0..HEIGHT {
        for _ in 0..WIDTH {
            print!("A");
        }
        println!(" [{}/{}]", i + 1, HEIGHT);
        sched_yield();
    }
    println!("Test write A OK!");
    0
}
