#![no_std]
#![no_main]

extern crate user_lib;
use user_lib::exit;

// 教学目标：
// 作为 wait/waitpid 的子进程样例，验证负数退出码传递。
// 辅助测例，正常退出，不输出 FAIL 即可。

#[allow(unreachable_code)]
#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    exit(-233);
    panic!("FAIL: T.T\n");
    0
}
