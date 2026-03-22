# 与 scripts/launch-qemu-gdb.sh 配合：在 crate 根目录执行
#   riscv-none-elf-gdb -x gdb/ch4.gdb
#
# 若使用 PROFILE=release 构建，请把下面 file 改为:
#   file target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch4-show

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch4-show

target remote :1234

# ===== 关键断点 =====

# M 态 SBI 入口（加电后 ROM 跳转到此）
break _m_start

# S 态内核入口（mret 后进入）
break _start

# 内核主函数（初始化 + 建立内核地址空间 + 加载 ELF）
break rust_main

# 调度函数（传送门 + 跨地址空间执行）
break schedule

# ===== 可选断点（取消注释以启用）=====

# 跨地址空间上下文切换核心（sret 到用户态 / trap 回内核态）
# break execute_naked

# M 态 trap 处理（S 态 ecall 到 M 态，用于 SBI 服务）
# break m_trap_vector

# ===== 提示 =====
# 使用 riscv-none-elf-gdb-py3 时可加载 Python 扩展：
#   source scripts/gdb_ch4_stages.py
#   ch4stage          -- 显示当前所处阶段（ROM/M-SBI/S-内核/U-用户）
#   show_csrs         -- 显示关键 CSR（含 satp 解析）
#   show_satp         -- 单独查看 satp（模式 + root_ppn）
#   watch_priv_switch -- 在 execute_naked 处设断点观察特权级切换
