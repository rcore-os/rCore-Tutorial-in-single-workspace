//! 第三讲（lec3）知识点在 Lab2 中的**可观测**锚点。
//!
//! 与 `discussions/lec-vs-lab/02-kp-lec3-ch2.md` 中 LabUnit 的
//! `observables` / `contract.postconditions` 对齐：每条串口行均可被脚本 grep，
//! 便于把课堂概念落到「命令 + 子串」证据链。

use tg_sbi::console_putchar;

fn put_line(s: &[u8]) {
    for c in s {
        console_putchar(*c);
    }
    console_putchar(b'\n');
}

fn put_hex_u64(x: u64) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for c in b"0x" {
        console_putchar(*c);
    }
    for shift in (0..16).rev() {
        let nibble = ((x >> (shift * 4)) & 0xf) as usize;
        console_putchar(DIGITS[nibble]);
    }
}

fn put_bytes(s: &[u8]) {
    for c in s {
        console_putchar(*c);
    }
}

fn put_decimal(mut n: usize) {
    if n == 0 {
        console_putchar(b'0');
        return;
    }
    let mut buf = [0u8; 20];
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

fn read_sp() -> usize {
    let sp: usize;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp);
    }
    sp
}

fn read_csr_stvec() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, stvec", out(reg) val);
    }
    val
}

fn read_csr_sstatus() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, sstatus", out(reg) val);
    }
    val
}

/// 批处理循环前调用：展示与 ch2 架构相关的静态知识点。
pub fn emit_pre_batch_observables() {
    // --- kp=isolation：OS 隔离机制概述（lec3 p1-osviewarch）---
    put_line(
        b"[LEC3-LAB2] kp=isolation note=U_mode_app_cannot_access_S_mode_resources_data_control_isolation",
    );

    // --- kp=privilege_u_s：U/S 特权级与 sstatus.SPP（lec3 p2-osviewrv）---
    put_bytes(b"[LEC3-LAB2] kp=privilege_u_s note=U_mode_for_app_S_mode_for_kernel sstatus=");
    put_hex_u64(read_csr_sstatus() as u64);
    put_bytes(b" SPP_bit8=controls_sret_target_privilege\n");

    // --- kp=trap_mechanism：trap 入口与 CSR（lec3 p2-osviewrv）---
    put_bytes(b"[LEC3-LAB2] kp=trap_mechanism stvec=");
    put_hex_u64(read_csr_stvec() as u64);
    put_bytes(b" note=stvec_set_by_execute_naked_before_sret\n");

    // --- kp=context_save_restore：上下文保存/恢复机制（lec3 p3-batchos）---
    put_line(
        b"[LEC3-LAB2] kp=context_save_restore note=LocalContext_holds_x1_x31_sepc_sscratch_swaps_sp_on_trap",
    );

    // --- kp=syscall_abi：系统调用寄存器约定（lec3 p3-batchos）---
    put_line(
        b"[LEC3-LAB2] kp=syscall_abi note=a7_syscall_id_a0_a5_args_a0_retval_ecall_from_U_to_S",
    );

    // --- kp=sepc_invariant：INV-SYSCALL-SEPC（sepc += 4）---
    put_line(
        b"[LEC3-LAB2] kp=sepc_invariant note=after_ecall_handler_must_sepc_plus_4_to_skip_ecall_instruction",
    );

    // --- kp=batch_execution：批处理顺序与 INV-BATCH-ORDER（lec3 p3-batchos）---
    put_line(
        b"[LEC3-LAB2] kp=batch_execution note=apps_loaded_and_run_sequentially_by_AppMeta_iter",
    );

    // --- kp=user_stack：用户栈分配（lec3 p3-batchos）---
    put_bytes(b"[LEC3-LAB2] kp=user_stack note=4KiB_per_app_via_MaybeUninit kernel_sp=");
    put_hex_u64(read_sp() as u64);
    console_putchar(b'\n');

    // --- kp=fence_i：指令缓存一致性（同址重载用户程序）---
    put_line(
        b"[LEC3-LAB2] kp=fence_i note=fence_i_after_app_exit_before_next_app_load_to_flush_icache",
    );

    // --- kp=compile_abi：交叉编译目标---
    put_line(
        concat!(
            "[LEC3-LAB2] kp=compile_abi target=",
            env!("LAB_TARGET_TRIPLE"),
            " panic=abort no_std",
        )
        .as_bytes(),
    );

    // --- kp=mem_layout：内存布局---
    put_line(
        concat!(
            "[LEC3-LAB2] kp=mem_layout M_BASE=",
            env!("LAB_M_BASE"),
            " S_BASE=",
            env!("LAB_S_BASE"),
            " APP_BASE=0x80400000",
        )
        .as_bytes(),
    );
}

/// 在加载每个 app 时调用，展示运行时加载信息。
pub fn emit_app_load_info(app_idx: usize, app_base: usize) {
    put_bytes(b"[LEC3-LAB2] kp=batch_execution app_idx=");
    put_decimal(app_idx);
    put_bytes(b" app_base=");
    put_hex_u64(app_base as u64);
    put_bytes(b" note=copy_to_base_then_LocalContext_user\n");
}

/// 第一次 syscall 时调用，展示系统调用 ABI 的实际值。
pub fn emit_syscall_info(syscall_id: usize, a0: usize, a1: usize, a2: usize) {
    put_bytes(b"[LEC3-LAB2] kp=syscall_abi_demo a7=");
    put_decimal(syscall_id);
    put_bytes(b" a0=");
    put_hex_u64(a0 as u64);
    put_bytes(b" a1=");
    put_hex_u64(a1 as u64);
    put_bytes(b" a2=");
    put_hex_u64(a2 as u64);
    console_putchar(b'\n');
}

/// 用户 trap 发生时调用，展示 trap 相关 CSR。
pub fn emit_trap_info(app_idx: usize, scause: usize, stval: usize, sepc: usize) {
    put_bytes(b"[LEC3-LAB2] kp=trap_dispatch app_idx=");
    put_decimal(app_idx);
    put_bytes(b" scause=");
    put_hex_u64(scause as u64);
    put_bytes(b" stval=");
    put_hex_u64(stval as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// 在 handle_syscall 中，展示 sepc += 4 的不变量执行。
pub fn emit_sepc_advance(sepc_before: usize, sepc_after: usize) {
    put_bytes(b"[LEC3-LAB2] kp=sepc_invariant_demo sepc_before=");
    put_hex_u64(sepc_before as u64);
    put_bytes(b" sepc_after=");
    put_hex_u64(sepc_after as u64);
    put_bytes(b" delta=4\n");
}
