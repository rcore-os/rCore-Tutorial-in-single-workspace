# GDB Python：tg-rcore-tutorial-ch2-show 批处理 / U 态 / 系统调用 / 下一应用 教学演示
#
# 在 crate 根目录启动 GDB 并连接 QEMU 后：
#   source scripts/gdb_trap_stage.py
#   source scripts/gdb_batch_os_tour.py
#
# 命令：
#   batch_os_intro     — 仅打印静态说明（与源码、链接布局对应）
#   batch_os_tour      — 自动下断点并 continue，分阶段讲解（首应用 + 进入下一应用）
#
# 需：riscv-none-elf-gdb-py3；ELF 须带调试信息；QEMU 使用 -machine virt -bios none -kernel <ELF> -s -S

from __future__ import annotations

import sys

import gdb  # type: ignore


def _banner(title: str) -> None:
    line = "=" * 72
    print(f"\n{line}\n {title}\n{line}\n")
    sys.stdout.flush()


def _ex(cmd: str) -> None:
    gdb.execute(cmd, to_string=False)


def _trapstage() -> None:
    try:
        gdb.execute("trapstage", to_string=False)
    except gdb.error as e:
        print(f"(trapstage 不可用: {e})")


def _read_csr(name: str) -> int | None:
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


class BatchOsIntroCmd(gdb.Command):
    """打印 ch2-show 批处理与特权级相关的静态说明（对照源码阅读）。"""

    def __init__(self) -> None:
        super().__init__("batch_os_intro", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("静态导读：QEMU / 应用加载 / 任务 / 系统调用 / 下一应用")

        print(
            """
【1】QEMU 与内核镜像
  - `qemu-system-riscv64 -bios none -kernel <内核 ELF>` 只把**内核 ELF**加载到内存；
    用户程序**不是**独立磁盘文件，而是由 build.rs 把各 app 二进制嵌入内核数据段。
  - 运行时 `tg_linker::AppMeta::locate().iter()`（见 tg-rcore-tutorial-linker）按
    `cases.toml` 顺序取出每一段 app 字节；当 base=0x80400000 时，把镜像**拷贝**到
    固定用户区 `0x80400000`（见 src/main.rs `run_batch` 与 lec3_lab2::emit_app_load_info）。

【2】“任务”与地址空间（ch2 批处理模型）
  - 每个 app：`LocalContext::user(app_base)` 设置 `sepc`、`*ctx.sp_mut()` 指向内核里
    分配的用户栈顶（`MaybeUninit<[usize;512]>`，见 main.rs）。
  - ch2 无独立页表：所有用户 app 共用同一虚拟地址 `APP_BASE=0x80400000` 装载代码，
    依次覆盖运行；这是“单地址槽 + 顺序批处理”，不是多进程并发隔离。

【3】进入用户态
  - `ctx.execute()`（tg-rcore-tutorial-kernel-context）设置 `sstatus.SPP=0`、`sepc`、
    `stvec` 等后，在 `execute_naked` 中 `sret` → PC=`sepc`，进入 **U 态**。

【4】系统调用
  - 用户 `ecall` → `scause=8` (UserEnvCall)，`sepc` 指向 `ecall` 指令；
    内核在 `run_batch` 里 `match scause` → `handle_syscall`（main.rs），按 a7 分发，
    `move_next` 使 `sepc+=4` 跳过 `ecall`。

【5】应用结束与下一应用
  - `SyscallId::EXIT` → 跳出内层 `loop`，执行 `fence.i`（main.rs:244）刷新指令缓存，
    再进入 `for` 下一轮：`iter()` 把**下一个** app 拷到 `0x80400000`，重复上述过程。

【6】本演示脚本默认路径
  - 演示从 `panic` 演示之后的 `run_batch` 开始（与 main.rs panic_handler 一致），
    串口会先有大量 `[LEC3-LAB2]` 输出，属正常现象。
"""
        )


class BatchOsTourCmd(gdb.Command):
    """自动断点 + continue，分阶段讲解首个用户 app 及加载下一 app。"""

    def __init__(self) -> None:
        super().__init__("batch_os_tour", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("自动化演示：batch_os_tour（将多次 continue，串口会有输出）")

        try:
            _ex("delete")
        except gdb.error:
            pass

        # --- 阶段 A：批处理入口 run_batch（通常来自 panic_handler 调用链）---
        _ex("break tg_rcore_tutorial_ch2_show::run_batch")
        print(
            "[A] 已下断点：tg_rcore_tutorial_ch2_show::run_batch\n"
            "    continue：运行至批处理主循环（此前为 lec3 可观测输出 + panic 演示）。\n"
        )
        _ex("continue")

        _banner("阶段 A — 到达 run_batch")
        print(
            "源码：src/main.rs `fn run_batch()`。\n"
            "此处开始 `for (i, app) in AppMeta::locate().iter()`：每个 app 的字节切片"
            "会被拷贝到 app_base（ch2 为 0x80400000），再构造 `LocalContext::user(app_base)`。\n"
        )
        _trapstage()
        _ex("info line")

        # --- 阶段 B：第一次 LocalContext::execute（即将首次 sret 进用户态）---
        _ex("delete")
        _ex("break tg_rcore_tutorial_kernel_context::LocalContext::execute")
        print(
            "\n[B] 已下断点：LocalContext::execute（仅第一次命中有效；进入 U 态前会反复进入该函数，"
            "故命中后将删除此断点）。\n"
        )
        _ex("continue")

        _banner("阶段 B — 首次 execute：内核已准备好用户上下文")
        print(
            "此时 `sepc` 应指向用户入口（通常为 0x80400000）；`sscratch` 等由 execute 路径写入。\n"
            "用户栈指针已在 run_batch 中写入 `ctx.sp_mut()`（内核栈上的 MaybeUninit 区域顶端）。\n"
        )
        sepc = _read_csr("sepc")
        print(
            "  说明：停在 `execute()` 的 Rust 序言时，硬件 CSR `sepc` 可能尚未写入；"
            "用户入口 PC 由 `LocalContext::user(app_base)` 保存在上下文中，随后经汇编写入 CSR。\n"
        )
        if sepc is not None and sepc != 0:
            print(f"  CSR sepc = 0x{sepc:x}")
        print("  当前 PC（仍在 S 态执行 execute）：")
        _ex("x/4i $pc")
        _ex("delete")

        # --- 阶段 C：用户程序入口 0x80400000 ---
        _ex("break *0x80400000")
        print("\n[C] 已下断点 *0x80400000（首个 app 的 ELF Entry，见 readelf -h）。continue。\n")
        _ex("continue")

        _banner("阶段 C — 用户态第一条指令（U-mode）")
        _trapstage()
        print(
            "（提示：停在普通用户指令时，`scause` 可能仍为 0 或残留值；以 `$pc∈[0x80400000,…)` 判断 U 态为主。）\n"
        )
        _ex("x/6i $pc")

        # --- 阶段 D：第一次系统调用（常为 write）---
        _ex("delete")
        _ex("break tg_rcore_tutorial_ch2_show::handle_syscall")
        print(
            "\n[D] 已下断点：handle_syscall。首个 app 会先 ecall（如 write），再 ecall（exit）。\n"
            "    第一次 continue → 观察系统调用服务路径。\n"
        )
        _ex("continue")

        _banner("阶段 D — 第一次 handle_syscall（示例：写控制台）")
        _trapstage()
        scause = _read_csr("scause")
        sepc = _read_csr("sepc")
        if scause is not None:
            print(f"  scause = 0x{scause:x}  （8 = UserEnvCall / ecall from U-mode）")
        if sepc is not None:
            print(f"  sepc   = 0x{sepc:x}  （陷入时保存的 PC，指向 ecall 指令）")
        print("  反汇编 sepc 处：")
        if sepc is not None:
            _ex(f"x/3i {sepc:#x}")
        print(
            "\nABI：a7=调用号，a0–a5 为参数；内核从 `ctx.a(7)` 等读取（用户寄存器已保存在 LocalContext，"
            "硬件 a0–a7 不一定等于 ABI）。对照串口 `[LEC3-LAB2] kp=syscall_abi_demo` 行。\n"
        )

        # --- 阶段 E：第二次系统调用（exit）---
        print("[E] continue → 第二次进入 handle_syscall（通常为 EXIT）。\n")
        _ex("continue")

        _banner("阶段 E — 第二次 handle_syscall（进程 exit）")
        _trapstage()
        print(
            "若 `ctx.a(7)` 对应 EXIT：`handle_syscall` 返回 `SyscallResult::Exit`，"
            "run_batch 中跳出内层 loop，准备 `fence.i` 与下一轮 app。\n"
        )

        # --- 阶段 F：fence.i（指令缓存一致性与“准备下一应用”）---
        _ex("delete")
        _ex("break main.rs:244")
        print("[F] 已下断点：main.rs:244（`fence.i`）。continue。\n")
        _ex("continue")

        _banner("阶段 F — fence.i：同一地址 0x80400000 将装入下一应用二进制")
        print(
            "源码：src/main.rs 内层 loop 结束后的 `unsafe { fence.i }`。\n"
            "作用：在覆盖 0x80400000 指令前刷新 I-cache，避免 CPU 执行旧指令。\n"
        )
        _ex("x/2i $pc")

        # --- 阶段 G：for 循环下一轮，加载 app_idx=1 ---
        _ex("delete")
        _ex("break main.rs:171")
        print("[G] 已下断点：main.rs:171（下一轮 `let app_base = app.as_ptr()`）。continue。\n")
        _ex("continue")

        _banner("阶段 G — 下一应用：再次拷贝到 0x80400000 并构造新 LocalContext")
        print(
            "此处 `i` 递增，从 `iter()` 取出下一段 app 字节并拷到同一 app_base；"
            "随后再次 `LocalContext::user(app_base)` 与 `execute()`，批处理继续。\n"
        )
        _ex("info line")

        _banner("演示结束")
        print(
            "可继续 `continue` 观察后续 app，或 `kill` 结束 QEMU（避免 guest 一直跑）。\n"
            "手动补充：`source scripts/gdb_trap_stage.py` 后随时 `trapstage`。\n"
        )


BatchOsIntroCmd()
BatchOsTourCmd()
