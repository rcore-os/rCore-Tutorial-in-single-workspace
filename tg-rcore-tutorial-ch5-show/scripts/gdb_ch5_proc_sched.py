# GDB Python 扩展：ch5-show 进程管理与调度演示辅助。
#
# 在 GDB 内（需 riscv-none-elf-gdb-py3）：
#   source scripts/gdb_ch5_proc_sched.py
#
from __future__ import annotations

import gdb  # type: ignore


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


def _read_csr(name: str) -> int | None:
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


class Ch5StageCmd(gdb.Command):
    """判断当前执行阶段（与 ch4-show 的 ch4stage 相同划分，便于对比 virt 启动链）。"""

    def __init__(self) -> None:
        super().__init__("ch5stage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        satp = _read_csr("satp")
        sstatus = _read_csr("sstatus")

        if pc < 0x80000000:
            if satp is not None and (satp >> 60) == 8:
                stage = "U-mode user (Sv39, VA < 0x80000000)"
            else:
                stage = "QEMU reset / ROM (not course kernel)"
        elif pc < 0x80200000:
            stage = "M-mode SBI (_m_start) — tg-rcore-tutorial-sbi"
        elif pc < 0x80400000:
            stage = "S-mode kernel (_start / rust_main / scheduler loop)"
        else:
            stage = "S-mode kernel data / heap region"

        spp_str = ""
        if sstatus is not None:
            spp = (sstatus >> 8) & 1
            spp_str = f"  sstatus.SPP = {'S(1)' if spp else 'U(0)'}"

        satp_str = ""
        if satp is not None:
            mode = (satp >> 60) & 0xF
            ppn = satp & ((1 << 44) - 1)
            mode_name = {0: "Bare", 8: "Sv39", 9: "Sv48"}.get(mode, f"unknown({mode})")
            satp_str = f"\n  satp = {satp:#018x}  [mode={mode_name} root_ppn={ppn:#x}]"

        print(f"ch5stage: {stage}")
        print(f"  $pc = {pc:#x}{spp_str}{satp_str}")


class ShowCsrsCmd(gdb.Command):
    """打印常用 S 态 CSR（含 scause / satp 解析）。"""

    CSRS = [
        "sstatus",
        "scause",
        "sepc",
        "stval",
        "stvec",
        "sscratch",
        "satp",
        "sie",
        "sip",
    ]

    def __init__(self) -> None:
        super().__init__("show_csrs", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print("=== S-mode CSRs (ch5-show) ===")
        for name in self.CSRS:
            val = _read_csr(name)
            if val is not None:
                extra = ""
                if name == "sstatus":
                    spp = (val >> 8) & 1
                    spie = (val >> 5) & 1
                    sie_bit = (val >> 1) & 1
                    extra = f"  [SPP={'S' if spp else 'U'} SPIE={spie} SIE={sie_bit}]"
                elif name == "scause":
                    interrupt = (val >> 63) & 1
                    code = val & 0x7FFFFFFFFFFFFFFF
                    if interrupt:
                        cause_map = {1: "SSoftware", 5: "STimer", 9: "SExternal"}
                        extra = f"  [Interrupt: {cause_map.get(code, f'code={code}')}]"
                    else:
                        cause_map = {
                            0: "InstrMisaligned",
                            1: "InstrFault",
                            2: "IllegalInstr",
                            3: "Breakpoint",
                            5: "LoadFault",
                            7: "StoreFault",
                            8: "UserEnvCall",
                            12: "InstrPageFault",
                            13: "LoadPageFault",
                            15: "StorePageFault",
                        }
                        extra = f"  [Exception: {cause_map.get(code, f'code={code}')}]"
                elif name == "satp":
                    mode = (val >> 60) & 0xF
                    ppn = val & ((1 << 44) - 1)
                    mode_name = {0: "Bare", 8: "Sv39", 9: "Sv48"}.get(
                        mode, f"unknown({mode})"
                    )
                    extra = f"  [mode={mode_name} root_ppn={ppn:#x}]"
                print(f"  {name:10s} = {val:#018x}{extra}")
            else:
                print(f"  {name:10s} = <read failed>")


class ShowSatpCmd(gdb.Command):
    """解析 satp（分页模式 + 根页表 PPN）。"""

    def __init__(self) -> None:
        super().__init__("show_satp", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        satp = _read_csr("satp")
        if satp is None:
            print("satp: <read failed>")
            return
        mode = (satp >> 60) & 0xF
        asid = (satp >> 44) & 0xFFFF
        ppn = satp & ((1 << 44) - 1)
        mode_name = {0: "Bare", 8: "Sv39", 9: "Sv48"}.get(mode, f"unknown({mode})")
        root_pa = ppn << 12
        print("=== satp ===")
        print(f"  raw      = {satp:#018x}")
        print(f"  MODE     = {mode} ({mode_name})")
        print(f"  ASID     = {asid}")
        print(f"  PPN      = {ppn:#x}")
        print(f"  root PA  = {root_pa:#x}")


class Ch5SchedLessonCmd(gdb.Command):
    """打印学习步骤：如何在 GDB 里跟进程 / 调度 / 系统调用。"""

    def __init__(self) -> None:
        super().__init__("ch5sched_lesson", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print(
            """
=== ch5-show: process / scheduling GDB lesson ===
1) Boot chain
   continue 到 _m_start -> _start -> rust_main，用 ch5stage 看当前阶段。
2) After rust_main
   内核建立 Sv39、创建 initproc；串口会出现 [LEC7-LAB5] 与 [BACKTRACE] 演示行。
3) Scheduler (see src/main.rs loop)
   PManager::find_next 从就绪队列取 PID，再取 PCB；ForeignContext::execute 经 MultislotPortal 切用户态。
   建议断点: ch5break_sched
4) Syscall / trap
   UserEnvCall 时 sepc 指向 ecall；内核 tg_syscall::kernel::handle 分发 fork/exec/wait/exit/yield。
   此时 show_csrs 看 scause=8 (UserEnvCall)，对比串口 [LEC7-LAB5] kp=syscall_trap。
5) Process switch
   不同进程的 satp 不同；在 find_next 命中两次之间用 show_satp 对比 root_ppn。
6) Kernel strings
   grep 串口: LEC7-LAB5 | BACKTRACE
"""
        )


class Ch5BreakSchedCmd(gdb.Command):
    """下与调度 / 系统调用相关的断点（符号名随优化级别可能需调整）。"""

    def __init__(self) -> None:
        super().__init__("ch5break_sched", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        cmds = [
            "break rust_main",
            # Rust mangled; rbreak 按正则匹配符号名
            "rbreak proc_manage.*find_next",
            "rbreak tg_rcore_tutorial_syscall.*kernel::handle",
        ]
        for c in cmds:
            try:
                gdb.execute(c)
                print(f"  OK: {c}")
            except gdb.error as e:
                print(f"  FAIL: {c}\n    {e}")


class Ch5TourCmd(gdb.Command):
    """单轮“可视化”提示：打印当前 PC 阶段 + CSR 摘要（可绑在 stop hook 上）。"""

    def __init__(self) -> None:
        super().__init__("ch5tour_snapshot", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print("---------- ch5tour_snapshot ----------")
        out = gdb.execute("ch5stage", to_string=True)
        if out:
            print(out, end="")
        print("--- scause / sepc / satp (compact) ---")
        for n in ("scause", "sepc", "satp"):
            v = _read_csr(n)
            if v is not None:
                print(f"  {n} = {v:#018x}")
        print("--------------------------------------")


Ch5StageCmd()
ShowCsrsCmd()
ShowSatpCmd()
Ch5SchedLessonCmd()
Ch5BreakSchedCmd()
Ch5TourCmd()

print(
    "[ch5-show] Loaded: ch5stage, show_csrs, show_satp, ch5sched_lesson, ch5break_sched, ch5tour_snapshot"
)
