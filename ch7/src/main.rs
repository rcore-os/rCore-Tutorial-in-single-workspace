//! # 第七章：进程间通信与信号
//!
//! 本章在第六章"文件系统"的基础上，引入了两大新机制：
//!
//! ## 1. 管道（Pipe）—— 进程间通信
//!
//! 管道是一对文件描述符（读端 + 写端），用于父子进程之间的**单向数据传递**。
//! 管道的读写端通过统一的 `Fd` 枚举类型管理，与普通文件和标准 I/O 共享 fd_table。
//!
//! ```text
//! 父进程                  管道                 子进程
//! write(pipe_fd[1]) ──→ [环形缓冲区] ──→ read(pipe_fd[0])
//! ```
//!
//! ## 2. 信号（Signal）—— 异步事件通知
//!
//! 信号允许一个进程异步通知另一个进程发生了某个事件（如 SIGKILL、SIGINT 等）。
//! 每个进程维护：
//! - 待处理信号集合（received）
//! - 信号屏蔽字（mask）
//! - 信号处理函数表（actions）
//!
//! 内核在系统调用返回前检查并处理待处理信号。
//!
//! ## 与第六章的主要区别
//!
//! | 特性 | 第六章 | 第七章 |
//! |------|--------|--------|
//! | fd 类型 | FileHandle（仅普通文件） | Fd 枚举（File/PipeRead/PipeWrite/Empty） |
//! | IPC | 无 | 管道（pipe 系统调用） |
//! | 信号 | 无 | kill/sigaction/sigprocmask/sigreturn |
//! | 依赖 | 无信号库 | tg-signal + tg-signal-impl |
//!
//! 教程阅读建议：
//!
//! - 先看 `rust_main` 中的 trap 主循环：理解系统调用返回前的信号处理时机；
//! - 再看 `impls::IO` 的 `pipe/read/write`：理解管道与普通文件的统一入口；
//! - 最后看 `impls::Signal`：掌握 kill/sigaction/sigreturn 的内核语义。

// 不使用标准库，裸机环境没有操作系统提供系统调用支持
#![no_std]
// 不使用默认的 main 函数入口
#![no_main]
// 在 RISC-V 架构上启用严格的编译警告和文档要求
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
// 在非 RISC-V 架构上允许未使用的代码
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code, unused_imports))]

/// 文件系统模块：easy-fs 封装 + 统一的 Fd 枚举
mod fs;
/// 进程模块：Process 结构体（含 fd_table 和 signal）
mod process;
/// 处理器模块：PROCESSOR 全局管理器
mod processor;
/// VirtIO 块设备驱动
mod virtio_block;

#[macro_use]
extern crate tg_console;

#[macro_use]
extern crate alloc;

use crate::{
    fs::{read_all, FS},
    impls::{Sv39Manager, SyscallContext},
    process::Process,
    processor::ProcManager,
};
use alloc::alloc::alloc;
use core::{alloc::Layout, cell::UnsafeCell, mem::MaybeUninit};
use impls::Console;
pub use processor::PROCESSOR;
use riscv::register::*;
#[cfg(not(target_arch = "riscv64"))]
use stub::Sv39;
use tg_console::log;
use tg_easy_fs::{FSManager, OpenFlags};
use tg_kernel_context::foreign::MultislotPortal;
#[cfg(target_arch = "riscv64")]
use tg_kernel_vm::page_table::Sv39;
use tg_kernel_vm::{
    page_table::{MmuMeta, VAddr, VmFlags, VmMeta, PPN, VPN},
    AddressSpace,
};
use tg_sbi;
use tg_signal::SignalResult;
use tg_syscall::Caller;
use tg_task_manage::{PManager, ProcId};
use xmas_elf::ElfFile;

/// 构建 VmFlags（虚拟内存标志位）。
#[cfg(target_arch = "riscv64")]
const fn build_flags(s: &str) -> VmFlags<Sv39> {
    VmFlags::build_from_str(s)
}

