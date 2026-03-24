#!/usr/bin/env bash
# 一键：后台启动 QEMU(GDB stub)，再用 riscv-none-elf-gdb-py3 运行并发可视化命令后进入交互。
#
# 用法（在 tg-rcore-tutorial-ch8-show 根目录）：
#   bash scripts/demo-concurrency-gdb.sh
#
# 环境变量：
#   GDB         默认 riscv-none-elf-gdb-py3
#   PROFILE     debug 或 release
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
GDB="${GDB:-riscv-none-elf-gdb-py3}"
PROFILE="${PROFILE:-debug}"

if ! command -v "${GDB}" >/dev/null 2>&1; then
  echo "error: 未找到 ${GDB}，请安装带 Python 的 RISC-V GDB 或设置 GDB=..." >&2
  exit 1
fi

if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release --manifest-path "${ROOT}/Cargo.toml"
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch8-show"
  FS_IMG="${ROOT}/target/riscv64gc-unknown-none-elf/release/fs.img"
else
  cargo build --manifest-path "${ROOT}/Cargo.toml"
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch8-show"
  FS_IMG="${ROOT}/target/riscv64gc-unknown-none-elf/debug/fs.img"
fi

QEMU_LOG="${ROOT}/target/qemu-ch8-show-gdb.log"
QEMU_PID_FILE="${ROOT}/target/qemu-ch8-show-gdb.pid"

mkdir -p "${ROOT}/target"

# 如已有 QEMU 占用 1234，先提示
if command -v ss >/dev/null 2>&1 && ss -ltn 2>/dev/null | grep -q ':1234'; then
  echo "warning: 端口 1234 已被占用；如非本演示的 QEMU，请先结束占用进程。" >&2
fi

echo "启动 QEMU（-s -S），日志: ${QEMU_LOG}"
qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -drive "file=${FS_IMG},if=none,format=raw,id=x0" \
  -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
  -kernel "${ELF}" \
  -s \
  -S >"${QEMU_LOG}" 2>&1 &
echo $! >"${QEMU_PID_FILE}"
sleep 0.4

cd "${ROOT}"

GDB_CMDS=$(mktemp)
trap 'rm -f "${GDB_CMDS}"' EXIT

cat >"${GDB_CMDS}" <<'EOF'
source scripts/gdb_ch8_concurrency.py
ch8_conc_tour
EOF

echo "连接 ${GDB}（工作目录 ${ROOT}）..."
"${GDB}" -q \
  -ex "set pagination off" \
  -ex "file ${ELF}" \
  -ex "target remote :1234" \
  -x "${GDB_CMDS}"

echo ""
echo "GDB 已退出。结束 QEMU (pid $(cat "${QEMU_PID_FILE}" 2>/dev/null || echo '?'))："
echo "  kill \$(cat ${QEMU_PID_FILE})"
