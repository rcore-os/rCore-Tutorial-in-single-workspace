# 与 scripts/launch-qemu-gdb.sh 配合：在 crate 根目录执行
#   riscv-none-elf-gdb -x gdb/ch2-trap.gdb
#
# 用于观察 ch2 批处理系统的特权级切换过程：
#   ROM -> SBI(M态) -> kernel(S态) -> user app(U态) -> trap -> kernel(S态)

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch2-show

target remote :1234

# ===== 启动链断点 =====
# M 态 SBI 入口
break _m_start
# S 态内核入口
break _start

# ===== 特权级切换观察断点 =====
# 批处理主函数（加载 app 前）
break run_batch
# 用户程序入口地址（0x80400000 是 ch2 的 APP_BASE）
break *0x80400000
# syscall 处理（U态ecall -> S态）
break handle_syscall

# 可选（需 riscv-none-elf-gdb-py3）:
#   source scripts/gdb_trap_stage.py
#   trapstage
#
# 批处理 / 系统调用 教学演示（自动化脚本）:
#   source scripts/gdb_batch_os_tour.py
#   batch_os_intro
#   batch_os_tour
# 或: bash scripts/demo-batch-os-gdb.sh
