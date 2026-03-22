//! 构建脚本：生成链接脚本，并从上一次构建产物中提取：
//!
//! 1. ELF `.symtab` 函数符号（地址 + mangled name）
//! 2. DWARF 行号表（每条指令地址 → 源文件 + 行号）
//! 3. 函数形参的栈偏移 + 类型（`DW_AT_location` + `DW_AT_type`）
//!
//! 结果嵌入为 `const` 数组，供运行时栈回溯逐帧显示
//! `fn=name(param=value, ...) at file:line`。

fn main() {
    use std::{env, fs, path::PathBuf};

    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "riscv64" {
        println!("cargo:rustc-env=LAB_M_BASE=0x80000000");
        println!("cargo:rustc-env=LAB_S_BASE=0x80200000");
        let triple = env::var("TARGET").unwrap_or_else(|_| "riscv64gc-unknown-none-elf".into());
        println!("cargo:rustc-env=LAB_TARGET_TRIPLE={triple}");

        let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        let ld = out_dir.join("linker.ld");
        fs::write(&ld, LINKER_SCRIPT).unwrap();
        println!("cargo:rustc-link-arg=-T{}", ld.display());

        // OUT_DIR ≈ target/<triple>/<profile>/build/<pkg>-<hash>/out
        let profile_dir = out_dir.ancestors().nth(3).unwrap().to_path_buf();
        let pkg_name = env::var("CARGO_PKG_NAME").unwrap();
        let prev_binary = profile_dir.join(&pkg_name);

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", prev_binary.display());

        let data = std::fs::read(&prev_binary).unwrap_or_default();
        let syms = extract_func_symbols(&data);
        let dwarf = extract_dwarf_info(&data);

        write_generated_rs(
            &syms,
            &dwarf,
            &out_dir.join("func_syms_generated.rs"),
        );
    }
}

// ===== ELF64 symbol extraction (raw byte parsing) =====

