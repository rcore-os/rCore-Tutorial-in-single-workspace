# GDB Python 扩展：ch3 多道程序与分时多任务的调试辅助命令。
#
# 在 GDB 内加载：
#   source scripts/gdb_ch3_stages.py
#
# 需要 riscv-none-elf-gdb-py3（带 Python 支持的 GDB）。

from __future__ import annotations

import gdb  # type: ignore


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


def _read_csr(name: str) -> int | None:
    """尝试读取 CSR，失败则返回 None。"""
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


# ===== ch3stage 命令 =====

class Ch3StageCmd(gdb.Command):
    """根据当前 PC 判断执行阶段（ROM / M-SBI / S-内核 / U-用户程序）。

    ch3 的内存布局：
      < 0x80000000          QEMU ROM / reset 代码
      0x80000000..0x80200000  M 态 SBI（_m_start，m_entry.asm）
      0x80200000..0x80400000  S 态内核（_start，src/main.rs）
      >= 0x80400000           用户程序（app0 @ 0x80400000, app1 @ 0x80600000, ...）
    """

    APP_BASE = 0x80400000
    APP_STEP = 0x200000

    def __init__(self) -> None:
        super().__init__("ch3stage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        if pc < 0x80000000:
            stage = "QEMU reset / ROM（非课程代码；用 x/i $pc、si 观察）"
        elif pc < 0x80200000:
            stage = "M 态 SBI（_m_start）— 对照 tg-rcore-tutorial-sbi/src/m_entry.asm"
        elif pc < 0x80400000:
            stage = "S 态内核（_start / rust_main）— 对照 src/main.rs"
        else:
            app_id = (pc - self.APP_BASE) // self.APP_STEP
            stage = f"U 态用户程序 app{app_id}（地址 {self.APP_BASE + app_id * self.APP_STEP:#x}）"

        # 尝试读取 sstatus.SPP 判断当前特权级
        sstatus = _read_csr("sstatus")
        spp_str = ""
        if sstatus is not None:
            spp = (sstatus >> 8) & 1
            spp_str = f"  sstatus.SPP = {'S(1)' if spp else 'U(0)'}"

        print(f"ch3stage: {stage}")
        print(f"  $pc = {pc:#x}{spp_str}")


# ===== show_csrs 命令 =====

class ShowCsrsCmd(gdb.Command):
    """打印 ch3 调试中常用的 S 态 CSR。"""

    CSRS = ["sstatus", "scause", "sepc", "stval", "stvec", "sscratch", "sie", "sip"]

    def __init__(self) -> None:
        super().__init__("show_csrs", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print("=== S 态关键 CSR ===")
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
                            0: "InstrMisaligned", 1: "InstrFault",
                            2: "IllegalInstr", 3: "Breakpoint",
                            5: "LoadFault", 7: "StoreFault",
                            8: "UserEnvCall", 12: "InstrPageFault",
                            13: "LoadPageFault", 15: "StorePageFault",
                        }
                        extra = f"  [Exception: {cause_map.get(code, f'code={code}')}]"
                print(f"  {name:10s} = {val:#018x}{extra}")
            else:
                print(f"  {name:10s} = <读取失败>")


# ===== watch_priv_switch 命令 =====

class WatchPrivSwitchCmd(gdb.Command):
    """在 execute_naked 中的 sret 指令处设断点，用于观察特权级切换。

    使用方式：
      watch_priv_switch
      continue

    每次断点命中时显示 sepc、sstatus.SPP、sp 等信息。
    """

    def __init__(self) -> None:
        super().__init__(
            "watch_priv_switch", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True
        )

    def invoke(self, argument: str, from_tty: bool) -> None:
        # 在 sret 指令处设断点（搜索 execute_naked 符号附近的 sret）
        try:
            gdb.execute("break execute_naked")
            print("已在 execute_naked 入口设断点。")
            print("到达后用 'si' 单步到 sret 指令，观察：")
            print("  info registers sepc sstatus sp")
            print("  show_csrs")
            print("  ch3stage")
        except gdb.error as e:
            print(f"设断点失败: {e}")
            print("提示：确认 ELF 已加载且包含 execute_naked 符号")


# 注册命令
Ch3StageCmd()
ShowCsrsCmd()
WatchPrivSwitchCmd()

print("[ch3-show] GDB 扩展已加载。可用命令：ch3stage, show_csrs, watch_priv_switch")
