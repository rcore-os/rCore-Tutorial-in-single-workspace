#!/usr/bin/env bash
# 一键提示：如何并行运行 QEMU(GDB stub) 与 riscv-none-elf-gdb-py3 观察 ch5-show 调度。
# 不自动拉起图形/终端多路复用；仅打印可复制命令。
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cat <<EOF
=== ch5-show: QEMU + GDB 双终端演示 ===

终端 A (${ROOT}):
  bash scripts/launch-qemu-gdb.sh

终端 B (${ROOT}):
  riscv-none-elf-gdb-py3 -x gdb/ch5.gdb

GDB 内:
  source scripts/gdb_ch5_proc_sched.py
  ch5sched_lesson
  continue
  (在 rust_main 或 find_next 停下后)
  ch5break_sched
  continue
  ch5tour_snapshot
  show_csrs

串口侧（QEMU 终端）可对照内核打印:
  grep 方向: LEC7-LAB5 | BACKTRACE
EOF
