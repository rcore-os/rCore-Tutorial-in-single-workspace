#![no_std]
#![no_main]

mod process;
mod processor;

#[macro_use]
extern crate rcore_console;

extern crate alloc;

use alloc::{alloc::alloc, collections::BTreeMap};
use core::{alloc::Layout, cell::UnsafeCell, ffi::CStr, mem::MaybeUninit};
use impls::{Console, Sv39Manager, SyscallContext};
use kernel_context::foreign::MultislotPortal;
use kernel_vm::{
    page_table::{MmuMeta, Sv39, VAddr, VmFlags, VmMeta, PPN, VPN},
    AddressSpace,
};
use process::Process;
use processor::{ProcManager, PROCESSOR};
use rcore_console::log;
use rcore_task_manage::{PManager, ProcId};
use riscv::register::*;
use sbi;
use spin::Lazy;
use syscall::Caller;
use xmas_elf::ElfFile;

// 应用程序内联进来。
core::arch::global_asm!(include_str!(env!("APP_ASM")));
// 定义内核入口。
linker::boot0!(rust_main; stack = 32 * 4096);
// 物理内存容量 = 48 MiB。
const MEMORY: usize = 48 << 20;
// 传送门所在虚页。
const PROTAL_TRANSIT: VPN<Sv39> = VPN::MAX;
// 内核地址空间。
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

    unsafe fn write(&self, space: AddressSpace<Sv39, Sv39Manager>) {
        *self.inner.get() = MaybeUninit::new(space);
    }

    unsafe fn assume_init_ref(&self) -> &AddressSpace<Sv39, Sv39Manager> {
        &*(*self.inner.get()).as_ptr()
    }
}

static KERNEL_SPACE: KernelSpace = KernelSpace::new();
/// 加载用户进程。
static APPS: Lazy<BTreeMap<&'static str, &'static [u8]>> = Lazy::new(|| {
    extern "C" {
        static app_names: u8;
    }
    unsafe {
        linker::AppMeta::locate()
            .iter()
            .scan(&app_names as *const _ as usize, |addr, data| {
                let name = CStr::from_ptr(*addr as _).to_str().unwrap();
                *addr += name.as_bytes().len() + 1;
                Some((name, data))
            })
    }
    .collect()
});

