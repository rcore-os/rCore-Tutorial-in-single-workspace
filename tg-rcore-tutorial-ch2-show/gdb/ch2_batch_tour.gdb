# 与 scripts/demo-batch-os-gdb.sh 配合：在 tg-rcore-tutorial-ch2-show 根目录执行
#   riscv-none-elf-gdb-py3 -x gdb/ch2_batch_tour.gdb
#
# release 构建时请把 file 行改为 release 路径。
# 连接 QEMU（-s -S）后：
#   batch_os_intro    — 静态说明
#   batch_os_tour     — 自动化分阶段演示（见 scripts/gdb_batch_os_tour.py）

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch2-show

target remote :1234

source scripts/gdb_trap_stage.py
source scripts/gdb_batch_os_tour.py

echo \n>>> 已加载 batch_os_intro / batch_os_tour。可先 batch_os_intro，再 batch_os_tour。\n
