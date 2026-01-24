#![no_std]

mod heap;

extern crate alloc;

use tg_console::log;

pub use tg_console::{print, println};
pub use tg_syscall::*;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG"));
    heap::init();

    extern "C" {
        fn main() -> i32;
    }

    // SAFETY: main 函数由用户程序提供，链接器保证其存在且符合 C ABI
    exit(unsafe { main() });
    unreachable!()
}

#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    let err = panic_info.message();
    if let Some(location) = panic_info.location() {
        log::error!("Panicked at {}:{}, {err}", location.file(), location.line());
    } else {
        log::error!("Panicked: {err}");
    }
    exit(1);
    unreachable!()
}

pub fn getchar() -> u8 {
    let mut c = [0u8; 1];
    read(STDIN, &mut c);
    c[0]
}

struct Console;

impl tg_console::Console for Console {
    #[inline]
    fn put_char(&self, c: u8) {
        tg_syscall::write(STDOUT, &[c]);
    }

    #[inline]
    fn put_str(&self, s: &str) {
        tg_syscall::write(STDOUT, s.as_bytes());
    }
}

pub fn sleep(period_ms: usize) {
    let mut time: TimeSpec = TimeSpec::ZERO;
    clock_gettime(ClockId::CLOCK_MONOTONIC, &mut time as *mut _ as _);
    let time = time + TimeSpec::from_millsecond(period_ms);
    loop {
        let mut now: TimeSpec = TimeSpec::ZERO;
        clock_gettime(ClockId::CLOCK_MONOTONIC, &mut now as *mut _ as _);
        if now > time {
            break;
        }
        sched_yield();
    }
}

pub fn get_time() -> isize {
    let mut time: TimeSpec = TimeSpec::ZERO;
    clock_gettime(ClockId::CLOCK_MONOTONIC, &mut time as *mut _ as _);
    (time.tv_sec * 1000 + time.tv_nsec / 1_000_000) as isize
}

pub fn trace_read(ptr: *const u8) -> Option<u8> {
    let ret = trace(0, ptr as usize, 0);
    if ret >= 0 && ret <= 255 {
        Some(ret as u8)
    } else {
        None
    }
}

pub fn trace_write(ptr: *const u8, value: u8) -> isize {
    trace(1, ptr as usize, value as usize)
}

pub fn count_syscall(syscall_id: usize) -> isize {
    trace(2, syscall_id, 0)
}
