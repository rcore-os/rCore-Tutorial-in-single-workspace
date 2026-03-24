#!/usr/bin/env bash
# 在本 crate 根目录启动 QEMU（GDB stub + 上电暂停），观察：
#   ROM → M 态 SBI → S 态内核 → VirtIO/easy-fs → 用户态与文件相关系统调用
#
# 用法：
#   cd tg-rcore-tutorial-ch6-show
#   bash scripts/launch-qemu-gdb.sh
#   PROFILE=release bash scripts/launch-qemu-gdb.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch6-show"
  FSIMG="${ROOT}/target/riscv64gc-unknown-none-elf/release/fs.img"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch6-show"
  FSIMG="${ROOT}/target/riscv64gc-unknown-none-elf/debug/fs.img"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF not found: ${ELF}" >&2
  exit 1
fi
if [[ ! -f "${FSIMG}" ]]; then
  echo "error: fs.img not found: ${FSIMG} (先 cargo build 生成磁盘镜像)" >&2
  exit 1
fi

echo "ELF:   ${ELF}"
echo "fs.img: ${FSIMG}"
echo ""
echo "另开终端："
echo "  cd ${ROOT}"
echo "  riscv-none-elf-gdb-py3 -x gdb/ch6.gdb"
echo ""
echo "=== ch6-show 调试建议 ==="
echo "  1. break rust_main        — 内核 C 入口，之后将访问 FS / initproc"
echo "  2. break kernel_space     — 观察 MMIO 映射（VirtIO 0x10001000）"
echo "  3. 加载 Python: source scripts/gdb_ch6_fs_tour.py"
echo "     ch6stage / show_satp / show_virtio_mmio / ch6_story"
echo ""

exec qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -drive "file=${FSIMG},if=none,format=raw,id=x0" \
  -device "virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0" \
  -kernel "${ELF}" \
  -s \
  -S
