use crate::{ClockId, SignalAction, SignalNo, Stat, SyscallId, TimeSpec};
use bitflags::*;
use native::*;

/// 向文件描述符写入数据。
///
/// see <https://man7.org/linux/man-pages/man2/write.2.html>.
#[inline]
pub fn write(fd: usize, buffer: &[u8]) -> isize {
    // SAFETY: buffer 是有效的切片引用，其指针和长度在调用期间有效
    unsafe { syscall3(SyscallId::WRITE, fd, buffer.as_ptr() as _, buffer.len()) }
}

/// 从文件描述符读取数据。
#[inline]
pub fn read(fd: usize, buffer: &[u8]) -> isize {
    // SAFETY: buffer 是有效的切片引用，其指针和长度在调用期间有效
    unsafe { syscall3(SyscallId::READ, fd, buffer.as_ptr() as _, buffer.len()) }
}

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

/// 打开文件。
#[inline]
pub fn open(path: &str, flags: OpenFlags) -> isize {
    // SAFETY: path 是有效的字符串引用
    unsafe {
        syscall2(
            SyscallId::OPENAT,
            path.as_ptr() as usize,
            flags.bits as usize,
        )
    }
}

/// 关闭文件描述符。
#[inline]
pub fn close(fd: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::CLOSE, fd) }
}

/// 创建一个文件的一个硬链接。
pub fn link(oldpath: &str, newpath: &str) -> isize {
    // SAFETY: oldpath 和 newpath 是有效的字符串引用
    unsafe {
        syscall5(
            SyscallId::LINKAT,
            -100isize as usize, // AT_FDCWD
            oldpath.as_ptr() as usize,
            -100isize as usize, // AT_FDCWD
            newpath.as_ptr() as usize,
            0,
        )
    }
}

/// 取消一个文件路径到文件的链接。
pub fn unlink(path: &str) -> isize {
    // SAFETY: path 是有效的字符串引用
    unsafe {
        syscall3(
            SyscallId::UNLINKAT,
            -100isize as usize, // AT_FDCWD
            path.as_ptr() as usize,
            0,
        )
    }
}

/// 获取文件状态。
pub fn fstat(fd: usize, st: &mut Stat) -> isize {
    // SAFETY: 调用者需要确保 st 指向有效的可写内存
    unsafe { syscall2(SyscallId::FSTAT, fd, st as *const _ as usize) }
}

/// 退出当前进程。
///
/// see <https://man7.org/linux/man-pages/man2/exit.2.html>.
#[inline]
pub fn exit(exit_code: i32) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::EXIT, exit_code as _) }
}

/// 主动让出 CPU。
///
/// see <https://man7.org/linux/man-pages/man2/sched_yield.2.html>.
#[inline]
pub fn sched_yield() -> isize {
    // SAFETY: 无参数系统调用
    unsafe { syscall0(SyscallId::SCHED_YIELD) }
}

/// 获取时钟时间。
///
/// see <https://man7.org/linux/man-pages/man2/clock_gettime.2.html>.
#[inline]
pub fn clock_gettime(clockid: ClockId, tp: *mut TimeSpec) -> isize {
    // SAFETY: 调用者需要确保 tp 指向有效的可写内存
    unsafe { syscall2(SyscallId::CLOCK_GETTIME, clockid.0, tp as _) }
}

/// 创建子进程。
pub fn fork() -> isize {
    // SAFETY: 无参数系统调用
    unsafe { syscall0(SyscallId::CLONE) }
}

/// 执行新程序。
pub fn exec(path: &str) -> isize {
    // SAFETY: path 是有效的字符串引用
    unsafe { syscall2(SyscallId::EXECVE, path.as_ptr() as usize, path.len()) }
}

/// 等待任意子进程退出。
pub fn wait(exit_code_ptr: *mut i32) -> isize {
    loop {
        // SAFETY: 调用者需要确保 exit_code_ptr 指向有效的可写内存（或为 null）
        match unsafe { syscall2(SyscallId::WAIT4, usize::MAX, exit_code_ptr as usize) } {
            -2 => {
                sched_yield();
            }
            exit_pid => return exit_pid,
        }
    }
}

/// 等待指定子进程退出。
pub fn waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    loop {
        // SAFETY: 调用者需要确保 exit_code_ptr 指向有效的可写内存（或为 null）
        match unsafe { syscall2(SyscallId::WAIT4, pid as usize, exit_code_ptr as usize) } {
            -2 => {
                sched_yield();
            }
            exit_pid => return exit_pid,
        }
    }
}

/// 获取当前进程 ID。
pub fn getpid() -> isize {
    // SAFETY: 无参数系统调用
    unsafe { syscall0(SyscallId::GETPID) }
}

/// 创建并运行一个新进程。
pub fn spawn(path: &str) -> isize {
    // SAFETY: path 是有效的字符串引用
    unsafe { syscall2(SyscallId::SPAWN, path.as_ptr() as usize, path.len()) }
}

