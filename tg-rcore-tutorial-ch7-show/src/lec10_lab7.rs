//! 第十讲（lec10）与 Lab7（管道 + 信号）对齐的**可观测**锚点。
//!
//! 每行以 `[LEC10-LAB7]` 前缀输出，便于 `grep` / 脚本验收，将课堂概念
//!（管道、环形缓冲、统一 fd、信号位图、屏蔽字、syscall 返回前处理信号等）
//! 映射到内核实际执行路径。

#![cfg(target_arch = "riscv64")]

use tg_sbi::console_putchar;

fn put_bytes(s: &[u8]) {
    for c in s {
        console_putchar(*c);
    }
}

fn put_line(s: &[u8]) {
    put_bytes(s);
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

/// 内核 satp 建立后输出（与 ch4-show 的 lec5 形式一致，便于对比地址空间）。
pub fn emit_kernel_space_created(root_ppn: usize, satp: usize) {
    put_bytes(b"[LEC10-LAB7] kp=kernel_space_created root_ppn=");
    put_hex_u64(root_ppn as u64);
    put_bytes(b" satp=");
    put_hex_u64(satp as u64);
    put_bytes(b" mode=Sv39\n");
}

/// 初始化完成后的静态知识点（管道 + 信号 + 与本章相关的 syscall 号）。
pub fn emit_init_observables(memory: usize) {
    put_line(
        b"[LEC10-LAB7] kp=pipe_ipc unidirectional read_fd=pipe[0] write_fd=pipe[1] ring_buffer=easy_fs",
    );
    put_line(
        b"[LEC10-LAB7] kp=unified_fd Fd=File|PipeRead|PipeWrite|Empty fd_table_shared_with_files_stdio",
    );
    put_line(
        b"[LEC10-LAB7] kp=signal_async received_bitset mask sigactions handling sigreturn_restores_context",
    );
    put_line(
        b"[LEC10-LAB7] kp=signal_timing check_on_syscall_return_before_sret_to_user handle_signals_in_trap_loop",
    );
    put_line(
        b"[LEC10-LAB7] kp=fork_inheritance child_copies_fd_table_and_signal_state via_Process::fork",
    );
    put_bytes(b"[LEC10-LAB7] kp=kernel_heap memory=");
    put_decimal(memory);
    console_putchar(b'\n');
    put_line(
        b"[LEC10-LAB7] kp=syscalls pipe2=59 read=63 write=64 kill=129 rt_sigaction=134 rt_sigprocmask=135 rt_sigreturn=139 fork=220",
    );
    put_line(
        concat!(
            "[LEC10-LAB7] kp=compile_info target=",
            env!("LAB_TARGET_TRIPLE"),
            " M_BASE=",
            env!("LAB_M_BASE"),
            " S_BASE=",
            env!("LAB_S_BASE"),
        )
        .as_bytes(),
    );
    put_line(
        b"[LEC10-LAB7] kp=control_flow _start->rust_main->kernel_space->init_signal->initproc->trap_loop{ecall->handle->handle_signals}",
    );
}

/// 用户态 ecall 进入内核、尚未 `move_next` 时的 PC。
pub fn emit_syscall_trap(proc_id: usize, syscall_id: usize, a0: usize, sepc: usize) {
    put_bytes(b"[LEC10-LAB7] kp=syscall_trap proc_id=");
    put_decimal(proc_id);
    put_bytes(b" syscall_id=");
    put_decimal(syscall_id);
    put_bytes(b" a0=");
    put_hex_u64(a0 as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// `pipe` 系统调用在内核中成功分配读写 fd 后。
pub fn emit_pipe_created(proc_id: usize, read_fd: usize, write_fd: usize) {
    put_bytes(b"[LEC10-LAB7] kp=pipe_created proc_id=");
    put_decimal(proc_id);
    put_bytes(b" read_fd=");
    put_decimal(read_fd);
    put_bytes(b" write_fd=");
    put_decimal(write_fd);
    put_bytes(b" via=make_pipe+fd_table_push\n");
}

/// `kill` 投递信号成功（目标 PCB 收到位图标记）。
pub fn emit_kill_enqueued(src_proc: usize, dst_pid: usize, signum: u8) {
    put_bytes(b"[LEC10-LAB7] kp=kill_enqueued from_proc=");
    put_decimal(src_proc);
    put_bytes(b" dst_pid=");
    put_decimal(dst_pid);
    put_bytes(b" signum=");
    put_decimal(signum as usize);
    put_bytes(b"\n");
}

/// 在 syscall 返回路径上即将调用 `handle_signals`。
pub fn emit_before_handle_signals(proc_id: usize) {
    put_bytes(b"[LEC10-LAB7] kp=before_handle_signals proc_id=");
    put_decimal(proc_id);
    put_bytes(b" point=after_syscall_before_user_return\n");
}

/// 信号处理导致进程被同步终止（如 SIGKILL）。
pub fn emit_signal_terminates(proc_id: usize, exit_code: i32) {
    put_bytes(b"[LEC10-LAB7] kp=signal_terminates proc_id=");
    put_decimal(proc_id);
    put_bytes(b" exit_code=");
    if exit_code < 0 {
        console_putchar(b'-');
        put_decimal((-exit_code) as usize);
    } else {
        put_decimal(exit_code as usize);
    }
    put_bytes(b" via=SignalResult::ProcessKilled\n");
}

/// 未因信号退出，继续写回 syscall 返回值。
pub fn emit_signal_check_done(proc_id: usize) {
    put_bytes(b"[LEC10-LAB7] kp=signal_check_done proc_id=");
    put_decimal(proc_id);
    put_bytes(b" continue=apply_syscall_ret\n");
}

/// 非法 trap 导致当前进程被内核结束。
pub fn emit_exception_kill(proc_id: usize, cause_str: &str, stval: usize, sepc: usize) {
    put_bytes(b"[LEC10-LAB7] kp=exception_kill proc_id=");
    put_decimal(proc_id);
    put_bytes(b" cause=");
    put_bytes(cause_str.as_bytes());
    put_bytes(b" stval=");
    put_hex_u64(stval as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}
