mod multislot_portal;

pub use multislot_portal::MultislotPortal;

use crate::{build_sstatus, LocalContext};
#[cfg(target_arch = "riscv64")]
use spin::Lazy;

/// 传送门缓存。
///
/// 映射到公共地址空间，在传送门一次往返期间暂存信息。
#[repr(C)]
pub struct PortalCache {
    a0: usize,       //    (a0) 目标控制流 a0
    a1: usize,       // 1*8(a0) 目标控制流 a1      （寄存，不用初始化）
    satp: usize,     // 2*8(a0) 目标控制流 satp
    sstatus: usize,  // 3*8(a0) 目标控制流 sstatus
    sepc: usize,     // 4*8(a0) 目标控制流 sepc
    stvec: usize,    // 5*8(a0) 当前控制流 stvec   （寄存，不用初始化）
    sscratch: usize, // 6*8(a0) 当前控制流 sscratch（寄存，不用初始化）
}

impl PortalCache {
    /// 初始化传送门缓存。
    #[inline]
    pub fn init(&mut self, satp: usize, pc: usize, a0: usize, supervisor: bool, interrupt: bool) {
        self.satp = satp;
        self.sepc = pc;
        self.a0 = a0;
        self.sstatus = build_sstatus(supervisor, interrupt);
    }

    /// 返回缓存地址。
    #[inline]
    pub fn address(&mut self) -> usize {
        self as *mut _ as _
    }
}

/// 异界传送门。
///
/// 用于将线程传送到另一个地址空间上执行的基础设施。
pub trait ForeignPortal {
    /// 映射到公共地址空间的代码入口。
    ///
    /// # Safety
    ///
    /// 调用者必须确保传送门已正确初始化且映射到公共地址空间。
    unsafe fn transit_entry(&self) -> usize;

    /// 映射到公共地址空间的 `key` 号传送门缓存。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `key` 对应的插槽已分配且有效。
    unsafe fn transit_cache(&mut self, key: impl SlotKey) -> &mut PortalCache;
}

/// 整体式异界传送门。
///
/// 传送门代码和插槽紧挨着放置。这样的传送门对象映射到公共地址空间时应同时具有读、写和执行权限。
pub trait MonoForeignPortal {
    /// 传送门对象的总字节数。
    fn total_size(&self) -> usize;

    /// 传送门对象在公共地址空间上的地址。
    fn transit_address(&self) -> usize;

    /// 传送门代码在对象中的偏移。
    fn text_offset(&self) -> usize;

    /// `key` 号插槽在传送门对象中的偏移。
    fn cache_offset(&self, key: usize) -> usize;
}

impl<T: MonoForeignPortal> ForeignPortal for T {
    #[inline]
    unsafe fn transit_entry(&self) -> usize {
        // SAFETY: 由 MonoForeignPortal 的实现者保证 transit_address 和 text_offset 的正确性
        self.transit_address() + self.text_offset()
    }

    #[inline]
    unsafe fn transit_cache(&mut self, key: impl SlotKey) -> &mut PortalCache {
        // SAFETY: 由调用者保证 key 对应的插槽已分配，
        // cache_offset 返回的偏移量指向有效的 PortalCache 结构
        &mut *((self.transit_address() + self.cache_offset(key.index())) as *mut _)
    }
}

/// 异界线程上下文。
///
/// 不在当前地址空间的线程。
pub struct ForeignContext {
    /// 目标地址空间上的线程上下文。
    pub context: LocalContext,
    /// 目标地址空间。
    pub satp: usize,
}

impl ForeignContext {
    /// 执行异界线程。
    ///
    /// # Safety
    ///
    /// 调用者必须确保：
    /// - `portal` 已正确初始化且映射到公共地址空间
    /// - `key` 对应的插槽已分配
    /// - `self.satp` 指向有效的页表
    /// - `self.context` 中的 `sepc` 指向有效的代码地址
    pub unsafe fn execute(&mut self, portal: &mut impl ForeignPortal, key: impl SlotKey) -> usize {
        use core::mem::replace;
        // 异界传送门需要特权态执行
        let supervisor = replace(&mut self.context.supervisor, true);
        // 异界传送门不能打开中断
        let interrupt = replace(&mut self.context.interrupt, false);
        // 找到公共空间上的缓存
        let entry = portal.transit_entry();
        let cache = portal.transit_cache(key);
        // 重置传送门上下文
        cache.init(
            self.satp,
            self.context.sepc,
            self.context.a(0),
            supervisor,
            interrupt,
        );
        // 执行传送门代码
        *self.context.pc_mut() = entry;
        *self.context.a_mut(0) = cache.address();
        let sstatus = self.context.execute();
        // 恢复线程属性
        self.context.supervisor = supervisor;
        self.context.interrupt = interrupt;
        // 从传送门读取上下文
        *self.context.a_mut(0) = cache.a0;
        sstatus
    }
}

/// 插槽选项。
pub trait SlotKey {
    /// 转化为插槽序号。
    fn index(self) -> usize;
}

impl SlotKey for () {
    #[inline]
    fn index(self) -> usize {
        0
    }
}

impl SlotKey for usize {
    #[inline]
    fn index(self) -> usize {
        self
    }
}

/// 从 `tp` 寄存器读取一个序号。
pub struct TpReg;

