#!/usr/bin/env bash
# QEMU virt + riscv-none-elf-gdb-py3：演示 ch3-show 时钟中断（SupervisorTimer）与轮转任务切换。
#
# 用法（在 tg-rcore-tutorial-ch3-show 根目录）：
#   bash scripts/demo-timer-task-gdb.sh
#       仅启动 QEMU（-s -S），打印在另一终端连接 GDB 的命令。
#
#   AUTO=1 bash scripts/demo-timer-task-gdb.sh
#       自动执行 timer_task_tour 并 kill（适合录屏；串口输出较多）。
#
# 勿使用 `--features coop` 构建（否则不编程时钟，可能等不到定时器中断）。
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
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch3-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch3-show"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF 不存在: ${ELF}" >&2
  exit 1
fi

PY_STAGES="${ROOT}/scripts/gdb_ch3_stages.py"
PY_TOUR="${ROOT}/scripts/gdb_timer_task_tour.py"
for f in "$PY_STAGES" "$PY_TOUR"; do
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
  echo "终端 2（crate 根目录）："
  echo "  cd ${ROOT}"
  echo "  ${GDB} -ex \"file ${ELF}\" -ex \"target remote :${GDB_PORT}\" \\"
  echo "    -ex \"source scripts/gdb_ch3_stages.py\" \\"
  echo "    -ex \"source scripts/gdb_timer_task_tour.py\""
  echo ""
  echo "在 GDB 内："
  echo "  timer_task_intro"
  echo "  timer_task_tour"
  echo "  timer_break_if_any   # 可选：仅尝试命中定时器断点（可能久等）"
  echo ""
  echo "或：  ${GDB} -x gdb/ch3_timer_task.gdb"
  echo ""
  exec "$QEMU" \
    -machine virt \
    -nographic \
    -bios none \
    -kernel "${ELF}" \
    -s \
    -S
fi

echo "AUTO=1：启动 QEMU，随后 ${GDB} 执行 timer_task_tour 并结束进程。"
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
  -ex "timer_task_tour" \
  -ex "kill" \
  -ex "quit"
GDB_STATUS=$?
set -e

cleanup || true
trap - EXIT
wait "${QEMU_PID}" 2>/dev/null || true

exit "${GDB_STATUS}"
