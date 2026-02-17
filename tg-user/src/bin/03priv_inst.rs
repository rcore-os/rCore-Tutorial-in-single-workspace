#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

// 教学目标：
// 在 U 态执行特权指令（sret），验证内核对非法指令异常的处理。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("Try to execute privileged instruction in U Mode");
    println!("Kernel should kill this application!");
    // 故意执行特权指令，期望被内核杀死。
    unsafe { core::arch::asm!("sret") };
    0
}
