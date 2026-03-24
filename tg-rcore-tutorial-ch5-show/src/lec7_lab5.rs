//! 第七讲（lec7）与 Lab5（进程管理）对齐的**可观测**知识点输出。
//!
//! 每行以 `[LEC7-LAB5]` 前缀开头，便于 grep / 脚本验收，将课堂概念（PCB、fork/exec/wait、
//! 进程树、就绪队列与调度、`PManager`/`ProcManager`、僵尸进程与回收、`sched_yield`、
//! 跨地址空间执行与 `satp` 等）映射到串口证据链。

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

fn read_csr_stvec() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, stvec", out(reg) val);
    }
    val
}

fn read_csr_satp() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, satp", out(reg) val);
    }
    val
}

/// 内核完成 Sv39 与堆初始化后的静态知识点（含进程抽象总览）。
pub fn emit_init_observables(app_count: usize, memory: usize) {
    put_line(
        b"[LEC7-LAB5] kp=process_abstraction PCB=pid+ForeignContext+AddressSpace+heap_brk",
    );
    put_line(
        b"[LEC7-LAB5] kp=process_lifecycle created->ready->running->blocked/zombie->reaped",
    );
    put_line(
        b"[LEC7-LAB5] kp=fork_exec_wait clone=fork(copy_addr_space) execve=exec(replace_elf_image) wait4=wait(reap_child)",
    );
    put_line(
        b"[LEC7-LAB5] kp=process_tree parent_child_links initproc_parent=MAX orphan_reparent_to_pid0",
    );
    put_line(
        b"[LEC7-LAB5] kp=scheduler_design Manage_trait=tasks_map Schedule_trait=ready_queue FIFO_RR_default stride_exercise",
    );
    put_line(
        b"[LEC7-LAB5] kp=pmanager_flow find_next=fetch+set_current trap_return=syscall_or_exception state_update",
    );
    put_line(
        b"[LEC7-LAB5] kp=suspend_vs_exit yield_or_io_done=suspend_exit=exited_then_wait_reclaim",
    );
    put_line(
        b"[LEC7-LAB5] kp=cross_space_execute MultislotPortal ForeignContext_execute satp_switch",
    );
    put_line(
        b"[LEC7-LAB5] kp=syscall_path UserEnvCall ecall sepc+=4 tg_syscall::handle translate_user_ptr",
    );
    put_bytes(b"[LEC7-LAB5] kp=kernel_heap memory_bytes=");
    put_decimal(memory);
    console_putchar(b'\n');
    put_bytes(b"[LEC7-LAB5] kp=embedded_apps count=");
    put_decimal(app_count);
    console_putchar(b'\n');
    put_bytes(b"[LEC7-LAB5] kp=stvec_trap_vector stvec=");
    put_hex_u64(read_csr_stvec() as u64);
    console_putchar(b'\n');
    put_bytes(b"[LEC7-LAB5] kp=kernel_satp_after_space satp=");
    put_hex_u64(read_csr_satp() as u64);
    console_putchar(b'\n');
    put_line(concat!(
        "[LEC7-LAB5] kp=compile_info target=",
        env!("LAB_TARGET_TRIPLE"),
        " M_BASE=",
        env!("LAB_M_BASE"),
        " S_BASE=",
        env!("LAB_S_BASE"),
    )
    .as_bytes());
    put_line(
        b"[LEC7-LAB5] kp=control_flow _start->rust_main->kernel_space->initproc->loop_find_next->execute->trap->handle",
    );
}

/// 内核地址空间就绪时输出。
pub fn emit_kernel_space_created(root_ppn: usize, satp: usize) {
    put_bytes(b"[LEC7-LAB5] kp=kernel_space_created root_ppn=");
    put_hex_u64(root_ppn as u64);
    put_bytes(b" satp=");
    put_hex_u64(satp as u64);
    put_bytes(b" mode=Sv39\n");
}

/// 首个用户进程 initproc 进入调度器时输出。
pub fn emit_initproc_scheduled(pid: usize, entry: usize, satp: usize) {
    put_bytes(b"[LEC7-LAB5] kp=initproc_scheduled pid=");
    put_decimal(pid);
    put_bytes(b" entry=");
    put_hex_u64(entry as u64);
    put_bytes(b" satp=");
    put_hex_u64(satp as u64);
    console_putchar(b'\n');
}

