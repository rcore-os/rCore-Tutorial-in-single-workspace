//! 运行时 DWARF 解析（`addr2line` + 链接进镜像的 `.debug_*`）。
//!
//! 需启用 Cargo feature `dwarf-symbols`，且在 **dev** 构建下使用（`--release` 若剥离调试信息会失败）。
//!
//! 段地址通过 `asm/dwarf_ptrs.S` 中 `.dword __start_*` / `__stop_*` 以 **绝对重定位**
//! 写入 `.data`，避免 `dwarf::init` 内对远端符号使用 PC 相对寻址导致 `R_RISCV_PCREL_HI20` 越界。

#![cfg(feature = "dwarf-symbols")]
#![allow(static_mut_refs)]

extern crate alloc;

use alloc::borrow::Cow;
use alloc::string::String;

use addr2line::Context;
use tg_sbi::console_putchar;

/// 与 `axbacktrace` 相同的 `gimli` 读类型。
pub type DwarfReader = gimli::EndianSlice<'static, gimli::RunTimeEndian>;

static mut CONTEXT: Option<Context<DwarfReader>> = None;

#[repr(C)]
struct PtrPair {
    start: usize,
    end: usize,
}

unsafe extern "C" {
    safe static ch1_dbg_abbrev: PtrPair;
    safe static ch1_dbg_addr: PtrPair;
    safe static ch1_dbg_aranges: PtrPair;
    safe static ch1_dbg_info: PtrPair;
    safe static ch1_dbg_line: PtrPair;
    safe static ch1_dbg_line_str: PtrPair;
    safe static ch1_dbg_ranges: PtrPair;
    safe static ch1_dbg_rnglists: PtrPair;
    safe static ch1_dbg_str: PtrPair;
    safe static ch1_dbg_str_offsets: PtrPair;
}

fn slice_pair(p: &PtrPair) -> DwarfReader {
    let len = p.end.saturating_sub(p.start);
    let sl = unsafe { core::slice::from_raw_parts(p.start as *const u8, len) };
    DwarfReader::new(sl, gimli::RunTimeEndian::default())
}

/// 解析 DWARF 并建立 `addr2line` 上下文（需在 [`crate::heap::init`] 之后调用）。
pub fn init() {
    let debug_abbrev = slice_pair(&ch1_dbg_abbrev).into();
    let debug_addr = slice_pair(&ch1_dbg_addr).into();
    let debug_aranges = slice_pair(&ch1_dbg_aranges).into();
    let debug_info = slice_pair(&ch1_dbg_info).into();
    let debug_line = slice_pair(&ch1_dbg_line).into();
    let debug_line_str = slice_pair(&ch1_dbg_line_str).into();
    let debug_ranges = slice_pair(&ch1_dbg_ranges).into();
    let debug_rnglists = slice_pair(&ch1_dbg_rnglists).into();
    let debug_str = slice_pair(&ch1_dbg_str).into();
    let debug_str_offsets = slice_pair(&ch1_dbg_str_offsets).into();

    let default_section = DwarfReader::new(&[], gimli::RunTimeEndian::default());

    // `from_sections` 会大量分配；堆须足够大（见 `build.rs` HEAP_SIZE）。
    match Context::from_sections(
        debug_abbrev,
        debug_addr,
        debug_aranges,
        debug_info,
        debug_line,
        debug_line_str,
        debug_ranges,
        debug_rnglists,
        debug_str,
        debug_str_offsets,
        default_section,
    ) {
        Ok(ctx) => unsafe {
            CONTEXT = Some(ctx);
        },
        Err(_) => unsafe {
            CONTEXT = None;
        },
    }
}

/// 是否已成功加载 DWARF（节为空或解析失败时为 `false`）。
pub fn is_ready() -> bool {
    unsafe { CONTEXT.as_ref().is_some() }
}

fn put_bytes(s: &[u8]) {
    for b in s {
        console_putchar(*b);
    }
}

fn put_usize_hex(x: usize) {
    put_bytes(b"0x");
    let mut x = x as u64;
    if x == 0 {
        console_putchar(b'0');
        return;
    }
    let mut buf = [0u8; 16];
    let mut i = 0usize;
    while x > 0 {
        let d = (x & 0xf) as u8;
        buf[i] = if d < 10 { b'0' + d } else { b'a' + (d - 10) };
        i += 1;
        x >>= 4;
    }
    while i > 0 {
        i -= 1;
        console_putchar(buf[i]);
    }
}

/// 将 `ra`（返回地址）解析为「函数名 + 位置」并打印到串口（前缀 `[BACKTRACE]`）。
///
/// 依次尝试 `ra-1` 与 `ra`（与 `axbacktrace::Frame::adjust_ip` 一致），再尝试仅行号查询。
/// 若仍无结果，打印占位说明（部分 rustc/目标组合下 DWARF 可能缺少可索引的 PC 范围或行表）。
pub fn print_location_for_ra(ra: usize) {
    put_bytes(b"[BACKTRACE]   sym=");
    let Some(ctx) = (unsafe { CONTEXT.as_ref() }) else {
        put_bytes(b"<no_dwarf_context>\n");
        return;
    };

    let probes = [ra.wrapping_sub(1) as u64, ra as u64];

    for &probe in &probes {
        let mut frames = match ctx.find_frames(probe).skip_all_loads() {
            Ok(f) => f,
            Err(_) => continue,
        };
        let frame = match frames.next() {
            Ok(Some(f)) => f,
            Ok(None) | Err(_) => continue,
        };

        let func = frame
            .function
            .as_ref()
            .and_then(|func| func.demangle().ok())
            .unwrap_or(Cow::Borrowed("<unknown>"));

        for c in func.as_bytes() {
            console_putchar(*c);
        }

        if let Some(loc) = &frame.location {
            if let Some(file) = loc.file {
                put_bytes(b" at ");
                for c in file.as_bytes() {
                    console_putchar(*c);
                }
                if let Some(line) = loc.line {
                    let mut s = String::new();
                    use core::fmt::Write;
                    let _ = write!(&mut s, ":{line}");
                    for c in s.as_bytes() {
                        console_putchar(*c);
                    }
                }
            }
        }
        put_bytes(b"\n");
        return;
    }

    for &probe in &probes {
        if let Ok(Some(loc)) = ctx.find_location(probe) {
            if let Some(file) = loc.file {
                put_bytes(b"<line_only> at ");
                for c in file.as_bytes() {
                    console_putchar(*c);
                }
                if let Some(line) = loc.line {
                    let mut s = String::new();
                    use core::fmt::Write;
                    let _ = write!(&mut s, ":{line}");
                    for c in s.as_bytes() {
                        console_putchar(*c);
                    }
                }
                put_bytes(b"\n");
                return;
            }
        }
    }

    put_bytes(b"<no_addr2line_match> ra=");
    put_usize_hex(ra);
    put_bytes(
        b" note=see_README_dwarf_symbols_toolchain\n",
    );
}
