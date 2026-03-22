# GDB Python: 根据 $pc 判断当前特权级与执行阶段（教学用）。
# 在 GDB 内加载：
#   source scripts/gdb_trap_stage.py
# 然后执行：
#   trapstage
#
# 需要使用 riscv-none-elf-gdb-py3

from __future__ import annotations

import gdb  # type: ignore


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


def _read_csr(name: str) -> int | None:
    """Try to read a CSR; return None if not accessible."""
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


class TrapStageCmd(gdb.Command):
    """Print current privilege level and trap state from PC and CSRs.

    Address ranges (virt + nobios layout):
        < 0x80000000          : QEMU reset / ROM
        0x80000000..0x80200000: M-mode SBI (_m_start)
        0x80200000..0x80400000: S-mode kernel (_start, handle_syscall)
        >= 0x80400000         : U-mode user application
    """

    def __init__(self) -> None:
        super().__init__("trapstage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()

        if pc < 0x80000000:
            stage = "QEMU reset / ROM"
            hint = "课内无源码；用 x/i $pc、si 观察"
        elif pc < 0x80200000:
            stage = "M-mode SBI"
            hint = "对照 ../tg-rcore-tutorial-sbi/src/m_entry.asm"
        elif pc < 0x80400000:
            stage = "S-mode kernel"
            hint = "对照 src/main.rs（_start / rust_main / handle_syscall）"
        else:
            stage = "U-mode user app"
            hint = "用户程序执行中；ecall 将触发 trap 回到 S-mode"

        print(f"trapstage: {stage}")
        print(f"  hint: {hint}")
        print(f"  $pc = 0x{pc:x}")

        # Try to display trap-related CSRs
        sstatus = _read_csr("sstatus")
        if sstatus is not None:
            spp = "S" if (sstatus >> 8) & 1 else "U"
            spie = "on" if (sstatus >> 5) & 1 else "off"
            print(f"  sstatus = 0x{sstatus:x}  (SPP={spp}, SPIE={spie})")

        scause = _read_csr("scause")
        if scause is not None:
            is_interrupt = (scause >> 63) & 1
            code = scause & 0x7FFFFFFFFFFFFFFF
            kind = "interrupt" if is_interrupt else "exception"
            cause_names = {
                0: "InstructionMisaligned",
                1: "InstructionFault",
                2: "IllegalInstruction",
                3: "Breakpoint",
                4: "LoadMisaligned",
                5: "LoadFault",
                6: "StoreMisaligned",
                7: "StoreFault",
                8: "UserEnvCall",
                9: "SupervisorEnvCall",
                12: "InstructionPageFault",
                13: "LoadPageFault",
                15: "StorePageFault",
            }
            cause_str = cause_names.get(code, f"code={code}")
            print(f"  scause  = 0x{scause:x}  ({kind}: {cause_str})")

        sepc = _read_csr("sepc")
        if sepc is not None:
            print(f"  sepc    = 0x{sepc:x}")

        stval = _read_csr("stval")
        if stval is not None and stval != 0:
            print(f"  stval   = 0x{stval:x}")

        stvec = _read_csr("stvec")
        if stvec is not None:
            print(f"  stvec   = 0x{stvec:x}")


TrapStageCmd()