/// 调度器选中下一个就绪进程。
pub fn emit_schedule_pick(pid: usize, pc: usize, satp: usize) {
    put_bytes(b"[LEC7-LAB5] kp=schedule_pick pid=");
    put_decimal(pid);
    put_bytes(b" user_pc=");
    put_hex_u64(pc as u64);
    put_bytes(b" satp=");
    put_hex_u64(satp as u64);
    console_putchar(b'\n');
}

/// 通过传送门进入用户态前。
pub fn emit_portal_enter_user(pid: usize, sepc: usize, user_satp: usize) {
    put_bytes(b"[LEC7-LAB5] kp=portal_enter_user pid=");
    put_decimal(pid);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    put_bytes(b" user_satp=");
    put_hex_u64(user_satp as u64);
    put_bytes(b" sret_to_U\n");
}

/// 用户态 ecall 进入内核（系统调用陷入）。
pub fn emit_syscall_trap(pid: usize, syscall_id: usize, a0: usize, sepc: usize) {
    put_bytes(b"[LEC7-LAB5] kp=syscall_trap pid=");
    put_decimal(pid);
    put_bytes(b" syscall_id=");
    put_decimal(syscall_id);
    put_bytes(b" a0=");
    put_hex_u64(a0 as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// fork 在父进程侧返回后（`ret` 为子进程 PID）。
pub fn emit_fork_return_parent(parent_pid: usize, child_pid: isize) {
    put_bytes(b"[LEC7-LAB5] kp=fork_parent_returns child_pid=");
    if child_pid < 0 {
        put_bytes(b"<err>");
    } else {
        put_decimal(child_pid as usize);
    }
    put_bytes(b" parent_pid=");
    put_decimal(parent_pid);
    console_putchar(b'\n');
}

/// exec 成功替换当前进程映像（PID 不变）。
pub fn emit_exec_success(pid: usize) {
    put_bytes(b"[LEC7-LAB5] kp=exec_success same_pid=");
    put_decimal(pid);
    put_bytes(b" new_elf_loaded\n");
}

/// wait 回收子进程成功。
pub fn emit_wait_reaped(parent_pid: usize, child_pid: usize, exit_code: isize) {
    put_bytes(b"[LEC7-LAB5] kp=wait_reaped parent=");
    put_decimal(parent_pid);
    put_bytes(b" child=");
    put_decimal(child_pid);
    put_bytes(b" exit_code=");
    if exit_code < 0 {
        console_putchar(b'-');
        put_decimal((-exit_code) as usize);
    } else {
        put_decimal(exit_code as usize);
    }
    console_putchar(b'\n');
}

/// `sched_yield` 或阻塞类 syscall 后挂起当前任务。
pub fn emit_task_suspended(pid: usize) {
    put_bytes(b"[LEC7-LAB5] kp=task_suspended requeued pid=");
    put_decimal(pid);
    console_putchar(b'\n');
}

/// 当前进程 exit。
pub fn emit_process_exit(pid: usize, exit_code: usize) {
    put_bytes(b"[LEC7-LAB5] kp=process_exit pid=");
    put_decimal(pid);
    put_bytes(b" exit_code=");
    put_decimal(exit_code);
    console_putchar(b'\n');
}

/// 逻辑上的切换提示（单核协作式下多为 satp/上下文切换）。
pub fn emit_context_switch_hint(from_pid: usize, to_pid: usize, new_satp: usize) {
    put_bytes(b"[LEC7-LAB5] kp=context_switch_hint from=");
    put_decimal(from_pid);
    put_bytes(b" to=");
    put_decimal(to_pid);
    put_bytes(b" new_satp=");
    put_hex_u64(new_satp as u64);
    console_putchar(b'\n');
}

/// 非法 trap 导致终止当前任务。
pub fn emit_exception_kill(pid: usize, cause_str: &str, stval: usize, sepc: usize) {
    put_bytes(b"[LEC7-LAB5] kp=exception_kill pid=");
    put_decimal(pid);
    put_bytes(b" cause=");
    put_bytes(cause_str.as_bytes());
    put_bytes(b" stval=");
    put_hex_u64(stval as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    put_bytes(b" will_print_backtrace\n");
}
