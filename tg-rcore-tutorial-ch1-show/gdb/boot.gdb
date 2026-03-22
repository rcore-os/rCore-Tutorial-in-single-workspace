# 与 scripts/launch-qemu-gdb.sh 配合：在 crate 根目录执行
#   riscv-none-elf-gdb -x gdb/boot.gdb
#
# 若使用 PROFILE=release 构建，请把下面 file 改为:
#   file target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch1-show

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch1-show

target remote :1234

break _m_start
break _start

# 可选（需 riscv-none-elf-gdb-py3）:
#   source scripts/gdb_bootstage.py
#   bootstage
