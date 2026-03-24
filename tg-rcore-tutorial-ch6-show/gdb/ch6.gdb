# 与 scripts/launch-qemu-gdb.sh 配合，在 crate 根目录执行：
#   riscv-none-elf-gdb-py3 -x gdb/ch6.gdb
#
# release 构建时请修改下面 file 路径为 release 目录下的 ELF。

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch6-show

target remote :1234

# ===== 关键断点（启动链）=====

break _m_start
break _start
break rust_main
break kernel_space

# ===== 可选：文件与块设备路径（符号依赖优化级别）=====
# break tg_rcore_tutorial_ch6_show::fs::FS::init
# （Lazy 静态初始化符号名因版本而异，建议结合 ch6story 文本断点 rust_main 后单步）

# ===== Python 扩展（需 riscv-none-elf-gdb-py3）=====
#   source scripts/gdb_ch6_fs_tour.py
#   ch6stage          — 当前阶段 + satp
#   show_csrs         — 常用 S 态 CSR
#   show_satp         — 解析 satp
#   show_virtio_mmio  — 查看 0x10001000 附近 MMIO
#   ch6_story         — 打印「virt 启动 → 文件系统 → 进程」故事线
#   ch6_next          — 建议下一步手工命令（教学提示）
