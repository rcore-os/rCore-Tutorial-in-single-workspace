//! 运行时 `ra` → 源码级函数信息解析（函数名 + 实际参数值 + 源文件:行号）。
//!
//! `build.rs` 在编译前从上一次构建产物中提取四类信息并生成 `func_syms_generated.rs`：
//!
//! - **`FUNC_SYMS`**：ELF `.symtab` 中的函数符号（按地址排序）
//! - **`LINE_TABLE` / `LINE_FILES`**：DWARF 行号表（指令地址 → 源文件 + 行号）
//! - **`FUNC_PARAM_LOCS`**：每个函数形参的 fbreg 栈偏移 + 字节大小 + 类型 kind
//!
//! 运行时通过帧指针 `fp + fbreg_offset` 直接读取各参数的实际值。
//!
//! 注：由 `main.rs` 用 `#[cfg(target_arch = "riscv64")]` 声明本模块；主机 `cargo check` 使用桩实现。

use tg_sbi::console_putchar;

include!(concat!(env!("OUT_DIR"), "/func_syms_generated.rs"));

const TEXT_BASE: u64 = 0x80200000;
const FP_RANGE: core::ops::Range<usize> = 0x8020_0000..0x8800_0000;

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

fn lookup_sym(ra: u64) -> Option<(u64, u64, &'static str)> {
    if FUNC_SYMS.is_empty() {
        return None;
    }
    let effective = ra.saturating_sub(1);
    let mut lo = 0usize;
    let mut hi = FUNC_SYMS.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if FUNC_SYMS[mid].0 <= effective {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return None;
    }
    let (addr, size, name) = FUNC_SYMS[lo - 1];
    if size != 0 && effective >= addr.saturating_add(size) {
        return None;
    }
    Some((addr, size, name))
}

fn lookup_line(ra: usize) -> Option<(&'static str, u32)> {
    if LINE_TABLE.is_empty() || ra < TEXT_BASE as usize {
        return None;
    }
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

fn param_range(func_addr: u64) -> (usize, usize) {
    if FUNC_PARAM_LOCS.is_empty() {
        return (0, 0);
    }
    let mut lo = 0usize;
    let mut hi = FUNC_PARAM_LOCS.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if FUNC_PARAM_LOCS[mid].0 < func_addr {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let start = lo;
    hi = FUNC_PARAM_LOCS.len();
    while lo < hi {
        if FUNC_PARAM_LOCS[lo].0 != func_addr {
            break;
        }
        lo += 1;
    }
    (start, lo)
}

unsafe fn read_stack_bytes(addr: usize, n: usize) -> Option<&'static [u8]> {
    if n == 0 || !FP_RANGE.contains(&addr) {
        return None;
    }
    let end = addr.checked_add(n)?;
    if !FP_RANGE.contains(&(end - 1)) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(addr as *const u8, n) })
}

fn print_decimal_u64(val: u64) {
    if val == 0 {
        console_putchar(b'0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut v = val;
    while v > 0 {
        buf[i] = b'0' + (v % 10) as u8;
        i += 1;
        v /= 10;
    }
    while i > 0 {
        i -= 1;
        console_putchar(buf[i]);
    }
}

fn print_decimal_i64(val: i64) {
    if val < 0 {
        console_putchar(b'-');
        if val == i64::MIN {
            put_bytes(b"9223372036854775808");
            return;
        }
        print_decimal_u64((-val) as u64);
    } else {
        print_decimal_u64(val as u64);
    }
}

fn print_hex_u64(val: u64) {
    put_bytes(b"0x");
    if val == 0 {
        console_putchar(b'0');
        return;
    }
    let mut buf = [0u8; 16];
    let mut i = 0;
    let mut v = val;
    while v > 0 {
        let d = (v & 0xf) as u8;
        buf[i] = if d < 10 { b'0' + d } else { b'a' + (d - 10) };
        i += 1;
        v >>= 4;
    }
    while i > 0 {
        i -= 1;
        console_putchar(buf[i]);
    }
}

fn print_param_value(fp: usize, fbreg_offset: i16, byte_size: u8, kind: u8) {
    let addr = (fp as isize).wrapping_add(fbreg_offset as isize) as usize;
    let bytes = match unsafe { read_stack_bytes(addr, byte_size as usize) } {
        Some(b) => b,
        None => {
            put_bytes(b"?");
            return;
        }
    };

    match kind {
        0 => {
            let val = read_le_uint(bytes);
            print_decimal_u64(val);
        }
        1 => {
            let val = read_le_int(bytes);
            print_decimal_i64(val);
        }
        2 => {
            if bytes[0] != 0 {
                put_bytes(b"true");
            } else {
                put_bytes(b"false");
            }
        }
        3 => {
            if bytes.len() < 16 {
                put_bytes(b"?");
                return;
            }
            let ptr = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
            let len = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
            print_str_value(ptr, len);
        }
        _ => {
            let val = read_le_uint(bytes);
            print_hex_u64(val);
        }
    }
}

fn read_le_uint(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    for (i, &b) in bytes.iter().enumerate().take(8) {
        val |= (b as u64) << (i * 8);
    }
    val
}

fn read_le_int(bytes: &[u8]) -> i64 {
    let raw = read_le_uint(bytes);
    let nbits = bytes.len().min(8) * 8;
    if nbits < 64 && (raw >> (nbits - 1)) & 1 != 0 {
        (raw | (!0u64 << nbits)) as i64
    } else {
        raw as i64
    }
}

fn print_str_value(ptr: usize, len: usize) {
    const MAX_DISPLAY: usize = 64;
    put_bytes(b"\"");
    if ptr == 0 || len == 0 {
        put_bytes(b"\"");
        return;
    }
    if !FP_RANGE.contains(&ptr) && !(0x8020_0000..0x8800_0000).contains(&ptr) {
        put_bytes(b"<invalid>\"");
        return;
    }
    let display_len = len.min(MAX_DISPLAY);
    let end = match ptr.checked_add(display_len) {
        Some(e) => e,
        None => {
            put_bytes(b"<invalid>\"");
            return;
        }
    };
    if end > 0x8800_0000 {
        put_bytes(b"<invalid>\"");
        return;
    }
    let str_bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, display_len) };
    for &b in str_bytes {
        if b >= 0x20 && b < 0x7f {
            console_putchar(b);
        } else {
            console_putchar(b'.');
        }
    }
    if len > MAX_DISPLAY {
        put_bytes(b"...");
    }
    put_bytes(b"\"");
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

/// 打印 `[BACKTRACE]   fn=name(p1=v1, p2=v2) at file:line`。
pub fn print_fn_for_ra(ra: usize, fp: usize) {
    use core::fmt::Write;
    put_bytes(b"[BACKTRACE]   fn=");

    let Some((func_addr, _size, name)) = lookup_sym(ra as u64) else {
        put_bytes(b"<no_symbol>\n");
        return;
    };

    let dem = rustc_demangle::demangle(name);
    let _ = write!(SbiWriter, "{dem}");

    put_bytes(b"(");
    let (pstart, pend) = param_range(func_addr);
    for i in pstart..pend {
        if i > pstart {
            put_bytes(b", ");
        }
        let (_, pname, fbreg_offset, byte_size, kind) = FUNC_PARAM_LOCS[i];
        for b in pname.bytes() {
            console_putchar(b);
        }
        put_bytes(b"=");
        print_param_value(fp, fbreg_offset, byte_size, kind);
    }
    put_bytes(b")");

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
