//! 进程管理模块
//!
//! 与第六章相比，本章的 `Process` 有两项重要变化：
//!
//! 1. **fd_table 类型变化**：从 `Vec<Option<Mutex<FileHandle>>>` 变为 `Vec<Option<Mutex<Fd>>>`，
//!    使用统一的 `Fd` 枚举同时管理普通文件、管道和标准 I/O。
//!
//! 2. **新增 signal 字段**：每个进程拥有独立的信号处理器（`Box<dyn Signal>`），
//!    支持信号的接收、屏蔽、处理和继承。

use crate::{build_flags, fs::Fd, map_portal, parse_flags, Sv39, Sv39Manager};
use alloc::{alloc::alloc_zeroed, boxed::Box, vec::Vec};
use core::alloc::Layout;
use spin::Mutex;
use tg_kernel_context::{foreign::ForeignContext, LocalContext};
use tg_kernel_vm::{
    page_table::{MmuMeta, VAddr, PPN, VPN},
    AddressSpace,
};
use tg_signal::Signal;
use tg_signal_impl::SignalImpl;
use tg_task_manage::ProcId;
use xmas_elf::{
    header::{self, HeaderPt2, Machine},
    program, ElfFile,
};

/// 进程结构体
///
/// 与第六章相比新增了 `signal` 字段，`fd_table` 改为存储 `Fd` 枚举。
pub struct Process {
    /// 进程标识符（PID）
    pub pid: ProcId,
    /// 用户态上下文（含 satp）
    pub context: ForeignContext,
    /// 进程的独立地址空间
    pub address_space: AddressSpace<Sv39, Sv39Manager>,
    /// 统一文件描述符表（本章使用 Fd 枚举替代 FileHandle）
    pub fd_table: Vec<Option<Mutex<Fd>>>,
    /// 信号处理器（**本章新增**）
    ///
    /// 使用 `Box<dyn Signal>` trait 对象，支持多态和 fork 时的继承。
    /// 默认实现为 `SignalImpl`，内部维护：
    /// - received：已接收的信号位图
    /// - mask：信号屏蔽字
    /// - handling：正在处理的信号状态
    /// - actions：信号处理函数表
    pub signal: Box<dyn Signal>,
    /// 堆底地址
    pub heap_bottom: usize,
    /// 当前程序 break 位置
    pub program_brk: usize,
}

impl Process {
    /// exec：用新程序替换当前进程（保留 PID、fd_table 和 signal）
    pub fn exec(&mut self, elf: ElfFile) {
        let proc = Process::from_elf(elf).unwrap();
        self.address_space = proc.address_space;
        self.context = proc.context;
        self.heap_bottom = proc.heap_bottom;
        self.program_brk = proc.program_brk;
    }

    /// fork：复制当前进程创建子进程
    ///
    /// 子进程继承：
    /// - 地址空间（深拷贝）
    /// - 文件描述符表（深拷贝，子进程继承所有已打开的文件/管道）
    /// - 信号配置（通过 `signal.from_fork()` 继承）
    pub fn fork(&mut self) -> Option<Process> {
        let pid = ProcId::new();
        // 复制地址空间
        let parent_addr_space = &self.address_space;
        let mut address_space: AddressSpace<Sv39, Sv39Manager> = AddressSpace::new();
        parent_addr_space.cloneself(&mut address_space);
        map_portal(&address_space);
        // 复制上下文
        let context = self.context.context.clone();
        let satp = (8 << 60) | address_space.root_ppn().val();
        let foreign_ctx = ForeignContext { context, satp };
        // 复制文件描述符表（子进程继承父进程所有 fd）
        let new_fd_table: Vec<Option<Mutex<Fd>>> = self
            .fd_table
            .iter()
            .map(|fd| fd.as_ref().map(|f| Mutex::new(f.lock().clone())))
            .collect();
        Some(Self {
            pid,
            context: foreign_ctx,
            address_space,
            fd_table: new_fd_table,
            signal: self.signal.from_fork(), // 子进程继承父进程的信号配置
            heap_bottom: self.heap_bottom,
            program_brk: self.program_brk,
        })
    }

