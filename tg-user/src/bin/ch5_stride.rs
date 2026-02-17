#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{set_priority, spawn, waitpid};

static TESTS: &[&str] = &[
    "ch5_stride0",
    "ch5_stride1",
    "ch5_stride2",
    "ch5_stride3",
    "ch5_stride4",
    "ch5_stride5",
];

// 教学目标：
// 统一启动不同优先级负载，观测 stride 调度在长时间运行下的分配效果。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let mut pid = [0isize; 6];
    for (i, test) in TESTS.iter().enumerate() {
        pid[i] = spawn(*test);
    }
    set_priority(4);
    for i in 0..6 {
        let mut xstate: i32 = Default::default();
        let wait_pid = waitpid(pid[i], &mut xstate);
        assert_eq!(pid[i], wait_pid);
    }
    0
}
