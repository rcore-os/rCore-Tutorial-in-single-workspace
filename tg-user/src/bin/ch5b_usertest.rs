#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

const TESTS: &[&str] = &[
    "00hello_world",
    "05write_a",
    "06write_b",
    "07write_c",
    "08power_3",
    "09power_5",
    "10power_7",
    "fork_exit",
    "forktest_simple",
    "12forktest",
    "sbrk",
];

const TEST_NUM: usize = TESTS.len();

use user_lib::{exec, fork, waitpid};

#[no_mangle]
extern "C" fn main() -> i32 {
    let mut pids = [0; TEST_NUM];
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
    println!("Basic usertests passed!");
    0
}