/// 运行时解析 VmFlags 字符串。
#[cfg(target_arch = "riscv64")]
fn parse_flags(s: &str) -> Result<VmFlags<Sv39>, ()> {
    s.parse()
}

#[cfg(not(target_arch = "riscv64"))]
use stub::{build_flags, parse_flags};

// 定义内核入口点，设置启动栈大小为 32 页 = 128 KiB。
//
// 这里不再调用 tg_linker::boot0! 宏，避免外部已发布版本与 Rust 2024
// 在属性语义上的兼容差异影响本 crate 的发布校验。
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 32 * 4096;
    #[unsafe(link_section = ".boot.stack")]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",
        "j  {main}",
        stack = sym STACK,
        stack_size = const STACK_SIZE,
        main = sym rust_main,
    )
}

/// 物理内存容量 = 48 MiB
const MEMORY: usize = 48 << 20;

/// 异界传送门所在虚页（虚拟地址空间最高页）
const PROTAL_TRANSIT: VPN<Sv39> = VPN::MAX;

/// 内核地址空间的全局存储（延迟初始化）
struct KernelSpace {
    inner: UnsafeCell<MaybeUninit<AddressSpace<Sv39, Sv39Manager>>>,
}

unsafe impl Sync for KernelSpace {}

impl KernelSpace {
    const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// 写入内核地址空间（仅在初始化时调用一次）
    unsafe fn write(&self, space: AddressSpace<Sv39, Sv39Manager>) {
        unsafe { *self.inner.get() = MaybeUninit::new(space) };
    }

    /// 获取内核地址空间的不可变引用
    unsafe fn assume_init_ref(&self) -> &AddressSpace<Sv39, Sv39Manager> {
        unsafe { &*(*self.inner.get()).as_ptr() }
    }
}

/// 内核地址空间全局实例
static KERNEL_SPACE: KernelSpace = KernelSpace::new();

/// VirtIO MMIO 设备地址范围
pub const MMIO: &[(usize, usize)] = &[(0x1000_1000, 0x00_1000)];

