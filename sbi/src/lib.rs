//! SBI call wrappers

#![no_std]

// M-Mode SBI implementation (for -bios none boot)
#[cfg(feature = "nobios")]
pub mod msbi;
// M-Mode SBI entry point (for -bios none boot)
#[cfg(feature = "nobios")]
core::arch::global_asm!(include_str!("m_entry.asm"));

use core::arch::asm;

// Legacy SBI call numbers (for compatibility)
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;

// SBI Extension IDs
const SBI_EXT_TIMER: usize = 0x54494D45;
const SBI_EXT_SRST: usize = 0x53525354;

/// General SBI call with extension ID and function ID
#[cfg(not(feature = "nobios"))]
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") fid,
            in("x17") eid,
        );
    }
    ret
}

#[cfg(feature = "nobios")]
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret1: isize;
    let mut ret2: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") arg0 => ret1,
            inlateout("x11") arg1 => ret2,
            in("x12") arg2,
            in("x16") fid,
            in("x17") eid
        );
    }
    if ret1 < 0 {
        panic!("SBI call failed: {}", ret1);
    }
    ret2
}

/// Set timer using the SBI Timer Extension
pub fn set_timer(timer: u64) {
    sbi_call(SBI_EXT_TIMER, 0, timer as usize, 0, 0);
}

/// Use SBI call to put a character to console
pub fn console_putchar(c: u8) {
    sbi_call(SBI_CONSOLE_PUTCHAR, 0, c as usize, 0, 0);
}

/// Use SBI call to get a character from console
pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0, 0)
}

/// Use SBI call to shutdown the system
pub fn shutdown(failure: bool) -> ! {
    if failure {
        sbi_call(SBI_EXT_SRST, 0, 1, 0, 0);
    } else {
        sbi_call(SBI_EXT_SRST, 0, 0, 0, 0);
    }
    panic!("It should shutdown!");
}
