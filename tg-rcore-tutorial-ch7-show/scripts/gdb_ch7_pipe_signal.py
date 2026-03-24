# GDB Python：动态观察 ch7-show 中与「管道 + 信号」相关的执行路径。
#
# 在 riscv-none-elf-gdb-py3 内：
#   source scripts/gdb_ch7_pipe_signal.py
#   ch7help
#   ch7break_ipc      # 设置 catch syscall + 内核符号断点（若存在）
#   continue
#
# 停止在 catch syscall 时，可用：
#   ch7frame          # 打印当前帧与常用寄存器（a0–a7）
#   ch7stage          # 粗粒度判断 PC 落在 ROM / SBI / 内核 / 用户 VA

from __future__ import annotations

import gdb  # type: ignore

SYSCALLS_IPC = [
    (59, "pipe2 — 创建管道，内核分配读/写 fd"),
    (63, "read — 可能阻塞在管道读端"),
    (64, "write — 可能阻塞在管道写端"),
    (129, "kill — 向目标进程位图投递信号"),
    (134, "rt_sigaction — 注册/查询信号处理"),
    (135, "rt_sigprocmask — 更新信号屏蔽字"),
    (139, "rt_sigreturn — 从信号处理返回并恢复上下文"),
]


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


def _reg(name: str) -> int | None:
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


def _banner(title: str) -> None:
    line = "=" * 56
    print(f"\n{line}\n  {title}\n{line}")


class Ch7HelpCmd(gdb.Command):
    """ch7help — 列出本脚本提供的命令与推荐实验顺序。"""

    def __init__(self) -> None:
        super().__init__("ch7help", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("ch7-show：管道 + 信号 — GDB 可视化命令")
        print(
            "推荐流程：\n"
            "  1) 终端 A: bash scripts/launch-qemu-gdb.sh\n"
            "  2) 终端 B: riscv-none-elf-gdb-py3 -x gdb/ch7.gdb\n"
            "  3) (gdb) source scripts/gdb_ch7_pipe_signal.py\n"
            "  4) (gdb) ch7break_ipc && continue\n"
            "  5) 在用户 shell 中运行 ch7b_pipetest 等，观察每次 syscall 停下的调用栈\n"
        )
        print("命令：")
        print("  ch7help        本帮助")
        print("  ch7stage       根据 $pc / satp 判断执行阶段")
        print("  ch7frame       打印 a0–a7 与 backtrace（若可用）")
        print("  ch7break_ipc   catch syscall：pipe2/read/write/kill/sig*")
        print("  ch7syscalls    仅打印 syscall 号与语义对照表")


class Ch7StageCmd(gdb.Command):
    """ch7stage — 类似 ch4 的 stage，用于区分 ROM / M-SBI / S-内核 / U-用户。"""

    def __init__(self) -> None:
        super().__init__("ch7stage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        satp = _reg("satp")
        sstatus = _reg("sstatus")

        if pc < 0x80000000:
            if satp is not None and (satp >> 60) == 8:
                stage = "U 态（Sv39 用户 VA 空间）"
            else:
                stage = "QEMU reset / ROM"
        elif pc < 0x80200000:
            stage = "M 态 SBI（_m_start）"
        elif pc < 0x81000000:
            stage = "S 态内核（含 rust_main / syscall 实现 / easy-fs pipe）"
        else:
            stage = "S 态内核高地址 / 数据区"

        print(f"ch7stage: {stage}")
        print(f"  $pc = {pc:#x}")
        if sstatus is not None:
            spp = (sstatus >> 8) & 1
            print(f"  sstatus.SPP = {'S(1)' if spp else 'U(0)'}")
        if satp is not None:
            mode = (satp >> 60) & 0xF
            ppn = satp & ((1 << 44) - 1)
            mode_name = {0: "Bare", 8: "Sv39", 9: "Sv48"}.get(mode, f"unknown({mode})")
            print(f"  satp: mode={mode_name} root_ppn={ppn:#x}")


class Ch7FrameCmd(gdb.Command):
    """ch7frame — 打印 a0–a7 与 backtrace，便于对照内核 syscall 参数。"""

    def __init__(self) -> None:
        super().__init__("ch7frame", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print("=== 寄存器 a0 – a7（RISC-V syscall 参数约定）===")
        for i in range(8):
            v = _reg(f"a{i}")
            if v is not None:
                print(f"  a{i} = {v:#x}")
        print("=== backtrace ===")
        try:
            gdb.execute("bt 16")
        except gdb.error as e:
            print(f"(bt 失败: {e})")


class Ch7SyscallsTableCmd(gdb.Command):
    def __init__(self) -> None:
        super().__init__("ch7syscalls", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print("本章相关 syscall（与 lec10_lab7 输出一致）：")
        for num, desc in SYSCALLS_IPC:
            print(f"  {num:3d}  {desc}")


def _try_break(spec: str) -> bool:
    try:
        gdb.execute(f"break {spec}", to_string=True)
        return True
    except gdb.error:
        return False


class Ch7BreakIpcCmd(gdb.Command):
    """ch7break_ipc — catch syscall（动态）并尝试在内核关键符号设断点。"""

    def __init__(self) -> None:
        super().__init__("ch7break_ipc", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("设置断点：管道 + 信号 syscall")
        for num, desc in SYSCALLS_IPC:
            try:
                gdb.execute(f"catch syscall {num}")
                print(f"  catch syscall {num}  # {desc}")
            except gdb.error as e:
                print(f"  (跳过 {num}: {e})")

        print("\n可选内核断点（符号因优化可能不存在，失败可忽略）：")
        candidates = [
            "rust_main",
            "tg_rcore_tutorial_easy_fs::pipe::make_pipe",
            "tg_syscall::handle",
        ]
        for c in candidates:
            if _try_break(c):
                print(f"  break {c}  # OK")
            else:
                print(f"  break {c}  # 未解析，改用 catch syscall 即可")

        print(
            "\n提示：每次停在 catch syscall 时执行 `ch7frame` 查看参数；"
            "`ch7stage` 确认是否在内核态处理 ecall。"
        )


Ch7HelpCmd()
Ch7StageCmd()
Ch7FrameCmd()
Ch7SyscallsTableCmd()
Ch7BreakIpcCmd()
