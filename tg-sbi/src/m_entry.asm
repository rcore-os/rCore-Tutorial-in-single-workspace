# `-bios none` 场景下的 M 态入口代码
# 当 QEMU 使用 `-bios none` 启动时，该代码在 M 态最先执行（常见入口 0x80000000）

    .section .text.m_entry
    .globl _m_start
_m_start:
    # 1) 初始化 M 态栈
    la sp, m_stack_top
    # 将 M 态栈顶保存到 mscratch，后续陷阱处理时用于切换栈
    csrw mscratch, sp

    # 2) 配置 mstatus：MPP=01（返回到 S 态），MPIE=1
    li t0, (1 << 11) | (1 << 7)
    csrw mstatus, t0

    # 3) 设置 mepc 为 S 态入口（由章节内核提供的 _start）
    la t0, _start
    csrw mepc, t0

    # 4) 设置 M 态陷阱向量
    la t0, m_trap_vector
    csrw mtvec, t0

    # 5) 中断/异常委托给 S 态（但“来自 S 态的 ecall”不委托）
    #    这样 S 态内核执行 SBI 调用时，仍会陷入 M 态由本文件处理
    li t0, 0xffff
    csrw mideleg, t0
    li t0, 0xffff
    li t1, (1 << 9)     # 异常号 9：Environment call from S-mode
    not t1, t1
    and t0, t0, t1
    csrw medeleg, t0

    # 6) 配置 PMP：允许 S 态访问全部物理地址空间（教学简化）
    li t0, -1
    csrw pmpaddr0, t0
    li t0, 0x0f         # TOR 模式 + RWX
    csrw pmpcfg0, t0

    # 7) 允许 S 态读取计数器（如 time）
    li t0, -1
    csrw mcounteren, t0

    # 8) mret 切到 S 态，开始执行章节内核入口
    mret

    .section .text.m_trap
    .globl m_trap_vector
    .align 4
m_trap_vector:
    # 最小 M 态陷阱入口：主要处理来自 S 态的 ecall（SBI 调用）
    # 先切换到 M 态专用栈，避免污染 S 态栈
    csrrw sp, mscratch, sp
    addi sp, sp, -128

    # 保存会被 Rust 处理函数使用/破坏的通用寄存器
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

    # 调用 Rust 侧分发函数（msbi.rs::m_trap_handler）
    call m_trap_handler

    # 跳过触发陷阱的 ecall 指令，避免返回后再次陷入
    csrr t0, mepc
    addi t0, t0, 4
    csrw mepc, t0

    # 恢复寄存器
    ld ra, 0(sp)
    ld t0, 8(sp)
    ld t1, 16(sp)
    ld t2, 24(sp)
    # 不恢复 a0/a1：它们保存 m_trap_handler 的返回值（SbiRet）
    ld a2, 48(sp)
    ld a3, 56(sp)
    ld a4, 64(sp)
    ld a5, 72(sp)
    ld a6, 80(sp)
    ld a7, 88(sp)

    addi sp, sp, 128
    # 切回原先（S 态侧）栈指针
    csrrw sp, mscratch, sp
    # 返回到触发 ecall 的 S 态上下文
    mret

    .section .bss.m_stack
    .globl m_stack_lower_bound
m_stack_lower_bound:
    # M 态专用栈（16 KiB）
    .space 4096 * 4
    .globl m_stack_top
m_stack_top:

    .section .bss.m_data
    # 预留少量 M 态数据区（当前实现未显式使用，便于后续扩展）
    .space 64
