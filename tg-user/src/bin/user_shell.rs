#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

use alloc::string::String;
use user_lib::{exec, fork, getchar, waitpid};

// 教学目标：
// 实现一个最小用户态 shell，串联 getchar/fork/exec/waitpid 整条控制流。

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    println!("Rust user shell");
    let mut line: String = String::new(); // 记录着当前输入的命令
    print!(">> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                // 换行
                println!();
                if !line.is_empty() {
                    let pid = fork();
                    if pid == 0 {
                        // child process
                        // 子进程通过 exec 覆盖自身映像执行新程序。
                        if exec(line.as_str()) == -1 {
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {
                        // 父进程等待子进程完成，打印退出信息。
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as isize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    line.clear();
                }
                print!(">> ");
            }
            BS | DL => {
                // backspace
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
