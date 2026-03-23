# 由 scripts/show-boot-chain-gdb.sh 在「file」「target remote」之后 source。
# 展示：复位后寄存器、ROM 启动片段、SBI 与内核入口各 10 条指令，然后 kill QEMU。

echo \n========== 1. virt 复位后、执行第一条指令前：全部通用寄存器与 PC ==========\n
info registers

echo \n========== 2. QEMU virt 内置启动代码（复位向量，当前 $pc）==========\n
printf "说明：QEMU 使用 -bios none -kernel ELF 时，CPU 从内置 ROM 开始；当前 PC 常为 0x1000 附近。\n"
printf "复位向量 PC = 0x%lx\n", $pc
x/10i $pc

echo \n========== 3. SBI（M 态，tg-rcore-tutorial-sbi）第一条指令地址及连续 10 条汇编 ==========\n
printf "物理入口 _m_start = 0x80000000（与 build.rs 中 M_BASE_ADDRESS 一致）\n"
x/10i 0x80000000

echo \n========== 4. tg-rcore-tutorial-ch1-show 内核（S 态）第一条指令地址及连续 10 条汇编 ==========\n
printf "物理入口 _start = 0x80200000（与 build.rs 中 S_BASE_ADDRESS 一致）\n"
x/10i 0x80200000

echo \n（结束：终止 QEMU 进程，避免断开后继续执行 guest。）\n
kill
quit
