//! Chapter 3 测试用例

use super::TestCase;

/// ch3 基础测试
pub fn base() -> TestCase {
    // ch3 base 关注任务切换与让出 CPU 后的输出完整性。
    TestCase {
        expected: vec![
            // ch3b_yield0
            "Test write A OK!",
            // ch3b_yield1
            "Test write B OK!",
            // ch3b_yield2
            "Test write C OK!",
        ],
        not_expected: vec!["FAIL: T.T"],
    }
}

/// ch3 exercise 测试
pub fn exercise() -> TestCase {
    // ch3 exercise 额外覆盖 sleep/trace 等实验功能。
    TestCase {
        expected: vec![
            // ch3_sleep
            r"get_time OK! (\d+)",
            "Test sleep OK!",
            // ch3_sleep1
            r"current time_msec = (\d+)",
            r"time_msec = (\d+) after sleeping (\d+) ticks, delta = (\d+)ms!",
            "Test sleep1 passed!",
            // ch3_trace
            "string from task trace test",
            "Test trace OK!",
        ],
        not_expected: vec![],
    }
}
