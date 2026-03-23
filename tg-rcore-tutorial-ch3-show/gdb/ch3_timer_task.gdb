# 与 scripts/demo-timer-task-gdb.sh 配合：在 tg-rcore-tutorial-ch3-show 根目录执行
#   riscv-none-elf-gdb-py3 -x gdb/ch3_timer_task.gdb
#
# release 构建请修改 file 路径。
# 连接 QEMU（-s -S）后：
#   timer_task_intro
#   timer_task_tour

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch3-show

target remote :1234

source scripts/gdb_ch3_stages.py
source scripts/gdb_timer_task_tour.py

echo \n>>> 已加载 timer_task_intro / timer_task_tour / timer_break_if_any（依赖 ch3stage、show_csrs）。\n
