#![no_std]
#![no_main]

extern crate alloc;

extern crate user_lib;
use user_lib::*;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;

// 教学目标：
// 绑定 SIGINT 处理函数，交互式验证 ctrl-c 对用户进程的影响。

fn func() {
    println!("signal_handler: caught signal SIGINT, and exit(1)");
    exit(1);
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    println!("sig_ctrlc starting....  Press 'ctrl-c' or 'ENTER'  will quit.");

    let mut new = SignalAction::default();
    let old = SignalAction::default();
    new.handler = func as *const () as usize;

    println!("sig_ctrlc: sigaction");
    if sigaction(SignalNo::SIGINT, &new, &old) < 0 {
        panic!("Sigaction failed!");
    }
    println!("sig_ctrlc: getchar....");
    loop {
        let c = getchar();

        println!("Got Char  {}", c);
        if c == LF || c == CR {
            break;
        }
    }
    println!("sig_ctrlc: Done");
    0
}
