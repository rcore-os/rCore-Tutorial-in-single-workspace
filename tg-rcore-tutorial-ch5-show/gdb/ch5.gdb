# 与 scripts/launch-qemu-gdb.sh 配合；在 crate 根目录执行：
#   riscv-none-elf-gdb-py3 -x gdb/ch5.gdb
#
# release 构建时请修改下方 file 路径为 release/tg-rcore-tutorial-ch5-show

set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch5-show

target remote :1234

# 启动链：固件 / 内核入口 / 主初始化
break _m_start
break _start
break rust_main

# 可选：进程调度与系统调用（亦可 GDB 内执行 ch5break_sched）
# break tg_rcore_tutorial_task_manage::proc_manage::PManager::find_next

echo \n[ch5.gdb] 已连接 :1234。建议：source scripts/gdb_ch5_proc_sched.py 后运行 ch5sched_lesson\n
