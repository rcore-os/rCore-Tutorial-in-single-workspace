# 与 scripts/launch-qemu-gdb.sh 配合；在 crate 根目录执行：
#   riscv-none-elf-gdb-py3 -x gdb/ch7.gdb
#
# release 构建时请修改 file 路径为:
#   target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch7-show

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch7-show

target remote :1234

# 加电 → SBI(M) → 内核(S)
break _m_start
break _start
break rust_main

# 可选：用户程序第一次 ecall 进入内核后，可在 GDB 中执行:
#   source scripts/gdb_ch7_pipe_signal.py
#   ch7help
#   ch7break_ipc
#   ch7stage
