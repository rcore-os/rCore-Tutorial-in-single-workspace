fn main() {
    use std::{env, fs, path::PathBuf};

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // 只在 RISC-V64 架构上使用链接脚本
    if target_arch == "riscv64" {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        let ld_out = out_dir.join("linker.ld");
        fs::write(&ld_out, NOBIOS_LINKER).unwrap_or_else(|err| {
            panic!("failed to write linker script to {}: {}", ld_out.display(), err)
        });
        println!("cargo:rustc-link-arg=-T{}", ld_out.display());

        let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let ld_root = root.join("linker.ld");
        fs::write(&ld_root, NOBIOS_LINKER).unwrap_or_else(|err| {
            panic!("failed to write linker script to {}: {}", ld_root.display(), err)
        });
    }

    println!("cargo:rerun-if-changed=build.rs");
}

const NOBIOS_LINKER: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_m_start)
M_BASE_ADDRESS = 0x80000000;
S_BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = M_BASE_ADDRESS;
    .text.m_entry : {
        *(.text.m_entry)
    }
    .text.m_trap : {
        *(.text.m_trap)
    }
    .bss.m_stack : {
        *(.bss.m_stack)
    }
    .bss.m_data : {
        *(.bss.m_data)
    }
    . = S_BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        *(.bss.uninit)
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
}";
