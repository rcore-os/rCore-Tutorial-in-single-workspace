#!/usr/bin/env bash
set -eu

cargo build

exec qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch2 \
    -S \
    -gdb tcp::"${GDB_PORT:-1234}"
