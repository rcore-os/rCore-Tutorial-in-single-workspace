//! Chapter 4 test cases

use super::TestCase;

/// ch4 base test
pub fn base() -> TestCase {
    TestCase {
        expected: vec![
            "Test write A OK!",
            "Test write B OK!",
            "Test write C OK!",
            "Test sbrk almost OK!",
        ],
        not_expected: vec!["FAIL: T.T", "Test sbrk failed!"],
    }
}

/// ch4 exercise test
pub fn exercise() -> TestCase {
    TestCase {
        expected: vec![
            r"get_time OK! (\d+)",
            "Test sleep OK!",
            r"current time_msec = (\d+)",
            r"time_msec = (\d+) after sleeping (\d+) ticks, delta = (\d+)ms!",
            "Test sleep1 passed!",
            "string from task trace test",
            "Test trace OK!",
            "Test 04_1 OK!",
            "Test 04_4 test OK!",
            "Test 04_5 ummap OK!",
            "Test 04_6 ummap2 OK!",
            "Test trace_1 OK!",
        ],
        not_expected: vec![
            "FAIL: T.T",
            "Test sbrk failed!",
            "Should cause error, Test 04_2 fail!",
            "Should cause error, Test 04_3 fail!",
        ],
    }
}
