#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, waitpid};

const TESTS: &[&str] = &[
    "00hello_world",
    "08power_3",
    "09power_5",
    "10power_7",
    "05write_a",
    "06write_b",
    "07write_c",
    "12forktest",
    "14forktest2",
    "fork_exit",
    "forktest_simple",
    "filetest_simple",
    "threads",
    "threads_arg",
    "mpsc_sem",
    "sync_sem",
    "race_adder_mutex_blocking",
    "phil_din_mutex",
    "test_condvar",
    "pipetest",
    "ch8_deadlock_mutex1",
    "ch8_deadlock_sem1",
    "ch8_deadlock_sem2",
];

const TEST_NUM: usize = TESTS.len();

#[no_mangle]
extern "C" fn main() -> i32 {
    let mut pids = [0isize; TEST_NUM];
    for (i, &test) in TESTS.iter().enumerate() {
        println!("Usertests: Running {}", test);
        let pid = fork();
        if pid == 0 {
            exec(test);
            panic!("unreachable!");
        } else {
            pids[i] = pid;
        }
    }
    let mut xstate: i32 = Default::default();
    for (i, &test) in TESTS.iter().enumerate() {
        let wait_pid = waitpid(pids[i], &mut xstate);
        assert_eq!(pids[i], wait_pid);
        println!(
            "\x1b[32mUsertests: Test {} in Process {} exited with code {}\x1b[0m",
            test, pids[i], xstate
        );
    }
    println!("ch8 Usertests passed!");
    0
}
