#!/usr/bin/env bash
# QEMU virt + riscv-none-elf-gdb-py3：演示 ch2-show 批处理加载应用、创建任务、U 态执行、
# 系统调用、exit、fence.i 与下一应用（见 scripts/gdb_batch_os_tour.py）。
#
# 用法（在 tg-rcore-tutorial-ch2-show 根目录）：
#   bash scripts/demo-batch-os-gdb.sh
#       仅启动 QEMU（-s -S），打印在另一终端连接 GDB 的命令。
#
#   AUTO=1 bash scripts/demo-batch-os-gdb.sh
#       启动 QEMU 后自动运行 batch_os_tour 并 kill（适合讲义录屏；串口输出较多）。
#
#   PROFILE=release bash scripts/demo-batch-os-gdb.sh
# 环境变量：GDB、QEMU、GDB_PORT 可覆盖默认命令与端口。
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
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch2-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch2-show"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF 不存在: ${ELF}" >&2
  exit 1
fi

PY_TRAP="${ROOT}/scripts/gdb_trap_stage.py"
PY_TOUR="${ROOT}/scripts/gdb_batch_os_tour.py"
for f in "$PY_TRAP" "$PY_TOUR"; do
  if [[ ! -f "$f" ]]; then
    echo "error: 缺少脚本: $f" >&2
    exit 1
  fi
done

echo "ELF: ${ELF}"
echo ""

if [[ "${AUTO}" != "1" ]]; then
  echo "终端 1（本脚本将启动 QEMU，上电暂停）："
  echo "  ${QEMU} -machine virt -nographic -bios none -kernel \"${ELF}\" -s -S"
  echo ""
  echo "终端 2（crate 根目录，使用 Python 扩展 GDB）："
  echo "  cd ${ROOT}"
  echo "  ${GDB} -ex \"file ${ELF}\" -ex \"target remote :${GDB_PORT}\" \\"
  echo "    -ex \"source scripts/gdb_trap_stage.py\" \\"
  echo "    -ex \"source scripts/gdb_batch_os_tour.py\""
  echo ""
  echo "在 GDB 内顺序执行："
  echo "  batch_os_intro      # 静态导读（可选）"
  echo "  batch_os_tour       # 自动化分阶段演示"
  echo ""
  echo "或使用 init 文件："
  echo "  ${GDB} -x gdb/ch2_batch_tour.gdb"
  echo "  （release 时请编辑 gdb/ch2_batch_tour.gdb 中的 file 路径）"
  echo ""
  exec "$QEMU" \
    -machine virt \
    -nographic \
    -bios none \
    -kernel "${ELF}" \
    -s \
    -S
fi

# ----- AUTO=1：批处理自动演示 -----
echo "AUTO=1：启动 QEMU，随后 ${GDB} 将执行 batch_os_tour 并结束进程。"
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
  -ex "source ${PY_TRAP}" \
  -ex "source ${PY_TOUR}" \
  -ex "batch_os_tour" \
  -ex "kill" \
  -ex "quit"
GDB_STATUS=$?
set -e

cleanup || true
trap - EXIT
wait "${QEMU_PID}" 2>/dev/null || true

exit "${GDB_STATUS}"
