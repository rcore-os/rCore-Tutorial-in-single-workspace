#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

// 教学目标：
// 在 U 态写特权 CSR（stvec），验证权限检查与异常上报链路。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("Try to access privileged CSR in U Mode");
    println!("Kernel should kill this application!");
    // 故意访问特权 CSR，期望被内核杀死。
    unsafe { core::arch::asm!("csrw stvec, zero") };
    0
}
