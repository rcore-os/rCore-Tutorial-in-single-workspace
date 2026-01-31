# tg-xtask

`tg-xtask` is a small CLI helper (an `xtask`) to build and run rCore tutorial chapters and local kernel projects using Cargo and QEMU.

## Install
You can build the binary from source:

```bash
cargo build -p xtask --release
# binary will be at target/release/xtask
```

## Usage
`tg-xtask` is exposed via the workspace `cargo` aliases as `cargo make` and `cargo qemu` in this repository. If installed as a standalone crate, run the binary `xtask` with the same subcommands.

- Build a chapter (print commands):

```bash
cargo make --ch 1 --print-cmd
```

- Run a chapter in QEMU (print commands):

```bash
cargo qemu --ch 1 --print-cmd
```

- Build or run a package located in an arbitrary directory (not necessarily a workspace member):

```bash
# Build using the package at ./myos and set target-dir to ./myos/target
cargo make --dir ./myos --print-cmd --nobios

# Run in QEMU (no BIOS) using ./ch1 as package directory
cargo qemu --dir ./ch1 --print-cmd --nobios
```

Notes:
- `--dir <PATH>` tells `xtask` to use `<PATH>/Cargo.toml` as the manifest and sets Cargo's `--target-dir` to `<PATH>/target`. The final ELF will be placed at `<PATH>/target/riscv64gc-unknown-none-elf/{debug|release}/<pkg>`.
- `--print-cmd` prints the exact `cargo build`, `rust-objcopy` and `qemu-system-*` commands that will be executed.
- `--nobios` adds the `nobios` feature during build and starts QEMU with `-bios none` so the kernel is loaded at 0x80000000.

## Publishing notes
When publishing `tg-xtask` to crates.io ensure all dependencies are published (no `path = "..."` dependencies remain). The crate ships a small CLI and is intended to be used as a development helper.
