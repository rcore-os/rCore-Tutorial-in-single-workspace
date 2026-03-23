# GDB Python：ch4-show 内核/用户 Sv39 地址空间建立、布局展示与 satp 切换
#
# 依赖：
#   source scripts/gdb_ch4_stages.py
#
# 命令：
#   addr_space_intro   — 静态导读（对照 main.rs、process.rs、lec5_lab4）
#   addr_space_tour    — 自动断点：内核空间建立 → 首个用户进程 → 页表摘要 → 进程切换
#
# 使用 riscv-none-elf-gdb-py3；在 crate 根目录连接 QEMU（-machine virt -bios none -kernel ELF -s -S）

from __future__ import annotations

import sys

import gdb  # type: ignore


def _banner(title: str) -> None:
    line = "=" * 72
    print(f"\n{line}\n {title}\n{line}\n")
    sys.stdout.flush()


def _ex(cmd: str) -> None:
    gdb.execute(cmd, to_string=False)


def _read_csr(name: str) -> int | None:
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


def _dump_root_ptes() -> None:
    """根据当前 satp 解析根页表物理地址，打印前若干项 PTE（教学用）。"""
    satp = _read_csr("satp")
    if satp is None:
        print("  （无法读取 satp，跳过根页表转储）")
        return
    mode = (satp >> 60) & 0xF
    if mode != 8:
        print(f"  （satp MODE={mode}，非 Sv39，跳过根页表示例）")
        return
    ppn = satp & ((1 << 44) - 1)
    root_pa = ppn << 12
    print(f"  根页表物理地址 = root_ppn<<12 = {root_pa:#x}；前 8 个 8 字节 PTE：")
    try:
        _ex(f"x/8gx {root_pa:#x}")
    except gdb.error as e:
        print(f"  （x/ 失败: {e}）")


SYM_KERNEL = "tg_rcore_tutorial_ch4_show::lec5_lab4::emit_kernel_space_created"
SYM_PROC = "tg_rcore_tutorial_ch4_show::lec5_lab4::emit_process_created"
SYM_PTSUM = "tg_rcore_tutorial_ch4_show::lec5_lab4::emit_page_table_summary"
SYM_SWITCH = "tg_rcore_tutorial_ch4_show::lec5_lab4::emit_process_switch"


