//! 基于帧指针的栈回溯（布局与遍历逻辑对齐 `axbacktrace` crate）。
//!
//! 沿 RISC-V `s0`（帧指针）链回溯，打印 `fp` 与 `ra`，
//! 并对每一帧通过 `symtab_resolve` 打印源码级函数路径、行号与形参值。
//!
//! **编译要求**：`.cargo/config.toml` 中为 `riscv64gc-unknown-none-elf` 设置
//! `-C force-frame-pointers=yes`，否则回溯可能不完整。

use tg_sbi::console_putchar;

#[repr(C)]
#[derive(Clone, Copy)]
struct Frame {
    fp: usize,
    ip: usize,
}

impl Frame {
    fn read(fp: usize) -> Option<Self> {
        if fp < core::mem::size_of::<Self>() {
            return None;
        }
        let p = unsafe { (fp as *const u8).sub(core::mem::size_of::<Self>()) };
        Some(unsafe {
            Self {
                fp: p.cast::<usize>().read_unaligned(),
                ip: p.add(core::mem::size_of::<usize>())
                    .cast::<usize>()
                    .read_unaligned(),
            }
        })
    }
}

/// S 态内核代码范围（用于过滤无效 `ra`）。
const IP_RANGE: core::ops::Range<usize> = 0x8020_0000..0x8100_0000;

/// 帧指针有效范围——内核恒等映射区域。
const FP_RANGE: core::ops::Range<usize> = 0x8020_0000..0x8800_0000;

const MAX_DEPTH: usize = 48;

fn put_bytes(s: &[u8]) {
    for b in s {
        console_putchar(*b);
    }
}

fn put_hex_u64(x: u64) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for c in b"0x" {
        console_putchar(*c);
    }
    for shift in (0..16).rev() {
        let nibble = ((x >> (shift * 4)) & 0xf) as usize;
        console_putchar(DIGITS[nibble]);
    }
}

fn put_usize_hex(x: usize) {
    put_hex_u64(x as u64);
}

fn read_frame_pointer() -> usize {
    let fp: usize;
    unsafe {
        core::arch::asm!("mv {fp}, s0", fp = out(reg) fp);
    }
    fp
}

/// 打印当前线程从 `s0` 出发的调用栈；每帧显示 `fn=name(params) at file:line`。
pub fn print_backtrace() {
    put_bytes(b"[BACKTRACE] note=fp_unwind_riscv64_s0_symtab_line_params\n");

    let mut fp = read_frame_pointer();
    let mut depth = 0usize;

    core::hint::black_box(());

    while FP_RANGE.contains(&fp) && depth < MAX_DEPTH {
        let Some(frame) = Frame::read(fp) else {
            put_bytes(b"[BACKTRACE] stop=frame_read_failed fp=");
            put_usize_hex(fp);
            put_bytes(b"\n");
            break;
        };

        if frame.ip == 0 {
            put_bytes(b"[BACKTRACE] end=ra_null_bottom_of_chain\n");
            break;
        }

        if !IP_RANGE.contains(&frame.ip) {
            put_bytes(b"[BACKTRACE] stop=ra_out_of_ip_range ra=");
            put_usize_hex(frame.ip);
            put_bytes(b"\n");
            break;
        }

        put_bytes(b"[BACKTRACE] #");
        print_decimal(depth);
        put_bytes(b" fp=");
        put_usize_hex(fp);
        put_bytes(b" ra=");
        put_usize_hex(frame.ip);
        put_bytes(b"\n");

        crate::symtab_resolve::print_fn_for_ra(frame.ip, frame.fp);

        if let Some(limit) = fp.checked_add(8 * 1024 * 1024)
            && frame.fp >= limit
        {
            put_bytes(b"[BACKTRACE] stop=fp_chain_suspicious\n");
            break;
        }

        fp = frame.fp;
        depth += 1;

        if fp == 0 {
            break;
        }
    }

    if depth == 0 {
        put_bytes(
            b"[BACKTRACE] hint=no_frames_try_rustflags_force_frame_pointers=yes\n",
        );
    }
}

fn print_decimal(mut n: usize) {
    if n == 0 {
        console_putchar(b'0');
        return;
    }
    let mut buf = [0u8; 10];
    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        i += 1;
        n /= 10;
    }
    while i > 0 {
        i -= 1;
        console_putchar(buf[i]);
    }
}
