#!/usr/bin/env bash
# 在 tg-rcore-tutorial-ch8-show 根目录启动 QEMU（GDB stub + 上电暂停），
# 挂载 easy-fs（fs.img），便于观察 virt 启动后线程调度与同步原语相关路径。
#
# 用法：
#   cd tg-rcore-tutorial-ch8-show
#   bash scripts/launch-qemu-gdb.sh
#   PROFILE=release bash scripts/launch-qemu-gdb.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch8-show"
  FS_IMG="${ROOT}/target/riscv64gc-unknown-none-elf/release/fs.img"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch8-show"
  FS_IMG="${ROOT}/target/riscv64gc-unknown-none-elf/debug/fs.img"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF not found: ${ELF}" >&2
  exit 1
fi
if [[ ! -f "${FS_IMG}" ]]; then
  echo "error: fs.img not found: ${FS_IMG} (cargo build 应已生成)" >&2
  exit 1
fi

echo "ELF: ${ELF}"
echo "FS:  ${FS_IMG}"
echo ""
echo "另开终端连接 GDB（建议带 Python）："
echo "  cd ${ROOT}"
echo "  riscv-none-elf-gdb-py3 -x gdb/ch8.gdb"
echo ""
echo "=== 演示建议（线程 / 同步）==="
echo "  1. source scripts/gdb_ch8_concurrency.py  然后  ch8_conc_tour"
echo "  2. 在 rust_main 断点 continue，观察 initproc 加载后调度"
echo "  3. break 或 rbreak re_enque —— 看信号量/锁/条件变量唤醒路径"
echo "  4. rbreak make_current_blocked —— 看 P/V、mutex_lock、condvar_wait 阻塞"
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
