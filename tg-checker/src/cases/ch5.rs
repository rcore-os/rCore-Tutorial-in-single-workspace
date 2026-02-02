//! Chapter 5 test cases

use super::TestCase;

/// ch5 base test
pub fn base() -> TestCase {
    TestCase {
        expected: vec![
            // inherited from ch2b
            "Hello, world from user mode program!",
            "Test power_3 OK!",
            "Test power_5 OK!",
            "Test power_7 OK!",
            // inherited from ch3b
            "Test write A OK!",
            "Test write B OK!",
            "Test write C OK!",
            // inherited from ch4b
            "Test sbrk almost OK!",
            // ch5b_exit.rs
            "exit pass.",
            // ch5b_forktest_simple.rs
            "hello child process!",
            r"child process pid = (\d+), exit code = (\d+)",
            // ch5b_forktest.rs
            "forktest pass.",
        ],
        not_expected: vec!["FAIL: T.T", "Test sbrk failed!"],
    }
}

/// ch5 exercise test
pub fn exercise() -> TestCase {
    TestCase {
        expected: vec![
            // inherited from ch4 exercise (without trace related)
            r"get_time OK! (\d+)",
            "Test sleep OK!",
            r"current time_msec = (\d+)",
            r"time_msec = (\d+) after sleeping (\d+) ticks, delta = (\d+)ms!",
            "Test sleep1 passed!",
            "Test 04_1 OK!",
            "Test 04_4 test OK!",
            "Test 04_5 ummap OK!",
            "Test 04_6 ummap2 OK!",
            // ch5_spawn0
            "Test spawn0 OK!",
            // ch5_spawn1
            "Test wait OK!",
            "Test waitpid OK!",
            // ch5_setprio
            "Test set_priority OK!",
        ],
        not_expected: vec![
            "FAIL: T.T",
            "Test sbrk failed!",
            "Should cause error, Test 04_2 fail!",
            "Should cause error, Test 04_3 fail!",
        ],
    }
}
