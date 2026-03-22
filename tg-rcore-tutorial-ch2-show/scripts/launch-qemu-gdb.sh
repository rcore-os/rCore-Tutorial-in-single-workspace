#!/usr/bin/env bash
# 在本 crate 根目录下启动 QEMU（GDB stub + 上电暂停），便于观察特权级切换。
# 用法：
#   cd tg-rcore-tutorial-ch2-show
#   bash scripts/launch-qemu-gdb.sh          # debug 构建
#   PROFILE=release bash scripts/launch-qemu-gdb.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch2-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch2-show"
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
echo "  riscv-none-elf-gdb -x gdb/ch2-trap.gdb"
echo ""
echo "Or manually:"
echo "  riscv-none-elf-gdb -ex 'file ${ELF}' -ex 'target remote :1234'"
echo ""
echo "Key observation points for privilege switching:"
echo "  - break _start                          # S-mode kernel entry"
echo "  - break handle_syscall                  # ecall from U-mode"
echo "  - break *0x80400000                     # U-mode app entry"
echo "  - watch \$sstatus                        # observe SPP bit changes"
echo ""

exec qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel "${ELF}" \
  -s \
  -S
