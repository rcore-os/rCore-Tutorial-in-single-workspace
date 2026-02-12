//! 进程管理模块
//!
//! 与第五章相比，本章的 `Process` 新增了**文件描述符表**（`fd_table`）字段，
//! 每个进程拥有自己的 fd_table，统一管理标准 I/O 和磁盘文件。
//!
//! ## 文件描述符表
//!
//! | fd | 用途 |
//! |----|------|
//! | 0 | 标准输入（stdin） |
//! | 1 | 标准输出（stdout） |
//! | 2 | 标准错误（stderr） |
//! | 3+ | 普通文件（通过 open 系统调用分配） |
//!
//! 教程阅读建议：
//!
//! - 先看 `from_elf`：理解用户地址空间与初始 fd_table 如何构建；
//! - 再看 `fork`：观察地址空间和文件描述符的继承规则；
//! - 最后看 `change_program_brk`：理解用户堆扩缩时的页映射变化。

use crate::{build_flags, map_portal, parse_flags, Sv39, Sv39Manager};
use alloc::{alloc::alloc_zeroed, vec::Vec};
use core::alloc::Layout;
use spin::Mutex;
use tg_easy_fs::FileHandle;
use tg_kernel_context::{foreign::ForeignContext, LocalContext};
use tg_kernel_vm::{
    page_table::{MmuMeta, VAddr, PPN, VPN},
    AddressSpace,
};
use tg_task_manage::ProcId;
use xmas_elf::{
    header::{self, HeaderPt2, Machine},
    program, ElfFile,
};

/// 进程结构体
///
/// 与第五章相比新增了 `fd_table` 字段。
pub struct Process {
    /// 进程标识符（PID），创建后不可变
    pub pid: ProcId,
    /// 用户态上下文（含 satp，支持跨地址空间切换）
    pub context: ForeignContext,
    /// 进程的独立地址空间
    pub address_space: AddressSpace<Sv39, Sv39Manager>,
    /// 文件描述符表
    ///
    /// 每个 fd 对应一个 `Option<Mutex<FileHandle>>`：
    /// - `Some(...)`: 有效的文件句柄
    /// - `None`: 该 fd 已关闭或未使用
    ///
    /// 预留 fd 0/1/2 分别为 stdin/stdout/stderr。
    pub fd_table: Vec<Option<Mutex<FileHandle>>>,
    /// 堆底地址
    pub heap_bottom: usize,
    /// 当前程序 break 位置（堆顶）
    pub program_brk: usize,
}

impl Process {
    /// exec：用新程序替换当前进程（保留 PID 和 fd_table）
    pub fn exec(&mut self, elf: ElfFile) {
        let proc = Process::from_elf(elf).unwrap();
        self.address_space = proc.address_space;
        self.context = proc.context;
        self.heap_bottom = proc.heap_bottom;
        self.program_brk = proc.program_brk;
    }

    /// fork：复制当前进程创建子进程
    ///
    /// 深拷贝地址空间和文件描述符表。
    /// 子进程继承父进程的所有已打开文件。
    pub fn fork(&mut self) -> Option<Process> {
        let pid = ProcId::new();
        // 复制父进程的完整地址空间
        let parent_addr_space = &self.address_space;
        let mut address_space: AddressSpace<Sv39, Sv39Manager> = AddressSpace::new();
        parent_addr_space.cloneself(&mut address_space);
        map_portal(&address_space);
        // 复制父进程上下文
        let context = self.context.context.clone();
        let satp = (8 << 60) | address_space.root_ppn().val();
        let foreign_ctx = ForeignContext { context, satp };
        // 复制父进程的文件描述符表
        // 子进程继承父进程所有已打开的文件
        let mut new_fd_table: Vec<Option<Mutex<FileHandle>>> = Vec::new();
        for fd in self.fd_table.iter_mut() {
            if let Some(file) = fd {
                new_fd_table.push(Some(Mutex::new(file.get_mut().clone())));
            } else {
                new_fd_table.push(None);
            }
        }
        Some(Self {
            pid,
            context: foreign_ctx,
            address_space,
            fd_table: new_fd_table,
            heap_bottom: self.heap_bottom,
            program_brk: self.program_brk,
        })
    }

    /// 从 ELF 文件创建新进程
    ///
    /// 与第五章相同的 ELF 解析流程，但新增了文件描述符表的初始化：
    /// - fd 0 = stdin（可读）
    /// - fd 1 = stdout（可写）
    /// - fd 2 = stderr（可写）
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
        // 映射异界传送门
        map_portal(&address_space);

        // 创建用户态上下文
        let mut context = LocalContext::user(entry);
        let satp = (8 << 60) | address_space.root_ppn().val();
        *context.sp_mut() = 1 << 38;
        Some(Self {
            pid: ProcId::new(),
            context: ForeignContext { context, satp },
            address_space,
            // 初始化文件描述符表：预留 stdin(0)、stdout(1)、stderr(2)
            fd_table: vec![
                Some(Mutex::new(FileHandle::empty(true, false))),  // fd 0: stdin（可读）
                Some(Mutex::new(FileHandle::empty(false, true))),  // fd 1: stdout（可写）
                Some(Mutex::new(FileHandle::empty(false, true))),  // fd 2: stderr（可写）
            ],
            heap_bottom,
            program_brk: heap_bottom,
        })
    }

    /// 修改程序 break 位置（实现 sbrk 系统调用）
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
