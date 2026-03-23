# GDB Python：ch3-show 时钟中断（源码路径）与任务切换（可观测）教学演示
#
# 依赖：
#   source scripts/gdb_ch3_stages.py
#
# 命令：
#   timer_task_intro   — 静态说明（main.rs 定时器分支 + 轮转切换）
#   timer_task_tour    — 自动断点：开 STIE → 首次任务切换（emit_task_switch）
#   timer_break_if_any  — 仅在 emit_timer_interrupt 设断点并 continue（部分环境可能长时间不命中）
#
# 说明：在常见 qemu-system-riscv64 运行本内核时，串口可能**始终不出现**
# `[LEC4-LAB3] kp=timer_interrupt`（Supervisor 定时器在 U 态执行期间可能因全局中断使能
# 等条件未满足而未投递）。任务切换仍可通过 `yield` 与外层轮转大量出现；
# 本脚本用 **emit_task_switch** 展示切换，并在 intro 中对照定时器源码路径。

from __future__ import annotations

import sys

import gdb  # type: ignore


def _banner(title: str) -> None:
    line = "=" * 72
    print(f"\n{line}\n {title}\n{line}\n")
    sys.stdout.flush()


def _ex(cmd: str) -> None:
    gdb.execute(cmd, to_string=False)


class TimerTaskIntroCmd(gdb.Command):
    """打印：时钟中断处理（源码）与任务切换（TCB + 外层 i 轮转）。"""

    def __init__(self) -> None:
        super().__init__("timer_task_intro", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("导读：SupervisorTimer（源码）与任务切换（运行时）")

        print(
            """
【1】开启与编程时钟
  - `sie::set_stimer()`（main.rs:164）：允许 S 态定时器中断（`sie.STIE`）。
  - 主循环内在非 `coop` 特性下：`tg_sbi::set_timer(time::read64() + 12500)` 设置时间片。

【2】响应时钟中断（源码路径，串口标签 kp=timer_interrupt）
  - `Trap::Interrupt(Interrupt::SupervisorTimer)`（main.rs:187–193）：`set_timer(u64::MAX)`、
    `lec4_lab3::emit_timer_interrupt(i)`。
  - 此时 `scause` 应为「中断位置位 + 原因码 5（STimer）」；可用 `show_csrs` 核对。

【3】为何有时看不到 kp=timer_interrupt？
  - 若 CPU 在用户态执行时全局未开 S 态中断等，定时器可能暂不进入该分支（与 QEMU/CSR 配置有关）。
  - 这不影响阅读源码；调试可手动 `break tg_rcore_tutorial_ch3_show::lec4_lab3::emit_timer_interrupt`。

【4】任务切换（一定可观测：kp=task_switch / yield_switch）
  - 抢占或协作式让出后，内层 `loop` 结束，`i = (i + 1) % index_mod`，满足条件时
    `emit_task_switch(prev_i, i)`（main.rs:263–265）。
  - `SCHED_YIELD` 路径会先 `emit_yield_switch`，再经同一外层逻辑到达 `emit_task_switch`。
  - 每个任务有独立 TCB（`task.rs`），`tcb.execute()` 恢复对应 `LocalContext`。

【5】演示脚本策略
  - `timer_task_tour`：停在「开 STIE」，再停在**首次** `emit_task_switch`，保证多数环境可复现。
  - 若需专门等定时器：使用 `timer_break_if_any`（可能长时间不命中）。
"""
        )


SYM_TASK_SWITCH = "tg_rcore_tutorial_ch3_show::lec4_lab3::emit_task_switch"
SYM_TIMER = "tg_rcore_tutorial_ch3_show::lec4_lab3::emit_timer_interrupt"
LINE_ENABLE_STIMER = 164


class TimerTaskTourCmd(gdb.Command):
    """自动化：main.rs:164（开 STIE）→ 首次 emit_task_switch（轮转切换）。"""

    def __init__(self) -> None:
        super().__init__("timer_task_tour", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("自动化演示：timer_task_tour")

        try:
            _ex("delete")
        except gdb.error:
            pass

        _ex(f"break main.rs:{LINE_ENABLE_STIMER}")
        print(
            f"[1] break main.rs:{LINE_ENABLE_STIMER}（`sie::set_stimer()`）→ continue。\n"
        )
        _ex("continue")

        _banner("阶段 1 — 打开 sie::STIE（定时器中断使能位）")
        try:
            _ex("show_csrs")
        except gdb.error:
            pass
        try:
            _ex("ch3stage")
        except gdb.error:
            pass
        _ex("delete")

        _ex(f"break {SYM_TASK_SWITCH}")
        print(
            f"[2] break {SYM_TASK_SWITCH} → continue，直到**首次**任务切换可观测点。\n"
            "    （常见为某次 yield/轮转；串口将有 `kp=task_switch`。）\n"
        )
        _ex("continue")

        _banner("阶段 2 — 任务切换：emit_task_switch（外层轮转）")
        print(
            "对照 main.rs：`prev_i`/`i` 更新后调用 `lec4_lab3::emit_task_switch`。\n"
            "下一轮流将 `execute` 另一 TCB 的 `LocalContext`，实现多道程序切换。\n"
        )
        try:
            _ex("show_csrs")
        except gdb.error:
            pass
        try:
            _ex("ch3stage")
        except gdb.error:
            pass
        try:
            _ex("frame")
            _ex("list")
        except gdb.error as e:
            print(f"(frame/list: {e})")

        _banner("演示结束")
        print(
            "进一步：手动 `break main.rs:188` 或在 `emit_timer_interrupt` 停表，"
            "配合 `show_csrs` 观察 SupervisorTimer。\n"
            "可选命令：`timer_break_if_any`（仅尝试等定时器断点）。\n"
        )


class TimerBreakIfAnyCmd(gdb.Command):
    """仅设断点 emit_timer_interrupt 并 continue（可能长时间不命中；用 Ctrl+C 中断）。"""

    def __init__(self) -> None:
        super().__init__(
            "timer_break_if_any", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True
        )

    def invoke(self, argument: str, from_tty: bool) -> None:
        print(
            f"在 `{SYM_TIMER}` 设断点并 continue。"
            "若环境未投递 S 态定时器中断，可能一直运行不停止——请 Ctrl+C 后 `kill` QEMU。\n"
        )
        try:
            _ex("delete")
        except gdb.error:
            pass
        _ex(f"break {SYM_TIMER}")
        _ex("continue")


TimerTaskIntroCmd()
TimerTaskTourCmd()
TimerBreakIfAnyCmd()
