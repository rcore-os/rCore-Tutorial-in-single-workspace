//! 构建脚本：为 RISC-V64 目标自动生成链接脚本，并从上次构建产物的
//! `.symtab` 中提取函数符号，嵌入为 `const` 数组供运行时栈回溯解析。
//!
//! 链接脚本控制程序各段在内存中的布局，确保：
//! - M-mode 代码（tg-sbi）从 0x80000000 开始
//! - S-mode 代码（_start 入口）从 0x80200000 开始
//! - （可选）堆区 `__heap_start` / `__heap_end` 供 `#[global_allocator]`
//! - （可选）`.debug_*` 收集进 PT_LOAD，供运行时 `addr2line` 读取

fn main() {
    use std::{env, fs, path::PathBuf};

    // 仅在交叉编译到 RISC-V64 时生成链接脚本
    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "riscv64" {
        // 供 `src/lec2_lab1.rs` 在串口打印固定布局锚点（与 README / lec2 幻灯对照）
        println!("cargo:rustc-env=LAB_M_BASE=0x80000000");
        println!("cargo:rustc-env=LAB_S_BASE=0x80200000");
        let triple = env::var("TARGET").unwrap_or_else(|_| "riscv64gc-unknown-none-elf".into());
        println!("cargo:rustc-env=LAB_TARGET_TRIPLE={triple}");

        let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        let ld = out_dir.join("linker.ld");
        fs::write(&ld, LINKER_SCRIPT).unwrap();
        // 告诉 rustc 使用此链接脚本
        println!("cargo:rustc-link-arg=-T{}", ld.display());

        // --- 从上一次构建产物中提取函数符号 ---
        // OUT_DIR 通常为 target/<triple>/<profile>/build/<pkg>-<hash>/out
        // 上溯 3 级得到 target/<triple>/<profile>/
        let profile_dir = out_dir.ancestors().nth(3).unwrap().to_path_buf();
        let pkg_name = env::var("CARGO_PKG_NAME").unwrap();
        let prev_binary = profile_dir.join(&pkg_name);

        // 当上次产物出现/变化时重新执行 build.rs（首次构建产物不存在→空表）
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", prev_binary.display());

        let syms = extract_func_symbols(&prev_binary);
        write_func_syms_rs(&syms, &out_dir.join("func_syms_generated.rs"));
    }
}

// ---------------------------------------------------------------------------
// ELF64 symbol extraction (runs on build host, reads previous RISC-V binary)
// ---------------------------------------------------------------------------

/// Try to extract FUNC symbols from a previously built ELF64-LE binary.
fn extract_func_symbols(path: &std::path::Path) -> Vec<(u64, u64, String)> {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    if data.len() < 64 || &data[0..4] != b"\x7fELF" || data[4] != 2 || data[5] != 1 {
        return Vec::new();
    }

    let u16le = |off: usize| u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
    let u32le = |off: usize| u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
    let u64le = |off: usize| u64::from_le_bytes(data[off..off + 8].try_into().unwrap());

    let e_shoff = u64le(40) as usize;
    let e_shentsize = u16le(58) as usize;
    let e_shnum = u16le(60) as usize;

    if e_shoff == 0 || e_shnum == 0 || e_shentsize < 64 {
        return Vec::new();
    }
    if e_shoff + e_shnum * e_shentsize > data.len() {
        return Vec::new();
    }

    // Find SHT_SYMTAB (type 2)
    let mut symtab_sh = None;
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        if u32le(sh + 4) == 2 {
            symtab_sh = Some(sh);
            break;
        }
    }
    let sh = match symtab_sh {
        Some(s) => s,
        None => return Vec::new(),
    };

    let sym_off = u64le(sh + 24) as usize;
    let sym_size = u64le(sh + 32) as usize;
    let sym_link = u32le(sh + 40) as usize;
    let sym_entsize = {
        let e = u64le(sh + 56) as usize;
        if e > 0 { e } else { 24 }
    };

    if sym_link >= e_shnum {
        return Vec::new();
    }

    let strtab_sh = e_shoff + sym_link * e_shentsize;
    let str_off = u64le(strtab_sh + 24) as usize;
    let str_size = u64le(strtab_sh + 32) as usize;

    if sym_off + sym_size > data.len() || str_off + str_size > data.len() {
        return Vec::new();
    }

    let sym_data = &data[sym_off..sym_off + sym_size];
    let str_data = &data[str_off..str_off + str_size];
    let n = sym_data.len() / sym_entsize;
    let mut result = Vec::new();

    for i in 0..n {
        let off = i * sym_entsize;
        if off + 24 > sym_data.len() {
            break;
        }
        let st_name = u32::from_le_bytes(sym_data[off..off + 4].try_into().unwrap());
        let st_info = sym_data[off + 4];
        let st_value = u64::from_le_bytes(sym_data[off + 8..off + 16].try_into().unwrap());
        let st_size = u64::from_le_bytes(sym_data[off + 16..off + 24].try_into().unwrap());

        if (st_info & 0xf) != 2 {
            continue; // not STT_FUNC
        }
        if st_name == 0 || st_value < 0x80200000 || st_value >= 0x81000000 {
            continue;
        }

        let name_start = st_name as usize;
        if name_start >= str_data.len() {
            continue;
        }
        let name_end = str_data[name_start..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| name_start + p)
            .unwrap_or(str_data.len());
        let name = match std::str::from_utf8(&str_data[name_start..name_end]) {
            Ok(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };

        result.push((st_value, st_size, name));
    }

    result.sort_by_key(|(addr, _, _)| *addr);
    result
}

