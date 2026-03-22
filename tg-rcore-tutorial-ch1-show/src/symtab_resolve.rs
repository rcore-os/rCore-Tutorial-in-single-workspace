//! 运行时 `ra` → 源码级函数信息解析（函数名 + 源文件:行号 + 形参名）。
//!
//! `build.rs` 在编译前从上一次构建产物中提取三类信息并生成 `func_syms_generated.rs`：
//!
//! - **`FUNC_SYMS`**：ELF `.symtab` 中的函数符号（按地址排序）
//! - **`LINE_TABLE` / `LINE_FILES`**：DWARF 行号表（指令地址 → 源文件 + 行号）
//! - **`FUNC_PARAMS`**：DWARF `.debug_info` 中各函数的形参名
//!
//! 从干净构建算起需两次 `cargo build` 填充数据（首次无上轮产物，生成空表）。

#![cfg(target_arch = "riscv64")]

use tg_sbi::console_putchar;

include!(concat!(env!("OUT_DIR"), "/func_syms_generated.rs"));

const TEXT_BASE: u64 = 0x80200000;

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

/// No-op；符号与行号信息已在编译期由 `build.rs` 嵌入。
pub fn init() {}

// ---- symbol lookup ----

fn lookup_sym(ra: u64) -> Option<(u64, u64, &'static str)> {
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

// ---- line table lookup ----

fn lookup_line(ra: usize) -> Option<(&'static str, u32)> {
    if LINE_TABLE.is_empty() || ra < TEXT_BASE as usize {
        return None;
    }
    // ra - 2: 指向 call 指令本身而非其后一条（RISC-V 最小指令 2 字节）
    let effective = ra.saturating_sub(2);
    let off = (effective as u64).wrapping_sub(TEXT_BASE) as u32;

    let mut lo = 0usize;
    let mut hi = LINE_TABLE.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if LINE_TABLE[mid].0 <= off {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return None;
    }
    let (_, file_idx, line) = LINE_TABLE[lo - 1];
    if line == 0 {
        return None;
    }
    let file = LINE_FILES.get(file_idx as usize)?;
    Some((file, line))
}

// ---- param lookup ----

fn lookup_params(func_addr: u64) -> Option<&'static str> {
    if FUNC_PARAMS.is_empty() {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = FUNC_PARAMS.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if FUNC_PARAMS[mid].0 < func_addr {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo < FUNC_PARAMS.len() && FUNC_PARAMS[lo].0 == func_addr {
        let s = FUNC_PARAMS[lo].1;
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
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

/// 打印 `[BACKTRACE]   fn=name(params) at file:line` 或占位行。
pub fn print_fn_for_ra(ra: usize) {
    use core::fmt::Write;
    put_bytes(b"[BACKTRACE]   fn=");

    let Some((func_addr, _size, name)) = lookup_sym(ra as u64) else {
        put_bytes(b"<no_symbol>\n");
        return;
    };

    let dem = rustc_demangle::demangle(name);
    let _ = write!(SbiWriter, "{dem}");

    // 形参名
    put_bytes(b"(");
    if let Some(params) = lookup_params(func_addr) {
        for b in params.bytes() {
            console_putchar(b);
        }
    }
    put_bytes(b")");

    // 源文件:行号
    if let Some((file, line)) = lookup_line(ra) {
        put_bytes(b" at ");
        for b in file.bytes() {
            console_putchar(b);
        }
        put_bytes(b":");
        print_decimal(line as usize);
    }

    put_bytes(b"\n");
}