/// 内核主函数——系统初始化和启动入口
///
/// 与第六章基本相同，但新增了信号系统调用的初始化。
///
/// 执行流程：
/// 1. 清零 BSS 段 → 初始化控制台 → 初始化堆
/// 2. 创建异界传送门 → 建立内核地址空间（含 MMIO 映射）
/// 3. 初始化系统调用（**新增 `init_signal`**）
/// 4. 从文件系统加载 initproc → 进入调度循环
///
/// 调度循环中的信号处理（本章新增）：
/// 在每次系统调用返回之前，检查当前进程的待处理信号并执行对应的处理：
/// - `SignalResult::ProcessKilled`：进程被终止
/// - 其他情况：正常处理系统调用返回值
extern "C" fn rust_main() -> ! {
    let layout = tg_linker::KernelLayout::locate();
    // 步骤 1：清零 BSS 段
    unsafe { layout.zero_bss() };
    // 步骤 2：初始化控制台输出和日志系统
    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG"));
    tg_console::test_log();
    // 步骤 3：初始化内核堆分配器
    tg_kernel_alloc::init(layout.start() as _);
    unsafe {
        tg_kernel_alloc::transfer(core::slice::from_raw_parts_mut(
            layout.end() as _,
            MEMORY - layout.len(),
        ))
    };
    // 步骤 4：分配异界传送门所需的物理页面
    let portal_size = MultislotPortal::calculate_size(1);
    let portal_layout = Layout::from_size_align(portal_size, 1 << Sv39::PAGE_BITS).unwrap();
    let portal_ptr = unsafe { alloc(portal_layout) };
    assert!(portal_layout.size() < 1 << Sv39::PAGE_BITS);
    // 步骤 5：建立内核地址空间并激活 Sv39 分页
    kernel_space(layout, MEMORY, portal_ptr as _);
    // 步骤 6：初始化异界传送门
    let portal = unsafe { MultislotPortal::init_transit(PROTAL_TRANSIT.base().val(), 1) };
    // 步骤 7：初始化系统调用处理器
    // 注意：与第六章相比，新增了 init_signal（信号相关系统调用）
    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);
    tg_syscall::init_scheduling(&SyscallContext);
    tg_syscall::init_clock(&SyscallContext);
    tg_syscall::init_signal(&SyscallContext);   // 本章新增：初始化信号系统调用
    // 步骤 8：从文件系统加载初始进程 initproc
    let initproc = read_all(FS.open("initproc", OpenFlags::RDONLY).unwrap());
    if let Some(process) = Process::from_elf(ElfFile::new(initproc.as_slice()).unwrap()) {
        PROCESSOR.get_mut().set_manager(ProcManager::new());
        PROCESSOR
            .get_mut()
            .add(process.pid, process, ProcId::from_usize(usize::MAX));
    }

    // ─── 主调度循环 ───
    loop {
        let processor: *mut PManager<Process, ProcManager> = PROCESSOR.get_mut() as *mut _;
        if let Some(task) = unsafe { (*processor).find_next() } {
            // 通过异界传送门切换到用户地址空间执行
            unsafe { task.context.execute(portal, ()) };

            // ─── Trap 返回后处理 ───
            match scause::read().cause() {
                // ─── 系统调用（ecall 指令触发） ───
                scause::Trap::Exception(scause::Exception::UserEnvCall) => {
                    use tg_syscall::{SyscallId as Id, SyscallResult as Ret};
                    let ctx = &mut task.context.context;
                    ctx.move_next();
                    let id: Id = ctx.a(7).into();
                    let args = [ctx.a(0), ctx.a(1), ctx.a(2), ctx.a(3), ctx.a(4), ctx.a(5)];
                    let syscall_ret = tg_syscall::handle(Caller { entity: 0, flow: 0 }, id, args);

                    // ─── 本章新增：信号处理 ───
                    // 在系统调用返回用户态之前，检查并处理待处理信号。
                    // 注意：这只是一个简化的实现位置。理想情况下，
                    // 信号应该在所有 trap 处理完毕、返回用户态之前统一检查。
                    match task.signal.handle_signals(ctx) {
                        // 收到终止信号（如 SIGKILL），进程应该退出
                        SignalResult::ProcessKilled(exit_code) => unsafe {
                            (*processor).make_current_exited(exit_code as _)
                        },
                        // 未被终止，继续处理系统调用返回值
                        _ => match syscall_ret {
                            Ret::Done(ret) => match id {
                                Id::EXIT => unsafe { (*processor).make_current_exited(ret) },
                                _ => {
                                    let ctx = &mut task.context.context;
                                    *ctx.a_mut(0) = ret as _;
                                    unsafe { (*processor).make_current_suspend() };
                                }
                            },
                            Ret::Unsupported(_) => {
                                log::info!("id = {id:?}");
                                unsafe { (*processor).make_current_exited(-2) };
                            }
                        },
                    }
                }
                // ─── 其他异常/中断：杀死进程 ───
                e => {
                    log::error!("unsupported trap: {e:?}");
                    unsafe { (*processor).make_current_exited(-3) };
                }
            }
        } else {
            println!("no task");
            break;
        }
    }

    tg_sbi::shutdown(false)
}

/// Rust panic 处理函数
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    tg_sbi::shutdown(true)
}

