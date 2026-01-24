//! 这个板块为内核提供链接脚本的文本，以及依赖于定制链接脚本的功能。
//!
//! build.rs 文件可依赖此板块，并将 [`SCRIPT`] 文本常量写入链接脚本文件：
//!
//! ```rust
//! use std::{env, fs, path::PathBuf};
//!
//! let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
//! fs::write(ld, linker::SCRIPT).unwrap();
//!
//! println!("cargo:rerun-if-changed=build.rs");
//! println!("cargo:rustc-link-arg=-T{}", ld.display());
//! ```
//!
//! 内核使用 [`boot0`] 宏定义内核启动栈和高级语言入口：
//!
//! ```rust
//! linker::boot0!(rust_main; stack = 4 * 4096);
//! ```
//!
//! 内核所在内核区域定义成 4 个部分（[`KernelRegionTitle`]）:
//!
//! 1. 代码段
//! 2. 只读数据段
//! 3. 数据段
//! 4. 启动数据段
//!
//! 启动数据段放在最后，以便启动完成后换栈。届时可放弃启动数据段，将其加入动态内存区。
//!
//! 用 [`KernelLayout`] 结构体定位、保存和访问内核内存布局。

#![no_std]
#![deny(warnings, missing_docs)]

mod app;

pub use app::{AppIterator, AppMeta};

