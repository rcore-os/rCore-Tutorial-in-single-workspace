//! 第九讲（lec9）「文件系统」知识点在 Lab6 中的**可观测**锚点。
//!
//! 串口行以 `[LEC9-LAB6]` 前缀开头，便于 `grep` 与 GDB 演示脚本关联课堂概念：
//! 磁盘与块设备、VirtIO-blk、easy-fs、inode/目录、文件描述符表、
//! open/read/write/close、exec 从磁盘加载 ELF、地址翻译与跨态执行等。

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

#[cfg(target_arch = "riscv64")]
fn read_sp() -> usize {
    let sp: usize;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp);
    }
    sp
}

#[cfg(not(target_arch = "riscv64"))]
fn read_sp() -> usize {
    0
}

#[cfg(target_arch = "riscv64")]
fn read_csr_satp() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, satp", out(reg) val);
    }
    val
}

#[cfg(not(target_arch = "riscv64"))]
fn read_csr_satp() -> usize {
    0
}

#[cfg(target_arch = "riscv64")]
fn read_csr_stvec() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, stvec", out(reg) val);
    }
    val
}

#[cfg(not(target_arch = "riscv64"))]
fn read_csr_stvec() -> usize {
    0
}

/// 内核完成 Sv39 + MMIO 映射后的静态信息。
pub fn emit_init_observables(
    memory: usize,
    initproc_bytes: usize,
    virtio_mmio: usize,
    root_ppn: usize,
) {
    // lec9 p1 文件系统总体
    put_line(
        b"[LEC9-LAB6] kp=fs_overview on_disk=fs.img qemu=virtio-blk block_sz=512 easy_fs=tg_easy_fs",
    );

    // VirtIO 块设备与 MMIO
    put_bytes(b"[LEC9-LAB6] kp=virtio_blk mmio_base=");
    put_hex_u64(virtio_mmio as u64);
    put_bytes(b" driver=virtio-drivers path=BLOCK_DEVICE->EasyFileSystem::open\n");

    // inode / 单级目录
    put_line(
        b"[LEC9-LAB6] kp=directory_model single_level_root readdir=FS.readdir inode=EasyFileSystem",
    );

    // 文件描述符表
    put_line(
        b"[LEC9-LAB6] kp=fd_table per_process vec stdin=0 stdout=1 stderr=2 files=fd>=3",
    );

    // 与 ch5 对比：程序不在内核镜像内嵌
    put_line(
        b"[LEC9-LAB6] kp=prog_storage not_APP_ASM load=FS.open+read_all ELF_to_Process::from_elf",
    );

    // 系统调用（文件与进程相关）
    put_line(
        b"[LEC9-LAB6] kp=syscalls open close read write exec fork wait exit sbrk yield getpid",
    );

    // 地址翻译（用户缓冲区）
    put_line(
        b"[LEC9-LAB6] kp=address_translation syscall_buffers via=AddressSpace::translate U/S_isolation",
    );

    // 内核 satp / 传送门
    put_bytes(b"[LEC9-LAB6] kp=kernel_satp satp=");
    put_hex_u64(read_csr_satp() as u64);
    put_bytes(b" root_ppn=");
    put_hex_u64(root_ppn as u64);
    put_bytes(b" sp=");
    put_hex_u64(read_sp() as u64);
    console_putchar(b'\n');

    put_bytes(b"[LEC9-LAB6] kp=stvec_trap_path stvec=");
    put_hex_u64(read_csr_stvec() as u64);
    put_bytes(b" portal=MultislotPortal\n");

    put_bytes(b"[LEC9-LAB6] kp=kernel_heap memory=");
    put_decimal(memory);
    put_bytes(b" initproc_elf_bytes=");
    put_decimal(initproc_bytes);
    console_putchar(b'\n');

    put_line(concat!(
        "[LEC9-LAB6] kp=compile_info target=",
        env!("LAB_TARGET_TRIPLE"),
        " M_BASE=",
        env!("LAB_M_BASE"),
        " S_BASE=",
        env!("LAB_S_BASE"),
    )
    .as_bytes());

    put_line(
        b"[LEC9-LAB6] kp=control_flow _start->rust_main->kernel_space->FS+initproc->schedule->syscalls->shutdown",
    );
}

/// 内核地址空间就绪（含 MMIO）。
pub fn emit_kernel_space_created(root_ppn: usize, satp: usize) {
    put_bytes(b"[LEC9-LAB6] kp=kernel_space_created root_ppn=");
    put_hex_u64(root_ppn as u64);
    put_bytes(b" satp=");
    put_hex_u64(satp as u64);
    put_bytes(b" mmio_mapped=virtio0\n");
}

/// 从磁盘读入 initproc ELF。
pub fn emit_initproc_loaded(bytes: usize) {
    put_bytes(b"[LEC9-LAB6] kp=initproc_loaded from_fs=initproc total_bytes=");
    put_decimal(bytes);
    console_putchar(b'\n');
}

/// 首个用户进程创建完成。
pub fn emit_process_created(proc_id: usize, entry: usize, user_satp: usize, heap_bottom: usize) {
    put_bytes(b"[LEC9-LAB6] kp=process_created proc_id=");
    put_decimal(proc_id);
    put_bytes(b" entry=");
    put_hex_u64(entry as u64);
    put_bytes(b" user_satp=");
    put_hex_u64(user_satp as u64);
    put_bytes(b" heap_bottom=");
    put_hex_u64(heap_bottom as u64);
    put_bytes(b" initial_fd=0,1,2\n");
}

