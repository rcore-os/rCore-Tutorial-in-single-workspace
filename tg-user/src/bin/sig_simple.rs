#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::*;

// 教学目标：
// 最小信号样例：注册 SIGUSR1 处理函数并向自身发送信号。

fn func() {
    println!("user_sig_test succsess");
    sigreturn();
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut new = SignalAction::default();
    let old = SignalAction::default();
    new.handler = func as *const () as usize;
    println!("pid = {}", getpid());
    println!("signal_simple: sigaction");
    if sigaction(SignalNo::SIGUSR1, &new, &old) < 0 {
        panic!("Sigaction failed!");
    }
    println!("signal_simple: kill");
    if kill(getpid(), SignalNo::SIGUSR1) < 0 {
        println!("Kill failed!");
        exit(1);
    }
    println!("signal_simple: Done");
    0
}