impl SlotKey for TpReg {
    #[inline]
    fn index(self) -> usize {
        #[cfg(target_arch = "riscv64")]
        {
            let ans: usize;
            // SAFETY: 只是读取 tp 寄存器的值，不会产生副作用
            unsafe { core::arch::asm!("mv {}, tp", out(reg) ans) };
            ans
        }
        #[cfg(not(target_arch = "riscv64"))]
        unimplemented!("TpReg::index() is only supported on riscv64")
    }
}

/// 传送门代码
#[cfg(target_arch = "riscv64")]
struct PortalText(&'static [u16]);

/// 定位传送门代码段。
///
/// 通过寻找结尾的 `jr a0` 和 `options(noreturn)`，在运行时定位传送门工作的裸函数代码段。
/// 不必在链接时决定代码位置，可以在运行时将这段代码加载到任意位置。
#[cfg(target_arch = "riscv64")]
static PORTAL_TEXT: Lazy<PortalText> = Lazy::new(PortalText::new);

#[cfg(target_arch = "riscv64")]
impl PortalText {
    pub fn new() -> Self {
        // 32 是一个任取的不可能的下限
        for len in 32.. {
            // SAFETY: foreign_execute 是一个有效的函数指针，
            // 我们通过查找结尾标记 [0x8502, 0] 来确定代码段的实际长度
            let slice = unsafe { core::slice::from_raw_parts(foreign_execute as *const _, len) };
            // 裸函数的 `options(noreturn)` 会在结尾生成一个 0 指令，这是一个 unstable 特性所以不一定可靠
            if slice.ends_with(&[0x8502, 0]) {
                return Self(slice);
            }
        }
        unreachable!()
    }

    #[inline]
    pub fn aligned_size(&self) -> usize {
        const USIZE_MASK: usize = core::mem::size_of::<usize>() - 1;
        (self.0.len() * core::mem::size_of::<u16>() + USIZE_MASK) & !USIZE_MASK
    }

    /// 将传送门代码拷贝到指定地址。
    ///
    /// # Safety
    ///
    /// 调用者必须确保 `address` 指向的内存区域：
    /// - 已分配且可写
    /// - 大小至少为 `aligned_size()` 字节
    /// - 与源数据不重叠
    #[inline]
    pub unsafe fn copy_to(&self, address: usize) {
        // SAFETY: 由调用者保证目标地址有效且不重叠
        (address as *mut u16).copy_from_nonoverlapping(self.0.as_ptr(), self.0.len());
    }
}

/// 切换地址空间然后 sret。
/// 地址空间恢复后一切都会恢复原状。
///
/// # Safety
///
/// 这是一个裸函数，只能由 `ForeignContext::execute()` 通过 `LocalContext::execute()` 间接调用。
/// 调用前必须确保：
/// - `ctx` 指向有效的 `PortalCache` 结构
/// - `PortalCache` 中的 `satp`、`sepc`、`sstatus` 已正确初始化
/// - 此函数的代码已被拷贝到公共地址空间
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
unsafe extern "C" fn foreign_execute(ctx: *mut PortalCache) {
    core::arch::naked_asm!(
        // 位置无关加载
        "   .option push
            .option nopic
        ",
        // 保存 ra，ra 会用来寄存
        "   sd    a1, 1*8(a0)",
        // 交换地址空间
        "   ld    a1, 2*8(a0)
            csrrw a1, satp, a1
            sfence.vma
            sd    a1, 2*8(a0)
        ",
        // 加载 sstatus
        "   ld    a1, 3*8(a0)
            csrw      sstatus, a1
        ",
        // 加载 sepc
        "   ld    a1, 4*8(a0)
            csrw      sepc, a1
        ",
        // 交换陷入入口
        "   la    a1, 1f
            csrrw a1, stvec, a1
            sd    a1, 5*8(a0)
        ",
        // 交换 sscratch
        "   csrrw a1, sscratch, a0
            sd    a1, 6*8(a0)
        ",
        // 加载通用寄存器
        "   ld    a1, 1*8(a0)
            ld    a0,    (a0)
        ",
        // 出发！
        "   sret",
        // 陷入
        "   .align 2",
        // 加载 a0
        "1: csrrw a0, sscratch, a0",
        // 保存 ra，ra 会用来寄存
        "   sd    a1, 1*8(a0)",
        // 交换 sscratch 并保存 a0
        "   ld    a1, 6*8(a0)
            csrrw a1, sscratch, a1
            sd    a1,    (a0)
        ",
        // 恢复地址空间
        "   ld    a1, 2*8(a0)
            csrrw a1, satp, a1
            sfence.vma
            sd    a1, 2*8(a0)
        ",
        // 恢复通用寄存器
        "   ld    a1, 1*8(a0)",
        // 恢复陷入入口
        "   ld    a0, 5*8(a0)
            csrw      stvec, a0
        ",
        // 回家！
        // 离开异界传送门直接跳到正常上下文的 stvec
        "   jr    a0",
        // 显式添加结束标记（c.unimp = 0），确保代码以 [0x8502, 0] 序列结尾
        // 这是因为新版 Rust 的 naked 函数不一定会自动在结尾生成 unimp 指令
        "   .half 0",
        "   .option pop",
    )
}
