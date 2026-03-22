//! 第四讲（lec4）知识点在 Lab3 中的**可观测**锚点。
//!
//! 每条串口行以 `[LEC4-LAB3]` 前缀开头，可被脚本 grep 验收，
//! 把课堂概念（多道程序、协作/抢占式调度、任务生命周期、特权级切换等）
//! 落到「命令 + 子串」证据链。
//!
//! 分为两类输出：
//! - **静态信息**：`emit_init_observables()` 在加载 app 后、主循环前一次性输出
//! - **动态信息**：`emit_*()` 函数在主循环关键路径中按事件输出

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

/// 初始化后的静态知识点输出（在加载 app 后、主循环前调用）。
pub fn emit_init_observables(app_count: usize, base: u64, step: u64) {
    // --- 多道程序驻留 ---
    put_bytes(b"[LEC4-LAB3] kp=multiprog app_count=");
    put_decimal(app_count);
    put_bytes(b" base=");
    put_hex_u64(base);
    put_bytes(b" step=");
    put_hex_u64(step);
    console_putchar(b'\n');

    // --- 任务控制块 ---
    put_line(
        b"[LEC4-LAB3] kp=tcb_layout fields=ctx(LocalContext)+finish+stack(8KiB)",
    );

    // --- 任务上下文 ---
    put_line(
        b"[LEC4-LAB3] kp=task_context type=LocalContext fields=sctx,x1..x31,sepc,supervisor,interrupt",
    );

    // --- 调度模型 ---
    #[cfg(feature = "coop")]
    put_line(b"[LEC4-LAB3] kp=scheduling_model mode=cooperative yield_only");
    #[cfg(not(feature = "coop"))]
    put_line(
        b"[LEC4-LAB3] kp=scheduling_model mode=preemptive timeslice=12500_cycles round_robin",
    );

    // --- 任务生命周期 ---
    put_line(b"[LEC4-LAB3] kp=task_lifecycle states=init,ready,running,exit");

    // --- 特权级 ---
    put_line(b"[LEC4-LAB3] kp=privilege_levels user=U(0) kernel=S(1) sbi=M(3)");

    // --- 系统调用表 ---
    put_line(
        b"[LEC4-LAB3] kp=syscalls write=64 exit=93 yield=124 clock_gettime=113 trace=410",
    );

    // --- 上下文切换机制 ---
    put_bytes(b"[LEC4-LAB3] kp=context_switch_mech via=LocalContext::execute trap=stvec(");
    put_hex_u64(read_csr_stvec() as u64);
    put_bytes(b") sret_to_user\n");

    // --- 内核栈指针 ---
    put_bytes(b"[LEC4-LAB3] kp=kernel_stack sp=");
    put_hex_u64(read_sp() as u64);
    console_putchar(b'\n');

    // --- 编译与启动 ---
    put_line(
        concat!(
            "[LEC4-LAB3] kp=compile_info target=",
            env!("LAB_TARGET_TRIPLE"),
            " M_BASE=",
            env!("LAB_M_BASE"),
            " S_BASE=",
            env!("LAB_S_BASE"),
        )
        .as_bytes(),
    );

    // --- 控制流 ---
    put_line(
        b"[LEC4-LAB3] kp=control_flow path=_start->rust_main->load_apps->main_loop->shutdown",
    );
}

/// 应用加载时输出。
pub fn emit_app_load(app_id: usize, entry: usize, stack_top: usize) {
    put_bytes(b"[LEC4-LAB3] kp=app_load app_id=");
    put_decimal(app_id);
    put_bytes(b" entry=");
    put_hex_u64(entry as u64);
    put_bytes(b" stack_top=");
    put_hex_u64(stack_top as u64);
    console_putchar(b'\n');
}

/// 首次进入用户态前输出。
pub fn emit_first_enter_user(app_id: usize, sepc: usize) {
    let sstatus = read_csr_sstatus();
    put_bytes(b"[LEC4-LAB3] kp=first_enter_user app_id=");
    put_decimal(app_id);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    put_bytes(b" sstatus=");
    put_hex_u64(sstatus as u64);
    put_bytes(b" sret_to_U\n");
}

/// 时钟中断抢占输出。
pub fn emit_timer_interrupt(app_id: usize) {
    put_bytes(b"[LEC4-LAB3] kp=timer_interrupt app_id=");
    put_decimal(app_id);
    put_bytes(b" scause=SupervisorTimer preempt\n");
}

/// 系统调用陷入输出。
pub fn emit_syscall_trap(app_id: usize, syscall_id: usize, a0: usize, sepc: usize) {
    put_bytes(b"[LEC4-LAB3] kp=syscall_trap app_id=");
    put_decimal(app_id);
    put_bytes(b" syscall_id=");
    put_decimal(syscall_id);
    put_bytes(b" a0=");
    put_hex_u64(a0 as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// yield 调度切换输出。
pub fn emit_yield_switch(from_app: usize, to_app: usize) {
    put_bytes(b"[LEC4-LAB3] kp=yield_switch from_app=");
    put_decimal(from_app);
    put_bytes(b" to_app=");
    put_decimal(to_app);
    console_putchar(b'\n');
}

/// 任务退出输出。
pub fn emit_task_exit(app_id: usize, exit_code: usize, remain: usize) {
    put_bytes(b"[LEC4-LAB3] kp=task_exit app_id=");
    put_decimal(app_id);
    put_bytes(b" exit_code=");
    put_decimal(exit_code);
    put_bytes(b" remain=");
    put_decimal(remain);
    console_putchar(b'\n');
}

/// 异常杀死任务输出。
pub fn emit_exception_kill(app_id: usize, cause_str: &str, stval: usize) {
    put_bytes(b"[LEC4-LAB3] kp=exception_kill app_id=");
    put_decimal(app_id);
    put_bytes(b" cause=");
    put_bytes(cause_str.as_bytes());
    put_bytes(b" stval=");
    put_hex_u64(stval as u64);
    console_putchar(b'\n');
}

/// 进程切换输出（轮转到下一个任务）。
pub fn emit_task_switch(from_app: usize, to_app: usize) {
    put_bytes(b"[LEC4-LAB3] kp=task_switch from_app=");
    put_decimal(from_app);
    put_bytes(b" to_app=");
    put_decimal(to_app);
    console_putchar(b'\n');
}
