#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{exec, fork, wait};

#[no_mangle]
extern "C" fn main() -> i32 {
    if fork() == 0 {
        let target = match option_env!("CHAPTER").unwrap_or("0") {
            "5" => "ch5_usertest",
            "6" => "ch6_usertest",
            "8" => "ch8_usertest",
            "-5" => "ch5b_usertest",
            "-6" => "ch6b_usertest",
            "-7" => "ch7b_usertest",
            "-8" => "ch8b_usertest",
            _ => "user_shell",
        };
        exec(target);
    } else {
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                break;
            }
        }
    }
    0
}