/// 设置进程优先级。
pub fn set_priority(prio: isize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::SETPRIORITY, prio as usize) }
}

/// 向进程发送信号。
#[inline]
pub fn kill(pid: isize, signum: SignalNo) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall2(SyscallId::KILL, pid as _, signum as _) }
}

/// 设置信号处理函数。
#[inline]
pub fn sigaction(
    signum: SignalNo,
    action: *const SignalAction,
    old_action: *const SignalAction,
) -> isize {
    // SAFETY: 调用者需要确保指针参数有效（或为 null）
    unsafe {
        syscall3(
            SyscallId::RT_SIGACTION,
            signum as _,
            action as _,
            old_action as _,
        )
    }
}

/// 设置信号掩码。
#[inline]
pub fn sigprocmask(mask: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::RT_SIGPROCMASK, mask) }
}

/// 从信号处理函数返回。
#[inline]
pub fn sigreturn() -> isize {
    // SAFETY: 无参数系统调用，只能从信号处理上下文中调用
    unsafe { syscall0(SyscallId::RT_SIGRETURN) }
}

/// 创建新线程。
#[inline]
pub fn thread_create(entry: usize, arg: usize) -> isize {
    // SAFETY: 调用者需要确保 entry 是有效的函数地址
    unsafe { syscall2(SyscallId::THREAD_CREATE, entry, arg) }
}

/// 获取当前线程 ID。
#[inline]
pub fn gettid() -> isize {
    // SAFETY: 无参数系统调用
    unsafe { syscall0(SyscallId::GETTID) }
}

/// 等待指定线程退出。
#[inline]
pub fn waittid(tid: usize) -> isize {
    loop {
        // SAFETY: 系统调用参数是简单的整数值
        match unsafe { syscall1(SyscallId::WAITID, tid) } {
            -2 => {
                sched_yield();
            }
            exit_code => return exit_code,
        }
    }
}

/// 创建信号量。
#[inline]
pub fn semaphore_create(res_count: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::SEMAPHORE_CREATE, res_count) }
}

/// 释放信号量（V 操作）。
#[inline]
pub fn semaphore_up(sem_id: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::SEMAPHORE_UP, sem_id) }
}

/// 获取信号量（P 操作）。
#[inline]
pub fn semaphore_down(sem_id: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::SEMAPHORE_DOWN, sem_id) }
}

/// 创建互斥锁。
#[inline]
pub fn mutex_create(blocking: bool) -> isize {
    // SAFETY: 系统调用参数是简单的布尔值
    unsafe { syscall1(SyscallId::MUTEX_CREATE, blocking as _) }
}

/// 获取互斥锁。
#[inline]
pub fn mutex_lock(mutex_id: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::MUTEX_LOCK, mutex_id) }
}

/// 释放互斥锁。
#[inline]
pub fn mutex_unlock(mutex_id: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::MUTEX_UNLOCK, mutex_id) }
}

/// 创建条件变量。
#[inline]
pub fn condvar_create() -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::CONDVAR_CREATE, 0) }
}

/// 唤醒等待条件变量的线程。
#[inline]
pub fn condvar_signal(condvar_id: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall1(SyscallId::CONDVAR_SIGNAL, condvar_id) }
}

/// 等待条件变量。
#[inline]
pub fn condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall2(SyscallId::CONDVAR_WAIT, condvar_id, mutex_id) }
}

/// 启用死锁检测。
#[inline]
pub fn enable_deadlock_detect(is_enable: bool) -> isize {
    // SAFETY: 系统调用参数是简单的布尔值
    unsafe { syscall1(SyscallId::ENABLE_DEADLOCK_DETECT, is_enable as usize) }
}

/// 记录系统调用。
#[inline]
pub fn trace(trace_request: usize, id: usize, data: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall3(SyscallId::TRACE, trace_request, id, data) }
}

/// 映射内存。
#[inline]
pub fn mmap(start: usize, len: usize, prot: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall6(SyscallId::MMAP, start, len, prot, 0, 0, 0) }
}

/// 取消内存映射。
#[inline]
pub fn munmap(start: usize, len: usize) -> isize {
    // SAFETY: 系统调用参数是简单的整数值
    unsafe { syscall2(SyscallId::MUNMAP, start, len) }
}

/// 这个模块包含调用系统调用的最小封装，用户可以直接使用这些函数调用自定义的系统调用。
///
/// # Safety
///
/// 所有 syscall 函数都是 unsafe 的，因为：
/// - 它们直接执行 `ecall` 指令触发系统调用
/// - 调用者需要确保参数符合对应系统调用的要求
/// - 传递无效的指针或参数可能导致未定义行为
#[cfg(target_arch = "riscv64")]
pub mod native {
    use crate::SyscallId;
    use core::arch::asm;

    /// 执行无参数的系统调用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `id` 是有效的系统调用号。
    #[inline(always)]
    pub unsafe fn syscall0(id: SyscallId) -> isize {
        let ret: isize;
        // SAFETY: ecall 指令触发系统调用，由内核处理
        asm!("ecall",
            in("a7") id.0,
            out("a0") ret,
        );
        ret
    }