class AddrSpaceIntroCmd(gdb.Command):
    """打印 Sv39 内核/用户地址空间与 satp 切换的静态导读。"""

    def __init__(self) -> None:
        super().__init__("addr_space_intro", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("导读：内核地址空间、用户地址空间、传送门与 satp")

        print(
            """
【1】建立内核自己的地址空间 — `kernel_space()`（main.rs）
  - `AddressSpace::new()`，对 `KernelLayout` 各段做**恒等映射**（VPN≈PPN），标志如 `X_RV`/`_WRV`。
  - 映射内核堆区间 `[layout.end(), layout.start()+MEMORY)`。
  - 将 **MultislotPortal** 物理页映射到 **VPN::MAX**（最高虚页），使传送门代码在各地址空间同虚址可执行。
  - `satp::set(Sv39, ASID=0, root_ppn)` 激活内核页表，串口 `kp=kernel_space_created` / `kp=kernel_satp`。

【2】展示内核地址空间布局
  - 串口：`emit_init_observables`、`emit_kernel_space_created`（root_ppn、satp）。
  - GDB：`show_satp`、`show_csrs`；可用 `x/8gx <根页表PA>` 查看根页表项（`addr_space_tour` 会示例）。

【3】创建用户态任务地址空间 — `Process::new`（process.rs）
  - `AddressSpace::new()`，解析 ELF **LOAD** 段 `map` 到用户 VA；用户栈 2 页映射在 **VPN [(1<<26)-2, 1<<26)**；`sp = 1<<38`。
  - `satp = (8<<60) | root_ppn`（Sv39）；`ForeignContext { context, satp }`。
  - 在 `rust_main` 里把**内核根页表中传送门项**复制到用户根页表：`address_space.root()[portal_idx] = ks.root()[portal_idx]`。
  - 串口：`kp=process_created`、`kp=elf_segment`、`kp=page_table` + `page_table_content=Debug`。

【4】展示用户地址空间布局
  - 停在 `emit_page_table_summary` 时，内核正打印 `AddressSpace` 的 Debug（各 VPN 映射）。
  - 对照串口 `kp=elf_segment` 各段 VA 范围与 flags（`U_XRWV`）。

【5】任务切换时切换地址空间
  - `ForeignContext::execute` + Portal：在传送门虚页上切换 **satp** 进入用户；trap 回内核再切回。
  - 进程顺序退出时：`emit_process_switch` 打出 `new_satp`（下一进程根页表），见 `schedule` 中 EXIT 分支。

【6】相关 GDB 命令
  - `ch4stage`、`show_csrs`、`show_satp`（本目录 gdb_ch4_stages.py）。
"""
        )


class AddrSpaceTourCmd(gdb.Command):
    """自动化演示：内核 satp → 首进程创建 → 页表摘要输出 → 首次进程切换（satp 变）。"""

    def __init__(self) -> None:
        super().__init__("addr_space_tour", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        _banner("自动化演示：addr_space_tour")

        try:
            _ex("delete")
        except gdb.error:
            pass

        # --- 阶段 1：内核地址空间已建立、即将打印 kernel_space_created ---
        _ex(f"break {SYM_KERNEL}")
        print("[1] break emit_kernel_space_created → continue（内核页表已写好且已 satp::set）。\n")
        _ex("continue")

        _banner("阶段 1 — 内核 Sv39 地址空间（恒等映射 + 传送门）")
        print(
            "对照 main.rs `kernel_space`：`satp` 已为 Sv39，指向内核根页表。\n"
            "串口含 `[LEC5-LAB4] kp=kernel_space_created`。\n"
        )
        try:
            _ex("show_satp")
            _ex("show_csrs")
        except gdb.error:
            pass
        _dump_root_ptes()
        try:
            _ex("ch4stage")
        except gdb.error:
            pass
        try:
            _ex("frame")
            _ex("list")
        except gdb.error as e:
            print(f"(frame/list: {e})")
        _ex("delete")

        # --- 阶段 2：首个用户进程创建（satp 变为用户根页表）---
        _ex(f"break {SYM_PROC}")
        print("\n[2] break emit_process_created → continue（首个 ELF 装载完成）。\n")
        _ex("continue")

        _banner("阶段 2 — 用户进程地址空间（独立 root_ppn / satp）")
        print(
            "此时 `satp` CSR 仍为**当前 CPU 视角**（可能尚未切到用户）；"
            "串口 `kp=process_created` 给出该进程 `satp` 与 `entry`、`heap_bottom`。\n"
            "用户栈映射见 process.rs：`VPN[(1<<26)-2 .. 1<<26)`，`sp = 1<<38`。\n"
        )
        try:
            _ex("show_satp")
            _ex("ch4stage")
        except gdb.error:
            pass
        _ex("delete")

        # --- 阶段 3：页表摘要（Debug 打印用户 AddressSpace）---
        _ex(f"break {SYM_PTSUM}")
        print("\n[3] break emit_page_table_summary → continue（打印用户页表摘要）。\n")
        _ex("continue")

        _banner("阶段 3 — 用户地址空间布局（page_table_content）")
        print(
            "串口将输出 `[LEC5-LAB4] page_table_content=...`（内核 Debug 格式）。\n"
            "此处停在内核打印前/打印中，可对照 ELF 段与用户栈映射。\n"
        )
        try:
            _ex("frame")
            _ex("list")
        except gdb.error as e:
            print(f"(frame/list: {e})")
        _ex("delete")

        # --- 阶段 4：进程切换 — new_satp ---
        _ex(f"break {SYM_SWITCH}")
        print(
            "\n[4] break emit_process_switch → continue（首个进程 exit 后切换到下一进程）。\n"
            "    若仅有一个用户程序则无此事件。\n"
        )
        _ex("continue")

        _banner("阶段 4 — 切换地址空间：emit_process_switch（new_satp）")
        print(
            "串口：`kp=process_switch` 含 `new_satp`（下一进程根页表）。"
            "调度路径见 `schedule` EXIT 分支与 `ForeignContext::execute`。\n"
            "注意：此时 CPU 多在 S 态处理 trap，`satp` **CSR** 常为内核页表；"
            "下一用户地址空间请对照本函数参数 `new_satp` 与串口十六进制输出。\n"
        )
        try:
            _ex("show_satp")
            _ex("show_csrs")
            _ex("ch4stage")
        except gdb.error:
            pass
        try:
            _ex("frame")
            _ex("list")
        except gdb.error as e:
            print(f"(frame/list: {e})")
        _dump_root_ptes()

        _banner("演示结束")
        print(
            "进一步：`watch_priv_switch` + `si` 到 `sret` 观察 satp 与 U 态；"
            "详见 docs/ch4-gdb-walkthrough.md。\n"
        )


AddrSpaceIntroCmd()
AddrSpaceTourCmd()
