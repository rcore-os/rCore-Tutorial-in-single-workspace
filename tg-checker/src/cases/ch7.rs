//! Chapter 7 test cases

use super::TestCase;

/// ch7 base test
pub fn base() -> TestCase {
    TestCase {
        expected: vec![
            // inherited from ch6b
            "Hello, world from user mode program!",
            "Test power_3 OK!",
            "Test power_5 OK!",
            "Test power_7 OK!",
            "Test write A OK!",
            "Test write B OK!",
            "Test write C OK!",
            "Test sbrk almost OK!",
            "exit pass.",
            "hello child process!",
            r"child process pid = (\d+), exit code = (\d+)",
            "forktest pass.",
            "file_test passed!",
            // ch7b_sig_simple
            "signal_simple: Done",
            // ch7b_pipetest
            "pipetest passed!",
            // ch7b_pipe_large_test
            "pipe_large_test passed!",
        ],
        not_expected: vec!["FAIL: T.T", "Test sbrk failed!"],
    }
}
