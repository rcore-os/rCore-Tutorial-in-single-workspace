#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{close, fstat, open, OpenFlags, Stat, StatMode};

// 教学目标：
// 验证 fstat 回填结构体是否与预期一致（文件类型/链接计数）。
/// 测试 fstat，输出 Test fstat OK! 就算正确。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let fname = "fname1\0";
    let fd = open(fname, OpenFlags::CREATE | OpenFlags::WRONLY);
    assert!(fd > 0);
    let fd = fd as usize;
    let mut stat: Stat = Stat::new();
    let ret = fstat(fd, &mut stat);
    assert_eq!(ret, 0);
    assert_eq!(stat.mode, StatMode::FILE);
    assert_eq!(stat.nlink, 1);
    close(fd);
    println!("Test fstat OK!");
    0
}
