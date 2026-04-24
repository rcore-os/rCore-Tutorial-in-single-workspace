set pagination off
set confirm off
set print pretty on

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch2
target remote :1234

break tg_rcore_tutorial_ch2::rust_main
continue
echo \n--- breakpoint: rust_main ---\n
bt

echo \n--- global: apps metadata ---\n
set language c
print/x (void*)&apps
print/x (void*)apps
x/11gx (void*)&apps
set language auto

echo \n--- global: boot stack ---\n
info address tg_rcore_tutorial_ch2::_start::STACK

break tg_rcore_tutorial_ch2::handle_syscall
continue
echo \n--- breakpoint: handle_syscall ---\n
bt
info args
print/x *ctx
