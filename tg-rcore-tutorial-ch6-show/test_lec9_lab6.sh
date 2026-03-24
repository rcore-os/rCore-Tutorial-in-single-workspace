#!/usr/bin/env bash
# 检查串口是否输出 [LEC9-LAB6] 知识点锚点（需 timeout 与 qemu 在 PATH）。
set -euo pipefail
cd "$(dirname "$0")"
# 无 fs.img 时先完整构建
if [[ ! -f target/riscv64gc-unknown-none-elf/debug/fs.img ]]; then
  echo "note: 正在完整构建以生成 fs.img …"
  cargo build
fi
if ! command -v timeout >/dev/null 2>&1; then
  echo "需要 coreutils timeout" >&2
  exit 1
fi
OUT=$(mktemp)
timeout 25s cargo run 2>&1 | tee "$OUT" || true
if grep -q '\[LEC9-LAB6\]' "$OUT"; then
  echo "OK: 发现 [LEC9-LAB6] 输出"
  rm -f "$OUT"
  exit 0
fi
echo "FAIL: 未找到 [LEC9-LAB6]" >&2
rm -f "$OUT"
exit 1
