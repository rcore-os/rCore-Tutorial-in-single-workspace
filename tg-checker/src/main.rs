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

    // List all available tests
    if args.list {
        println!("Available tests:");
        for (ch, exercise, desc) in cases::list_available_tests() {
            let mode = if exercise { "--exercise" } else { "" };
            println!("  tg-checker --ch {} {:<12} # {}", ch, mode, desc);
        }
        return ExitCode::SUCCESS;
    }

    // Check that chapter is provided
    let chapter = match args.ch {
        Some(ch) => ch,
        None => {
            eprintln!("Error: --ch <CHAPTER> is required");
            eprintln!("Use --list to see available tests");
            return ExitCode::FAILURE;
        }
    };

    // Get test case
    let test_case = match cases::get_test_case(chapter, args.exercise) {
        Some(tc) => tc,
        None => {
            let mode = if args.exercise { "exercise" } else { "base" };
            eprintln!("Error: No {} test available for chapter {}", mode, chapter);
            eprintln!("Use --list to see available tests");
            return ExitCode::FAILURE;
        }
    };

    // Read program output from stdin
    let mut output = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut output) {
        eprintln!("Error reading from stdin: {}", e);
        return ExitCode::FAILURE;
    }

    // Print test info
    let mode = if args.exercise { "exercise" } else { "base" };
    println!("========== Testing ch{} {} ==========", chapter, mode);
    println!(
        "Expected patterns: {}, Not expected: {}",
        test_case.expected.len(),
        test_case.not_expected.len()
    );
    println!();

    // Run checks
    let result = checker::check(&output, &test_case);

    // Print result
    checker::print_result(&result, args.verbose);

    // Return exit code
    if result.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
