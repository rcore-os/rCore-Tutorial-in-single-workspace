//! 第十一讲、第十二讲与第八章（线程与同步原语）对齐的**可观测**知识点输出。
//!
//! 串口行前缀 `[LEC11-CH8]` 对应 **线程模型 / 双层调度 / 与进程资源的关系**；
//! 前缀 `[LEC12-CH8]` 对应 **互斥锁、信号量、条件变量、阻塞与唤醒**。
//!
//! 便于实验报告 grep、与 GDB 断点对照，并与用户态 `initproc` 中的同步系统调用形成证据链。

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

fn read_csr_satp() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, satp", out(reg) val);
    }
    val
}

/// 内核完成堆、地址空间、传送门后的一次性静态知识点（lec11 + lec12）。
pub fn emit_init_observables(memory: usize) {
    // --- Lec11：线程与执行模型 ---
    put_line(
        b"[LEC11-CH8] kp=thread_vs_process Process=resource_container Thread=execution_unit shared_address_space",
    );
    put_line(
        b"[LEC11-CH8] kp=pthread_manager PThreadManager ProcManager+ThreadManager FIFO_ready_queue",
    );
    put_line(
        b"[LEC11-CH8] kp=per_thread_context ForeignContext satp+LocalContext independent_user_stack",
    );
    put_line(
        b"[LEC11-CH8] kp=syscalls_thread thread_create gettid waittid tg_syscall::Thread",
    );
    put_line(
        b"[LEC11-CH8] kp=scheduling_granularity schedule_by_ThreadId not_ProcId rust_main_loop_find_next",
    );

    // --- Lec12：同步原语 ---
    put_line(
        b"[LEC12-CH8] kp=mutex critical_section MutexBlocking lock_unlock owner_queue",
    );
    put_line(
        b"[LEC12-CH8] kp=semaphore counting_PV Semaphore down_up resource_count wait_queue",
    );
    put_line(
        b"[LEC12-CH8] kp=condvar wait_signal Condvar wait_with_mutex release_lock_while_waiting",
    );
    put_line(
        b"[LEC12-CH8] kp=blocking_syscall ret_minus_one make_current_blocked re_enque_on_wake",
    );
    put_line(
        b"[LEC12-CH8] kp=sync_stored_in_process semaphore_list mutex_list condvar_list shared_by_all_threads",
    );

    put_bytes(b"[LEC11-CH8] kp=kernel_heap memory=");
    put_decimal(memory);
    console_putchar(b'\n');

    put_bytes(b"[LEC11-CH8] kp=kernel_satp satp=");
    put_hex_u64(read_csr_satp() as u64);
    console_putchar(b'\n');

    put_line(concat!(
        "[LEC11-CH8] kp=compile_info target=",
        env!("LAB_TARGET_TRIPLE"),
        " M_BASE=",
        env!("LAB_M_BASE"),
        " S_BASE=",
        env!("LAB_S_BASE"),
    )
    .as_bytes());

    put_line(
        b"[LEC11-CH8] kp=control_flow _start->rust_main->kernel_space->initproc->loop{find_next->execute->trap->syscall/sync}",
    );
}

/// initproc 加载后：首个进程与主线程就绪。
pub fn emit_initproc_main_thread(pid: usize, tid: usize) {
    put_bytes(b"[LEC11-CH8] kp=initproc_loaded pid=");
    put_decimal(pid);
    put_bytes(b" main_tid=");
    put_decimal(tid);
    put_bytes(b" ready_to_run\n");
}

/// 调度器从就绪队列取出下一线程、即将 `execute`。
pub fn emit_scheduler_pick(tid: usize) {
    put_bytes(b"[LEC11-CH8] kp=scheduler_pick next_tid=");
    put_decimal(tid);
    put_bytes(b" path=ThreadManager::fetch->ForeignContext::execute\n");
}

/// 用户态 ecall 进入内核分发点（在 `handle` 之前或之后均可；此处记录原始寄存器意图）。
pub fn emit_user_ecall(tid: usize, syscall_a7: usize, a0: usize, a1: usize) {
    put_bytes(b"[LEC11-CH8] kp=user_ecall tid=");
    put_decimal(tid);
    put_bytes(b" a7_syscall_id=");
    put_decimal(syscall_a7);
    put_bytes(b" a0=");
    put_hex_u64(a0 as u64);
    put_bytes(b" a1=");
    put_hex_u64(a1 as u64);
    console_putchar(b'\n');
}

/// 系统调用处理完成后的返回值（含阻塞语义提示）。
pub fn emit_syscall_result(tid: usize, syscall_a7: usize, ret: isize) {
    put_bytes(b"[LEC12-CH8] kp=syscall_result tid=");
    put_decimal(tid);
    put_bytes(b" a7=");
    put_decimal(syscall_a7);
    put_bytes(b" ret=");
    if ret < 0 {
        console_putchar(b'-');
        put_decimal((-ret) as usize);
    } else {
        put_decimal(ret as usize);
    }
    console_putchar(b'\n');
}

/// 同步类调用返回 -1：内核将线程标为阻塞态。
pub fn emit_thread_blocked(tid: usize, syscall_a7: usize) {
    put_bytes(b"[LEC12-CH8] kp=thread_blocked tid=");
    put_decimal(tid);
    put_bytes(b" syscall_a7=");
    put_decimal(syscall_a7);
    put_bytes(b" action=make_current_blocked removed_from_ready_queue\n");
}

/// 时间片或自愿让出后的挂起（仍在就绪语义上由 suspend 处理）。
pub fn emit_thread_suspend(tid: usize) {
    put_bytes(b"[LEC11-CH8] kp=thread_suspend tid=");
    put_decimal(tid);
    put_bytes(b" action=make_current_suspend requeue_later\n");
}

/// 资源释放路径唤醒等待线程（sem_up / mutex_unlock / condvar_signal / condvar_wait 内部）。
pub fn emit_resource_wake(op: &[u8], resource_id: usize, waking_tid: usize) {
    put_bytes(b"[LEC12-CH8] kp=resource_wake op=");
    put_bytes(op);
    put_bytes(b" resource_id=");
    put_decimal(resource_id);
    put_bytes(b" waking_tid=");
    put_decimal(waking_tid);
    put_bytes(b" action=re_enque\n");
}

/// `thread_create` 成功：新 TID 进入就绪队列。
pub fn emit_thread_created(new_tid: usize, parent_pid: usize, entry: usize, arg: usize) {
    put_bytes(b"[LEC11-CH8] kp=thread_created new_tid=");
    put_decimal(new_tid);
    put_bytes(b" pid=");
    put_decimal(parent_pid);
    put_bytes(b" entry=");
    put_hex_u64(entry as u64);
    put_bytes(b" arg=");
    put_hex_u64(arg as u64);
    console_putchar(b'\n');
}

/// 异常类 trap（非 UserEnvCall）：与 lec5 风格一致，便于对照调试。
pub fn emit_trap_kill(tid: usize, cause_str: &str) {
    put_bytes(b"[LEC11-CH8] kp=unsupported_trap tid=");
    put_decimal(tid);
    put_bytes(b" cause=");
    put_bytes(cause_str.as_bytes());
    console_putchar(b'\n');
}
