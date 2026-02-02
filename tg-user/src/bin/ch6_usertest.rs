#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{spawn, waitpid};

static TESTS: &[&str] = &[
    "00hello_world",
    "08power_3",
    "09power_5",
    "10power_7",
    "05write_a",
    "06write_b",
    "07write_c",
    "ch3_sleep",
    "ch3_sleep1",
    "ch4_mmap",
    "ch4_mmap1",
    "ch4_mmap2",
    "ch4_mmap3",
    "ch4_unmap",
    "ch4_unmap2",
    "ch5_spawn0",
    "ch5_spawn1",
    "12forktest",
    "14forktest2",
    "fork_exit",
    "forktest_simple",
    "sbrk",
    "filetest_simple",
    "ch6_file0",
    "ch6_file1",
    "ch6_file2",
    "ch6_file3",
];

/// 辅助测例，运行所有其他测例。

#[no_mangle]
extern "C" fn main() -> i32 {
    for test in TESTS {
        println!("Usertests: Running {}", test);
        let pid = spawn(*test);
        let mut xstate: i32 = Default::default();
        let wait_pid = waitpid(pid, &mut xstate);
        assert_eq!(pid, wait_pid);
        println!(
            "\x1b[32mUsertests: Test {} in Process {} exited with code {}\x1b[0m",
            test, pid, xstate
        );
    }
    println!("ch6 Usertests passed!");
    0
}
