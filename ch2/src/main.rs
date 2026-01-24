#![no_std]
#![no_main]
#![deny(warnings)]

#[macro_use]
extern crate tg_console;

use impls::{Console, SyscallContext};
use riscv::register::*;
use tg_console::log;
use tg_kernel_context::LocalContext;
use tg_sbi;
use tg_syscall::{Caller, SyscallId};

// 用户程序内联进来。
core::arch::global_asm!(include_str!(env!("APP_ASM")));
// 定义内核入口。
tg_linker::boot0!(rust_main; stack = 4 * 4096);

extern "C" fn rust_main() -> ! {
    // bss 段清零
    unsafe { tg_linker::KernelLayout::locate().zero_bss() };
    // 初始化 `console`
    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG"));
    tg_console::test_log();
    // 初始化 syscall
    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);
    // 批处理
    for (i, app) in tg_linker::AppMeta::locate().iter().enumerate() {
        let app_base = app.as_ptr() as usize;
        log::info!("load app{i} to {app_base:#x}");
        // 初始化上下文
        let mut ctx = LocalContext::user(app_base);
        // 设置用户栈（使用 MaybeUninit 避免 release 模式下零初始化的问题）
        let mut user_stack: core::mem::MaybeUninit<[usize; 256]> = core::mem::MaybeUninit::uninit();
        let user_stack_ptr = user_stack.as_mut_ptr() as *mut usize;
        *ctx.sp_mut() = unsafe { user_stack_ptr.add(256) } as usize;
        loop {
            unsafe { ctx.execute() };

            use scause::{Exception, Trap};
            match scause::read().cause() {
                Trap::Exception(Exception::UserEnvCall) => {
                    use SyscallResult::*;
                    match handle_syscall(&mut ctx) {
                        Done => continue,
                        Exit(code) => log::info!("app{i} exit with code {code}"),
                        Error(id) => log::error!("app{i} call an unsupported syscall {}", id.0),
                    }
                }
                trap => log::error!("app{i} was killed because of {trap:?}"),
            }
            // 清除指令缓存
            unsafe { core::arch::asm!("fence.i") };
            break;
        }
        // 防止 user_stack 被优化
        let _ = core::hint::black_box(&user_stack);
        println!();
    }

    tg_sbi::shutdown(false)
}

/// Rust 异常处理函数，以异常方式关机。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    tg_sbi::shutdown(true)
}

enum SyscallResult {
    Done,
    Exit(usize),
    Error(SyscallId),
}

/// 处理系统调用，返回是否应该终止程序。
fn handle_syscall(ctx: &mut LocalContext) -> SyscallResult {
    use tg_syscall::{SyscallId as Id, SyscallResult as Ret};

    let id = ctx.a(7).into();
    let args = [ctx.a(0), ctx.a(1), ctx.a(2), ctx.a(3), ctx.a(4), ctx.a(5)];
    match tg_syscall::handle(Caller { entity: 0, flow: 0 }, id, args) {
        Ret::Done(ret) => match id {
            Id::EXIT => SyscallResult::Exit(ctx.a(0)),
            _ => {
                *ctx.a_mut(0) = ret as _;
                ctx.move_next();
                SyscallResult::Done
            }
        },
        Ret::Unsupported(id) => SyscallResult::Error(id),
    }
}

/// 各种接口库的实现
mod impls {
    use tg_syscall::{STDDEBUG, STDOUT};

    pub struct Console;

    impl tg_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            tg_sbi::console_putchar(c);
        }
    }

    pub struct SyscallContext;

    impl tg_syscall::IO for SyscallContext {
        fn write(&self, _caller: tg_syscall::Caller, fd: usize, buf: usize, count: usize) -> isize {
            match fd {
                STDOUT | STDDEBUG => {
                    print!("{}", unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            buf as *const u8,
                            count,
                        ))
                    });
                    count as _
                }
                _ => {
                    tg_console::log::error!("unsupported fd: {fd}");
                    -1
                }
            }
        }
    }

    impl tg_syscall::Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: tg_syscall::Caller, _status: usize) -> isize {
            0
        }
    }
}
