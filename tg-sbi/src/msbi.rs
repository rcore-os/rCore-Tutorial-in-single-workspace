//! 用于 `-bios none` 启动的最小 M-Mode SBI 实现。
//!
//! 本模块在没有外部引导程序（如 RustSBI）的情况下提供基本的 SBI 服务。
//! 它处理来自 S-mode 的 ecall 并提供：
//! - 控制台 I/O（UART）
//! - 定时器管理
//! - 系统重置

use core::arch::asm;

const UART_BASE: usize = 0x1000_0000;

/// UART 操作（16550 兼容）。
mod uart {
    use super::UART_BASE;

    const THR: usize = UART_BASE; // 发送保持寄存器
    const LSR: usize = UART_BASE + 5; // 线路状态寄存器

    /// 检查 UART 是否准备好发送。
    #[inline]
    fn is_tx_ready() -> bool {
        // SAFETY: 从 QEMU virt 机器的已知 MMIO 地址读取 UART LSR 寄存器。
        // 这是只读操作，用于检查 THRE（发送保持寄存器空）位。
        unsafe {
            let lsr = (LSR as *const u8).read_volatile();
            (lsr & 0x20) != 0 // THRE bit
        }
    }

    /// 向 UART 写入一个字节。
    pub fn putchar(c: u8) {
        while !is_tx_ready() {}
        // SAFETY: 向 QEMU virt 机器的已知 MMIO 地址写入 UART THR 寄存器。
        // 我们已通过 is_tx_ready() 验证 UART 已准备好。
        unsafe {
            (THR as *mut u8).write_volatile(c);
        }
    }

    /// 从 UART 读取一个字节（非阻塞）。
    pub fn getchar() -> Option<u8> {
        // SAFETY: 从 QEMU virt 机器的已知 MMIO 地址读取 UART LSR 寄存器，
        // 检查数据就绪状态。
        let lsr = unsafe { (LSR as *const u8).read_volatile() };
        if lsr & 1 != 0 {
            // SAFETY: 从 UART THR 寄存器读取。我们已通过 LSR 数据就绪位验证数据可用。
            Some(unsafe { (THR as *const u8).read_volatile() })
        } else {
            None
        }
    }
}

/// SBI 扩展 ID。
mod eid {
    pub const CONSOLE_PUTCHAR: usize = 0x01;
    pub const CONSOLE_GETCHAR: usize = 0x02;
    pub const SHUTDOWN: usize = 0x08;
    pub const BASE: usize = 0x10;
    pub const SRST: usize = 0x53525354;
    pub const TIMER: usize = 0x54494D45;
}

/// SBI 功能 ID。
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

/// SBI 错误码。
mod error {
    pub const SUCCESS: isize = 0;
    pub const ERR_NOT_SUPPORTED: isize = -2;
}

/// SBI 返回值。
#[repr(C)]
pub struct SbiRet {
    /// 错误码。
    pub error: isize,
    /// 返回值。
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

/// 处理 Legacy 控制台 putchar（EID 0x01）。
fn handle_console_putchar(c: usize) -> SbiRet {
    uart::putchar(c as u8);
    SbiRet::success(0)
}

/// 处理 Legacy 控制台 getchar（EID 0x02）。
fn handle_console_getchar() -> SbiRet {
    loop {
        if let Some(c) = uart::getchar() {
            return SbiRet::success(c as usize);
        } else {
            continue;
        }
    }
}

/// 处理 Timer 扩展（EID 0x54494D45）。
fn handle_timer(time: u64) -> SbiRet {
    const CLINT_MTIMECMP: usize = 0x200_4000;
    // SAFETY: 向 QEMU virt 机器的已知 MMIO 地址写入 CLINT mtimecmp 寄存器。
    // 这将设置下一次定时器中断的触发时间。
    unsafe {
        (CLINT_MTIMECMP as *mut u64).write_volatile(time);
    }
    // 清除挂起的 S-mode 定时器中断
    // SAFETY: 修改 mip CSR 以清除 STIP 位是有效的 M-mode 操作。
    // 这是确认定时器中断所必需的。
    unsafe {
        asm!(
            "csrc mip, {}",
            in(reg) (1 << 5), // Clear STIP
        );
    }
    SbiRet::success(0)
}

/// 处理系统重置请求。
fn handle_system_reset(fid: usize) -> SbiRet {
    const VIRT_TEST: usize = 0x10_0000;
    const EXIT_SUCCESS: u32 = 0x5555;
    const EXIT_RESET: u32 = 0x3333;

    match fid {
        fid::SRST_SHUTDOWN => {
            // SAFETY: 向 QEMU virt test 设备的已知 MMIO 地址写入会触发系统关机。
            // 这是终止 QEMU virt 机器仿真的标准方式。
            unsafe {
                (VIRT_TEST as *mut u32).write_volatile(EXIT_SUCCESS);
            }
        }
        _ => {
            // SAFETY: 同上，但触发重置而非干净关机。
            unsafe {
                (VIRT_TEST as *mut u32).write_volatile(EXIT_RESET);
            }
        }
    }
    loop {}
}

/// 处理 SBI Base 扩展调用。
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

/// 从汇编调用的主 M-mode 陷阱处理程序。
///
/// 此函数处理来自 S-mode 的 ecall，并根据扩展 ID 和功能 ID 分发到相应的处理程序。
///
/// # Safety
///
/// 此函数使用 `#[unsafe(no_mangle)]` 标记，因为它直接从汇编陷阱向量
/// （`m_trap_vector`）调用。调用者必须确保寄存器状态符合预期的调用约定。
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
    // 检查 mcause，仅处理来自 S-mode 的 ecall（cause = 9）
    let mcause: usize;
    // SAFETY: 读取 mcause CSR 是有效的 M-mode 操作，它告诉我们陷阱的原因。
    // 我们需要此信息来验证这是一个 S-mode ecall。
    unsafe {
        core::arch::asm!("csrr {}, mcause", out(reg) mcause);
    }

    if mcause != 9 {
        return SbiRet::not_supported();
    }

    // 根据 EID 处理 SBI 调用
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