/// 建立内核地址空间（与第六章相同）
///
/// 包含：内核段恒等映射、堆区域、异界传送门、VirtIO MMIO 映射
fn kernel_space(layout: tg_linker::KernelLayout, memory: usize, portal: usize) {
    let mut space = AddressSpace::new();
    // 映射内核各段（恒等映射）
    for region in layout.iter() {
        log::info!("{region}");
        use tg_linker::KernelRegionTitle::*;
        let flags = match region.title {
            Text => "X_RV",
            Rodata => "__RV",
            Data | Boot => "_WRV",
        };
        let s = VAddr::<Sv39>::new(region.range.start);
        let e = VAddr::<Sv39>::new(region.range.end);
        space.map_extern(
            s.floor()..e.ceil(),
            PPN::new(s.floor().val()),
            build_flags(flags),
        )
    }
    // 映射堆区域
    let s = VAddr::<Sv39>::new(layout.end());
    let e = VAddr::<Sv39>::new(layout.start() + memory);
    log::info!("(heap) ---> {:#10x}..{:#10x}", s.val(), e.val());
    space.map_extern(
        s.floor()..e.ceil(),
        PPN::new(s.floor().val()),
        build_flags("_WRV"),
    );
    // 映射异界传送门页面
    space.map_extern(
        PROTAL_TRANSIT..PROTAL_TRANSIT + 1,
        PPN::new(portal >> Sv39::PAGE_BITS),
        build_flags("__G_XWRV"),
    );
    println!();

    // 映射 VirtIO MMIO 设备地址
    for (base, len) in MMIO {
        let s = VAddr::<Sv39>::new(*base);
        let e = VAddr::<Sv39>::new(*base + *len);
        log::info!("MMIO range -> {:#10x}..{:#10x}", s.val(), e.val());
        space.map_extern(
            s.floor()..e.ceil(),
            PPN::new(s.floor().val()),
            build_flags("_WRV"),
        );
    }

    // 激活 Sv39 分页模式
    unsafe { satp::set(satp::Mode::Sv39, 0, space.root_ppn().val()) };
    unsafe { KERNEL_SPACE.write(space) };
}

/// 将内核地址空间中的异界传送门页表项复制到用户地址空间
fn map_portal(space: &AddressSpace<Sv39, Sv39Manager>) {
    let portal_idx = PROTAL_TRANSIT.index_in(Sv39::MAX_LEVEL);
    space.root()[portal_idx] = unsafe { KERNEL_SPACE.assume_init_ref() }.root()[portal_idx];
}

/// 各种接口库的实现
///
/// 本模块为 tg-syscall 提供的各个 trait 提供具体实现。
/// 与第六章相比，本章新增了：
/// - `pipe` 系统调用：创建管道，分配读端和写端的文件描述符
/// - `Signal` trait 实现：kill/sigaction/sigprocmask/sigreturn
/// - read/write 扩展：支持管道的读写
mod impls {
    use crate::{
        build_flags,
        fs::{read_all, Fd, FS},
        process::Process as ProcStruct,
        processor::ProcManager,
        Sv39, PROCESSOR,
    };
    use alloc::{alloc::alloc_zeroed, string::String, vec::Vec};
    use core::{alloc::Layout, ptr::NonNull};
    use spin::Mutex;
    use tg_console::log;
    use tg_easy_fs::{make_pipe, FSManager, OpenFlags, UserBuffer};
    use tg_kernel_vm::{
        page_table::{MmuMeta, Pte, VAddr, VmFlags, PPN, VPN},
        PageManager,
    };
    use tg_signal::SignalNo;
    use tg_syscall::*;
    use tg_task_manage::{PManager, ProcId};
    use xmas_elf::ElfFile;

    // ─── Sv39 页表管理器（与第六章相同） ───

    /// Sv39 页表管理器
    #[repr(transparent)]
    pub struct Sv39Manager(NonNull<Pte<Sv39>>);

    impl Sv39Manager {
        /// 自定义标志位：标记此页面由内核分配
        const OWNED: VmFlags<Sv39> = unsafe { VmFlags::from_raw(1 << 8) };

        /// 分配对齐的物理页面（已清零）
        #[inline]
        fn page_alloc<T>(count: usize) -> *mut T {
            unsafe {
                alloc_zeroed(Layout::from_size_align_unchecked(
                    count << Sv39::PAGE_BITS,
                    1 << Sv39::PAGE_BITS,
                ))
            }
            .cast()
        }
    }

