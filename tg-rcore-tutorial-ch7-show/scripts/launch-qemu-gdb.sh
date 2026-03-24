#!/usr/bin/env bash
# 在 crate 根目录启动 QEMU（GDB stub :1234 + 上电暂停 -S），配合 ch7-show 的 fs.img。
#
# 用法：
#   cd tg-rcore-tutorial-ch7-show
#   bash scripts/launch-qemu-gdb.sh
#   PROFILE=release bash scripts/launch-qemu-gdb.sh
#
# 另一终端（推荐带 Python 的 GDB）：
#   riscv-none-elf-gdb-py3 -x gdb/ch7.gdb
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

PROFILE="${PROFILE:-debug}"
TARGET="riscv64gc-unknown-none-elf"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/${TARGET}/release/tg-rcore-tutorial-ch7-show"
else
  cargo build
  ELF="${ROOT}/target/${TARGET}/debug/tg-rcore-tutorial-ch7-show"
fi

FS_IMG="${ROOT}/target/${TARGET}/${PROFILE}/fs.img"
if [[ ! -f "${FS_IMG}" ]]; then
  echo "error: fs.img not found at ${FS_IMG} (build user apps first)" >&2
  exit 1
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF not found: ${ELF}" >&2
  exit 1
fi

echo "ELF:    ${ELF}"
echo "fs.img: ${FS_IMG}"
echo ""
echo "Starting QEMU with -s -S. Connect:"
echo "  cd ${ROOT}"
echo "  riscv-none-elf-gdb-py3 -x gdb/ch7.gdb"
echo ""
echo "在 GDB 中加载 Python 扩展后："
echo "  source scripts/gdb_ch7_pipe_signal.py"
echo "  ch7help"
echo "  ch7break_ipc    # catch syscall: pipe2/kill/sigaction/sigprocmask/sigreturn + 常用断点"
echo "  continue"
echo ""

exec qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -drive "file=${FS_IMG},if=none,format=raw,id=x0" \
  -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
  -kernel "${ELF}" \
  -s \
  -S
