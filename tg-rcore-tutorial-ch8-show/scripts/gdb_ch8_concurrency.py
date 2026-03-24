# GDB Python：第八章「线程 + 同步互斥」动态演示辅助命令。
#
# 在 riscv-none-elf-gdb-py3 中（已 file + target remote 之后）：
#   source scripts/gdb_ch8_concurrency.py
#
# 命令：
#   ch8stage           — 根据 PC/satp 判断大致执行阶段
#   show_csrs_ch8      — 打印常用 S 态 CSR（含 scause 语义）
#   ch8_conc_tour      — 文本「导览」+ 自动设一批与并发相关的断点
#   ch8_break_concurrency — 仅设置断点（不打印导览）

from __future__ import annotations

import gdb  # type: ignore


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


def _read_csr(name: str) -> int | None:
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


class Ch8StageCmd(gdb.Command):
    """判断当前执行阶段：ROM / M-SBI / S-内核 / U-用户（结合 satp）。"""

    def __init__(self) -> None:
        super().__init__("ch8stage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        satp = _read_csr("satp")
        sstatus = _read_csr("sstatus")

        if pc < 0x80000000:
            if satp is not None and (satp >> 60) == 8:
                stage = "U 态用户线程（Sv39 用户地址空间）"
            else:
                stage = "QEMU reset / ROM（非课程内核代码）"
        elif pc < 0x80200000:
            stage = "M 态 SBI（_m_start）— tg-rcore-tutorial-sbi"
        elif pc < 0x80400000:
            stage = "S 态内核（rust_main / 调度循环 / 同步路径）— ch8-show"
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

        print(f"ch8stage: {stage}")
        print(f"  $pc = {pc:#x}{spp_str}{satp_str}")
        print("")
        print("第八章要点：调度粒度为线程；Mutex/Semaphore/Condvar 阻塞时 make_current_blocked，")
        print("释放资源时 re_enque 唤醒。串口可 grep [LEC11-CH8] / [LEC12-CH8]。")


class ShowCsrsCh8Cmd(gdb.Command):
    """打印 ch8 调试常用 CSR。"""

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
        super().__init__("show_csrs_ch8", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print("=== S 态 CSR（ch8-show）===")
        for name in self.CSRS:
            val = _read_csr(name)
            if val is None:
                print(f"  {name:10s} = <读取失败>")
                continue
            extra = ""
            if name == "scause":
                interrupt = (val >> 63) & 1
                code = val & 0x7FFFFFFFFFFFFFFF
                if interrupt:
                    cause_map = {1: "SSoftware", 5: "STimer", 9: "SExternal"}
                    extra = f"  [Interrupt: {cause_map.get(code, f'code={code}')}]"
                else:
                    cause_map = {
                        8: "UserEnvCall(ecall from U)",
                        12: "InstrPageFault",
                        13: "LoadPageFault",
                        15: "StorePageFault",
                    }
                    extra = f"  [Exception: {cause_map.get(code, f'code={code}')}]"
            elif name == "satp":
                mode = (val >> 60) & 0xF
                ppn = val & ((1 << 44) - 1)
                mode_name = {0: "Bare", 8: "Sv39"}.get(mode, f"mode={mode}")
                extra = f"  [{mode_name} root_ppn={ppn:#x}]"
            print(f"  {name:10s} = {val:#018x}{extra}")


def _print_concurrency_tour() -> None:
    print("")
    print("========== ch8-show 并发 / 同步 可视化导览 ==========")
    print("")
    print("1) 执行阶段")
    print("   - 多次: continue  然后  ch8stage")
    print("   - 进入 rust_main 后 satp 应变为 Sv39；调度用户线程后 sepc 指向用户 VA。")
    print("")
    print("2) 线程调度（PThreadManager）")
    print("   - 断点: rust_main, find_next")
    print("   - 每次命中 find_next：观察即将运行的线程上下文（可结合串口 [LEC11-CH8] kp=scheduler_pick）。")
    print("")
    print("3) 阻塞与唤醒（与 [LEC12-CH8] 串口标签对照）")
    print("   - make_current_blocked：mutex_lock / semaphore_down / condvar_wait 返回 -1 时")
    print("   - re_enque：semaphore_up / mutex_unlock / condvar_signal / condvar_wait 内部唤醒")
    print("")
    print("4) 系统调用号（用户态 a7）")
    print("   - 在 UserEnvCall 命中后: show_csrs_ch8  看 sepc；打印用户寄存器用  info registers")
    print("")
    print("5) 源码栈")
    print("   - bt  （需带调试信息构建；内核已演示串口 backtrace + 形参值）")
    print("")
    print("====================================================")
    print("")


def _set_concurrency_breakpoints() -> None:
    """设置一组与第八章教学相关的断点（忽略未知符号）。"""

    def _try_break(loc: str) -> None:
        try:
            gdb.execute(f"break {loc}", to_string=True)
        except gdb.error as e:
            print(f"(skip break {loc}: {e})")

    # 若已按 ch8.gdb 设置过，重复 break 会多实例；教学演示可接受
    for sym in ("rust_main", "_start", "_m_start"):
        _try_break(sym)

    # 使用 rbreak 按正则匹配 Rust 符号子串（不同 rustc 版本符号略有差异）
    for pattern in (
        "find_next",
        "make_current_blocked",
        "make_current_suspend",
        "re_enque",
    ):
        try:
            gdb.execute(f"rbreak {pattern}", to_string=True)
        except gdb.error as e:
            print(f"(skip rbreak {pattern}: {e})")


class Ch8ConcTourCmd(gdb.Command):
    """打印导览说明并设置并发相关断点。"""

    def __init__(self) -> None:
        super().__init__("ch8_conc_tour", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _print_concurrency_tour()
        _set_concurrency_breakpoints()
        print("已尝试设置断点。建议: continue  然后在 UserEnvCall 或 find_next 上停下，执行 ch8stage。")


class Ch8BreakConcurrencyCmd(gdb.Command):
    """仅设置并发相关断点（无长篇说明）。"""

    def __init__(self) -> None:
        super().__init__(
            "ch8_break_concurrency",
            gdb.COMMAND_USER,
            gdb.COMPLETE_NONE,
            True,
        )

    def invoke(self, argument: str, from_tty: bool) -> None:
        _set_concurrency_breakpoints()


Ch8StageCmd()
ShowCsrsCh8Cmd()
Ch8ConcTourCmd()
Ch8BreakConcurrencyCmd()

print("[gdb_ch8_concurrency] loaded: ch8stage, show_csrs_ch8, ch8_conc_tour, ch8_break_concurrency")
