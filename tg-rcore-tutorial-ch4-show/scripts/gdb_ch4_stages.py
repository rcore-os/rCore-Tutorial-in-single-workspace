# GDB Python 扩展：ch4 地址空间管理的调试辅助命令。
#
# 在 GDB 内加载：
#   source scripts/gdb_ch4_stages.py
#
# 需要 riscv-none-elf-gdb-py3（带 Python 支持的 GDB）。

from __future__ import annotations

import gdb  # type: ignore


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


def _read_csr(name: str) -> int | None:
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


# ===== ch4stage 命令 =====

class Ch4StageCmd(gdb.Command):
    """根据当前 PC 判断执行阶段（ROM / M-SBI / S-内核 / U-用户程序）。

    ch4 启用 Sv39 后，用户程序运行在独立地址空间（VA 通常从 0x10000 开始），
    内核恒等映射在 0x80200000 区域。通过 satp 和 sstatus.SPP 辅助判断阶段。
    """

    def __init__(self) -> None:
        super().__init__("ch4stage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        satp = _read_csr("satp")
        sstatus = _read_csr("sstatus")

        if pc < 0x80000000:
            if satp is not None and (satp >> 60) == 8:
                stage = "U 态用户程序（Sv39 地址空间，VA < 0x80000000）"
            else:
                stage = "QEMU reset / ROM（非课程代码）"
        elif pc < 0x80200000:
            stage = "M 态 SBI（_m_start）— tg-rcore-tutorial-sbi"
        elif pc < 0x80400000:
            stage = "S 态内核（_start / rust_main / schedule）— src/main.rs"
        else:
            stage = "S 态内核数据/堆区域"

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

        print(f"ch4stage: {stage}")
        print(f"  $pc = {pc:#x}{spp_str}{satp_str}")


# ===== show_csrs 命令 =====

class ShowCsrsCmd(gdb.Command):
    """打印 ch4 调试中常用的 S 态 CSR（含 satp 解析）。"""

    CSRS = [
        "sstatus", "scause", "sepc", "stval",
        "stvec", "sscratch", "satp", "sie", "sip",
    ]

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
                elif name == "satp":
                    mode = (val >> 60) & 0xF
                    ppn = val & ((1 << 44) - 1)
                    mode_name = {0: "Bare", 8: "Sv39", 9: "Sv48"}.get(
                        mode, f"unknown({mode})"
                    )
                    extra = f"  [mode={mode_name} root_ppn={ppn:#x}]"
                print(f"  {name:10s} = {val:#018x}{extra}")
            else:
                print(f"  {name:10s} = <读取失败>")


# ===== show_satp 命令 =====

class ShowSatpCmd(gdb.Command):
    """解析并显示 satp 寄存器（分页模式 + 根页表物理页号）。"""

    def __init__(self) -> None:
        super().__init__("show_satp", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        satp = _read_csr("satp")
        if satp is None:
            print("satp: <读取失败>")
            return

        mode = (satp >> 60) & 0xF
        asid = (satp >> 44) & 0xFFFF
        ppn = satp & ((1 << 44) - 1)
        mode_name = {0: "Bare", 8: "Sv39", 9: "Sv48"}.get(mode, f"unknown({mode})")
        root_pa = ppn << 12

        print(f"=== satp 寄存器 ===")
        print(f"  raw    = {satp:#018x}")
        print(f"  MODE   = {mode} ({mode_name})")
        print(f"  ASID   = {asid}")
        print(f"  PPN    = {ppn:#x}")
        print(f"  根页表PA = {root_pa:#x}")


# ===== watch_priv_switch 命令 =====

class WatchPrivSwitchCmd(gdb.Command):
    """在 execute_naked 处设断点，用于观察跨地址空间特权级切换。

    ch4 的特权级切换通过 MultislotPortal 完成：
    1. 内核切换 satp 到用户地址空间
    2. 恢复用户寄存器
    3. sret 进入 U-mode
    """

    def __init__(self) -> None:
        super().__init__(
            "watch_priv_switch", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True
        )

    def invoke(self, argument: str, from_tty: bool) -> None:
        try:
            gdb.execute("break execute_naked")
            print("已在 execute_naked 入口设断点。")
            print("到达后用 'si' 单步到 sret 指令，观察：")
            print("  show_csrs             -- 查看 CSR（含 satp）")
            print("  show_satp             -- 查看 satp 解析")
            print("  ch4stage              -- 判断当前阶段")
            print("  info registers sepc sstatus sp")
        except gdb.error as e:
            print(f"设断点失败: {e}")
            print("提示：确认 ELF 已加载且包含 execute_naked 符号")


# 注册命令
Ch4StageCmd()
ShowCsrsCmd()
ShowSatpCmd()
WatchPrivSwitchCmd()

print("[ch4-show] GDB 扩展已加载。可用命令：ch4stage, show_csrs, show_satp, watch_priv_switch")
