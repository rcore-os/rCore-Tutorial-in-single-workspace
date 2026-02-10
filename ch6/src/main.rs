//! # 第六章：文件系统
//!
//! 本章在第五章"进程管理"的基础上，引入了 **文件系统** 支持。
//! 用户程序不再嵌入内核镜像，而是存放在 **磁盘镜像**（fs.img）中，
//! 内核通过 **VirtIO 块设备驱动** 和 **easy-fs 文件系统** 按名称加载和执行程序。
//!
//! ## 核心概念
//!
//! - **文件系统（easy-fs）**：简单的类 UNIX inode 文件系统，支持单级目录
//! - **块设备驱动（VirtIO-blk）**：通过 MMIO 访问虚拟块设备
//! - **文件描述符表**：每个进程维护 fd_table，统一管理标准 I/O 和普通文件
//! - **文件操作系统调用**：open、close、read、write
//!
//! ## 与第五章的区别
//!
//! | 特性 | 第五章 | 第六章 |
//! |------|--------|--------|
//! | 程序存储 | 嵌入内核镜像（APP_ASM） | 磁盘镜像（fs.img） |
//! | 程序加载 | 按名称查内存表 | 通过文件系统 open + read |
//! | I/O 方式 | 仅 SBI 控制台 | 文件描述符表 + 文件句柄 |
//! | 块设备 | 无 | VirtIO-blk 驱动 |
//! | QEMU 参数 | 无磁盘 | 挂载 fs.img 块设备 |

// 不使用标准库，裸机环境没有操作系统提供系统调用支持
#![no_std]
// 不使用默认的 main 函数入口，裸机环境需要自定义入口点
#![no_main]
// 在 RISC-V 架构上启用严格的编译警告和文档要求
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
// 在非 RISC-V 架构上允许未使用的代码（用于 IDE 开发体验）
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code, unused_imports))]

/// 文件系统模块：easy-fs 文件系统管理器
mod fs;
/// 进程模块：定义 Process 结构体（含文件描述符表）
mod process;
/// 处理器模块：定义 PROCESSOR 全局变量和进程管理器
mod processor;
/// VirtIO 块设备驱动模块
mod virtio_block;

#[macro_use]
extern crate tg_console;

#[macro_use]
extern crate alloc;

use crate::{
    fs::{read_all, FS},
    impls::{Console, Sv39Manager, SyscallContext},
    process::Process,
    processor::ProcManager,
};
use alloc::alloc::alloc;
use core::{alloc::Layout, cell::UnsafeCell, mem::MaybeUninit};
use processor::PROCESSOR;
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

// 定义内核入口点，设置启动栈大小为 32 页 = 128 KiB
#[cfg(target_arch = "riscv64")]
tg_linker::boot0!(rust_main; stack = 32 * 4096);

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
///
/// QEMU virt 平台上 VirtIO 块设备的 MMIO 基地址为 0x1000_1000，大小 0x1000。
/// 需要在内核地址空间中进行恒等映射，以便驱动程序访问。
pub const MMIO: &[(usize, usize)] = &[(0x1000_1000, 0x00_1000)];

/// 内核主函数——系统初始化和启动入口
///
/// 执行流程：
/// 1. 清零 BSS 段
/// 2. 初始化控制台和日志系统
/// 3. 初始化内核堆分配器
/// 4. 分配并创建异界传送门
/// 5. 建立内核地址空间（恒等映射 + MMIO 映射 + 传送门映射），激活 Sv39 分页
/// 6. 初始化异界传送门和系统调用处理器
/// 7. 从文件系统加载初始进程 `initproc`，进入调度循环
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
    // 步骤 5：建立内核地址空间并激活 Sv39 分页（包含 MMIO 映射）
    kernel_space(layout, MEMORY, portal_ptr as _);
    // 步骤 6：初始化异界传送门
    let portal = unsafe { MultislotPortal::init_transit(PROTAL_TRANSIT.base().val(), 1) };
    // 步骤 7：初始化系统调用处理器
    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);
    tg_syscall::init_scheduling(&SyscallContext);
    tg_syscall::init_clock(&SyscallContext);
    tg_syscall::init_memory(&SyscallContext);
    // 步骤 8：从文件系统加载初始进程 initproc
    // 与第五章不同：程序从磁盘镜像（fs.img）中读取，而非内核内嵌
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
            // 通过异界传送门切换到用户地址空间执行用户程序
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
                    match tg_syscall::handle(Caller { entity: 0, flow: 0 }, id, args) {
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

/// Rust panic 处理函数，打印错误信息并以异常方式关机
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    tg_sbi::shutdown(true)
}

