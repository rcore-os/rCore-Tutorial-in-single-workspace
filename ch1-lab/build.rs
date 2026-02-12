fn main() {
    use std::{env, fs, path::PathBuf};

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_NOBIOS");
    println!("cargo:rerun-if-env-changed=TARGET");

    // 仅在 RISC-V 目标下注入链接脚本，避免主机平台 dry-run 链接失败。
    if !env::var("TARGET").unwrap_or_default().starts_with("riscv64") {
        return;
    }

    let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(
        ld,
        if env::var("CARGO_FEATURE_NOBIOS").is_ok() {
            NOBIOS_LINKER
        } else {
            LINKER
        },
    )
    .unwrap();
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

const LINKER: &[u8] = b"
OUTPUT_ARCH(riscv)
SECTIONS {
    .text 0x80200000 : {
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
