# 与 scripts/launch-qemu-gdb.sh 配合：在 crate 根目录执行
#   riscv-none-elf-gdb-py3 -x gdb/ch8.gdb
#
# release 构建时请把 file 改为:
#   file target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch8-show

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch8-show

target remote :1234

# ===== 启动与内核 =====

break _m_start
break _start
break rust_main

# ===== 调度与线程（第八章粒度 = 线程）=====

# 主循环中选取就绪线程（符号名随优化可能变化，可用 rbreak 补充）
rbreak find_next

# 同步阻塞 / 唤醒（依赖 debug 符号）
rbreak make_current_blocked
rbreak make_current_suspend
rbreak re_enque

# ===== 可选：用户态返回路径 =====
# break execute_naked

# ===== Python 扩展（riscv-none-elf-gdb-py3）=====
#   source scripts/gdb_ch8_concurrency.py
#   ch8stage
#   ch8_conc_tour
#   ch8_break_concurrency
