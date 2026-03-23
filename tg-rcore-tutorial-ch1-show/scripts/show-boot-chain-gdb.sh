#!/usr/bin/env bash
# 使用 qemu-system-riscv64 + riscv-none-elf-gdb-py3 展示 virt 上电暂停时：
#   - 执行第一条指令前的寄存器
#   - QEMU 内置 ROM 启动代码（当前 PC 处反汇编）
#   - SBI（_m_start @ 0x80000000）第一条及随后 9 条指令
#   - 内核（_start @ 0x80200000）第一条及随后 9 条指令
#
# 用法（在 tg-rcore-tutorial-ch1-show 根目录）：
#   bash scripts/show-boot-chain-gdb.sh
#   PROFILE=release bash scripts/show-boot-chain-gdb.sh
#
# 依赖：qemu-system-riscv64、riscv-none-elf-gdb-py3（PATH 可寻址）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

GDB="${GDB:-riscv-none-elf-gdb-py3}"
QEMU="${QEMU:-qemu-system-riscv64}"
GDB_PORT="${GDB_PORT:-1234}"

for cmd in "$GDB" "$QEMU"; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: 未找到命令: $cmd（请安装或加入 PATH，或通过 GDB=/path QEMU=/path 指定）" >&2
    exit 1
  fi
done

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch1-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch1-show"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF 不存在: ${ELF}" >&2
  exit 1
fi

GDB_SCRIPT="${ROOT}/gdb/show_boot_chain.gdb"
if [[ ! -f "${GDB_SCRIPT}" ]]; then
  echo "error: 找不到 GDB 脚本: ${GDB_SCRIPT}" >&2
  exit 1
fi

# 若端口已被占用，可另开终端结束占用进程或设置 GDB_PORT
if command -v ss >/dev/null 2>&1; then
  if ss -ltn 2>/dev/null | grep -q ":${GDB_PORT} "; then
    echo "warning: 端口 ${GDB_PORT} 已被监听，GDB 连接可能失败。可设置 GDB_PORT=其他端口。" >&2
  fi
fi

echo "ELF: ${ELF}"
echo "启动 ${QEMU}（-machine virt -bios none -kernel … -s -S），随后 ${GDB} 批处理连接并打印启动链信息…"
echo ""

# 后台 QEMU：GDB stub + 上电即停（尚未执行第一条指令）
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

# 等待 stub 就绪
sleep 0.35

set +e
"$GDB" -batch \
  -ex "set confirm off" \
  -ex "set pagination off" \
  -ex "set architecture riscv:rv64" \
  -ex "file ${ELF}" \
  -ex "target remote :${GDB_PORT}" \
  -x "${GDB_SCRIPT}"
GDB_STATUS=$?
set -e

# 若 GDB 未通过 kill 结束 QEMU，兜底清理
cleanup || true
trap - EXIT
wait "${QEMU_PID}" 2>/dev/null || true

exit "${GDB_STATUS}"
