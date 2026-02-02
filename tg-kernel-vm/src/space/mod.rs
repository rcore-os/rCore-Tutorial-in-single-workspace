mod mapper;
mod visitor;

extern crate alloc;

use crate::PageManager;
use alloc::vec::Vec;
use core::{fmt, ops::Range, ptr::NonNull};
use mapper::Mapper;
use page_table::{PageTable, PageTableFormatter, Pos, VAddr, VmFlags, VmMeta, PPN, VPN};
use visitor::Visitor;

/// 地址空间。
pub struct AddressSpace<Meta: VmMeta, M: PageManager<Meta>> {
    /// 虚拟地址块
    pub areas: Vec<Range<VPN<Meta>>>,
    page_manager: M,
}

impl<Meta: VmMeta, M: PageManager<Meta>> AddressSpace<Meta, M> {
    /// 创建新地址空间。
    #[inline]
    pub fn new() -> Self {
        Self {
            areas: Vec::new(),
            page_manager: M::new_root(),
        }
    }

    /// 地址空间根页表的物理页号。
    #[inline]
    pub fn root_ppn(&self) -> PPN<Meta> {
        self.page_manager.root_ppn()
    }

    /// 地址空间根页表
    #[inline]
    pub fn root(&self) -> PageTable<Meta> {
        // SAFETY: page_manager.root_ptr() 返回的是有效的根页表指针，
        // 由 PageManager::new_root() 创建时保证其有效性
        unsafe { PageTable::from_root(self.page_manager.root_ptr()) }
    }

    /// 向地址空间增加映射关系。
    pub fn map_extern(&mut self, range: Range<VPN<Meta>>, pbase: PPN<Meta>, flags: VmFlags<Meta>) {
        self.areas.push(range.start..range.end);
        let count = range.end.val() - range.start.val();
        let mut root = self.root();
        let mut mapper = Mapper::new(self, pbase..pbase + count, flags);
        root.walk_mut(Pos::new(range.start, 0), &mut mapper);
        if !mapper.ans() {
            // 映射失败，需要回滚吗？
            todo!()
        }
    }

    /// 分配新的物理页，拷贝数据并建立映射。
    pub fn map(
        &mut self,
        range: Range<VPN<Meta>>,
        data: &[u8],
        offset: usize,
        mut flags: VmFlags<Meta>,
    ) {
        let count = range.end.val() - range.start.val();
        let size = count << Meta::PAGE_BITS;
        assert!(size >= data.len() + offset);
        let page = self.page_manager.allocate(count, &mut flags);
        // SAFETY: page 是刚分配的有效内存，大小为 size 字节。
        // 我们按顺序填充：[0, offset) 清零，[offset, offset+data.len()) 拷贝数据，
        // [offset+data.len(), size) 清零。
        unsafe {
            use core::slice::from_raw_parts_mut as slice;
            let mut ptr = page.as_ptr();
            slice(ptr, offset).fill(0);
            ptr = ptr.add(offset);
            slice(ptr, data.len()).copy_from_slice(data);
            ptr = ptr.add(data.len());
            slice(ptr, page.as_ptr().add(size).offset_from(ptr) as _).fill(0);
        }
        self.map_extern(range, self.page_manager.v_to_p(page), flags)
    }

