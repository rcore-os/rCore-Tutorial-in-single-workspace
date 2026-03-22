#!/bin/bash
# 第四讲（lec4）× Lab3：可观测标签与 backtrace 全量验收
set -euo pipefail

# build.rs 从上次产物提取 .symtab 函数符号；预构建一次确保符号表非空
echo "=== 第一次构建（生成符号表）==="
cargo build 2>&1 >/dev/null
echo "=== 第二次构建（嵌入符号表）==="
cargo build 2>&1 >/dev/null

# 程序以 panic 结束，QEMU 返回非零码；用 || true 捕获全部输出
echo "=== 运行 ==="
OUTPUT=$(cargo run 2>&1) || true

required=(
  # ---- lec4 静态知识点 ----
  "[LEC4-LAB3] kp=multiprog app_count="
  "[LEC4-LAB3] kp=tcb_layout"
  "[LEC4-LAB3] kp=task_context type=LocalContext"
  "[LEC4-LAB3] kp=scheduling_model mode=preemptive"
  "[LEC4-LAB3] kp=task_lifecycle states=init,ready,running,exit"
  "[LEC4-LAB3] kp=privilege_levels user=U(0) kernel=S(1) sbi=M(3)"
  "[LEC4-LAB3] kp=syscalls write=64 exit=93 yield=124"
  "[LEC4-LAB3] kp=context_switch_mech via=LocalContext::execute"
  "[LEC4-LAB3] kp=kernel_stack sp=0x"
  "[LEC4-LAB3] kp=compile_info target=riscv64gc-unknown-none-elf"
  "[LEC4-LAB3] kp=control_flow path=_start->rust_main->load_apps->main_loop->shutdown"

  # ---- lec4 动态知识点 ----
  "[LEC4-LAB3] kp=app_load app_id=0"
  "[LEC4-LAB3] kp=first_enter_user app_id=0"
  "[LEC4-LAB3] kp=syscall_trap"
  "[LEC4-LAB3] kp=task_exit"
  "[LEC4-LAB3] kp=task_switch"
  "[LEC4-LAB3] kp=exception_kill"

  # ---- 正常执行 backtrace ----
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
  'name="multitask_os"'
  "value=-1"
  "count=42"
  "flag=true"

  # ---- panic 路径 backtrace ----
  "[PANIC]"
  "index out of bounds"
  "buggy_access"
  "trigger_error"
  "index=10"
  'kind="oob"'
)

missing=()
for s in "${required[@]}"; do
  if ! echo "$OUTPUT" | grep -Fq "$s"; then
    missing+=("$s")
  fi
done

if ((${#missing[@]} == 0)); then
  echo "LEC4 Lab3 observables: PASSED (${#required[@]} required checks)"
else
  echo "LEC4 Lab3 observables: FAILED — missing lines:"
  printf '  - %s\n' "${missing[@]}"
  echo "--- actual output ---"
  echo "$OUTPUT"
  exit 1
fi

# ---- 软性检查（timer/yield 受 QEMU 时序影响，不强制要求）----
optional=(
  "[LEC4-LAB3] kp=timer_interrupt"
  "[LEC4-LAB3] kp=yield_switch"
)
opt_found=0
for s in "${optional[@]}"; do
  if echo "$OUTPUT" | grep -Fq "$s"; then
    opt_found=$((opt_found + 1))
  fi
done
echo "  optional checks: $opt_found/${#optional[@]} found (timer/yield, QEMU-dependent)"
exit 0