fn extract_func_symbols(data: &[u8]) -> Vec<(u64, u64, String)> {
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
            continue;
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

// ===== DWARF line table + parameter extraction (via addr2line / gimli) =====

const TEXT_BASE: u64 = 0x80200000;
const TEXT_LIMIT: u64 = 0x81000000;

struct ParamLoc {
    func_addr: u64,
    name: String,
    fbreg_offset: i16,
    byte_size: u8,
    /// 0=unsigned, 1=signed, 2=bool, 3=str_ref, 4=raw_hex
    kind: u8,
}

struct DwarfInfo {
    line_table: Vec<(u32, u16, u32)>,
    file_paths: Vec<String>,
    param_locs: Vec<ParamLoc>,
}

fn extract_dwarf_info(data: &[u8]) -> DwarfInfo {
    use addr2line::gimli;
    use object::{Object, ObjectSection};

    let empty = DwarfInfo {
        line_table: Vec::new(),
        file_paths: Vec::new(),
        param_locs: Vec::new(),
    };
    if data.len() < 64 {
        return empty;
    }

    let object = match object::File::parse(data) {
        Ok(o) => o,
        Err(_) => return empty,
    };

    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    let dwarf = match gimli::Dwarf::load(|id| -> Result<_, gimli::Error> {
        let sec = object
            .section_by_name(id.name())
            .and_then(|s| s.data().ok())
            .unwrap_or(&[]);
        Ok(gimli::EndianSlice::new(sec, endian))
    }) {
        Ok(d) => d,
        Err(_) => return empty,
    };

    let param_locs = extract_param_locs(&dwarf);

    let ctx = match addr2line::Context::from_dwarf(dwarf) {
        Ok(c) => c,
        Err(_) => {
            return DwarfInfo {
                param_locs,
                ..empty
            }
        }
    };

    // --- line table ---
    let text_end = object
        .section_by_name(".text")
        .map(|s| s.address() + s.size())
        .unwrap_or(TEXT_LIMIT)
        .min(TEXT_LIMIT);

    let mut line_entries: Vec<(u32, u16, u32)> = Vec::new();
    let mut file_paths: Vec<String> = Vec::new();
    let mut file_map = std::collections::HashMap::<String, u16>::new();
    let mut prev_file_idx: u16 = u16::MAX;
    let mut prev_line: u32 = u32::MAX;

    let mut addr = TEXT_BASE;
    while addr < text_end {
        if let Ok(Some(loc)) = ctx.find_location(addr) {
            let file = loc.file.unwrap_or("??");
            let line = loc.line.unwrap_or(0);
            if line != 0 {
                let display_path = simplify_path(file);
                let file_idx = *file_map.entry(display_path.clone()).or_insert_with(|| {
                    let idx = file_paths.len() as u16;
                    file_paths.push(display_path);
                    idx
                });
                if file_idx != prev_file_idx || line != prev_line {
                    let off = (addr - TEXT_BASE) as u32;
                    line_entries.push((off, file_idx, line));
                    prev_file_idx = file_idx;
                    prev_line = line;
                }
            }
        }
        addr += 2;
    }

    DwarfInfo {
        line_table: line_entries,
        file_paths,
        param_locs,
    }
}

// ===== DWARF parameter location + type extraction =====

type DwarfSlice<'a> = addr2line::gimli::EndianSlice<'a, addr2line::gimli::RunTimeEndian>;

fn extract_param_locs(dwarf: &addr2line::gimli::Dwarf<DwarfSlice<'_>>) -> Vec<ParamLoc> {
    use addr2line::gimli;

    let mut result = Vec::new();

    let mut units = dwarf.units();
    while let Ok(Some(header)) = units.next() {
        let unit = match dwarf.unit(header) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let mut entries = unit.entries();
        let mut current_func: Option<u64> = None;
        let mut depth: isize = 0;
        let mut func_depth: isize = -1;

        while let Ok(Some((delta, entry))) = entries.next_dfs() {
            depth += delta;

            if current_func.is_some() && depth <= func_depth {
                current_func = None;
                func_depth = -1;
            }

            match entry.tag() {
                gimli::DW_TAG_subprogram => {
                    let addr = entry
                        .attr_value(gimli::DW_AT_low_pc)
                        .ok()
                        .flatten()
                        .and_then(|v| match v {
                            gimli::AttributeValue::Addr(a) => Some(a),
                            _ => None,
                        });
                    if let Some(a) = addr {
                        if a >= TEXT_BASE && a < TEXT_LIMIT {
                            current_func = Some(a);
                            func_depth = depth;
                        }
                    }
                }
                gimli::DW_TAG_formal_parameter
                    if current_func.is_some() && depth == func_depth + 1 =>
                {
                    let func_addr = current_func.unwrap();

                    let name = entry
                        .attr(gimli::DW_AT_name)
                        .ok()
                        .flatten()
                        .and_then(|a| dwarf.attr_string(&unit, a.value()).ok())
                        .and_then(|s| s.to_string().ok().map(|x| x.to_string()))
                        .unwrap_or_default();
                    if name.is_empty() || name.starts_with("__") || name == "self" {
                        continue;
                    }

                    let fbreg_offset = entry
                        .attr_value(gimli::DW_AT_location)
                        .ok()
                        .flatten()
                        .and_then(|v| match v {
                            gimli::AttributeValue::Exprloc(expr) => {
                                parse_fbreg_offset(expr.0.slice())
                            }
                            _ => None,
                        });
                    let fbreg_offset = match fbreg_offset {
                        Some(off) if off >= i16::MIN as i64 && off <= i16::MAX as i64 => {
                            off as i16
                        }
                        _ => continue,
                    };

                    let (byte_size, kind) = entry
                        .attr_value(gimli::DW_AT_type)
                        .ok()
                        .flatten()
                        .and_then(|v| match v {
                            gimli::AttributeValue::UnitRef(offset) => {
                                resolve_type(&unit, dwarf, offset)
                            }
                            _ => None,
                        })
                        .unwrap_or((8, 4));

                    result.push(ParamLoc {
                        func_addr,
                        name,
                        fbreg_offset,
                        byte_size,
                        kind,
                    });
                }
                _ => {}
            }
        }
    }
    result.sort_by(|a, b| a.func_addr.cmp(&b.func_addr).then(a.fbreg_offset.cmp(&b.fbreg_offset)));
    result
}

/// Parse a `DW_OP_fbreg(N)` DWARF expression. Returns the SLEB128-decoded offset N.
fn parse_fbreg_offset(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() || bytes[0] != 0x91 {
        return None;
    }
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    for &byte in &bytes[1..] {
        result |= ((byte & 0x7f) as i64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < 64 && (byte & 0x40) != 0 {
                result |= !0i64 << shift;
            }
            return Some(result);
        }
    }
    None
}

/// Resolve a DWARF type DIE to (byte_size, kind).
fn resolve_type(
    unit: &addr2line::gimli::Unit<DwarfSlice<'_>>,
    dwarf: &addr2line::gimli::Dwarf<DwarfSlice<'_>>,
    offset: addr2line::gimli::UnitOffset<usize>,
) -> Option<(u8, u8)> {
    use addr2line::gimli;

    let mut cursor = match unit.entries_at_offset(offset) {
        Ok(c) => c,
        Err(_) => return None,
    };
    match cursor.next_entry() {
        Ok(Some(())) => {}
        _ => return None,
    }
    let entry = cursor.current()?;

    match entry.tag() {
        gimli::DW_TAG_base_type => {
            let byte_size = entry
                .attr_value(gimli::DW_AT_byte_size)
                .ok()
                .flatten()
                .and_then(|v: gimli::AttributeValue<DwarfSlice<'_>>| v.udata_value())
                .unwrap_or(8) as u8;
            let kind = entry
                .attr_value(gimli::DW_AT_encoding)
                .ok()
                .flatten()
                .and_then(|v: gimli::AttributeValue<DwarfSlice<'_>>| match v {
                    gimli::AttributeValue::Encoding(e) => Some(e),
                    _ => None,
                })
                .map(|e| match e {
                    gimli::DW_ATE_boolean => 2u8,
                    gimli::DW_ATE_signed | gimli::DW_ATE_signed_char => 1,
                    _ => 0,
                })
                .unwrap_or(0);
            Some((byte_size, kind))
        }
        gimli::DW_TAG_structure_type => {
            let name = entry
                .attr(gimli::DW_AT_name)
                .ok()
                .flatten()
                .and_then(|a: gimli::Attribute<DwarfSlice<'_>>| {
                    dwarf.attr_string(unit, a.value()).ok()
                })
                .and_then(|s: DwarfSlice<'_>| s.to_string().ok().map(|x| x.to_string()));
            if name.as_deref() == Some("&str") {
                Some((16, 3))
            } else {
                let byte_size = entry
                    .attr_value(gimli::DW_AT_byte_size)
                    .ok()
                    .flatten()
                    .and_then(|v: gimli::AttributeValue<DwarfSlice<'_>>| v.udata_value())
                    .unwrap_or(8) as u8;
                Some((byte_size, 4))
            }
        }
        _ => Some((8, 4)),
    }
}

/// Strip build-dir prefixes to produce short display paths like `src/main.rs`.
fn simplify_path(full: &str) -> String {
    // Try to find "src/" and take from there
    if let Some(pos) = full.find("/src/") {
        return full[pos + 1..].to_string();
    }
    if full.starts_with("src/") {
        return full.to_string();
    }
    // For library crates, show just the filename
    if let Some(pos) = full.rfind('/') {
        return full[pos + 1..].to_string();
    }
    full.to_string()
}

// ===== Code generation =====

fn write_generated_rs(
    syms: &[(u64, u64, String)],
    dwarf: &DwarfInfo,
    out: &std::path::Path,
) {
    use std::fmt::Write;

    let mut c = String::with_capacity(
        syms.len() * 100 + dwarf.line_table.len() * 30 + dwarf.file_paths.len() * 60 + 1024,
    );

    writeln!(c, "// Auto-generated by build.rs \u{2014} do not edit").unwrap();
    writeln!(
        c,
        "// {} syms, {} line entries, {} files",
        syms.len(),
        dwarf.line_table.len(),
        dwarf.file_paths.len()
    )
    .unwrap();

    // --- FUNC_SYMS ---
    writeln!(c, "const FUNC_SYMS: &[(u64, u64, &str)] = &[").unwrap();
    for (addr, size, name) in syms {
        let esc = name.replace('\\', "\\\\").replace('"', "\\\"");
        writeln!(c, "    (0x{addr:x}, {size}, \"{esc}\"),").unwrap();
    }
    writeln!(c, "];").unwrap();

    // --- LINE_TABLE ---
    writeln!(c, "const LINE_TABLE: &[(u32, u16, u32)] = &[").unwrap();
    for (off, fi, ln) in &dwarf.line_table {
        writeln!(c, "    ({off}, {fi}, {ln}),").unwrap();
    }
    writeln!(c, "];").unwrap();

    // --- LINE_FILES ---
    writeln!(c, "const LINE_FILES: &[&str] = &[").unwrap();
    for f in &dwarf.file_paths {
        let esc = f.replace('\\', "\\\\").replace('"', "\\\"");
        writeln!(c, "    \"{esc}\",").unwrap();
    }
    writeln!(c, "];").unwrap();

    // --- FUNC_PARAM_LOCS: (func_addr, name, fbreg_offset, byte_size, kind) ---
    // kind: 0=unsigned 1=signed 2=bool 3=str_ref 4=raw_hex
    writeln!(
        c,
        "const FUNC_PARAM_LOCS: &[(u64, &str, i16, u8, u8)] = &["
    )
    .unwrap();
    for p in &dwarf.param_locs {
        let esc = p.name.replace('\\', "\\\\").replace('"', "\\\"");
        writeln!(
            c,
            "    (0x{:x}, \"{esc}\", {}, {}, {}),",
            p.func_addr, p.fbreg_offset, p.byte_size, p.kind
        )
        .unwrap();
    }
    writeln!(c, "];").unwrap();

    let existing = std::fs::read_to_string(out).unwrap_or_default();
    if c != existing {
        std::fs::write(out, &c).unwrap();
    }
}

// ===== Linker script =====

const LINKER_SCRIPT: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_m_start)

M_BASE_ADDRESS = 0x80000000;
S_BASE_ADDRESS = 0x80200000;

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

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
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