    impl PageManager<Sv39> for Sv39Manager {
        #[inline]
        fn new_root() -> Self {
            Self(NonNull::new(Self::page_alloc(1)).unwrap())
        }
        #[inline]
        fn root_ppn(&self) -> PPN<Sv39> {
            PPN::new(self.0.as_ptr() as usize >> Sv39::PAGE_BITS)
        }
        #[inline]
        fn root_ptr(&self) -> NonNull<Pte<Sv39>> {
            self.0
        }
        #[inline]
        fn p_to_v<T>(&self, ppn: PPN<Sv39>) -> NonNull<T> {
            unsafe { NonNull::new_unchecked(VPN::<Sv39>::new(ppn.val()).base().as_mut_ptr()) }
        }
        #[inline]
        fn v_to_p<T>(&self, ptr: NonNull<T>) -> PPN<Sv39> {
            PPN::new(VAddr::<Sv39>::new(ptr.as_ptr() as _).floor().val())
        }
        #[inline]
        fn check_owned(&self, pte: Pte<Sv39>) -> bool {
            pte.flags().contains(Self::OWNED)
        }
        #[inline]
        fn allocate(&mut self, len: usize, flags: &mut VmFlags<Sv39>) -> NonNull<u8> {
            *flags |= Self::OWNED;
            NonNull::new(Self::page_alloc(len)).unwrap()
        }
        fn deallocate(&mut self, _pte: Pte<Sv39>, _len: usize) -> usize {
            todo!()
        }
        fn drop_root(&mut self) {
            todo!()
        }
    }

    // ─── 控制台实现 ───

    /// 控制台输出实现
    pub struct Console;