/// 首次经传送门进入用户态。
pub fn emit_portal_enter_user(proc_id: usize, sepc: usize, user_satp: usize) {
    put_bytes(b"[LEC9-LAB6] kp=portal_enter_user proc_id=");
    put_decimal(proc_id);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    put_bytes(b" user_satp=");
    put_hex_u64(user_satp as u64);
    put_bytes(b" sret_to_U\n");
}

/// 用户态系统调用陷入。
pub fn emit_syscall_trap(proc_id: usize, syscall_id: usize, a0: usize, sepc: usize) {
    put_bytes(b"[LEC9-LAB6] kp=syscall_trap proc_id=");
    put_decimal(proc_id);
    put_bytes(b" syscall_id=");
    put_decimal(syscall_id);
    put_bytes(b" a0=");
    put_hex_u64(a0 as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// 进程退出。
pub fn emit_process_exit(proc_id: usize, exit_code: isize) {
    put_bytes(b"[LEC9-LAB6] kp=process_exit proc_id=");
    put_decimal(proc_id);
    put_bytes(b" exit_code=");
    if exit_code < 0 {
        console_putchar(b'-');
        put_decimal((-exit_code) as usize);
    } else {
        put_decimal(exit_code as usize);
    }
    console_putchar(b'\n');
}

/// 用户态异常导致进程终止。
pub fn emit_exception_kill(proc_id: usize, cause_str: &str, stval: usize, sepc: usize) {
    put_bytes(b"[LEC9-LAB6] kp=exception_kill proc_id=");
    put_decimal(proc_id);
    put_bytes(b" cause=");
    put_bytes(cause_str.as_bytes());
    put_bytes(b" stval=");
    put_hex_u64(stval as u64);
    put_bytes(b" sepc=");
    put_hex_u64(sepc as u64);
    console_putchar(b'\n');
}

/// open：内核路径解析结果。
pub fn emit_open_result(proc_id: usize, path: &str, flags: u32, new_fd: isize) {
    put_bytes(b"[LEC9-LAB6] kp=open proc_id=");
    put_decimal(proc_id);
    put_bytes(b" path=");
    put_bytes(path.as_bytes());
    put_bytes(b" flags=");
    put_hex_u64(flags as u64);
    put_bytes(b" new_fd=");
    if new_fd < 0 {
        console_putchar(b'-');
        put_decimal((-new_fd) as usize);
    } else {
        put_decimal(new_fd as usize);
    }
    console_putchar(b'\n');
}

/// read 返回。
pub fn emit_read_result(proc_id: usize, fd: usize, requested: usize, got: isize) {
    put_bytes(b"[LEC9-LAB6] kp=read proc_id=");
    put_decimal(proc_id);
    put_bytes(b" fd=");
    put_decimal(fd);
    put_bytes(b" requested=");
    put_decimal(requested);
    put_bytes(b" ret=");
    if got < 0 {
        console_putchar(b'-');
        put_decimal((-got) as usize);
    } else {
        put_decimal(got as usize);
    }
    console_putchar(b'\n');
}

/// write 返回。
pub fn emit_write_result(proc_id: usize, fd: usize, count: usize, ret: isize) {
    put_bytes(b"[LEC9-LAB6] kp=write proc_id=");
    put_decimal(proc_id);
    put_bytes(b" fd=");
    put_decimal(fd);
    put_bytes(b" count=");
    put_decimal(count);
    put_bytes(b" ret=");
    if ret < 0 {
        console_putchar(b'-');
        put_decimal((-ret) as usize);
    } else {
        put_decimal(ret as usize);
    }
    console_putchar(b'\n');
}

/// close。
pub fn emit_close(proc_id: usize, fd: usize) {
    put_bytes(b"[LEC9-LAB6] kp=close proc_id=");
    put_decimal(proc_id);
    put_bytes(b" fd=");
    put_decimal(fd);
    console_putchar(b'\n');
}

/// exec：从文件系统加载新镜像。
pub fn emit_exec_load(proc_id: usize, name: &str) {
    put_bytes(b"[LEC9-LAB6] kp=exec_load proc_id=");
    put_decimal(proc_id);
    put_bytes(b" elf_path=");
    put_bytes(name.as_bytes());
    put_bytes(b" via=FS.open+read_all+Process::exec\n");
}

/// fork。
pub fn emit_fork(proc_id: usize, child_pid: usize) {
    put_bytes(b"[LEC9-LAB6] kp=fork proc_id=");
    put_decimal(proc_id);
    put_bytes(b" child_pid=");
    put_decimal(child_pid);
    put_bytes(b" fd_table_copied\n");
}

/// sbrk。
pub fn emit_sbrk(proc_id: usize, size: i32, old_brk: usize) {
    put_bytes(b"[LEC9-LAB6] kp=sbrk proc_id=");
    put_decimal(proc_id);
    put_bytes(b" size=");
    if size < 0 {
        console_putchar(b'-');
        put_decimal((-size) as usize);
    } else {
        put_decimal(size as usize);
    }
    put_bytes(b" old_brk=");
    put_hex_u64(old_brk as u64);
    console_putchar(b'\n');
}