    /// 从 ELF 文件创建新进程
    ///
    /// 与第六章类似，但：
    /// - fd_table 使用 Fd::Empty 替代 FileHandle::empty
    /// - 新增 signal 字段初始化
    pub fn from_elf(elf: ElfFile) -> Option<Self> {
        let entry = match elf.header.pt2 {
            HeaderPt2::Header64(pt2)
                if pt2.type_.as_type() == header::Type::Executable
                    && pt2.machine.as_machine() == Machine::RISC_V =>
            {
                pt2.entry_point as usize
            }
            _ => None?,
        };

        const PAGE_SIZE: usize = 1 << Sv39::PAGE_BITS;
        const PAGE_MASK: usize = PAGE_SIZE - 1;

        let mut address_space = AddressSpace::new();
        let mut max_end_va: usize = 0;
        // 遍历 ELF LOAD 段，映射到地址空间
        for program in elf.program_iter() {
            if !matches!(program.get_type(), Ok(program::Type::Load)) {
                continue;
            }

            let off_file = program.offset() as usize;
            let len_file = program.file_size() as usize;
            let off_mem = program.virtual_addr() as usize;
            let end_mem = off_mem + program.mem_size() as usize;
            assert_eq!(off_file & PAGE_MASK, off_mem & PAGE_MASK);

            if end_mem > max_end_va {
                max_end_va = end_mem;
            }

            let mut flags: [u8; 5] = *b"U___V";
            if program.flags().is_execute() {
                flags[1] = b'X';
            }
            if program.flags().is_write() {
                flags[2] = b'W';
            }
            if program.flags().is_read() {
                flags[3] = b'R';
            }
            address_space.map(
                VAddr::new(off_mem).floor()..VAddr::new(end_mem).ceil(),
                &elf.input[off_file..][..len_file],
                off_mem & PAGE_MASK,
                parse_flags(unsafe { core::str::from_utf8_unchecked(&flags) }).unwrap(),
            );
        }

        // 堆底从 ELF 加载的最高地址的下一页开始
        let heap_bottom = VAddr::<Sv39>::new(max_end_va).ceil().base().val();

        // 映射用户栈（2 页 = 8 KiB）
        let stack = unsafe {
            alloc_zeroed(Layout::from_size_align_unchecked(
                2 << Sv39::PAGE_BITS,
                1 << Sv39::PAGE_BITS,
            ))
        };
        address_space.map_extern(
            VPN::new((1 << 26) - 2)..VPN::new(1 << 26),
            PPN::new(stack as usize >> Sv39::PAGE_BITS),
            build_flags("U_WRV"),
        );
        map_portal(&address_space);

        let mut context = LocalContext::user(entry);
        let satp = (8 << 60) | address_space.root_ppn().val();
        *context.sp_mut() = 1 << 38;
        Some(Self {
            pid: ProcId::new(),
            context: ForeignContext { context, satp },
            address_space,
            // fd_table 使用 Fd::Empty 表示标准 I/O
            fd_table: vec![
                Some(Mutex::new(Fd::Empty { read: true, write: false })),   // fd 0: stdin
                Some(Mutex::new(Fd::Empty { read: false, write: true })),   // fd 1: stdout
                Some(Mutex::new(Fd::Empty { read: false, write: true })),   // fd 2: stderr
            ],
            // 初始化空的信号处理器
            signal: Box::new(SignalImpl::new()),
            heap_bottom,
            program_brk: heap_bottom,
        })
    }

    /// 修改程序 break 位置（实现 sbrk）
    pub fn change_program_brk(&mut self, size: isize) -> Option<usize> {
        let old_brk = self.program_brk;
        let new_brk = self.program_brk as isize + size;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let new_brk = new_brk as usize;

        let old_brk_ceil = VAddr::<Sv39>::new(old_brk).ceil();
        let new_brk_ceil = VAddr::<Sv39>::new(new_brk).ceil();

        if size > 0 {
            if new_brk_ceil.val() > old_brk_ceil.val() {
                self.address_space
                    .map(old_brk_ceil..new_brk_ceil, &[], 0, build_flags("U_WRV"));
            }
        } else if size < 0 {
            if old_brk_ceil.val() > new_brk_ceil.val() {
                self.address_space.unmap(new_brk_ceil..old_brk_ceil);
            }
        }

        self.program_brk = new_brk;
        Some(old_brk)
    }
}
