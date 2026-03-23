#!/usr/bin/env bash
# QEMU + riscv-none-elf-gdb-py3：演示 ch4-show 内核/用户 Sv39 地址空间与 satp 切换。
#
# 用法（在 tg-rcore-tutorial-ch4-show 根目录）：
#   bash scripts/demo-address-space-gdb.sh
#       仅启动 QEMU（-s -S），打印连接 GDB 与加载 Python 的命令。
#
#   AUTO=1 bash scripts/demo-address-space-gdb.sh
#       自动执行 addr_space_tour 并 kill QEMU。
#
# 环境变量：GDB、QEMU、GDB_PORT、PROFILE
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

GDB="${GDB:-riscv-none-elf-gdb-py3}"
QEMU="${QEMU:-qemu-system-riscv64}"
GDB_PORT="${GDB_PORT:-1234}"
AUTO="${AUTO:-0}"

for cmd in "$GDB" "$QEMU"; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: 未找到命令: $cmd" >&2
    exit 1
  fi
done

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch4-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch4-show"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF 不存在: ${ELF}" >&2
  exit 1
fi

PY_STAGES="${ROOT}/scripts/gdb_ch4_stages.py"
PY_TOUR="${ROOT}/scripts/gdb_address_space_tour.py"
for f in "$PY_STAGES" "$PY_TOUR"; do
  if [[ ! -f "$f" ]]; then
    echo "error: 缺少脚本: $f" >&2
    exit 1
  fi
done

echo "ELF: ${ELF}"
echo ""

if [[ "${AUTO}" != "1" ]]; then
  echo "终端 1："
  echo "  ${QEMU} -machine virt -nographic -bios none -kernel \"${ELF}\" -s -S"
  echo ""
  echo "终端 2："
  echo "  cd ${ROOT}"
  echo "  ${GDB} -ex \"file ${ELF}\" -ex \"target remote :${GDB_PORT}\" \\"
  echo "    -ex \"source scripts/gdb_ch4_stages.py\" \\"
  echo "    -ex \"source scripts/gdb_address_space_tour.py\""
  echo ""
  echo "  (gdb) addr_space_intro"
  echo "  (gdb) addr_space_tour"
  echo ""
  echo "或： ${GDB} -x gdb/ch4_address_space.gdb"
  echo ""
  exec "$QEMU" \
    -machine virt \
    -nographic \
    -bios none \
    -kernel "${ELF}" \
    -s \
    -S
fi

echo "AUTO=1：执行 addr_space_tour 并结束 QEMU。"
echo ""

"$QEMU" \
  -machine virt \
  -nographic \
  -bios none \
  -kernel "${ELF}" \
  -s \
  -S &
QEMU_PID=$!

cleanup() {
  if kill -0 "${QEMU_PID}" 2>/dev/null; then
    kill -9 "${QEMU_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

sleep 0.35

set +e
"$GDB" -batch \
  -ex "set confirm off" \
  -ex "set pagination off" \
  -ex "set architecture riscv:rv64" \
  -ex "file ${ELF}" \
  -ex "target remote :${GDB_PORT}" \
  -ex "source ${PY_STAGES}" \
  -ex "source ${PY_TOUR}" \
  -ex "addr_space_tour" \
  -ex "kill" \
  -ex "quit"
GDB_STATUS=$?
set -e

cleanup || true
trap - EXIT
wait "${QEMU_PID}" 2>/dev/null || true

exit "${GDB_STATUS}"
