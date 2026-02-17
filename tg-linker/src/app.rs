/// 应用程序元数据。
///
/// 链接器会把“应用起始地址表 + 应用数据”打包到镜像中，
/// `AppMeta` 是运行期读取这些信息的入口结构。
#[repr(C)]
pub struct AppMeta {
    base: u64,
    step: u64,
    count: u64,
    first: u64,
}

impl AppMeta {
    /// 定位应用程序元数据。
    ///
    /// 返回由链接脚本定义的应用程序元数据的静态引用。
    #[inline]
    pub fn locate() -> &'static Self {
        unsafe extern "C" {
            static apps: AppMeta;
        }
        // SAFETY: `apps` 是由链接脚本定义的静态符号，在程序运行期间始终有效。
        // 它的内存布局与 AppMeta 结构体匹配（由 #[repr(C)] 保证）。
        unsafe { &apps }
    }

    /// 遍历链接进来的应用程序。
    #[inline]
    pub fn iter(&'static self) -> AppIterator {
        // 迭代器按链接顺序输出每个 app 的字节切片。
        AppIterator { meta: self, i: 0 }
    }
}

/// 应用程序迭代器。
pub struct AppIterator {
    meta: &'static AppMeta,
    i: u64,
}

impl Iterator for AppIterator {
    type Item = &'static [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.meta.count {
            None
        } else {
            let i = self.i as usize;
            self.i += 1;
            // SAFETY: 以下操作基于链接脚本定义的应用程序布局：
            // - `first` 之后紧跟着 count+1 个 usize，存储每个应用的位置和结尾
            // - 如果 base != 0，则将应用拷贝到指定的基地址
            // - 应用数据由链接器嵌入，在程序运行期间始终有效
            unsafe {
                let slice = core::slice::from_raw_parts(
                    &self.meta.first as *const _ as *const usize,
                    (self.meta.count + 1) as _,
                );
                let pos = slice[i];
                let size = slice[i + 1] - pos;
                let base = self.meta.base as usize + i * self.meta.step as usize;
                if base != 0 {
                    // 章节中常用该模式把 app 拷贝到固定虚拟地址（如 0x8040_0000）再执行。
                    // SAFETY: pos 指向有效的应用数据，base 是调用者指定的目标地址，
                    // 调用者负责确保 base 处有足够的内存空间
                    core::ptr::copy_nonoverlapping::<u8>(pos as _, base as _, size);
                    // 将目标区域剩余部分清零
                    core::slice::from_raw_parts_mut(base as *mut u8, 0x20_0000)[size..].fill(0);
                    Some(core::slice::from_raw_parts(base as _, size))
                } else {
                    Some(core::slice::from_raw_parts(pos as _, size))
                }
            }
        }
    }
}
