#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::mmap;

// 教学目标：
// 覆盖 mmap 参数校验：地址对齐、prot 合法性、标志位范围等错误输入。
// 理想结果：对于错误的 mmap 返回 -1。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let start: usize = 0x10000000;
    let len: usize = 4096;
    let prot: usize = 3;
    assert_eq!(0, mmap(start, len, prot));
    assert_eq!(mmap(start - len, len + 1, prot), -1); // 地址未对齐
    assert_eq!(mmap(start + len + 1, len, prot), -1); // 地址未对齐
    assert_eq!(mmap(start + len, len, 0), -1); // prot 为 0 无意义
    assert_eq!(mmap(start + len, len, prot | 8), -1); // prot 其他位非 0
    println!("Test 04_4 test OK!");
    0
}
