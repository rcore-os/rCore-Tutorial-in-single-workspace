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
        __start = .;
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : ALIGN(4K) {
        __rodata = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : ALIGN(4K) {
        __data = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : ALIGN(8) {
        __sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        __ebss = .;
    }
    .boot : ALIGN(4K) {
        __boot = .;
        KEEP(*(.boot.stack))
    }
    __end = .;
}