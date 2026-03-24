//! 进程管理模块
//!
//! 定义 `Process` 结构体，封装一个用户进程的所有资源：
//! - PID：进程唯一标识符
//! - 上下文（ForeignContext）：包含用户态寄存器和 satp（页表基地址）
//! - 地址空间（AddressSpace）：独立的 Sv39 虚拟内存页表
//! - 堆管理：heap_bottom 和 program_brk，支持 sbrk 系统调用
//!
//! ## 与第四章的区别
//!
//! | 特性 | 第四章 | 第五章 |
//! |------|--------|--------|
//! | 进程创建 | 仅从 ELF 加载 | 支持 fork 和 exec |
//! | 地址空间 | 创建后不可变 | fork 复制、exec 替换 |
//! | 进程关系 | 无 | 父子关系（PID） |
//!
//! 教程阅读建议：
//!
//! - 先看 `from_elf`：理解“进程初始化”与“程序装载”基础路径；
//! - 再看 `fork`：重点理解地址空间深拷贝与上下文复制；
//! - 最后看 `exec`：对比“保留 PID、替换执行映像”的设计含义。

use crate::{build_flags, map_portal, parse_flags, Sv39, Sv39Manager};
use alloc::alloc::alloc_zeroed;
use core::alloc::Layout;
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
/// 每个进程拥有独立的地址空间和执行上下文。
/// 进程是操作系统管理的基本单位，包含运行程序所需的所有资源。
pub struct Process {
    /// 进程标识符（PID），创建后不可变
    pub pid: ProcId,
    /// 用户态上下文，包含 satp 和通用寄存器
    /// ForeignContext 支持跨地址空间的 Trap 切换（通过异界传送门）
    pub context: ForeignContext,
    /// 进程的独立地址空间（Sv39 页表）
    pub address_space: AddressSpace<Sv39, Sv39Manager>,
    /// 堆底地址（ELF 加载的最高地址的下一页）
    pub heap_bottom: usize,
    /// 当前程序 break 位置（堆顶），通过 sbrk 调整
    pub program_brk: usize,
}

impl Process {
    /// exec 系统调用的核心实现：用新程序替换当前进程
    ///
    /// 替换地址空间和上下文，但保留 PID。
    /// 原有的地址空间会被释放，物理页面被回收。
    pub fn exec(&mut self, elf: ElfFile) {
        let proc = Process::from_elf(elf).unwrap();
        self.address_space = proc.address_space;
        self.context = proc.context;
        self.heap_bottom = proc.heap_bottom;
        self.program_brk = proc.program_brk;
    }

    /// fork 系统调用的核心实现：复制当前进程创建子进程
    ///
    /// 深拷贝父进程的地址空间（包括所有映射的物理页面），
    /// 子进程获得独立的 PID 和地址空间，但初始上下文与父进程相同。
    pub fn fork(&mut self) -> Option<Process> {
        // 分配新的 PID
        let pid = ProcId::new();
        // 复制父进程的完整地址空间（深拷贝所有页表和物理页面数据）
        let parent_addr_space = &self.address_space;
        let mut address_space: AddressSpace<Sv39, Sv39Manager> = AddressSpace::new();
        parent_addr_space.cloneself(&mut address_space);
        // 在子进程地址空间中映射异界传送门
        map_portal(&address_space);
        // 复制父进程的用户态上下文（通用寄存器状态）
        let context = self.context.context.clone();
        // 构建子进程的 satp 值（Mode=Sv39 | 根页表物理页号）
        let satp = (8 << 60) | address_space.root_ppn().val();
        let foreign_ctx = ForeignContext { context, satp };
        Some(Self {
            pid,
            context: foreign_ctx,
            address_space,
            heap_bottom: self.heap_bottom,
            program_brk: self.program_brk,
        })
    }