extern "C" fn rust_main() -> ! {
    let layout = linker::KernelLayout::locate();
    // bss 段清零
    unsafe { layout.zero_bss() };
    // 初始化 `console`
    rcore_console::init_console(&Console);
    rcore_console::set_log_level(option_env!("LOG"));
    rcore_console::test_log();
    // 初始化内核堆
    kernel_alloc::init(layout.start() as _);
    unsafe {
        kernel_alloc::transfer(core::slice::from_raw_parts_mut(
            layout.end() as _,
            MEMORY - layout.len(),
        ))
    };
    // 建立异界传送门
    let portal_size = MultislotPortal::calculate_size(1);
    let portal_layout = Layout::from_size_align(portal_size, 1 << Sv39::PAGE_BITS).unwrap();
    let portal_ptr = unsafe { alloc(portal_layout) };
    assert!(portal_layout.size() < 1 << Sv39::PAGE_BITS);
    // 建立内核地址空间
    kernel_space(layout, MEMORY, portal_ptr as _);
    // 初始化异界传送门
    let portal = unsafe { MultislotPortal::init_transit(PROTAL_TRANSIT.base().val(), 1) };
    // 初始化 syscall
    syscall::init_io(&SyscallContext);
    syscall::init_process(&SyscallContext);
    syscall::init_scheduling(&SyscallContext);
    syscall::init_clock(&SyscallContext);
    syscall::init_memory(&SyscallContext);
    // 加载初始进程
    let initproc_data = APPS.get("initproc").unwrap();
    if let Some(process) = Process::from_elf(ElfFile::new(initproc_data).unwrap()) {
        PROCESSOR.get_mut().set_manager(ProcManager::new());
        PROCESSOR
            .get_mut()
            .add(process.pid, process, ProcId::from_usize(usize::MAX));
    }
    loop {
        let processor: *mut PManager<Process, ProcManager> = PROCESSOR.get_mut() as *mut _;
        if let Some(task) = unsafe { (*processor).find_next() } {
            unsafe { task.context.execute(portal, ()) };
            match scause::read().cause() {
                scause::Trap::Exception(scause::Exception::UserEnvCall) => {
                    use syscall::{SyscallId as Id, SyscallResult as Ret};
                    let ctx = &mut task.context.context;
                    ctx.move_next();
                    let id: Id = ctx.a(7).into();
                    let args = [ctx.a(0), ctx.a(1), ctx.a(2), ctx.a(3), ctx.a(4), ctx.a(5)];
                    match syscall::handle(Caller { entity: 0, flow: 0 }, id, args) {
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
    sbi::shutdown(false)
}

/// Rust 异常处理函数，以异常方式关机。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    sbi::shutdown(true)
}

fn kernel_space(layout: linker::KernelLayout, memory: usize, portal: usize) {
    let mut space = AddressSpace::new();
    for region in layout.iter() {
        log::info!("{region}");
        use linker::KernelRegionTitle::*;
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
            VmFlags::build_from_str(flags),
        )
    }
    let s = VAddr::<Sv39>::new(layout.end());
    let e = VAddr::<Sv39>::new(layout.start() + memory);
    log::info!("(heap) ---> {:#10x}..{:#10x}", s.val(), e.val());
    space.map_extern(
        s.floor()..e.ceil(),
        PPN::new(s.floor().val()),
        VmFlags::build_from_str("_WRV"),
    );
    space.map_extern(
        PROTAL_TRANSIT..PROTAL_TRANSIT + 1,
        PPN::new(portal >> Sv39::PAGE_BITS),
        VmFlags::build_from_str("__G_XWRV"),
    );
    println!();
    unsafe { satp::set(satp::Mode::Sv39, 0, space.root_ppn().val()) };
    unsafe { KERNEL_SPACE.write(space) };
}

/// 映射异界传送门。
fn map_portal(space: &AddressSpace<Sv39, Sv39Manager>) {
    let portal_idx = PROTAL_TRANSIT.index_in(Sv39::MAX_LEVEL);
    space.root()[portal_idx] = unsafe { KERNEL_SPACE.assume_init_ref() }.root()[portal_idx];
}

/// 各种接口库的实现。
mod impls {
    use crate::{process::Process as ProcStruct, processor::ProcManager, APPS, PROCESSOR};
    use alloc::alloc::alloc_zeroed;
    use core::{alloc::Layout, ptr::NonNull};
    use kernel_vm::{
        page_table::{MmuMeta, Pte, Sv39, VAddr, VmFlags, PPN, VPN},
        PageManager,
    };
    use rcore_console::log;
    use rcore_task_manage::{PManager, ProcId};
    use syscall::*;
    use xmas_elf::ElfFile;

    #[repr(transparent)]
    pub struct Sv39Manager(NonNull<Pte<Sv39>>);

    impl Sv39Manager {
        const OWNED: VmFlags<Sv39> = unsafe { VmFlags::from_raw(1 << 8) };

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

    pub struct Console;

    impl rcore_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            sbi::console_putchar(c);
        }
    }

    pub struct SyscallContext;

    impl IO for SyscallContext {
        fn write(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
            match fd {
                STDOUT | STDDEBUG => {
                    const READABLE: VmFlags<Sv39> = VmFlags::build_from_str("RV");
                    if let Some(ptr) = PROCESSOR
                        .get_mut()
                        .current()
                        .unwrap()
                        .address_space
                        .translate(VAddr::new(buf), READABLE)
                    {
                        print!("{}", unsafe {
                            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                                ptr.as_ptr(),
                                count,
                            ))
                        });
                        count as _
                    } else {
                        log::error!("ptr not readable");
                        -1
                    }
                }
                _ => {
                    log::error!("unsupported fd: {fd}");
                    -1
                }
            }
        }

        #[inline]
        fn read(&self, _caller: Caller, fd: usize, buf: usize, count: usize) -> isize {
            if fd == STDIN {
                const WRITEABLE: VmFlags<Sv39> = VmFlags::build_from_str("W_V");
                if let Some(mut ptr) = PROCESSOR
                    .get_mut()
                    .current()
                    .unwrap()
                    .address_space
                    .translate(VAddr::new(buf), WRITEABLE)
                {
                    let mut ptr = unsafe { ptr.as_mut() } as *mut u8;
                    for _ in 0..count {
                        let c = sbi::console_getchar() as u8;
                        unsafe {
                            *ptr = c;
                            ptr = ptr.add(1);
                        }
                    }
                    count as _
                } else {
                    log::error!("ptr not writeable");
                    -1
                }
            } else {
                log::error!("unsupported fd: {fd}");
                -1
            }
        }
    }

    impl Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: Caller, exit_code: usize) -> isize {
            exit_code as isize
        }

        fn fork(&self, _caller: Caller) -> isize {
            let processor: *mut PManager<ProcStruct, ProcManager> = PROCESSOR.get_mut() as *mut _;
            let current = unsafe { (*processor).current().unwrap() };
            let mut child_proc = current.fork().unwrap();
            let pid = child_proc.pid;
            let context = &mut child_proc.context.context;
            *context.a_mut(0) = 0 as _;
            unsafe { (*processor).add(pid, child_proc, current.pid) };
            pid.get_usize() as isize
        }

        fn exec(&self, _caller: Caller, path: usize, count: usize) -> isize {
            const READABLE: VmFlags<Sv39> = VmFlags::build_from_str("RV");
            let current = PROCESSOR.get_mut().current().unwrap();
            current
                .address_space
                .translate(VAddr::new(path), READABLE)
                .map(|ptr| unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr.as_ptr(), count))
                })
                .and_then(|name| APPS.get(name))
                .and_then(|input| ElfFile::new(input).ok())
                .map_or_else(
                    || {
                        log::error!("unknown app, select one in the list: ");
                        APPS.keys().for_each(|app| println!("{app}"));
                        println!();
                        -1
                    },
                    |data| {
                        current.exec(data);
                        0
                    },
                )
        }

        fn wait(&self, _caller: Caller, pid: isize, exit_code_ptr: usize) -> isize {
            let processor: *mut PManager<ProcStruct, ProcManager> = PROCESSOR.get_mut() as *mut _;
            let current = unsafe { (*processor).current().unwrap() };
            const WRITABLE: VmFlags<Sv39> = VmFlags::build_from_str("W_V");
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
                // 等待的子进程不存在
                return -1;
            }
        }

        fn getpid(&self, _caller: Caller) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            current.pid.get_usize() as _
        }

        // TODO: 实现 spawn 系统调用
        fn spawn(&self, _caller: Caller, _path: usize, _count: usize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            rcore_console::log::info!(
                "spawn: parent pid = {}, not implemented",
                current.pid.get_usize()
            );
            -1
        }
    }

    impl Scheduling for SyscallContext {
        #[inline]
        fn sched_yield(&self, _caller: Caller) -> isize {
            0
        }

        // TODO: 实现 set_priority 系统调用
        fn set_priority(&self, _caller: Caller, prio: isize) -> isize {
            let current = PROCESSOR.get_mut().current().unwrap();
            rcore_console::log::info!(
                "set_priority: pid = {}, prio = {}, not implemented",
                current.pid.get_usize(),
                prio
            );
            -1
        }
    }

    impl Clock for SyscallContext {
        #[inline]
        fn clock_gettime(&self, _caller: Caller, clock_id: ClockId, tp: usize) -> isize {
            const WRITABLE: VmFlags<Sv39> = VmFlags::build_from_str("W_V");
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

    impl Memory for SyscallContext {
        // TODO: 实现 mmap 系统调用
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
            rcore_console::log::info!(
                "mmap: addr = {addr:#x}, len = {len}, prot = {prot}, not implemented"
            );
            -1
        }

        // TODO: 实现 munmap 系统调用
        fn munmap(&self, _caller: Caller, addr: usize, len: usize) -> isize {
            rcore_console::log::info!("munmap: addr = {addr:#x}, len = {len}, not implemented");
            -1
        }
    }
}
