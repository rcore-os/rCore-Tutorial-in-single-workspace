fn main() {
    use std::{env, fs, path::PathBuf};

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rerun-if-env-changed=APP_ASM");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_NOBIOS");

    let nobios = env::var("CARGO_FEATURE_NOBIOS").is_ok();

    let linker_script = if nobios {
        tg_linker::NOBIOS_SCRIPT
    } else {
        tg_linker::SCRIPT
    };

    let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(ld, linker_script).unwrap();
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}