    /// 从 ELF 文件创建新进程
    ///
    /// 解析流程：
    /// 1. 验证 ELF 头（必须是 RISC-V 64 位可执行文件）
    /// 2. 遍历 LOAD 类型的程序段，映射到新地址空间
    /// 3. 记录最高虚拟地址作为堆底
    /// 4. 分配并映射用户栈（2 页 = 8 KiB）
    /// 5. 映射异界传送门页面
    /// 6. 创建用户态上下文，设置入口地址和栈指针
    pub fn from_elf(elf: ElfFile) -> Option<Self> {
        // 验证 ELF 头：必须是 RISC-V 64 位可执行文件
        let entry = match elf.header.pt2 {
            HeaderPt2::Header64(pt2)
                if pt2.type_.as_type() == header::Type::Executable
                    && pt2.machine.as_machine() == Machine::RISC_V =>
            {
                pt2.entry_point as usize
            }
            _ => None?,
        };

        const PAGE_SIZE: usize = 1 << Sv39::PAGE_BITS; // 4 KiB
        const PAGE_MASK: usize = PAGE_SIZE - 1;

        let mut address_space = AddressSpace::new();
        let mut max_end_va: usize = 0;

        // 遍历 ELF 的所有程序段，只处理 LOAD 类型的段
        for program in elf.program_iter() {
            if !matches!(program.get_type(), Ok(program::Type::Load)) {
                continue;
            }

            let off_file = program.offset() as usize;     // 段在文件中的偏移
            let len_file = program.file_size() as usize;  // 文件中的数据长度
            let off_mem = program.virtual_addr() as usize; // 虚拟地址起始
            let end_mem = off_mem + program.mem_size() as usize; // 虚拟地址结束
            assert_eq!(off_file & PAGE_MASK, off_mem & PAGE_MASK);

            // 记录最高虚拟地址（用于确定堆底位置）
            if end_mem > max_end_va {
                max_end_va = end_mem;
            }

            // 根据 ELF 段的权限标志设置页表项权限
            // U: 用户态可访问（必须设置）
            // X/W/R: 可执行/可写/可读
            // V: 有效位
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
            // 映射段到地址空间，同时复制 ELF 文件中的数据
            address_space.map(
                VAddr::new(off_mem).floor()..VAddr::new(end_mem).ceil(),
                &elf.input[off_file..][..len_file],
                off_mem & PAGE_MASK,
                parse_flags(unsafe { core::str::from_utf8_unchecked(&flags) }).unwrap(),
            );
        }

        // 堆底从 ELF 加载的最高地址的下一页开始
        let heap_bottom = VAddr::<Sv39>::new(max_end_va).ceil().base().val();

        // 映射用户栈：2 页 = 8 KiB，位于虚拟地址空间高位
        // 栈顶为 1 << 38（256 GiB），栈底为 (1<<38) - 8KiB
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

        // 映射异界传送门（与内核地址空间共享同一物理页面）
        map_portal(&address_space);

        // 创建用户态上下文
        let mut context = LocalContext::user(entry);
        // 构建 satp 值：Mode=Sv39(8) | 根页表物理页号
        let satp = (8 << 60) | address_space.root_ppn().val();
        // 设置用户栈指针
        *context.sp_mut() = 1 << 38;

        Some(Self {
            pid: ProcId::new(),
            context: ForeignContext { context, satp },
            address_space,
            heap_bottom,
            program_brk: heap_bottom,
        })
    }

    /// 修改程序 break 位置（实现 sbrk 系统调用）
    ///
    /// - `size > 0`：扩展堆，必要时映射新的物理页面
    /// - `size < 0`：收缩堆，必要时取消映射物理页面
    /// - `size == 0`：返回当前堆顶地址
    /// - 返回旧的 break 地址，失败返回 None
    pub fn change_program_brk(&mut self, size: isize) -> Option<usize> {
        let old_brk = self.program_brk;
        let new_brk = self.program_brk as isize + size;
        // 不允许堆顶低于堆底
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let new_brk = new_brk as usize;

        // 按页对齐计算需要映射/取消映射的页面范围
        let old_brk_ceil = VAddr::<Sv39>::new(old_brk).ceil();
        let new_brk_ceil = VAddr::<Sv39>::new(new_brk).ceil();

        if size > 0 {
            // 扩展堆：映射新页面
            if new_brk_ceil.val() > old_brk_ceil.val() {
                self.address_space
                    .map(old_brk_ceil..new_brk_ceil, &[], 0, build_flags("U_WRV"));
            }
        } else if size < 0 {
            // 收缩堆：取消映射多余页面
            if old_brk_ceil.val() > new_brk_ceil.val() {
                self.address_space.unmap(new_brk_ceil..old_brk_ceil);
            }
        }

        self.program_brk = new_brk;
        Some(old_brk)
    }
}
