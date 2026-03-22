#!/usr/bin/env bash
# 在本 crate 根目录下启动 QEMU（GDB stub + 上电暂停），
# 便于观察 ROM → SBI(M 态) → 内核(S 态) → 用户程序(U 态) 的执行过程。
#
# ch4-show 特有：虚拟内存 Sv39、跨地址空间传送门、进程独立页表。
#
# 用法：
#   cd tg-rcore-tutorial-ch4-show
#   bash scripts/launch-qemu-gdb.sh          # debug 构建
#   PROFILE=release bash scripts/launch-qemu-gdb.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

PROFILE="${PROFILE:-debug}"
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --release
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch4-show"
else
  cargo build
  ELF="${ROOT}/target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch4-show"
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
echo "  riscv-none-elf-gdb -x gdb/ch4.gdb"
echo ""
echo "Or manually:"
echo "  riscv-none-elf-gdb -ex 'file ${ELF}' -ex 'target remote :1234'"
echo ""
echo "=== 演示场景（ch4-show 地址空间） ==="
echo "  1. 应用加载：break process::Process::new，观察 ELF 段映射"
echo "  2. 内核→用户态：watch_priv_switch 或 break execute_naked，观察 sret + satp 切换"
echo "  3. 用户→内核态（ecall）：在 stvec trap 入口设断点"
echo "  4. 进程页表：show_satp 查看 satp，break schedule 观察页表切换"
echo "  5. 时钟中断抢占：在 SupervisorTimer 分支设断点"
echo ""

exec qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel "${ELF}" \
  -s \
  -S
