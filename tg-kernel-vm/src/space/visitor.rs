use crate::{AddressSpace, PageManager};
use core::ptr::NonNull;
use page_table::{Pos, Pte, VmMeta};

pub(super) struct Visitor<'a, Meta: VmMeta, M: PageManager<Meta>> {
    space: &'a AddressSpace<Meta, M>,
    ans: Option<Pte<Meta>>,
}

impl<'a, Meta: VmMeta, M: PageManager<Meta>> Visitor<'a, Meta, M> {
    #[inline]
    pub const fn new(space: &'a AddressSpace<Meta, M>) -> Self {
        Self { space, ans: None }
    }

    #[inline]
    pub const fn ans(self) -> Option<Pte<Meta>> {
        self.ans
    }
}

impl<'a, Meta: VmMeta, M: PageManager<Meta>> page_table::Visitor<Meta> for Visitor<'a, Meta, M> {
    #[inline]
    fn arrive(&mut self, pte: Pte<Meta>, _target_hint: Pos<Meta>) -> Pos<Meta> {
        // arrive 表示已走到目标层；若有效则记录结果并停止遍历。
        if pte.is_valid() {
            self.ans = Some(pte);
        }
        Pos::stop()
    }

    #[inline]
    fn meet(
        &mut self,
        _level: usize,
        pte: Pte<Meta>,
        _target_hint: Pos<Meta>,
    ) -> Option<NonNull<Pte<Meta>>> {
        // meet 表示遇到中间页表项，返回下一层页表地址继续 walk。
        Some(self.space.page_manager.p_to_v(pte.ppn()))
    }

    #[inline]
    fn block(&mut self, _level: usize, _pte: Pte<Meta>, _target: Pos<Meta>) -> Pos<Meta> {
        Pos::stop()
    }
}
