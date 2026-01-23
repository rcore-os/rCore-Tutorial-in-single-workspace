#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{count_syscall, get_time, sleep, trace_read, trace_write};

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_SCHED_YIELD: usize = 124;
const SYS_CLOCK_GETTIME: usize = 113;
const SYS_TRACE: usize = 410;

fn write_const(var: &u8, new_val: u8) {
    trace_write(var as *const _, new_val);
}

#[no_mangle]
extern "C" fn main() -> i32 {
    let t1 = get_time() as usize;
    get_time();
    sleep(500);
    let t2 = get_time() as usize;
    let t3 = get_time() as usize;
    assert!(3 <= count_syscall(SYS_CLOCK_GETTIME));
    // 注意这次 trace 调用本身也计入
    assert_eq!(2, count_syscall(SYS_TRACE));
    assert_eq!(0, count_syscall(SYS_WRITE));
    assert!(0 < count_syscall(SYS_SCHED_YIELD));
    assert_eq!(0, count_syscall(SYS_EXIT));

    // 想想为什么 write 调用是两次
    println!("string from task trace test\n");
    let t4 = get_time() as usize;
    let t5 = get_time() as usize;
    assert!(5 <= count_syscall(SYS_CLOCK_GETTIME));
    assert_eq!(7, count_syscall(SYS_TRACE));
    assert_eq!(2, count_syscall(SYS_WRITE));
    assert!(0 < count_syscall(SYS_SCHED_YIELD));
    assert_eq!(0, count_syscall(SYS_EXIT));

    #[allow(unused_mut)]
    let mut var = 111u8;
    assert_eq!(Some(111), trace_read(&var as *const u8));
    write_const(&var, (t1 ^ t2 ^ t3 ^ t4 ^ t5) as u8);
    assert_eq!((t1 ^ t2 ^ t3 ^ t4 ^ t5) as u8, unsafe {
        core::ptr::read_volatile(&var)
    });

    assert!(trace_read(main as *const _).is_some());
    println!("Test trace OK!");
    0
}
