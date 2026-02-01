#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{mmap, munmap, trace_read, trace_write};

#[no_mangle]
extern "C" fn main() -> i32 {
    // 测试正常的内存读写
    #[allow(unused_mut)]
    let mut var = 111u8;
    assert_eq!(Some(111), trace_read(&var as *const _));
    var = 22;
    assert_eq!(Some(22), trace_read(&var as *const _));
    assert_eq!(0, trace_write(&var as *const _, 33));

    // 测试非法地址的读写
    assert_eq!(None, trace_read(isize::MAX as usize as *const _));
    assert_eq!(-1, trace_write(isize::MAX as usize as *const _, 0));
    assert_eq!(None, trace_read(0x80200000 as *const _)); // 内核地址
    assert_eq!(-1, trace_write(0x80200000 as *const _, 0));

    // 测试 mmap 只读页
    let start: usize = 0x10000000;
    let len: usize = 4096;
    let prot: usize = 1; // READONLY

    assert_eq!(0, mmap(start, len, prot));

    assert!(trace_read(start as *const u8).is_some()); // 可读
    assert_eq!(-1, trace_write(start as *const u8, 0)); // 不可写

    assert_eq!(0, munmap(start, len));

    // unmap 后不可访问
    assert_eq!(None, trace_read(start as *const u8));
    assert_eq!(-1, trace_write(start as *const u8, 0));

    println!("Test trace_1 OK!");
    0
}
