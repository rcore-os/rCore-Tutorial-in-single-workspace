mod cases;
mod checker;

use clap::Parser;
use std::io::{self, Read};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "tg-checker")]
#[command(author, version, about = "rCore-Tutorial test output checker")]
struct Args {
    /// Chapter number (2-8)
    #[arg(long, value_name = "CHAPTER", required = false)]
    ch: Option<u8>,

    /// Exercise mode (default is base test)
    #[arg(long, default_value_t = false)]
    exercise: bool,

    /// Show verbose output
    #[arg(short, long, default_value_t = true)]
    verbose: bool,

    /// List all available tests
    #[arg(long, default_value_t = false)]
    list: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    // 1) 列出可用测试用例
    if args.list {
        println!("Available tests:");
        for (ch, exercise, desc) in cases::list_available_tests() {
            let mode = if exercise { "--exercise" } else { "" };
            println!("  tg-checker --ch {} {:<12} # {}", ch, mode, desc);
        }
        return ExitCode::SUCCESS;
    }

    // 2) 校验参数并确定章节
    let chapter = match args.ch {
        Some(ch) => ch,
        None => {
            eprintln!("Error: --ch <CHAPTER> is required");
            eprintln!("Use --list to see available tests");
            return ExitCode::FAILURE;
        }
    };

    // 3) 根据章节与模式选择测试模板
    let test_case = match cases::get_test_case(chapter, args.exercise) {
        Some(tc) => tc,
        None => {
            let mode = if args.exercise { "exercise" } else { "base" };
            eprintln!("Error: No {} test available for chapter {}", mode, chapter);
            eprintln!("Use --list to see available tests");
            return ExitCode::FAILURE;
        }
    };

    // 4) 从 stdin 读取被测程序输出（通常来自 qemu 输出重定向）
    let mut output = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut output) {
        eprintln!("Error reading from stdin: {}", e);
        return ExitCode::FAILURE;
    }

    // 5) 输出测试元信息
    let mode = if args.exercise { "exercise" } else { "base" };
    println!("========== Testing ch{} {} ==========", chapter, mode);
    println!(
        "Expected patterns: {}, Not expected: {}",
        test_case.expected.len(),
        test_case.not_expected.len()
    );
    println!();

    // 6) 执行正则检测
    let result = checker::check(&output, &test_case);

    // 7) 输出检测结果
    checker::print_result(&result, args.verbose);

    // Return exit code
    if result.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
