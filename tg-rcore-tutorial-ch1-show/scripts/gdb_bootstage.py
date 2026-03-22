# GDB Python: 根据 $pc 粗分启动阶段（教学用）。
# 在 GDB 内加载：
#   python
#   import sys, os
#   sys.path.insert(0, "scripts")
#   import gdb_bootstage
#   end
# 然后执行：bootstage
#
# 或使用 riscv-none-elf-gdb-py3 -x gdb/boot.gdb，在 GDB 里：
#   source scripts/gdb_bootstage.py

from __future__ import annotations

import gdb  # type: ignore


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


class BootStageCmd(gdb.Command):
    """Print coarse boot stage from current PC (virt + nobios layout)."""

    def __init__(self) -> None:
        super().__init__("bootstage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        if pc < 0x80000000:
            print(
                "bootstage: QEMU reset / ROM 区（课内无源码；用 x/i $pc、si 观察）"
            )
        elif pc < 0x80200000:
            print(
                "bootstage: M 态 SBI（_m_start 一带）— 对照 ../tg-rcore-tutorial-sbi/src/m_entry.asm"
            )
        else:
            print("bootstage: S 态内核（_start）— 对照 src/main.rs")
        print(f"  $pc = 0x{pc:x}")


BootStageCmd()
