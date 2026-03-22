//! 运行时 `ra` → demangled Rust 函数名解析。
//!
//! `build.rs` 在编译前从上一次构建产物的 ELF `.symtab` 中提取函数符号，
//! 生成 `func_syms_generated.rs`（按地址排序的 `const` 数组）。运行时通过
//! 二分查找匹配 `ra`，再经 `rustc_demangle` 输出可读函数路径。
//!
//! 从干净构建算起需要两次 `cargo build` 才能填充符号（首次无上一轮产物，生成空表）。

#![cfg(target_arch = "riscv64")]

use tg_sbi::console_putchar;

include!(concat!(env!("OUT_DIR"), "/func_syms_generated.rs"));

struct SbiWriter;

impl core::fmt::Write for SbiWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            console_putchar(b);
        }
        Ok(())
    }
}

fn put_bytes(b: &[u8]) {
    for x in b {
        console_putchar(*x);
    }
}

/// No-op；符号已在编译期由 `build.rs` 嵌入。
pub fn init() {}

fn lookup(ra: u64) -> Option<(u64, u64, &'static str)> {
    if FUNC_SYMS.is_empty() {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = FUNC_SYMS.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if FUNC_SYMS[mid].0 <= ra {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return None;
    }
    let (addr, size, name) = FUNC_SYMS[lo - 1];
    if size != 0 && ra >= addr.saturating_add(size) {
        return None;
    }
    Some((addr, size, name))
}

/// 打印 `[BACKTRACE]   fn=<demangled>` 或占位行。
pub fn print_fn_for_ra(ra: usize) {
    use core::fmt::Write;
    put_bytes(b"[BACKTRACE]   fn=");
    let Some((_addr, _size, name)) = lookup(ra as u64) else {
        put_bytes(b"<no_symbol>\n");
        return;
    };
    let dem = rustc_demangle::demangle(name);
    let _ = write!(SbiWriter, "{dem}");
    put_bytes(b"\n");
}