/// 链接脚本。
pub const SCRIPT: &[u8] = b"\
OUTPUT_ARCH(riscv)
SECTIONS {
    .text 0x80200000 : {
        __start = .;
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : ALIGN(4K) {
        __rodata = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : ALIGN(4K) {
        __data = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : ALIGN(8) {
        __sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        __ebss = .;
    }
    .boot : ALIGN(4K) {
        __boot = .;
        KEEP(*(.boot.stack))
    }
    __end = .;
}";

/// 链接脚本（nobios 模式）。
///
/// M-Mode 入口在 0x80000000，S-Mode 内核在 0x80200000。
pub const NOBIOS_SCRIPT: &[u8] = b"\
OUTPUT_ARCH(riscv)
ENTRY(_m_start)
M_BASE_ADDRESS = 0x80000000;
S_BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = M_BASE_ADDRESS;

    .text.m_entry : {
        *(.text.m_entry)
    }

    .text.m_trap : {
        *(.text.m_trap)
    }

    .bss.m_stack : {
        *(.bss.m_stack)
    }

    .bss.m_data : {
        *(.bss.m_data)
    }

    . = S_BASE_ADDRESS;

    .text : {
        __start = .;
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : ALIGN(4K) {
        __rodata = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : ALIGN(4K) {
        __data = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : ALIGN(8) {
        __sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        __ebss = .;
    }
    .boot : ALIGN(4K) {
        __boot = .;
        KEEP(*(.boot.stack))
    }
    __end = .;
}";

/// 定义内核入口。
///
/// 将设置一个启动栈，并在启动栈上调用高级语言入口。
///
/// # Safety
///
/// 此宏生成的 `_start` 函数是一个裸函数，作为内核的入口点。
/// 它会：
/// - 设置栈指针到 `__end`（由链接脚本定义）
/// - 跳转到指定的入口函数
///
/// 调用者需要确保链接脚本正确定义了相关符号。
#[macro_export]
macro_rules! boot0 {
    ($entry:ident; stack = $stack:expr) => {
        /// 内核入口点。
        ///
        /// # Safety
        ///
        /// 这是一个裸函数，由 bootloader 直接调用。
        /// 调用时 CPU 处于 M 模式或 S 模式，需要正确设置栈指针后才能执行 Rust 代码。
        #[cfg(target_arch = "riscv64")]
        #[unsafe(naked)]
        #[no_mangle]
        #[link_section = ".text.entry"]
        unsafe extern "C" fn _start() -> ! {
            #[link_section = ".boot.stack"]
            static mut STACK: [u8; $stack] = [0u8; $stack];

            // SAFETY: 设置栈指针并跳转到高级语言入口。
            // __end 由链接脚本定义，指向启动栈的末尾。
            core::arch::naked_asm!(
                "la sp, __end",
                "j  {main}",
                main = sym rust_main,
            )
        }

        #[cfg(not(target_arch = "riscv64"))]
        #[no_mangle]
        unsafe extern "C" fn _start() -> ! {
            unimplemented!("_start() is only supported on riscv64")
        }
    };
}

/// 内核地址信息。
#[derive(Debug)]
pub struct KernelLayout {
    text: usize,
    rodata: usize,
    data: usize,
    sbss: usize,
    ebss: usize,
    boot: usize,
    end: usize,
}

impl KernelLayout {
    /// 非零初始化，避免 bss。
    pub const INIT: Self = Self {
        text: usize::MAX,
        rodata: usize::MAX,
        data: usize::MAX,
        sbss: usize::MAX,
        ebss: usize::MAX,
        boot: usize::MAX,
        end: usize::MAX,
    };

    /// 定位内核布局。
    #[inline]
    pub fn locate() -> Self {
        extern "C" {
            fn __start();
            fn __rodata();
            fn __data();
            fn __sbss();
            fn __ebss();
            fn __boot();
            fn __end();
        }

        Self {
            text: __start as _,
            rodata: __rodata as _,
            data: __data as _,
            sbss: __sbss as _,
            ebss: __ebss as _,
            boot: __boot as _,
            end: __end as _,
        }
    }

    /// 内核起始地址。
    #[inline]
    pub const fn start(&self) -> usize {
        self.text
    }

    /// 内核结尾地址。
    #[inline]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// 内核静态二进制长度。
    #[inline]
    pub const fn len(&self) -> usize {
        self.end - self.text
    }

    /// 清零 .bss 段。
    ///
    /// # Safety
    ///
    /// 调用者必须确保：
    /// - 此函数在访问任何 .bss 段中的静态变量之前调用
    /// - .bss 段的地址范围（`sbss` 到 `ebss`）是有效的
    /// - 此函数只被调用一次
    #[inline]
    pub unsafe fn zero_bss(&self) {
        let mut ptr = self.sbss as *mut u8;
        let end = self.ebss as *mut u8;
        while ptr < end {
            // SAFETY: ptr 在 [sbss, ebss) 范围内，这是有效的 .bss 段内存。
            // 使用 volatile write 确保多核场景下其他核能看到写入。
            ptr.write_volatile(0);
            ptr = ptr.offset(1);
        }
    }

    /// 内核区段迭代器。
    #[inline]
    pub fn iter(&self) -> KernelRegionIterator<'_> {
        KernelRegionIterator {
            layout: self,
            next: Some(KernelRegionTitle::Text),
        }
    }
}

use core::{fmt, ops::Range};

/// 内核内存分区迭代器。
pub struct KernelRegionIterator<'a> {
    layout: &'a KernelLayout,
    next: Option<KernelRegionTitle>,
}

/// 内核内存分区名称。
#[derive(Clone, Copy)]
pub enum KernelRegionTitle {
    /// 代码段。
    Text,
    /// 只读数据段。
    Rodata,
    /// 数据段。
    Data,
    /// 启动数据段。
    Boot,
}

/// 内核内存分区。
pub struct KernelRegion {
    /// 分区名称。
    pub title: KernelRegionTitle,
    /// 分区地址范围。
    pub range: Range<usize>,
}

impl fmt::Display for KernelRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.title {
            KernelRegionTitle::Text => write!(f, ".text ----> ")?,
            KernelRegionTitle::Rodata => write!(f, ".rodata --> ")?,
            KernelRegionTitle::Data => write!(f, ".data ----> ")?,
            KernelRegionTitle::Boot => write!(f, ".boot ----> ")?,
        }
        write!(f, "{:#10x}..{:#10x}", self.range.start, self.range.end)
    }
}

impl Iterator for KernelRegionIterator<'_> {
    type Item = KernelRegion;

    fn next(&mut self) -> Option<Self::Item> {
        use KernelRegionTitle::*;
        match self.next? {
            Text => {
                self.next = Some(Rodata);
                Some(KernelRegion {
                    title: Text,
                    range: self.layout.text..self.layout.rodata,
                })
            }
            Rodata => {
                self.next = Some(Data);
                Some(KernelRegion {
                    title: Rodata,
                    range: self.layout.rodata..self.layout.data,
                })
            }
            Data => {
                self.next = Some(Boot);
                Some(KernelRegion {
                    title: Data,
                    range: self.layout.data..self.layout.ebss,
                })
            }
            Boot => {
                self.next = None;
                Some(KernelRegion {
                    title: Boot,
                    range: self.layout.boot..self.layout.end,
                })
            }
        }
    }
}
