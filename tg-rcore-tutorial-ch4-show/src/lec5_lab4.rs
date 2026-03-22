//! 第五讲（lec5）知识点在 Lab4 中的**可观测**锚点。
//!
//! 每条串口行以 `[LEC5-LAB4]` 前缀开头，可被脚本 grep 验收，
//! 将课堂概念（Sv39 虚拟内存、地址空间隔离、ELF 加载、页表管理、
//! 跨地址空间上下文切换等）落到「标签 + 可搜索子串」证据链。
//!
//! 分为三类输出：
//! - **静态信息**：`emit_init_observables()` 在内核地址空间建立后一次性输出
//! - **动态信息**：`emit_*()` 函数在调度循环关键路径中按事件输出
//! - **页表摘要**：`emit_page_table_summary()` 格式化打印进程页表

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

fn read_csr_satp() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, satp", out(reg) val);
    }
    val
}

/// 初始化后的静态知识点输出（在内核地址空间建立、加载 app 后调用）。
pub fn emit_init_observables(app_count: usize, memory: usize) {
    // --- Sv39 虚拟内存 ---
    put_line(
        b"[LEC5-LAB4] kp=sv39_paging levels=3 va_bits=39 page_size=4096 pte_size=8",
    );

    // --- 地址空间隔离 ---
    put_line(
        b"[LEC5-LAB4] kp=address_space_isolation each_process_has_own_page_table satp_per_process",
    );

    // --- 内核恒等映射 ---
    put_line(
        b"[LEC5-LAB4] kp=identity_mapping kernel_va==pa direct_access_physical_memory",
    );

    // --- 内核堆分配器 ---
    put_bytes(b"[LEC5-LAB4] kp=kernel_heap allocator=tg_kernel_alloc memory=");
    put_decimal(memory);
    console_putchar(b'\n');

    // --- MultislotPortal（异界传送门） ---
    put_line(
        b"[LEC5-LAB4] kp=multislot_portal cross_address_space_context_switch portal_at_VPN_MAX",
    );

    // --- 页表项结构 ---
    put_line(
        b"[LEC5-LAB4] kp=pte_flags V=valid R=read W=write X=exec U=user G=global A=accessed D=dirty",
    );

    // --- 多道程序 ---
    put_bytes(b"[LEC5-LAB4] kp=multiprog app_count=");
    put_decimal(app_count);
    console_putchar(b'\n');

    // --- ELF 加载 ---
    put_line(
        b"[LEC5-LAB4] kp=elf_loading parse_LOAD_segments map_to_user_address_space with_permissions",
    );

    // --- 系统调用表 ---
    put_line(
        b"[LEC5-LAB4] kp=syscalls write=64 exit=93 yield=124 clock_gettime=113 sbrk=214 mmap=222 munmap=215",
    );

    // --- 地址翻译 ---
    put_line(
        b"[LEC5-LAB4] kp=address_translation syscall_translate_user_va_to_pa via=AddressSpace::translate",
    );

    // --- 上下文切换机制 ---
    put_bytes(b"[LEC5-LAB4] kp=context_switch_mech via=ForeignContext::execute portal=MultislotPortal stvec=");
    put_hex_u64(read_csr_stvec() as u64);
    console_putchar(b'\n');

    // --- 内核 satp ---
    put_bytes(b"[LEC5-LAB4] kp=kernel_satp satp=");
    put_hex_u64(read_csr_satp() as u64);
    put_bytes(b" sp=");
    put_hex_u64(read_sp() as u64);
    console_putchar(b'\n');

    // --- 特权级 ---
    put_line(b"[LEC5-LAB4] kp=privilege_levels user=U(0) kernel=S(1) sbi=M(3)");

    // --- 编译与启动 ---
    put_line(
        concat!(
            "[LEC5-LAB4] kp=compile_info target=",
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
        b"[LEC5-LAB4] kp=control_flow path=_start->rust_main->kernel_space->load_elf->schedule->shutdown",
    );
}

/// 内核地址空间建立时输出。
pub fn emit_kernel_space_created(root_ppn: usize, satp: usize) {
    put_bytes(b"[LEC5-LAB4] kp=kernel_space_created root_ppn=");
    put_hex_u64(root_ppn as u64);
    put_bytes(b" satp=");
    put_hex_u64(satp as u64);
    put_bytes(b" mode=Sv39\n");
}

/// ELF 段映射输出。
pub fn emit_elf_segment(
    proc_id: usize,
    seg_idx: usize,
    va_start: usize,
    va_end: usize,
    flags: &str,
) {
    put_bytes(b"[LEC5-LAB4] kp=elf_segment proc_id=");
    put_decimal(proc_id);
    put_bytes(b" seg=");
    put_decimal(seg_idx);
    put_bytes(b" va=");
    put_hex_u64(va_start as u64);
    put_bytes(b"..");
    put_hex_u64(va_end as u64);
    put_bytes(b" flags=");
    put_bytes(flags.as_bytes());
    console_putchar(b'\n');
}

/// 进程创建输出（ELF 加载完成后）。
pub fn emit_process_created(proc_id: usize, entry: usize, satp: usize, heap_bottom: usize) {
    put_bytes(b"[LEC5-LAB4] kp=process_created proc_id=");
    put_decimal(proc_id);
    put_bytes(b" entry=");
    put_hex_u64(entry as u64);
    put_bytes(b" satp=");
    put_hex_u64(satp as u64);
    put_bytes(b" heap_bottom=");
    put_hex_u64(heap_bottom as u64);
    console_putchar(b'\n');
}

/// 首次通过传送门进入用户态。
pub fn emit_portal_enter_user(proc_id: usize, sepc: usize, user_satp: usize) {
    put_bytes(b"[LEC5-LAB4] kp=portal_enter_user proc_id=");
    put_decimal(proc_id);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    put_bytes(b" user_satp=");
    put_hex_u64(user_satp as u64);
    put_bytes(b" sret_to_U\n");
}

/// 系统调用陷入输出（含地址翻译信息）。
pub fn emit_syscall_trap(proc_id: usize, syscall_id: usize, a0: usize, sepc: usize) {
    put_bytes(b"[LEC5-LAB4] kp=syscall_trap proc_id=");
    put_decimal(proc_id);
    put_bytes(b" syscall_id=");
    put_decimal(syscall_id);
    put_bytes(b" a0=");
    put_hex_u64(a0 as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// 时钟中断抢占输出。
pub fn emit_timer_interrupt(proc_id: usize) {
    put_bytes(b"[LEC5-LAB4] kp=timer_interrupt proc_id=");
    put_decimal(proc_id);
    put_bytes(b" scause=SupervisorTimer preempt\n");
}

/// 进程退出输出。
pub fn emit_process_exit(proc_id: usize, exit_code: usize, remain: usize) {
    put_bytes(b"[LEC5-LAB4] kp=process_exit proc_id=");
    put_decimal(proc_id);
    put_bytes(b" exit_code=");
    put_decimal(exit_code);
    put_bytes(b" remain=");
    put_decimal(remain);
    console_putchar(b'\n');
}

/// 异常杀死进程输出。
pub fn emit_exception_kill(proc_id: usize, cause_str: &str, stval: usize, sepc: usize) {
    put_bytes(b"[LEC5-LAB4] kp=exception_kill proc_id=");
    put_decimal(proc_id);
    put_bytes(b" cause=");
    put_bytes(cause_str.as_bytes());
    put_bytes(b" stval=");
    put_hex_u64(stval as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// 进程切换（包含页表/satp 切换信息）。
pub fn emit_process_switch(from_proc: usize, to_proc: usize, new_satp: usize) {
    put_bytes(b"[LEC5-LAB4] kp=process_switch from_proc=");
    put_decimal(from_proc);
    put_bytes(b" to_proc=");
    put_decimal(to_proc);
    put_bytes(b" new_satp=");
    put_hex_u64(new_satp as u64);
    console_putchar(b'\n');
}

/// 打印进程页表摘要（使用 AddressSpace 的 Debug 实现）。
pub fn emit_page_table_summary(proc_id: usize, address_space: &impl core::fmt::Debug) {
    put_bytes(b"[LEC5-LAB4] kp=page_table proc_id=");
    put_decimal(proc_id);
    console_putchar(b'\n');

    struct SbiWriter;
    impl core::fmt::Write for SbiWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for b in s.bytes() {
                console_putchar(b);
            }
            Ok(())
        }
    }

    use core::fmt::Write;
    let _ = write!(SbiWriter, "[LEC5-LAB4] page_table_content={:?}\n", address_space);
}

/// sbrk 系统调用执行时输出。
#[allow(dead_code)]
pub fn emit_sbrk(proc_id: usize, old_brk: usize, new_brk: usize, size: isize) {
    put_bytes(b"[LEC5-LAB4] kp=sbrk proc_id=");
    put_decimal(proc_id);
    put_bytes(b" old_brk=");
    put_hex_u64(old_brk as u64);
    put_bytes(b" new_brk=");
    put_hex_u64(new_brk as u64);
    put_bytes(b" size=");
    if size < 0 {
        console_putchar(b'-');
        put_decimal((-size) as usize);
    } else {
        put_decimal(size as usize);
    }
    console_putchar(b'\n');
}
