# GDB Python 扩展：ch6-show 文件系统 / VirtIO / 进程与 fd 教学演示。
#
# 在 GDB 内：
#   source scripts/gdb_ch6_fs_tour.py
#
# 依赖：riscv-none-elf-gdb-py3（带 Python3 的 GDB）。

from __future__ import annotations

import gdb  # type: ignore

VIRTIO_MMIO = 0x1000_1000
KERNEL_LO = 0x8020_0000
KERNEL_HI = 0x8100_0000


def _pc() -> int:
    return int(gdb.parse_and_eval("$pc"))


def _read_csr(name: str) -> int | None:
    try:
        return int(gdb.parse_and_eval(f"${name}"))
    except gdb.error:
        return None


class Ch6StageCmd(gdb.Command):
    """判断当前 PC 所处阶段，并结合 satp 说明是否已开启 Sv39。"""

    def __init__(self) -> None:
        super().__init__("ch6stage", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        satp = _read_csr("satp")

        if pc < 0x8000_0000:
            stage = "QEMU reset / ROM（非课程 ELF 代码）"
        elif pc < KERNEL_LO:
            stage = "M 态 SBI（_m_start）— tg-rcore-tutorial-sbi"
        elif KERNEL_LO <= pc < KERNEL_HI:
            stage = "S 态内核代码区（含 rust_main / kernel_space / 调度）"
        elif VIRTIO_MMIO <= pc < VIRTIO_MMIO + 0x1000:
            stage = "VirtIO MMIO 窗口（访问磁盘前的寄存器轮询/队列）"
        elif satp is not None and (satp >> 60) == 8 and pc < 0x8000_0000:
            stage = "U 态用户程序（Sv39 用户 VA 空间）"
        else:
            stage = "S 态其它区域或用户地址（结合 show_satp 判断）"

        satp_str = ""
        if satp is not None:
            mode = (satp >> 60) & 0xF
            ppn = satp & ((1 << 44) - 1)
            mode_name = {0: "Bare", 8: "Sv39"}.get(mode, f"mode={mode}")
            satp_str = f"\n  satp: {mode_name} root_ppn={ppn:#x}"

        print(f"ch6stage: {stage}\n  $pc = {pc:#x}{satp_str}")


class ShowCsrsCmd(gdb.Command):
    """打印 ch6 调试常用 S 态 CSR。"""

    CSRS = [
        "sstatus",
        "scause",
        "sepc",
        "stval",
        "stvec",
        "satp",
        "sie",
        "sip",
    ]

    def __init__(self) -> None:
        super().__init__("show_csrs", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print("=== S 态 CSR（节选）===")
        for name in self.CSRS:
            val = _read_csr(name)
            if val is not None:
                extra = ""
                if name == "scause":
                    interrupt = (val >> 63) & 1
                    code = val & ((1 << 63) - 1)
                    extra = f"  [{'int' if interrupt else 'exc'} code={code}]"
                elif name == "satp":
                    mode = (val >> 60) & 0xF
                    ppn = val & ((1 << 44) - 1)
                    extra = f"  [Sv39 root_ppn={ppn:#x}]" if mode == 8 else f"  [mode={mode}]"
                print(f"  {name:8s} = {val:#018x}{extra}")
            else:
                print(f"  {name:8s} = <read fail>")


class ShowSatpCmd(gdb.Command):
    """解析 satp（分页模式 + 根页表 PPN）。"""

    def __init__(self) -> None:
        super().__init__("show_satp", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        satp = _read_csr("satp")
        if satp is None:
            print("satp: <read fail>")
            return
        mode = (satp >> 60) & 0xF
        ppn = satp & ((1 << 44) - 1)
        mode_name = {0: "Bare", 8: "Sv39", 9: "Sv48"}.get(mode, f"unknown({mode})")
        print("=== satp ===")
        print(f"  raw      = {satp:#018x}")
        print(f"  MODE     = {mode_name}")
        print(f"  root PPN = {ppn:#x}  (PA {ppn << 12:#x})")


class ShowVirtioMmioCmd(gdb.Command):
    """查看 VirtIO0 MMIO 映射区开头（magic/version/device_id 等）。"""

    def __init__(self) -> None:
        super().__init__("show_virtio_mmio", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        print(f"=== x/8wx {VIRTIO_MMIO:#x} (VirtIO MMIO) ===")
        try:
            gdb.execute(f"x/8wx {VIRTIO_MMIO:#x}")
        except gdb.error as e:
            print(f"(无法读内存: {e})")
        print("提示：magic 应为 0x74726976（'virt' 小端）。")


class Ch6StoryCmd(gdb.Command):
    """打印「virt 机启动 → 内核 → 块设备/文件系统 → 进程/fd」故事线（供课堂对照串口 [LEC9-LAB6]）。"""

    def __init__(self) -> None:
        super().__init__("ch6_story", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        story = """
┌─────────────────────────────────────────────────────────────────────┐
│  QEMU virt 加电                                                      │
│    → PC 进入 OpenSBI/内嵌 M 态（_m_start）                            │
│    → mret 进入 S 态内核 _start → rust_main                            │
├─────────────────────────────────────────────────────────────────────┤
│  内核建立 Sv39（kernel_space）                                       │
│    → 恒等映射内核代码/数据/堆                                         │
│    → 映射 MMIO 0x10001000（VirtIO-blk，对接 fs.img）                 │
│    → 映射 MultislotPortal 最高 VPN                                    │
├─────────────────────────────────────────────────────────────────────┤
│  首次访问 FS（Lazy）                                                  │
│    → BLOCK_DEVICE 初始化 VirtIO 队列                                  │
│    → EasyFileSystem::open 读超级块/位图/inode（块设备 read_block）    │
│    → read_all(open(\"initproc\")) 把 ELF 读入内存                     │
│    → Process::from_elf 建页表与 fd_table(0,1,2)                       │
├─────────────────────────────────────────────────────────────────────┤
│  调度循环                                                             │
│    → execute(portal): sret 到 U 态                                    │
│    → ecall: sepc 指向 ecall，a7=调用号，open/read/write/exec…         │
│    → 内核 translate 用户 VA，经 fd_table 访问 FileHandle → 磁盘      │
└─────────────────────────────────────────────────────────────────────┘
串口关键字：grep [LEC9-LAB6] — 与课堂 lec9「文件系统」幻灯对齐。
"""
        print(story)


class Ch6NextCmd(gdb.Command):
    """根据当前是否已停在断点上，给出下一步建议（不自动 continue）。"""

    def __init__(self) -> None:
        super().__init__("ch6_next", gdb.COMMAND_USER, gdb.COMPLETE_NONE, True)

    def invoke(self, argument: str, from_tty: bool) -> None:
        pc = _pc()
        tips = [
            "1) continue 到 rust_main 后: ch6stage ; show_satp",
            "2) 单步越过 FS::Lazy / read_all 可在 rust_main 对 initproc 加载设临时断点",
            f"3) 查看 MMIO: show_virtio_mmio  （基址 {VIRTIO_MMIO:#x}）",
            "4) 用户 ecall 后: show_csrs 看 scause=8(UserEnvCall), sepc 指向 ecall",
            "5) 对照 QEMU 串口输出中的 [LEC9-LAB6] 行理解动态路径",
        ]
        print(f"当前 $pc={pc:#x} — 建议步骤：")
        for t in tips:
            print(f"  {t}")


Ch6StageCmd()
ShowCsrsCmd()
ShowSatpCmd()
ShowVirtioMmioCmd()
Ch6StoryCmd()
Ch6NextCmd()

print(
    "[ch6-show] 已加载 gdb_ch6_fs_tour.py — "
    "ch6stage, show_csrs, show_satp, show_virtio_mmio, ch6_story, ch6_next"
)
