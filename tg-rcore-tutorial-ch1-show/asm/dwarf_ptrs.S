/* Absolute symbol addresses via .dword (R_RISCV_64), avoiding PCREL_HI20 limits in Rust code. */
    .section .data.ch1_dwarf_ptrs,"aw",@progbits
    .align 3

    .globl ch1_dbg_abbrev
ch1_dbg_abbrev:
    .dword __start_debug_abbrev
    .dword __stop_debug_abbrev

    .globl ch1_dbg_addr
ch1_dbg_addr:
    .dword __start_debug_addr
    .dword __stop_debug_addr

    .globl ch1_dbg_aranges
ch1_dbg_aranges:
    .dword __start_debug_aranges
    .dword __stop_debug_aranges

    .globl ch1_dbg_info
ch1_dbg_info:
    .dword __start_debug_info
    .dword __stop_debug_info

    .globl ch1_dbg_line
ch1_dbg_line:
    .dword __start_debug_line
    .dword __stop_debug_line

    .globl ch1_dbg_line_str
ch1_dbg_line_str:
    .dword __start_debug_line_str
    .dword __stop_debug_line_str

    .globl ch1_dbg_ranges
ch1_dbg_ranges:
    .dword __start_debug_ranges
    .dword __stop_debug_ranges

    .globl ch1_dbg_rnglists
ch1_dbg_rnglists:
    .dword __start_debug_rnglists
    .dword __stop_debug_rnglists

    .globl ch1_dbg_str
ch1_dbg_str:
    .dword __start_debug_str
    .dword __stop_debug_str

    .globl ch1_dbg_str_offsets
ch1_dbg_str_offsets:
    .dword __start_debug_str_offsets
    .dword __stop_debug_str_offsets
