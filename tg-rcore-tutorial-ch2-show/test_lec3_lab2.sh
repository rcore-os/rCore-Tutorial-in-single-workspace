#!/bin/bash
# 第三讲（lec3）× Lab2：可观测标签全量验收（对齐 LabUnit observables / 02-kp-lec3-ch2.md）
set -euo pipefail

# build.rs 从上次产物提取 .symtab 函数符号；预构建一次确保符号表非空
cargo build 2>&1 >/dev/null
# 程序以 panic 结束再继续批处理；用 || true 捕获全部输出
OUTPUT=$(cargo run 2>&1) || true

required=(
  # ---- 基本输出 ----
  "Hello, world from ch2-show!"

  # ---- lec3 知识点标签 ----
  "[LEC3-LAB2] kp=isolation"
  "[LEC3-LAB2] kp=privilege_u_s"
  "[LEC3-LAB2] kp=trap_mechanism"
  "[LEC3-LAB2] kp=context_save_restore"
  "[LEC3-LAB2] kp=syscall_abi"
  "[LEC3-LAB2] kp=sepc_invariant"
  "[LEC3-LAB2] kp=batch_execution"
  "[LEC3-LAB2] kp=user_stack"
  "[LEC3-LAB2] kp=fence_i"
  "[LEC3-LAB2] kp=compile_abi target=riscv64gc-unknown-none-elf"
  "[LEC3-LAB2] kp=mem_layout M_BASE=0x80000000 S_BASE=0x80200000"

  # ---- 正常 backtrace 验证 ----
  "[BACKTRACE] note=fp_unwind_riscv64_s0_symtab_line_params"
  "[BACKTRACE] #0 fp="
  "[BACKTRACE]   fn="
  "[BACKTRACE] #1 fp="
  "print_backtrace"
  "bt_depth"
  "rust_main"
  "at src/main.rs:"
  "at src/stackwalk.rs:"
  "id=42"
  'name="batch_os"'
  "value=-1"
  "count=42"
  "flag=true"

  # ---- panic 路径 backtrace 验证 ----
  "[PANIC]"
  "index out of bounds"
  "buggy_access"
  "trigger_error"
  "index=10"
  'kind="oob"'

  # ---- 批处理运行时知识点 ----
  "[LEC3-LAB2] kp=batch_execution app_idx="
  "[LEC3-LAB2] kp=syscall_abi_demo a7="
  "[LEC3-LAB2] kp=trap_dispatch"
  "[LEC3-LAB2] kp=sepc_invariant_demo"
  "delta=4"
  "load app"

  # ---- 用户程序输出 ----
  "Hello, world!"
)

missing=()
for s in "${required[@]}"; do
  if ! echo "$OUTPUT" | grep -Fq "$s"; then
    missing+=("$s")
  fi
done

if ((${#missing[@]} == 0)); then
  echo "LEC3 Lab2 observables: PASSED (${#required[@]} checks)"
  exit 0
fi

echo "LEC3 Lab2 observables: FAILED — missing lines:"
printf '  - %s\n' "${missing[@]}"
echo "--- actual output ---"
echo "$OUTPUT"
exit 1
