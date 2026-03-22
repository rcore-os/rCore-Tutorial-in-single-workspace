//! 基于帧指针的栈回溯（布局与遍历逻辑对齐工作区中的 `axbacktrace` crate）。
//!
//! 裸机无 `std`；沿 **RISC-V `s0`（帧指针）** 链回溯，打印 **`fp` 与 `ra`**，
//! 并对每一帧打印 **`.symtab` + `rustc_demangle` 的 Rust 函数路径**（源码级符号，见 [`crate::symtab_resolve`]）。
//! 启用 Cargo feature **`dwarf-symbols`** 时，在 DWARF 就绪后额外尝试 **`addr2line` 行号**（可选）。
//!
//! **编译要求**：`.cargo/config.toml` 中为 `riscv64gc-unknown-none-elf` 设置
//! `-C force-frame-pointers=yes`，否则 LLVM 可能省略帧指针链，回溯会不完整或为空。
//!
//! 帧内存布局与 [`axbacktrace::Frame`] 一致：非 x86/aarch64 架构下从 `fp - sizeof(Frame)` 读取
//! `{ prev_fp, return_addr }`。

use tg_sbi::console_putchar;

/// 与 `axbacktrace::Frame` 相同：栈上保存的上一帧指针与返回地址。
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
        // 与 axbacktrace 相同：在 `fp - 2*sizeof(usize)` 处为 `{prev_fp, ra}`。
        // 使用非对齐读：LLVM 可能使 `s0` 仅 4 字节对齐，严格 `align_of::<Frame>` 会误判。
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

/// 可执行代码大致范围（S 态镜像，用于过滤无效 `ra`；与 `build.rs` 中 `S_BASE` 同量级）。
///
/// 过宽会降低误跟随指针的风险；过窄可能截断合法尾帧。教学场景下取一段连续 RAM 即可。
const IP_RANGE: core::ops::Range<usize> = 0x8020_0000..0x8100_0000;

/// 帧指针 `s0` 应落在此区间（与 `lec2_lab1` 打印的 `sp` 同一片 RAM：`.bss` 上栈，约 `0x806xxxxx`）。
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

/// 读取当前 `s0`（帧指针）。与 `axbacktrace::Backtrace::capture` 中 RISC-V 分支一致。
fn read_frame_pointer() -> usize {
    let fp: usize;
    unsafe {
        core::arch::asm!("mv {fp}, s0", fp = out(reg) fp);
    }
    fp
}

/// 打印当前线程从 `s0` 出发的调用栈；每帧带 demangled `fn=`，可选 DWARF `sym=`。
pub fn print_backtrace() {
    #[cfg(feature = "dwarf-symbols")]
    put_bytes(
        b"[BACKTRACE] note=fp_unwind_riscv64_s0_symtab_demangle plus_addr2line_if_ready\n",
    );
    #[cfg(not(feature = "dwarf-symbols"))]
    put_bytes(b"[BACKTRACE] note=fp_unwind_riscv64_s0_symtab_demangle same_layout_as_axbacktrace\n");

    let mut fp = read_frame_pointer();
    let mut depth = 0usize;

    // 防止尾调用优化掉本帧：保留对自身的引用。
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

        crate::symtab_resolve::print_fn_for_ra(frame.ip);

        #[cfg(feature = "dwarf-symbols")]
        {
            if crate::dwarf::is_ready() {
                crate::dwarf::print_location_for_ra(frame.ip);
            } else {
                put_bytes(b"[BACKTRACE]   sym=<dwarf_not_ready>\n");
            }
        }

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
