//! 核心检测逻辑

use colored::Colorize;
use regex::Regex;

use crate::cases::TestCase;

/// 检测结果
#[derive(Debug)]
pub struct CheckResult {
    pub passed: usize,
    pub total: usize,
    pub details: Vec<CheckDetail>,
}

/// 单个检测项的详情
#[derive(Debug)]
pub struct CheckDetail {
    pub pattern: String,
    pub passed: bool,
    pub is_expected: bool,
}

impl CheckResult {
    /// 是否全部通过
    pub fn is_success(&self) -> bool {
        self.passed == self.total
    }
}

/// 执行检测
pub fn check(output: &str, test_case: &TestCase) -> CheckResult {
    let mut details = Vec::new();
    let mut passed = 0;
    let total = test_case.expected.len() + test_case.not_expected.len();

    // 先检查“必须出现”的模式
    for pattern in &test_case.expected {
        let re = Regex::new(pattern).expect("Invalid regex pattern");
        let found = re.is_match(output);
        if found {
            passed += 1;
        }
        details.push(CheckDetail {
            pattern: pattern.to_string(),
            passed: found,
            is_expected: true,
        });
    }

    // 再检查“禁止出现”的模式
    for pattern in &test_case.not_expected {
        let re = Regex::new(pattern).expect("Invalid regex pattern");
        let found = re.is_match(output);
        let check_passed = !found;
        if check_passed {
            passed += 1;
        }
        details.push(CheckDetail {
            pattern: pattern.to_string(),
            passed: check_passed,
            is_expected: false,
        });
    }

    CheckResult {
        passed,
        total,
        details,
    }
}

/// 打印检测结果
pub fn print_result(result: &CheckResult, verbose: bool) {
    if verbose {
        for detail in &result.details {
            if detail.is_expected {
                if detail.passed {
                    println!("{} found <{}>", "[PASS]".green(), detail.pattern);
                } else {
                    println!("{} not found <{}>", "[FAIL]".red(), detail.pattern);
                }
            } else {
                // not_expected 模式
                if detail.passed {
                    println!("{} not found <{}>", "[PASS]".green(), detail.pattern);
                } else {
                    println!("{} found <{}>", "[FAIL]".red(), detail.pattern);
                }
            }
        }
        println!();
    }

    let status = if result.is_success() {
        "PASSED".green().bold()
    } else {
        "FAILED".red().bold()
    };

    println!("Test {}: {}/{}", status, result.passed, result.total);
}
