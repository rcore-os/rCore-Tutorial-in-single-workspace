#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{clock_gettime, exit, fork, getpid, sleep, wait, ClockId, TimeSpec};

static NUM: usize = 30;

// 教学目标：
// 让多子进程按不同睡眠时长完成，验证 wait 对“乱序结束”的回收能力。

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for _ in 0..NUM {
        let pid = fork();
        if pid == 0 {
            let mut time: TimeSpec = TimeSpec::ZERO;
            clock_gettime(ClockId::CLOCK_MONOTONIC, &mut time as *mut _ as _);
            let current_time = (time.tv_sec * 1000) + (time.tv_nsec / 1000000);
            // 根据当前时间构造不同 sleep 长度，制造完成顺序差异。
            let sleep_length =
                (current_time as i32 as isize) * (current_time as i32 as isize) % 1000 + 1000;
            println!("pid {} sleep for {} ms", getpid(), sleep_length);
            sleep(sleep_length as usize);
            println!("pid {} OK!", getpid());
            exit(0);
        }
    }

    let mut exit_code: i32 = 0;
    for _ in 0..NUM {
        // println!("child {}", wait(&mut exit_code));
        assert!(wait(&mut exit_code) > 0);
        assert_eq!(exit_code, 0);
    }
    assert!(wait(&mut exit_code) < 0);
    println!("forktest2 test passed!");
    0
}