    impl tg_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            tg_sbi::console_putchar(c);
        }
    }

    // ─── 系统调用实现 ───

    /// 系统调用上下文
    pub struct SyscallContext;

    /// 可读权限标志
    const READABLE: VmFlags<Sv39> = build_flags("RV");
    /// 可写权限标志
    const WRITEABLE: VmFlags<Sv39> = build_flags("W_V");

    /// IO 系统调用实现
    ///
    /// 与第六章相比：
    /// - fd_table 存储 `Fd` 枚举而非 `FileHandle`，统一管理文件/管道/空描述符
    /// - 新增 `pipe` 系统调用
    /// - read/write 通过 `Fd` 的统一接口处理文件和管道
    impl IO for SyscallContext {
        /// write 系统调用：写入文件/管道/标准输出
        fn write(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Some(ptr) = current.address_space.translate(VAddr::new(buf), READABLE) {
                if fd == STDOUT || fd == STDDEBUG {
                    // 标准输出：直接打印到控制台
                    print!("{}", unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            ptr.as_ptr(),
                            count,
                        ))
                    });
                    count as _
                } else if let Some(file) = &current.fd_table[fd] {
                    // 普通文件或管道：通过 Fd 统一接口写入
                    let file = file.lock();
                    if file.writable() {
                        let mut v: Vec<&'static mut [u8]> = Vec::new();
                        unsafe { v.push(core::slice::from_raw_parts_mut(ptr.as_ptr(), count)) };
                        file.write(UserBuffer::new(v)) as _
                    } else {
                        log::error!("file not writable");
                        -1
                    }
                } else {
                    log::error!("unsupported fd: {fd}");
                    -1
                }
            } else {
                log::error!("ptr not readable");
                -1
            }
        }

        /// read 系统调用：从文件/管道/标准输入读取
        fn read(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Some(ptr) = current.address_space.translate(VAddr::new(buf), WRITEABLE) {
                if fd == STDIN {
                    // 标准输入：通过 SBI 逐字符读取
                    let mut ptr = ptr.as_ptr();
                    for _ in 0..count {
                        unsafe {
                            *ptr = tg_sbi::console_getchar() as u8;
                            ptr = ptr.add(1);
                        }
                    }
                    count as _
                } else if let Some(file) = &current.fd_table[fd] {
                    // 普通文件或管道：通过 Fd 统一接口读取
                    let file = file.lock();
                    if file.readable() {
                        let mut v: Vec<&'static mut [u8]> = Vec::new();
                        unsafe { v.push(core::slice::from_raw_parts_mut(ptr.as_ptr(), count)) };
                        file.read(UserBuffer::new(v)) as _
                    } else {
                        log::error!("file not readable");
                        -1
                    }
                } else {
                    log::error!("unsupported fd: {fd}");
                    -1
                }
            } else {
                log::error!("ptr not writeable");
                -1
            }
        }

        /// open 系统调用：打开文件（与第六章相同，但 fd_table 中存 Fd::File）
        fn open(&self, _caller: Caller, path: usize, flags: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Some(ptr) = current.address_space.translate(VAddr::new(path), READABLE) {
                // 从用户空间逐字符读取文件路径
                let mut string = String::new();
                let mut raw_ptr: *mut u8 = ptr.as_ptr();
                loop {
                    unsafe {
                        let ch = *raw_ptr;
                        if ch == 0 {
                            break;
                        }
                        string.push(ch as char);
                        raw_ptr = (raw_ptr as usize + 1) as *mut u8;
                    }
                }

                if let Some(file_handle) =
                    FS.open(string.as_str(), OpenFlags::from_bits(flags as u32).unwrap())
                {
                    let new_fd = current.fd_table.len();
                    // 将 FileHandle 包装为 Fd::File 存入 fd_table
                    current
                        .fd_table
                        .push(Some(Mutex::new(Fd::File((*file_handle).clone()))));
                    new_fd as isize
                } else {
                    -1
                }
            } else {
                log::error!("ptr not writeable");
                -1
            }
        }

        /// close 系统调用
        #[inline]
        fn close(&self, _caller: Caller, fd: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if fd >= current.fd_table.len() || current.fd_table[fd].is_none() {
                return -1;
            }
            current.fd_table[fd].take();
            0
        }

        /// pipe 系统调用：创建管道（**本章新增**）
        ///
        /// 创建一对管道文件描述符：
        /// - pipe[0] = 读端 fd（只读）
        /// - pipe[1] = 写端 fd（只写）
        ///
        /// 典型使用流程：
        /// 1. 父进程调用 pipe() 获得 (read_fd, write_fd)
        /// 2. fork() 创建子进程（继承 fd_table）
        /// 3. 子进程关闭写端，从读端读取；父进程关闭读端，向写端写入
        fn pipe(&self, _caller: Caller, pipe: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            // 创建管道（环形缓冲区 + 读端 + 写端）
            let (read_end, write_end) = make_pipe();
            let read_fd = current.fd_table.len();
            let write_fd = read_fd + 1;
            // 将 read_fd 写入用户空间的 pipe[0]
            if let Some(mut ptr) = current
                .address_space
                .translate::<usize>(VAddr::new(pipe), WRITEABLE)
            {
                unsafe { *ptr.as_mut() = read_fd };
            } else {
                return -1;
            }
            // 将 write_fd 写入用户空间的 pipe[1]
            if let Some(mut ptr) = current
                .address_space
                .translate::<usize>(VAddr::new(pipe + core::mem::size_of::<usize>()), WRITEABLE)
            {
                unsafe { *ptr.as_mut() = write_fd };
            } else {
                return -1;
            }
            // 将读端和写端加入 fd_table
            current
                .fd_table
                .push(Some(Mutex::new(Fd::PipeRead(read_end))));
            current
                .fd_table
                .push(Some(Mutex::new(Fd::PipeWrite(write_end))));
            0
        }
    }

    /// 进程管理系统调用实现（与第六章基本相同）
    impl Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: Caller, exit_code: usize) -> isize {
            exit_code as isize
        }

        /// fork 系统调用（子进程继承 fd_table 和信号配置）
        fn fork(&self, _caller: Caller) -> isize {
            let processor: *mut PManager<ProcStruct, ProcManager> = PROCESSOR.get_mut() as *mut _;
            let current = unsafe { (*processor).current().unwrap() };
            let parent_pid = current.pid;
            let mut child_proc = current.fork().unwrap();
            let pid = child_proc.pid;
            let context = &mut child_proc.context.context;
            *context.a_mut(0) = 0 as _;
            unsafe {
                (*processor).add(pid, child_proc, parent_pid);
            }
            pid.get_usize() as isize
        }

        /// exec 系统调用：从文件系统加载新程序
        fn exec(&self, _caller: Caller, path: usize, count: usize) -> isize {
            const READABLE: VmFlags<Sv39> = build_flags("RV");
            let current = PROCESSOR.get_mut().current().unwrap();
            current
                .address_space
                .translate(VAddr::new(path), READABLE)
                .map(|ptr| unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr.as_ptr(), count))
                })
                .and_then(|name| FS.open(name, OpenFlags::RDONLY))
                .map_or_else(
                    || {
                        log::error!("unknown app, select one in the list: ");
                        FS.readdir("")
                            .unwrap()
                            .into_iter()
                            .for_each(|app| println!("{app}"));
                        println!();
                        -1
                    },
                    |fd| {
                        current.exec(ElfFile::new(&read_all(fd)).unwrap());
                        0
                    },
                )
        }

        /// wait 系统调用
        fn wait(&self, _caller: Caller, pid: isize, exit_code_ptr: usize) -> isize {
            let processor: *mut PManager<ProcStruct, ProcManager> = PROCESSOR.get_mut() as *mut _;
            let current = unsafe { (*processor).current().unwrap() };
            const WRITABLE: VmFlags<Sv39> = build_flags("W_V");
            if let Some((dead_pid, exit_code)) =
                unsafe { (*processor).wait(ProcId::from_usize(pid as usize)) }
            {
                if let Some(mut ptr) = current
                    .address_space
                    .translate::<i32>(VAddr::new(exit_code_ptr), WRITABLE)
                {
                    unsafe { *ptr.as_mut() = exit_code as i32 };
                }
                return dead_pid.get_usize() as isize;
            } else {
                return -1;
            }
        }

        /// getpid 系统调用
        fn getpid(&self, _caller: Caller) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            current.pid.get_usize() as _
        }

        /// sbrk 系统调用
        fn sbrk(&self, _caller: Caller, size: i32) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Some(old_brk) = current.change_program_brk(size as isize) {
                old_brk as isize
            } else {
                -1
            }
        }
    }

    /// 调度系统调用实现
    impl Scheduling for SyscallContext {
        #[inline]
        fn sched_yield(&self, _caller: Caller) -> isize {
            0
        }
    }

    /// 时钟系统调用实现
    impl Clock for SyscallContext {
        #[inline]
        fn clock_gettime(&self, _caller: Caller, clock_id: ClockId, tp: usize) -> isize {
            const WRITABLE: VmFlags<Sv39> = build_flags("W_V");
            match clock_id {
                ClockId::CLOCK_MONOTONIC => {
                    if let Some(mut ptr) = PROCESSOR
                        .get_mut()
                        .current()
                        .unwrap()
                        .address_space
                        .translate(VAddr::new(tp), WRITABLE)
                    {
                        let time = riscv::register::time::read() * 10000 / 125;
                        *unsafe { ptr.as_mut() } = TimeSpec {
                            tv_sec: time / 1_000_000_000,
                            tv_nsec: time % 1_000_000_000,
                        };
                        0
                    } else {
                        log::error!("ptr not readable");
                        -1
                    }
                }
                _ => -1,
            }
        }
    }

    /// 信号系统调用实现（**本章新增**）
    ///
    /// 实现了四个信号相关的系统调用：
    /// - `kill`：向指定进程发送信号
    /// - `sigaction`：设置信号处理函数
    /// - `sigprocmask`：设置信号屏蔽字
    /// - `sigreturn`：从信号处理函数返回
    impl Signal for SyscallContext {
        /// kill 系统调用：向指定 PID 的进程发送信号
        fn kill(&self, _caller: Caller, pid: isize, signum: u8) -> isize {
            if let Some(target_task) = PROCESSOR
                .get_mut()
                .get_task(ProcId::from_usize(pid as usize))
            {
                if let Ok(signal_no) = SignalNo::try_from(signum) {
                    if signal_no != SignalNo::ERR {
                        target_task.signal.add_signal(signal_no);
                        return 0;
                    }
                }
            }
            -1
        }

        /// sigaction 系统调用：设置或获取信号处理函数
        ///
        /// - old_action != 0 时：将当前信号处理函数写入 old_action 指向的地址
        /// - action != 0 时：从 action 指向的地址读取新的信号处理函数并设置
        fn sigaction(
            &self,
            _caller: Caller,
            signum: u8,
            action: usize,
            old_action: usize,
        ) -> isize {
            if signum as usize > tg_signal::MAX_SIG {
                return -1;
            }
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Ok(signal_no) = SignalNo::try_from(signum) {
                if signal_no == SignalNo::ERR {
                    return -1;
                }
                // 如果需要返回旧的处理函数
                if old_action as usize != 0 {
                    if let Some(mut ptr) = current
                        .address_space
                        .translate(VAddr::new(old_action), WRITEABLE)
                    {
                        if let Some(signal_action) = current.signal.get_action_ref(signal_no) {
                            *unsafe { ptr.as_mut() } = signal_action;
                        } else {
                            return -1;
                        }
                    } else {
                        return -1;
                    }
                }
                // 如果需要设置新的处理函数
                if action as usize != 0 {
                    if let Some(ptr) = current
                        .address_space
                        .translate(VAddr::new(action), READABLE)
                    {
                        if !current
                            .signal
                            .set_action(signal_no, &unsafe { *ptr.as_ptr() })
                        {
                            return -1;
                        }
                    } else {
                        return -1;
                    }
                }
                return 0;
            }
            -1
        }

        /// sigprocmask 系统调用：更新信号屏蔽字
        fn sigprocmask(&self, _caller: Caller, mask: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            current.signal.update_mask(mask) as isize
        }

        /// sigreturn 系统调用：从信号处理函数返回
        ///
        /// 恢复进程被信号中断前的上下文（LocalContext）
        fn sigreturn(&self, _caller: Caller) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if current.signal.sig_return(&mut current.context.context) {
                0
            } else {
                -1
            }
        }
    }
}