/// Generate `func_syms_generated.rs` with a sorted const array of (addr, size, name).
fn write_func_syms_rs(syms: &[(u64, u64, String)], out: &std::path::Path) {
    use std::fmt::Write;

    let mut content = String::with_capacity(syms.len() * 100 + 256);
    writeln!(content, "// Auto-generated by build.rs — do not edit").unwrap();
    writeln!(content, "// {count} function symbol(s) extracted from previous build", count = syms.len()).unwrap();
    writeln!(content, "const FUNC_SYMS: &[(u64, u64, &str)] = &[").unwrap();
    for (addr, size, name) in syms {
        let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
        writeln!(content, "    (0x{addr:x}, {size}, \"{escaped}\"),").unwrap();
    }
    writeln!(content, "];").unwrap();

    // Only write when content actually changed to avoid unnecessary recompilation
    let existing = std::fs::read_to_string(out).unwrap_or_default();
    if content != existing {
        std::fs::write(out, &content).unwrap();
    }
}

/// 链接脚本内容。
///
/// 不显式声明 `PHDRS`，由 `rust-lld` 自动合并 `PT_LOAD`。
///
/// 注意：链接脚本是字节字符串，不能包含非 ASCII 字符。
const LINKER_SCRIPT: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_m_start)

M_BASE_ADDRESS = 0x80000000;
S_BASE_ADDRESS = 0x80200000;

/* 4 MiB heap for addr2line / alloc (must match src/heap.rs HEAP_SIZE if changed) */
HEAP_SIZE = 0x1000000;

SECTIONS {
    . = M_BASE_ADDRESS;
    .text.m_entry : { *(.text.m_entry) }
    .text.m_trap  : { *(.text.m_trap)  }
    .bss.m_stack  : { *(.bss.m_stack)  }
    .bss.m_data   : { *(.bss.m_data)   }

    . = S_BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    /*
     * DWARF: merge into SHF_ALLOC .rodata. Standalone .debug_* gets VMA=0 and no PT_LOAD on rust-lld.
     * Symbol ranges use asm/dwarf_ptrs.S absolute .dword relocs (not PCREL to .text).
     */
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)

        . = ALIGN(8);
        PROVIDE(__start_debug_abbrev = .);
        KEEP(*(.debug_abbrev))
        PROVIDE(__stop_debug_abbrev = .);
        PROVIDE(__start_debug_addr = .);
        KEEP(*(.debug_addr))
        PROVIDE(__stop_debug_addr = .);
        PROVIDE(__start_debug_aranges = .);
        KEEP(*(.debug_aranges))
        PROVIDE(__stop_debug_aranges = .);
        PROVIDE(__start_debug_info = .);
        KEEP(*(.debug_info))
        PROVIDE(__stop_debug_info = .);
        PROVIDE(__start_debug_line = .);
        KEEP(*(.debug_line))
        PROVIDE(__stop_debug_line = .);
        PROVIDE(__start_debug_line_str = .);
        KEEP(*(.debug_line_str))
        PROVIDE(__stop_debug_line_str = .);
        PROVIDE(__start_debug_ranges = .);
        KEEP(*(.debug_ranges .debug_ranges.*))
        PROVIDE(__stop_debug_ranges = .);
        PROVIDE(__start_debug_rnglists = .);
        KEEP(*(.debug_rnglists .debug_rnglists.*))
        PROVIDE(__stop_debug_rnglists = .);
        PROVIDE(__start_debug_str = .);
        KEEP(*(.debug_str))
        PROVIDE(__stop_debug_str = .);
        PROVIDE(__start_debug_str_offsets = .);
        KEEP(*(.debug_str_offsets))
        PROVIDE(__stop_debug_str_offsets = .);
    }
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        *(.bss.uninit)
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(8);
    PROVIDE(__heap_start = .);
    .heap (NOLOAD) : {
        . += HEAP_SIZE;
    }
    PROVIDE(__heap_end = .);

    /DISCARD/ : {
        *(.eh_frame .eh_frame.*)
    }
}";
