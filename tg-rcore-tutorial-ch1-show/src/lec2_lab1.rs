//! 第二讲（lec2）知识点在 Lab1 中的**可观测**锚点。
//!
//! 与 `discussions/lec-vs-lab/01-lec2-ch1-ir.md` 中 LabUnit 的
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

/// 读取当前栈指针（观测「LibOS 自建栈」后可用的 sp）。
fn read_sp() -> usize {
    let sp: usize;
    // SAFETY: 仅读取 sp，无副作用。
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp);
    }
    sp
}

/// 故意 `extern "C"` 且 `inline(never)`，便于用 objdump 看到参数经 a0/a1 传递。
#[inline(never)]
extern "C" fn lec2_demo_add(a: usize, b: usize) -> usize {
    a.wrapping_add(b)
}

/// 在 `Hello, world!` 之后打印一组稳定、可 grep 的标签行（第二讲 × Lab1 对齐）。
pub fn emit_all_observables() {
    // --- kp=curriculum：实验整体递进（与 lec2 p1-labintro 对齐）---
    put_line(
        b"[LEC2-LAB1] kp=curriculum note=ch1_libos_minimal_then_ch2_batch_ch3_multitask_ch4_vm...",
    );

    // --- kp=compile_abi：交叉编译目标三元组 + 编译器/链接器共识（lec2 p2-compiling）---
    put_line(
        concat!(
            "[LEC2-LAB1] kp=compile_abi target=",
            env!("LAB_TARGET_TRIPLE"),
            " panic=abort no_std",
        )
        .as_bytes(),
    );

    // --- kp=boot_chain：QEMU virt + nobios 契约（lec2 p3-boot；IR: platform.boot_model）---
    put_line(
        b"[LEC2-LAB1] kp=boot_chain model=nobios qemu=-machine virt -nographic -bios none -kernel <ELF>",
    );

    // --- kp=mem_layout：linker.ld / ELF 段布局锚点（build.rs 注入常量）---
    put_line(
        concat!(
            "[LEC2-LAB1] kp=mem_layout M_BASE=",
            env!("LAB_M_BASE"),
            " S_BASE=",
            env!("LAB_S_BASE"),
            " linker=OUT_DIR/linker.ld",
        )
        .as_bytes(),
    );

    // --- kp=libos_stack：进入 rust_main 后的 sp（对照 INV-CH1-STACK）---
    for c in b"[LEC2-LAB1] kp=libos_stack sp=" {
        console_putchar(*c);
    }
    put_hex_u64(read_sp() as u64);
    console_putchar(b'\n');

    // --- kp=callconv：函数调用与参数槽位（对照 a0~a7；与 syscall 同属控制流切换但语义不同）---
    let sum = lec2_demo_add(0x10, 0x20);
    put_line(
        b"[LEC2-LAB1] kp=callconv demo=extern_C_add(0x10,0x20) hint=objdump_-d_see_a0_a1",
    );
    for c in b"[LEC2-LAB1] kp=callconv result=" {
        console_putchar(*c);
    }
    put_hex_u64(sum as u64);
    console_putchar(b'\n');

    // --- kp=sbi_vs_syscall：S 态 ecall→M 态 SBI 服务（预告 U→S syscall）---
    put_line(
        b"[LEC2-LAB1] kp=sbi_vs_syscall note=S_mode_ecall_to_M_for_console_shutdown_U_will_ecall_to_S",
    );

    // --- kp=control_flow：_start → rust_main → shutdown（intent.primary_mechanism）---
    put_line(b"[LEC2-LAB1] kp=control_flow path=_start->rust_main->shutdown(false)");

    // --- kp=panic_contract：异常路径收口（对照 panic_handler / INV-CH1-IO-CLOSE）---
    put_line(
        b"[LEC2-LAB1] kp=panic_contract note=panic_path_calls_shutdown(true)_try_rust_panic_macro",
    );
}
