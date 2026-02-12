//! Chapter 6 test cases

use super::TestCase;

/// ch6 base test
pub fn base() -> TestCase {
    // ch6 base 在进程能力之外，新增文件系统基础读写验证。
    TestCase {
        expected: vec![
            // inherited from ch5b
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
            // ch6b_filetest_simple.rs
            "file_test passed!",
        ],
        not_expected: vec!["FAIL: T.T", "Test sbrk failed!"],
    }
}

/// ch6 exercise test
pub fn exercise() -> TestCase {
    // ch6 exercise 增强覆盖 fstat/link/unlink/批量 open 等文件接口。
    TestCase {
        expected: vec![
            // inherited from ch5 exercise (without set_priority)
            r"get_time OK! (\d+)",
            "Test sleep OK!",
            r"current time_msec = (\d+)",
            r"time_msec = (\d+) after sleeping (\d+) ticks, delta = (\d+)ms!",
            "Test sleep1 passed!",
            "Test 04_1 OK!",
            "Test 04_4 test OK!",
            "Test 04_5 ummap OK!",
            "Test 04_6 ummap2 OK!",
            "Test spawn0 OK!",
            "Test wait OK!",
            "Test waitpid OK!",
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
            // ch6_file0
            "Test file0 OK!",
            // ch6_file1
            "Test fstat OK!",
            // ch6_file2
            "Test link OK!",
            // ch6_file3
            "Test mass open/unlink OK!",
        ],
        not_expected: vec![
            "FAIL: T.T",
            "Test sbrk failed!",
            "Should cause error, Test 04_2 fail!",
            "Should cause error, Test 04_3 fail!",
        ],
    }
}
