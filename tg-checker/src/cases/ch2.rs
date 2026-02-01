//! Chapter 2 测试用例

use super::TestCase;

/// ch2 基础测试
pub fn base() -> TestCase {
    TestCase {
        expected: vec![
            // ch2b_hello_world
            "Hello, world from user mode program!",
            // ch2b_power_3
            "Test power_3 OK!",
            // ch2b_power_5
            "Test power_5 OK!",
            // ch2b_power_7
            "Test power_7 OK!",
        ],
        not_expected: vec!["FAIL: T.T"],
    }
}
