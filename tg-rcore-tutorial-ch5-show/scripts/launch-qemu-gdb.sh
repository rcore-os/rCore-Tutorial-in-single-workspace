#!/usr/bin/env bash
# 在 crate 根目录启动 QEMU（GDB stub + 上电暂停），配合 riscv-none-elf-gdb-py3
# 观察：ROM → M 态 SBI → S 态内核 → 调度 find_next → 用户态 ecall → 进程切换。
#
# 用法：
#   cd tg-rcore-tutorial-ch5-show
#   bash scripts/launch-qemu-gdb.sh
#   PROFILE=release bash scripts/launch-qemu-gdb.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch5-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch5-show"
fi

if [[ ! -f "${ELF}" ]]; then
  echo "error: ELF not found: ${ELF}" >&2
  exit 1
fi

echo "ELF: ${ELF}"
echo ""
echo "另开终端，在仓库根目录执行："
echo "  riscv-none-elf-gdb-py3 -x gdb/ch5.gdb"
echo ""
echo "GDB 内加载 Python 扩展后可用："
echo "  source scripts/gdb_ch5_proc_sched.py"
echo "  ch5stage              # 判断 ROM / SBI / 内核 / 用户态"
echo "  ch5sched_lesson       # 打印进程/调度相关断点与学习步骤"
echo "  ch5break_sched        # 自动下 rust_main、find_next、syscall::handle 断点"
echo "  show_csrs / show_satp"
echo ""

exec qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel "${ELF}" \
  -s \
  -S
