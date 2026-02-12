#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{clock_gettime, sched_yield, ClockId, TimeSpec};

// 教学目标：
// 通过轮询 CLOCK_MONOTONIC + yield，验证“睡眠”语义和时钟系统调用。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let mut time: TimeSpec = TimeSpec::ZERO;
    clock_gettime(ClockId::CLOCK_MONOTONIC, &mut time as *mut _ as _);
    let time = time + TimeSpec::SECOND;
    loop {
        let mut now: TimeSpec = TimeSpec::ZERO;
        clock_gettime(ClockId::CLOCK_MONOTONIC, &mut now as *mut _ as _);
        if now > time {
            break;
        }
        // 未到目标时间时让出 CPU，避免忙等独占处理器。
        sched_yield();
    }
    println!("Test sleep OK!");
    0
}
