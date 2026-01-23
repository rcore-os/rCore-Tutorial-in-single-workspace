//! Minimal M-Mode SBI implementation for -bios none boot
//!
//! This module provides basic SBI services when running without an external
//! bootloader like RustSBI. It handles ecalls from S-mode and provides:
//! - Console I/O (UART)
//! - Timer management
//! - System reset

use core::arch::asm;

const UART_BASE: usize = 0x1000_0000;

/// UART operations (16550 compatible)
mod uart {
    use super::UART_BASE;

    const THR: usize = UART_BASE; // Transmit Holding Register
    const LSR: usize = UART_BASE + 5; // Line Status Register

    /// Check if UART is ready to send
    #[inline]
    fn is_tx_ready() -> bool {
        unsafe {
            let lsr = (LSR as *const u8).read_volatile();
            (lsr & 0x20) != 0 // THRE bit
        }
    }

    /// Write a byte to UART
    pub fn putchar(c: u8) {
        while !is_tx_ready() {}
        unsafe {
            (THR as *mut u8).write_volatile(c);
        }
    }

    /// Read a byte from UART (non-blocking)
    pub fn getchar() -> Option<u8> {
        let lsr = unsafe { (LSR as *const u8).read_volatile() };
        if lsr & 1 != 0 {
            Some(unsafe { (THR as *const u8).read_volatile() })
        } else {
            None
        }
    }
}

/// SBI Extension IDs
mod eid {
    pub const CONSOLE_PUTCHAR: usize = 0x01;
    pub const CONSOLE_GETCHAR: usize = 0x02;
    pub const SHUTDOWN: usize = 0x08;
    pub const BASE: usize = 0x10;
    pub const SRST: usize = 0x53525354;
    pub const TIMER: usize = 0x54494D45;
}

/// SBI Function IDs
mod fid {
    pub const BASE_GET_SBI_VERSION: usize = 0;
    pub const BASE_GET_IMPL_ID: usize = 1;
    pub const BASE_GET_IMPL_VERSION: usize = 2;
    pub const BASE_PROBE_EXTENSION: usize = 3;
    pub const BASE_GET_MVENDORID: usize = 4;
    pub const BASE_GET_MARCHID: usize = 5;
    pub const BASE_GET_MIMPID: usize = 6;

    pub const SRST_SHUTDOWN: usize = 0;
    #[allow(dead_code)]
    pub const SRST_COLD_REBOOT: usize = 1;
    #[allow(dead_code)]
    pub const SRST_WARM_REBOOT: usize = 2;
}

/// SBI error codes
mod error {
    pub const SUCCESS: isize = 0;
    pub const ERR_NOT_SUPPORTED: isize = -2;
}

/// SBI return value
#[repr(C)]
pub struct SbiRet {
    pub error: isize,
    pub value: usize,
}

impl SbiRet {
    fn success(value: usize) -> Self {
        SbiRet {
            error: error::SUCCESS,
            value,
        }
    }

    fn not_supported() -> Self {
        SbiRet {
            error: error::ERR_NOT_SUPPORTED,
            value: 0,
        }
    }
}

/// Handle legacy console putchar (EID 0x01)
fn handle_console_putchar(c: usize) -> SbiRet {
    uart::putchar(c as u8);
    SbiRet::success(0)
}

/// Handle legacy console getchar (EID 0x02)
fn handle_console_getchar() -> SbiRet {
    loop {
        if let Some(c) = uart::getchar() {
            return SbiRet::success(c as usize);
        } else {
            continue;
        }
    }
}

/// Handle timer extension (EID 0x54494D45)
fn handle_timer(time: u64) -> SbiRet {
    const CLINT_MTIMECMP: usize = 0x200_4000;
    unsafe {
        (CLINT_MTIMECMP as *mut u64).write_volatile(time);
    }
    // Clear pending timer interrupt by setting mtimecmp
    unsafe {
        asm!(
            "csrc mip, {}",
            in(reg) (1 << 5), // Clear STIP
        );
    }
    SbiRet::success(0)
}

/// Handle system reset
fn handle_system_reset(fid: usize) -> SbiRet {
    const VIRT_TEST: usize = 0x10_0000;
    const EXIT_SUCCESS: u32 = 0x5555;
    const EXIT_RESET: u32 = 0x3333;

    match fid {
        fid::SRST_SHUTDOWN => unsafe {
            (VIRT_TEST as *mut u32).write_volatile(EXIT_SUCCESS);
        },
        _ => unsafe {
            (VIRT_TEST as *mut u32).write_volatile(EXIT_RESET);
        },
    }
    loop {}
}

/// Handle SBI Base extension calls
fn handle_base(fid: usize) -> SbiRet {
    match fid {
        fid::BASE_GET_SBI_VERSION => SbiRet::success(0x01000000), // SBI v1.0.0
        fid::BASE_GET_IMPL_ID => SbiRet::success(0xFFFF),         // Custom implementation
        fid::BASE_GET_IMPL_VERSION => SbiRet::success(1),
        fid::BASE_PROBE_EXTENSION => SbiRet::success(1), // All extensions supported
        fid::BASE_GET_MVENDORID => SbiRet::success(0),
        fid::BASE_GET_MARCHID => SbiRet::success(0),
        fid::BASE_GET_MIMPID => SbiRet::success(0),
        _ => SbiRet::not_supported(),
    }
}

/// Main M-mode trap handler called from assembly
///
/// This function handles ecalls from S-mode and dispatches them to the
/// appropriate handler based on the extension ID and function ID.
#[unsafe(no_mangle)]
pub fn m_trap_handler(
    a0: usize,
    _a1: usize,
    _a2: usize,
    _a3: usize,
    _a4: usize,
    _a5: usize,
    fid: usize,
    eid: usize,
) -> SbiRet {
    // Check mcause, only handle ecall from S-mode (cause = 9)
    let mcause: usize;
    unsafe {
        core::arch::asm!("csrr {}, mcause", out(reg) mcause);
    }

    if mcause != 9 {
        return SbiRet::not_supported();
    }

    // Handle SBI calls based on EID
    match eid {
        eid::CONSOLE_PUTCHAR => handle_console_putchar(a0),
        eid::CONSOLE_GETCHAR => handle_console_getchar(),
        eid::TIMER => handle_timer(a0 as u64),
        eid::SHUTDOWN => handle_system_reset(fid::SRST_SHUTDOWN),
        eid::BASE => handle_base(fid),
        eid::SRST => handle_system_reset(fid),
        _ => SbiRet::not_supported(),
    }
}
