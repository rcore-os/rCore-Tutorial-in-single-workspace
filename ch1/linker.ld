
OUTPUT_ARCH(riscv)
ENTRY(_m_start)
M_BASE_ADDRESS = 0x80000000;
S_BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = M_BASE_ADDRESS;
    .text.m_entry : {
        *(.text.m_entry)
    }
    .text.m_trap : {
        *(.text.m_trap)
    }
    .bss.m_stack : {
        *(.bss.m_stack)
    }
    .bss.m_data : {
        *(.bss.m_data)
    }
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
}