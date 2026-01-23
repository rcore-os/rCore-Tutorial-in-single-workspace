# M-Mode entry point for -bios none boot
# This code runs at 0x80000000 in M-Mode when QEMU starts with -bios none

    .section .text.m_entry
    .globl _m_start
_m_start:
    # Set up M-Mode stack
    la sp, m_stack_top
    # Save M-Mode sp to mscratch for trap handler
    csrw mscratch, sp

    # Set mstatus: MPP=01 (S-Mode), MPIE=1
    li t0, (1 << 11) | (1 << 7)
    csrw mstatus, t0

    # Set mepc to S-Mode entry point
    la t0, _start
    csrw mepc, t0

    # Set mtvec to M-Mode trap handler
    la t0, m_trap_vector
    csrw mtvec, t0

    # Delegate interrupts and exceptions to S-Mode (except ecall from S-Mode)
    li t0, 0xffff
    csrw mideleg, t0
    li t0, 0xffff
    li t1, (1 << 9)     # Environment call from S-mode
    not t1, t1
    and t0, t0, t1
    csrw medeleg, t0

    # Set up PMP to allow S-Mode full access
    li t0, -1
    csrw pmpaddr0, t0
    li t0, 0x0f         # TOR, RWX
    csrw pmpcfg0, t0

    # Enable S-Mode to access counters
    li t0, -1
    csrw mcounteren, t0

    # Jump to S-Mode
    mret

    .section .text.m_trap
    .globl m_trap_vector
    .align 4
m_trap_vector:
    # Simple trap handler: handle ecall from S-Mode
    csrrw sp, mscratch, sp
    addi sp, sp, -128

    # Save registers
    sd ra, 0(sp)
    sd t0, 8(sp)
    sd t1, 16(sp)
    sd t2, 24(sp)
    sd a0, 32(sp)
    sd a1, 40(sp)
    sd a2, 48(sp)
    sd a3, 56(sp)
    sd a4, 64(sp)
    sd a5, 72(sp)
    sd a6, 80(sp)
    sd a7, 88(sp)

    # Call Rust trap handler
    call m_trap_handler

    # Advance mepc past ecall instruction
    csrr t0, mepc
    addi t0, t0, 4
    csrw mepc, t0

    # Restore registers
    ld ra, 0(sp)
    ld t0, 8(sp)
    ld t1, 16(sp)
    ld t2, 24(sp)
    # Skip a0, a1 - they contain the return value
    ld a2, 48(sp)
    ld a3, 56(sp)
    ld a4, 64(sp)
    ld a5, 72(sp)
    ld a6, 80(sp)
    ld a7, 88(sp)

    addi sp, sp, 128
    csrrw sp, mscratch, sp
    mret

    .section .bss.m_stack
    .globl m_stack_lower_bound
m_stack_lower_bound:
    .space 4096 * 4
    .globl m_stack_top
m_stack_top:

    .section .bss.m_data
    .space 64
