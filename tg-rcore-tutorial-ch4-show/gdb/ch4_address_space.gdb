# 与 scripts/demo-address-space-gdb.sh 配合，在 tg-rcore-tutorial-ch4-show 根目录：
#   riscv-none-elf-gdb-py3 -x gdb/ch4_address_space.gdb
#
# release 请修改 file 路径。
# 连接后：addr_space_intro / addr_space_tour

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch4-show

target remote :1234

source scripts/gdb_ch4_stages.py
source scripts/gdb_address_space_tour.py

echo \n>>> 已加载 addr_space_intro / addr_space_tour（依赖 ch4stage、show_satp、show_csrs）。\n
