#!/usr/bin/env bash
# 在本 crate 根目录下启动 QEMU（GDB stub + 上电暂停），便于观察 ROM → _m_start → _start。
# 用法：
#   cd tg-rcore-tutorial-ch1-show
#   bash scripts/launch-qemu-gdb.sh          # debug 构建
#   PROFILE=release bash scripts/launch-qemu-gdb.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch1-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch1-show"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF not found: ${ELF}" >&2
  exit 1
fi

echo "ELF: ${ELF}"
echo ""
echo "Starting QEMU with -s (GDB :1234) and -S (frozen at reset). Open another terminal:"
echo ""
echo "  cd ${ROOT}"
echo "  riscv-none-elf-gdb -x gdb/boot.gdb"
echo ""
echo "Or manually:"
echo "  riscv-none-elf-gdb -ex 'file ${ELF}' -ex 'target remote :1234'"
echo ""

exec qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel "${ELF}" \
  -s \
  -S
