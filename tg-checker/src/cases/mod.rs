//! 测试用例定义模块
//!
//! 每个章节有两种测试模式：
//! - base: 基础测试 (ch2-ch8)
//! - exercise: 练习测试 (ch3, ch4, ch5, ch6, ch8)
//!
//! 教程说明：
//! - `expected` 里的每一项都必须匹配；
//! - `not_expected` 里的每一项都必须“不匹配”；
//! - 模式使用 Rust 正则表达式语法。

mod ch2;
mod ch3;
mod ch4;
mod ch5;
mod ch6;
mod ch7;
mod ch8;

/// 测试用例结构
#[derive(Debug, Clone)]
pub struct TestCase {
    /// 期望匹配的模式（正则表达式）
    pub expected: Vec<&'static str>,
    /// 不应该出现的模式（正则表达式）
    pub not_expected: Vec<&'static str>,
}

impl TestCase {
    #[allow(dead_code)]
    pub const fn new(expected: Vec<&'static str>, not_expected: Vec<&'static str>) -> Self {
        Self {
            expected,
            not_expected,
        }
    }
}

/// 获取指定章节的测试用例
///
/// # Arguments
/// * `chapter` - 章节号 (2-8)
/// * `exercise` - 是否为 exercise 模式
///
/// # Returns
/// 返回对应的测试用例，如果章节不存在或该章节没有对应模式的测试则返回 None
pub fn get_test_case(chapter: u8, exercise: bool) -> Option<TestCase> {
    // 统一入口：根据章节 + 模式路由到对应用例定义函数。
    match (chapter, exercise) {
        // Base tests (ch2-ch8)
        (2, false) => Some(ch2::base()),
        (3, false) => Some(ch3::base()),
        (4, false) => Some(ch4::base()),
        (5, false) => Some(ch5::base()),
        (6, false) => Some(ch6::base()),
        (7, false) => Some(ch7::base()),
        (8, false) => Some(ch8::base()),

        // Exercise tests (ch3, ch4, ch5, ch6, ch8)
        (3, true) => Some(ch3::exercise()),
        (4, true) => Some(ch4::exercise()),
        (5, true) => Some(ch5::exercise()),
        (6, true) => Some(ch6::exercise()),
        (8, true) => Some(ch8::exercise()),

        _ => None,
    }
}

/// 列出所有可用的测试
pub fn list_available_tests() -> Vec<(u8, bool, &'static str)> {
    vec![
        (2, false, "ch2 base test"),
        (3, false, "ch3 base test"),
        (3, true, "ch3 exercise test"),
        (4, false, "ch4 base test"),
        (4, true, "ch4 exercise test"),
        (5, false, "ch5 base test"),
        (5, true, "ch5 exercise test"),
        (6, false, "ch6 base test"),
        (6, true, "ch6 exercise test"),
        (7, false, "ch7 base test"),
        (8, false, "ch8 base test"),
        (8, true, "ch8 exercise test"),
    ]
}
