# 与 scripts/launch-qemu-gdb.sh 配合：在 crate 根目录执行
#   riscv-none-elf-gdb -x gdb/ch3.gdb
#
# 若使用 PROFILE=release 构建，请把下面 file 改为:
#   file target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch3-show

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch3-show

target remote :1234

# ===== 关键断点 =====

# M 态 SBI 入口（加电后 ROM 跳转到此）
break _m_start

# S 态内核入口（mret 后进入）
break _start

# 内核主函数（初始化 + 主循环）
break rust_main

# ===== 可选断点（取消注释以启用）=====

# 上下文切换核心（sret 到用户态 / trap 回内核态）
# break execute_naked

# M 态 trap 处理（S 态 ecall 到 M 态，用于 SBI 服务）
# break m_trap_vector

# ===== 提示 =====
# 使用 riscv-none-elf-gdb-py3 时可加载 Python 扩展：
#   source scripts/gdb_ch3_stages.py
#   ch3stage          -- 显示当前所处阶段
#   show_csrs         -- 显示关键 CSR
#   watch_priv_switch -- 在 sret 处设断点观察特权级切换
#
# 时钟中断（源码）+ 任务切换（自动化演示）：
#   source scripts/gdb_timer_task_tour.py
#   timer_task_intro
#   timer_task_tour
#   timer_break_if_any   # 可选：仅断在 emit_timer_interrupt（部分环境可能不命中）
# 或: bash scripts/demo-timer-task-gdb.sh