/// 建立内核地址空间
///
/// 内核使用**恒等映射**（Identity Mapping）：虚拟地址 == 物理地址。
///
/// 与第五章相比，本章新增了 **MMIO 映射**，用于访问 VirtIO 块设备。
///
/// 映射内容：
/// 1. 内核代码段、数据段（恒等映射）
/// 2. 堆区域（恒等映射）
/// 3. 异界传送门页面
/// 4. VirtIO MMIO 设备地址（0x10001000，恒等映射）
fn kernel_space(layout: tg_linker::KernelLayout, memory: usize, portal: usize) {
    let mut space = AddressSpace::new();
    // 映射内核各段（恒等映射：VPN == PPN）
    for region in layout.iter() {
        log::info!("{region}");
        use tg_linker::KernelRegionTitle::*;
        let flags = match region.title {
            Text => "X_RV",       // 代码段：可执行、可读
            Rodata => "__RV",     // 只读数据：可读
            Data | Boot => "_WRV", // 数据段：可写、可读
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

    // 映射 VirtIO MMIO 设备地址（恒等映射）
    // 这是本章新增的：VirtIO 块设备通过 MMIO 方式访问
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
    // 保存内核地址空间到全局变量
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
/// 与第五章相比，本章新增了文件系统相关的系统调用：
/// - `open`：打开文件，返回文件描述符
/// - `close`：关闭文件描述符
/// - `read`/`write`：支持文件读写（不仅限于标准 I/O）
/// - `linkat`/`unlinkat`/`fstat`：硬链接相关（TODO 练习题）
mod impls {
    use crate::{
        build_flags,
        fs::{read_all, FS},
        process::Process as ProcStruct,
        processor::ProcManager,
        Sv39, PROCESSOR,
    };
    use alloc::vec::Vec;
    use alloc::{alloc::alloc_zeroed, string::String};
    use core::{alloc::Layout, ptr::NonNull};
    use spin::Mutex;
    use tg_console::log;
    use tg_easy_fs::UserBuffer;
    use tg_easy_fs::{FSManager, OpenFlags};
    use tg_kernel_vm::{
        page_table::{MmuMeta, Pte, VAddr, VmFlags, PPN, VPN},
        PageManager,
    };
    use tg_syscall::*;
    use tg_task_manage::{PManager, ProcId};
    use xmas_elf::ElfFile;

    // ─── Sv39 页表管理器 ───

    /// Sv39 页表管理器（与第五章相同）
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

    /// 控制台输出实现，通过 SBI 接口逐字符输出
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

    /// 可读权限标志（用于地址翻译时的权限检查）
    const READABLE: VmFlags<Sv39> = build_flags("RV");
    /// 可写权限标志
    const WRITEABLE: VmFlags<Sv39> = build_flags("W_V");

    /// IO 系统调用实现：read、write、open、close
    ///
    /// 与第五章的关键区别：
    /// - read/write 不仅支持标准 I/O，还支持通过文件描述符读写文件
    /// - 新增 open/close 系统调用，通过 easy-fs 打开磁盘上的文件
    impl IO for SyscallContext {
        /// write 系统调用：写入文件或标准输出
        ///
        /// - fd == STDOUT/STDDEBUG：直接通过控制台输出
        /// - 其他 fd：通过文件描述符表查找文件句柄，写入文件
        fn write(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Some(ptr) = current
                .address_space
                .translate::<u8>(VAddr::new(buf), READABLE)
            {
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
                    // 普通文件：通过文件句柄写入
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

        /// read 系统调用：从文件或标准输入读取
        ///
        /// - fd == STDIN：通过 SBI console_getchar 逐字符读取
        /// - 其他 fd：通过文件句柄从磁盘文件读取
        fn read(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Some(ptr) = current
                .address_space
                .translate::<u8>(VAddr::new(buf), WRITEABLE)
            {
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
                    // 普通文件：通过文件句柄读取
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

        /// open 系统调用：打开文件
        ///
        /// 从用户空间读取文件路径（以 '\0' 结尾的字符串），
        /// 通过 easy-fs 文件系统打开文件，分配新的文件描述符。
        fn open(&self, _caller: Caller, path: usize, flags: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if let Some(ptr) = current.address_space.translate(VAddr::new(path), READABLE) {
                // 从用户空间逐字符读取文件路径（需要地址翻译）
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

                // 通过文件系统打开文件，分配新的文件描述符
                if let Some(fd) =
                    FS.open(string.as_str(), OpenFlags::from_bits(flags as u32).unwrap())
                {
                    let new_fd = current.fd_table.len();
                    current.fd_table.push(Some(Mutex::new(fd.as_ref().clone())));
                    new_fd as isize
                } else {
                    -1
                }
            } else {
                log::error!("ptr not writeable");
                -1
            }
        }

        /// close 系统调用：关闭文件描述符
        #[inline]
        fn close(&self, _caller: Caller, fd: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            if fd >= current.fd_table.len() || current.fd_table[fd].is_none() {
                return -1;
            }
            current.fd_table[fd].take();
            0
        }

        /// linkat 系统调用：创建硬链接
        ///
        /// TODO: 实现 linkat 系统调用（练习题）
        fn linkat(
            &self,
            _caller: Caller,
            _olddirfd: i32,
            _oldpath: usize,
            _newdirfd: i32,
            _newpath: usize,
            _flags: u32,
        ) -> isize {
            tg_console::log::info!("linkat: not implemented");
            -1
        }

        /// unlinkat 系统调用：删除硬链接
        ///
        /// TODO: 实现 unlinkat 系统调用（练习题）
        fn unlinkat(&self, _caller: Caller, _dirfd: i32, _path: usize, _flags: u32) -> isize {
            tg_console::log::info!("unlinkat: not implemented");
            -1
        }

        /// fstat 系统调用：获取文件状态
        ///
        /// TODO: 实现 fstat 系统调用（练习题）
        fn fstat(&self, _caller: Caller, _fd: usize, _st: usize) -> isize {
            tg_console::log::info!("fstat: not implemented");
            -1
        }
    }

    /// 进程管理系统调用实现（与第五章基本相同）
    impl Process for SyscallContext {
        /// exit 系统调用
        #[inline]
        fn exit(&self, _caller: Caller, exit_code: usize) -> isize {
            exit_code as isize
        }

        /// fork 系统调用：创建子进程（包含复制文件描述符表）
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
        ///
        /// 与第五章不同：程序从 easy-fs 文件系统中读取，而非内存中的 APPS 表
        fn exec(&self, _caller: Caller, path: usize, count: usize) -> isize {
            const READABLE: VmFlags<Sv39> = build_flags("RV");
            let current = PROCESSOR.get_mut().current().unwrap();
            current
                .address_space
                .translate::<u8>(VAddr::new(path), READABLE)
                .map(|ptr| unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr.as_ptr(), count))
                })
                .and_then(|name| FS.open(name, OpenFlags::RDONLY))
                .map_or_else(
                    || {
                        log::error!("unknown app, select one in the list: ");
                        // 列出文件系统中所有可用程序
                        FS.readdir("")
                            .unwrap()
                            .into_iter()
                            .for_each(|app| println!("{app}"));
                        println!();
                        -1
                    },
                    |fd| {
                        // 从文件系统读取完整 ELF 数据并加载
                        current.exec(ElfFile::new(&read_all(fd)).unwrap());
                        0
                    },
                )
        }

        /// wait 系统调用：等待子进程退出
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

        /// spawn 系统调用（TODO 练习题）
        fn spawn(&self, _caller: Caller, _path: usize, _count: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            tg_console::log::info!(
                "spawn: parent pid = {}, not implemented",
                current.pid.get_usize()
            );
            -1
        }

        /// sbrk 系统调用：调整堆大小
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

        /// set_priority 系统调用（TODO 练习题）
        fn set_priority(&self, _caller: Caller, prio: isize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            tg_console::log::info!(
                "set_priority: pid = {}, prio = {}, not implemented",
                current.pid.get_usize(),
                prio
            );
            -1
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
                        .translate::<TimeSpec>(VAddr::new(tp), WRITABLE)
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

    /// 内存管理系统调用实现
    impl Memory for SyscallContext {
        /// mmap 系统调用（TODO 练习题）
        fn mmap(
            &self,
            _caller: Caller,
            addr: usize,
            len: usize,
            prot: i32,
            _flags: i32,
            _fd: i32,
            _offset: usize,
        ) -> isize {
            tg_console::log::info!(
                "mmap: addr = {addr:#x}, len = {len}, prot = {prot}, not implemented"
            );
            -1
        }

        /// munmap 系统调用（TODO 练习题）
        fn munmap(&self, _caller: Caller, addr: usize, len: usize) -> isize {
            tg_console::log::info!("munmap: addr = {addr:#x}, len = {len}, not implemented");
            -1
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
