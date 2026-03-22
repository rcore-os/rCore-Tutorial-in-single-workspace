#!/bin/bash
# 第二讲（lec2）× Lab1：可观测标签全量验收（对齐 LabUnit observables / 01-kp-lec2-ch1.md）
set -euo pipefail

OUTPUT=$(cargo run 2>&1)

required=(
  "Hello, world!"
  "[LEC2-LAB1] kp=curriculum"
  "[LEC2-LAB1] kp=compile_abi target=riscv64gc-unknown-none-elf"
  "[LEC2-LAB1] kp=boot_chain model=nobios"
  "[LEC2-LAB1] kp=mem_layout M_BASE=0x80000000 S_BASE=0x80200000"
  "[LEC2-LAB1] kp=libos_stack sp=0x"
  "[LEC2-LAB1] kp=callconv demo=extern_C_add"
  "[LEC2-LAB1] kp=callconv result=0x0000000000000030"
  "[LEC2-LAB1] kp=sbi_vs_syscall"
  "[LEC2-LAB1] kp=control_flow path=_start->rust_main->shutdown(false)"
  "[LEC2-LAB1] kp=panic_contract"
  "[BACKTRACE] note=fp_unwind_riscv64_s0_same_layout_as_axbacktrace"
  "[BACKTRACE] #0 fp="
  "[BACKTRACE] #1 fp="
  "[BACKTRACE] end=ra_null_bottom_of_chain"
)

missing=()
for s in "${required[@]}"; do
  if ! echo "$OUTPUT" | grep -Fq "$s"; then
    missing+=("$s")
  fi
done

if ((${#missing[@]} == 0)); then
  echo "LEC2 Lab1 observables: PASSED (${#required[@]} checks)"
  exit 0
fi

echo "LEC2 Lab1 observables: FAILED — missing lines:"
printf '  - %s\n' "${missing[@]}"
echo "--- actual output ---"
echo "$OUTPUT"
exit 1