    /// 执行带 1 个参数的系统调用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `id` 是有效的系统调用号，且 `a0` 符合该系统调用的参数要求。
    #[inline(always)]
    pub unsafe fn syscall1(id: SyscallId, a0: usize) -> isize {
        let ret: isize;
        // SAFETY: ecall 指令触发系统调用，由内核处理
        asm!("ecall",
            inlateout("a0") a0 => ret,
            in("a7") id.0,
        );
        ret
    }

    /// 执行带 2 个参数的系统调用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `id` 是有效的系统调用号，且参数符合该系统调用的要求。
    #[inline(always)]
    pub unsafe fn syscall2(id: SyscallId, a0: usize, a1: usize) -> isize {
        let ret: isize;
        // SAFETY: ecall 指令触发系统调用，由内核处理
        asm!("ecall",
            in("a7") id.0,
            inlateout("a0") a0 => ret,
            in("a1") a1,
        );
        ret
    }

    /// 执行带 3 个参数的系统调用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `id` 是有效的系统调用号，且参数符合该系统调用的要求。
    #[inline(always)]
    pub unsafe fn syscall3(id: SyscallId, a0: usize, a1: usize, a2: usize) -> isize {
        let ret: isize;
        // SAFETY: ecall 指令触发系统调用，由内核处理
        asm!("ecall",
            in("a7") id.0,
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
        );
        ret
    }

    /// 执行带 4 个参数的系统调用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `id` 是有效的系统调用号，且参数符合该系统调用的要求。
    #[inline(always)]
    pub unsafe fn syscall4(id: SyscallId, a0: usize, a1: usize, a2: usize, a3: usize) -> isize {
        let ret: isize;
        // SAFETY: ecall 指令触发系统调用，由内核处理
        asm!("ecall",
            in("a7") id.0,
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
        );
        ret
    }

    /// 执行带 5 个参数的系统调用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `id` 是有效的系统调用号，且参数符合该系统调用的要求。
    #[inline(always)]
    pub unsafe fn syscall5(
        id: SyscallId,
        a0: usize,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
    ) -> isize {
        let ret: isize;
        // SAFETY: ecall 指令触发系统调用，由内核处理
        asm!("ecall",
            in("a7") id.0,
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
        );
        ret
    }

    /// 执行带 6 个参数的系统调用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `id` 是有效的系统调用号，且参数符合该系统调用的要求。
    #[inline(always)]
    pub unsafe fn syscall6(
        id: SyscallId,
        a0: usize,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        a5: usize,
    ) -> isize {
        let ret: isize;
        // SAFETY: ecall 指令触发系统调用，由内核处理
        asm!("ecall",
            in("a7") id.0,
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a5") a5,
        );
        ret
    }
}

/// 非 riscv64 架构的 stub 实现。
#[cfg(not(target_arch = "riscv64"))]
pub mod native {
    use crate::SyscallId;

    /// 执行无参数的系统调用。
    #[inline(always)]
    pub unsafe fn syscall0(_id: SyscallId) -> isize {
        unimplemented!("syscall is only supported on riscv64")
    }

    /// 执行带 1 个参数的系统调用。
    #[inline(always)]
    pub unsafe fn syscall1(_id: SyscallId, _a0: usize) -> isize {
        unimplemented!("syscall is only supported on riscv64")
    }

    /// 执行带 2 个参数的系统调用。
    #[inline(always)]
    pub unsafe fn syscall2(_id: SyscallId, _a0: usize, _a1: usize) -> isize {
        unimplemented!("syscall is only supported on riscv64")
    }

    /// 执行带 3 个参数的系统调用。
    #[inline(always)]
    pub unsafe fn syscall3(_id: SyscallId, _a0: usize, _a1: usize, _a2: usize) -> isize {
        unimplemented!("syscall is only supported on riscv64")
    }

    /// 执行带 4 个参数的系统调用。
    #[inline(always)]
    pub unsafe fn syscall4(
        _id: SyscallId,
        _a0: usize,
        _a1: usize,
        _a2: usize,
        _a3: usize,
    ) -> isize {
        unimplemented!("syscall is only supported on riscv64")
    }

    /// 执行带 5 个参数的系统调用。
    #[inline(always)]
    pub unsafe fn syscall5(
        _id: SyscallId,
        _a0: usize,
        _a1: usize,
        _a2: usize,
        _a3: usize,
        _a4: usize,
    ) -> isize {
        unimplemented!("syscall is only supported on riscv64")
    }

    /// 执行带 6 个参数的系统调用。
    #[inline(always)]
    pub unsafe fn syscall6(
        _id: SyscallId,
        _a0: usize,
        _a1: usize,
        _a2: usize,
        _a3: usize,
        _a4: usize,
        _a5: usize,
    ) -> isize {
        unimplemented!("syscall is only supported on riscv64")
    }
}
