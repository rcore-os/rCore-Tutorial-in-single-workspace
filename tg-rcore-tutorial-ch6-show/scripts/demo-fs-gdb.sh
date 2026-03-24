#!/usr/bin/env bash
# 一键提示：先起 QEMU（GDB 端口），再给出连接 GDB + 加载 Python 的命令。
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "终端 A — 启动 QEMU（冻结在复位）："
echo "  cd ${ROOT} && bash scripts/launch-qemu-gdb.sh"
echo ""
echo "终端 B — GDB + Python 可视化命令："
echo "  cd ${ROOT}"
echo "  riscv-none-elf-gdb-py3 -x gdb/ch6.gdb"
echo "  (gdb) source scripts/gdb_ch6_fs_tour.py"
echo "  (gdb) ch6_story"
echo "  (gdb) continue"
echo "  (gdb) ch6stage"
echo "  (gdb) show_satp"
echo "  (gdb) show_virtio_mmio"
echo "  (gdb) ch6_next"