    /// 取消指定 VPN 范围的映射
    pub fn unmap(&mut self, range: Range<VPN<Meta>>) {
        // 从 areas 中移除该范围（可能需要拆分现有区域）
        let mut new_areas = Vec::new();
        for area in self.areas.drain(..) {
            if area.end <= range.start || area.start >= range.end {
                // 不重叠，保留原区域
                new_areas.push(area);
            } else {
                // 有重叠，需要拆分
                if area.start < range.start {
                    new_areas.push(area.start..range.start);
                }
                if area.end > range.end {
                    new_areas.push(range.end..area.end);
                }
            }
        }
        self.areas = new_areas;

        // 清除页表项（将 PTE 设为无效，即写入 0）
        let mut vpn = range.start;
        while vpn < range.end {
            // 使用 visitor 找到 PTE 并清除
            if let Some(pte_ptr) = self.find_pte_mut(vpn) {
                unsafe {
                    core::ptr::write_bytes(
                        pte_ptr as *mut u8,
                        0,
                        core::mem::size_of::<page_table::Pte<Meta>>(),
                    )
                };
            }
            vpn = vpn + 1;
        }

        // 刷新地址空间
        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!("sfence.vma")
        };
    }

    /// 查找指定 VPN 的 PTE 指针（用于修改）
    fn find_pte_mut(&self, vpn: VPN<Meta>) -> Option<*mut page_table::Pte<Meta>> {
        let mut current = self.page_manager.root_ptr();

        for level in (0..=Meta::MAX_LEVEL).rev() {
            let idx = vpn.index_in(level);
            let pte_ptr = unsafe { current.as_ptr().add(idx) };
            let pte = unsafe { *pte_ptr };

            if level == 0 {
                return Some(pte_ptr);
            }

            if !pte.is_valid() {
                return None;
            }

            // 如果是叶子节点（大页），也返回
            // 检查 R 或 X 位来判断是否是叶子节点
            let flags_raw = pte.flags().val();
            let is_leaf = (flags_raw & 0b1010) != 0; // R=bit1, X=bit3
            if is_leaf {
                return Some(pte_ptr);
            }

            current = self.page_manager.p_to_v(pte.ppn());
        }
        None
    }

    /// 检查 `flags` 的属性要求，然后将地址空间中的一个虚地址翻译成当前地址空间中的指针。
    pub fn translate<T>(&self, addr: VAddr<Meta>, flags: VmFlags<Meta>) -> Option<NonNull<T>> {
        let mut visitor = Visitor::new(self);
        self.root().walk(Pos::new(addr.floor(), 0), &mut visitor);
        visitor
            .ans()
            .filter(|pte| pte.flags().contains(flags))
            .map(|pte| {
                // SAFETY: pte 是有效的页表项，ppn 对应有效的物理页。
                // p_to_v 返回当前地址空间中的有效指针。
                // add(addr.offset()) 计算页内偏移，不会越界（offset < PAGE_SIZE）。
                // 使用 new_unchecked 是因为 p_to_v 返回的是 NonNull，不可能为空。
                unsafe {
                    NonNull::new_unchecked(
                        self.page_manager
                            .p_to_v::<u8>(pte.ppn())
                            .as_ptr()
                            .add(addr.offset())
                            .cast(),
                    )
                }
            })
    }

    /// 遍历地址空间，将其中的地址映射添加进自己的地址空间中，重新分配物理页并拷贝所有数据及代码
    pub fn cloneself(&self, new_addrspace: &mut AddressSpace<Meta, M>) {
        let root = self.root();
        let areas = &self.areas;
        for (_, range) in areas.iter().enumerate() {
            let mut visitor = Visitor::new(self);
            // 虚拟地址块的首地址的 vpn
            let vpn = range.start;
            // 利用 visitor 访问页表，并获取这个虚拟地址块的页属性
            root.walk(Pos::new(vpn, 0), &mut visitor);
            // 利用 visitor 获取这个虚拟地址块的页属性，以及起始地址
            let (mut flags, mut data_ptr) = visitor
                .ans()
                .filter(|pte| pte.is_valid())
                .map(|pte| {
                    // SAFETY: pte 是有效的页表项，p_to_v 返回有效的指针
                    (pte.flags(), unsafe {
                        NonNull::new_unchecked(self.page_manager.p_to_v::<u8>(pte.ppn()).as_ptr())
                    })
                })
                .unwrap();
            let vpn_range = range.start..range.end;
            // 虚拟地址块中页数量
            let count = range.end.val() - range.start.val();
            let size = count << Meta::PAGE_BITS;
            // 分配 count 个 flags 属性的物理页面
            let paddr = new_addrspace.page_manager.allocate(count, &mut flags);
            let ppn = new_addrspace.page_manager.v_to_p(paddr);
            // SAFETY: data_ptr 指向源地址空间中 size 字节的有效数据，
            // paddr 指向新分配的 size 字节内存，两者不重叠
            unsafe {
                use core::slice::from_raw_parts_mut as slice;
                let data = slice(data_ptr.as_mut(), size);
                let ptr = paddr.as_ptr();
                slice(ptr, size).copy_from_slice(data);
            }
            new_addrspace.map_extern(vpn_range, ppn, flags);
        }
    }
}

impl<Meta: VmMeta, P: PageManager<Meta>> fmt::Debug for AddressSpace<Meta, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "root: {:#x}", self.root_ppn().val())?;
        write!(
            f,
            "{:?}",
            PageTableFormatter {
                pt: self.root(),
                f: |ppn| self.page_manager.p_to_v(ppn)
            }
        )
    }
}