/// 非 RISC-V64 架构的占位实现
#[cfg(not(target_arch = "riscv64"))]
mod stub {
    use tg_kernel_vm::page_table::{MmuMeta, VmFlags};

    /// Sv39 占位类型
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub struct Sv39;

    impl MmuMeta for Sv39 {
        const P_ADDR_BITS: usize = 56;
        const PAGE_BITS: usize = 12;
        const LEVEL_BITS: &'static [usize] = &[9, 9, 9];
        const PPN_POS: usize = 10;
        #[inline]
        fn is_leaf(value: usize) -> bool {
            value & 0b1110 != 0
        }
    }

    /// 构建 VmFlags 占位实现
    pub const fn build_flags(_s: &str) -> VmFlags<Sv39> {
        unsafe { VmFlags::from_raw(0) }
    }

    /// 解析 VmFlags 占位实现
    pub fn parse_flags(_s: &str) -> Result<VmFlags<Sv39>, ()> {
        Ok(unsafe { VmFlags::from_raw(0) })
    }

    /// 主机平台占位入口
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        0
    }

    /// libc 启动占位
    #[unsafe(no_mangle)]
    pub extern "C" fn __libc_start_main() -> i32 {
        0
    }

    /// 异常处理占位
    #[unsafe(no_mangle)]
    pub extern "C" fn rust_eh_personality() {}
}
