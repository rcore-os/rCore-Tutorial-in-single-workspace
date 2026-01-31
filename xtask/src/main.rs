mod fs_pack;
mod user;
mod utils;

#[macro_use]
extern crate clap;

use clap::Parser;
use once_cell::sync::Lazy;
use crate::utils::{BinUtil, Cargo, Qemu};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

const TARGET_ARCH: &str = "riscv64gc-unknown-none-elf";

static PROJECT: Lazy<&'static Path> =
    Lazy::new(|| Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap());

static TARGET: Lazy<PathBuf> = Lazy::new(|| PROJECT.join("target").join(TARGET_ARCH));

#[derive(Parser)]
#[clap(name = "rCore-Tutorial")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Make(BuildArgs),
    Asm(AsmArgs),
    Qemu(QemuArgs),
}

fn main() {
    use Commands::*;
    match Cli::parse().command {
        Make(args) => {
            let _ = args.make();
        }
        Asm(args) => args.dump(),
        Qemu(args) => args.run(),
    }
}

#[derive(Args, Default)]
struct BuildArgs {
    /// chapter number
    #[clap(short, long)]
    ch: Option<u8>,
    /// lab or not
    #[clap(long)]
    lab: bool,
    /// exercise mode (load exercise test cases)
    #[clap(long)]
    exercise: bool,
    /// ci or not
    #[clap(long)]
    ci: bool,
    /// features
    #[clap(short, long)]
    features: Option<String>,
    /// log level
    #[clap(long)]
    log: Option<String>,
    /// build in release mode
    #[clap(long)]
    release: bool,
    /// build for no-bios mode (kernel at 0x80000000)
    #[clap(long)]
    nobios: bool,
    /// print external commands before executing
    #[clap(long)]
    print_cmd: bool,
    /// force using package-local target directory
    #[clap(long)]
    local_target: bool,
    /// prefer package-local target if present
    #[clap(long)]
    prefer_local_target: bool,
    /// explicitly specify package name (overrides --ch)
    #[clap(long)]
    pkg: Option<String>,
    /// explicitly specify package directory
    #[clap(long)]
    pkg_dir: Option<String>,
    /// explicitly specify package directory (preferred flag)
    #[clap(long)]
    dir: Option<String>,
}

impl BuildArgs {
    fn make(&self) -> PathBuf {
        let mut env: HashMap<&str, OsString> = HashMap::new();
        // Determine package name and possibly package directory.
        let package: String;
        let mut inferred_pkg_dir: Option<PathBuf> = None;

        if let Some(pkg_arg) = &self.pkg {
            // If user passed a package directory name (e.g. `myos`), prefer that if it contains Cargo.toml
            let cand_dir = PROJECT.join(pkg_arg);
            if cand_dir.join("Cargo.toml").exists() {
                // read package name from Cargo.toml
                if let Ok(contents) = std::fs::read_to_string(cand_dir.join("Cargo.toml")) {
                    if let Ok(value) = contents.parse::<toml::Value>() {
                        if let Some(name) = value.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                            package = name.to_string();
                            inferred_pkg_dir = Some(cand_dir);
                        } else {
                            package = pkg_arg.clone();
                        }
                    } else {
                        package = pkg_arg.clone();
                    }
                } else {
                    package = pkg_arg.clone();
                }
            } else {
                // treat as package name
                package = pkg_arg.clone();
                // try to infer directory by stripping "tg-" prefix
                if let Some(stripped) = package.strip_prefix("tg-") {
                    let d = PROJECT.join(stripped);
                    if d.join("Cargo.toml").exists() {
                        inferred_pkg_dir = Some(d);
                    }
                }
                // also try direct directory named as package
                if inferred_pkg_dir.is_none() {
                    let d = PROJECT.join(&package);
                    if d.join("Cargo.toml").exists() {
                        inferred_pkg_dir = Some(d);
                    }
                }
            }
        } else if let Some(dir_arg) = &self.dir {
            // user provided --dir: try to read package name from its Cargo.toml
            let cand_dir = Path::new(dir_arg);
            if cand_dir.join("Cargo.toml").exists() {
                if let Ok(contents) = std::fs::read_to_string(cand_dir.join("Cargo.toml")) {
                    if let Ok(value) = contents.parse::<toml::Value>() {
                        if let Some(name) = value.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                            package = name.to_string();
                            inferred_pkg_dir = Some(cand_dir.to_path_buf());
                        } else {
                            package = cand_dir.file_name().unwrap().to_string_lossy().to_string();
                            inferred_pkg_dir = Some(cand_dir.to_path_buf());
                        }
                    } else {
                        package = cand_dir.file_name().unwrap().to_string_lossy().to_string();
                        inferred_pkg_dir = Some(cand_dir.to_path_buf());
                    }
                } else {
                    package = cand_dir.file_name().unwrap().to_string_lossy().to_string();
                    inferred_pkg_dir = Some(cand_dir.to_path_buf());
                }
            } else {
                // fallback to ch-based behavior if dir not containing Cargo.toml
                match self.ch {
                        Some(1) => {
                            package = if self.lab { "tg-ch1-lab".to_string() } else { "tg-ch1".to_string() };
                            inferred_pkg_dir = Some(PROJECT.join(if self.lab { "ch1-lab" } else { "ch1" }));
                        }
                        Some(n) if (2..=8).contains(&n) => {
                            user::build_for(n, false, self.exercise, self.ci);
                            env.insert(
                                "APP_ASM",
                                TARGET
                                    .join("debug")
                                    .join("app.asm")
                                    .as_os_str()
                                    .to_os_string(),
                            );
                            package = format!("tg-ch{}", n);
                            inferred_pkg_dir = Some(PROJECT.join(format!("ch{}", n)));
                    }
                    _ => unreachable!(),
                }
            }
        } else if let Some(dir_arg) = &self.pkg_dir {
            // legacy pkg_dir handling
            let cand_dir = Path::new(dir_arg);
            if cand_dir.join("Cargo.toml").exists() {
                if let Ok(contents) = std::fs::read_to_string(cand_dir.join("Cargo.toml")) {
                    if let Ok(value) = contents.parse::<toml::Value>() {
                        if let Some(name) = value.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                            package = name.to_string();
                            inferred_pkg_dir = Some(cand_dir.to_path_buf());
                        } else {
                            package = dir_arg.clone();
                            inferred_pkg_dir = Some(cand_dir.to_path_buf());
                        }
                    } else {
                        package = dir_arg.clone();
                        inferred_pkg_dir = Some(cand_dir.to_path_buf());
                    }
                } else {
                    package = dir_arg.clone();
                    inferred_pkg_dir = Some(cand_dir.to_path_buf());
                }
            } else {
                // treat as package name
                package = dir_arg.clone();
                // try to infer directory by stripping "tg-" prefix
                if let Some(stripped) = package.strip_prefix("tg-") {
                    let d = PROJECT.join(stripped);
                    if d.join("Cargo.toml").exists() {
                        inferred_pkg_dir = Some(d);
                    }
                }
                if inferred_pkg_dir.is_none() {
                    let d = PROJECT.join(&package);
                    if d.join("Cargo.toml").exists() {
                        inferred_pkg_dir = Some(d);
                    }
                }
            }
        } else {
            // fallback to ch-based behavior
            match self.ch {
                Some(1) => package = if self.lab { "tg-ch1-lab".to_string() } else { "tg-ch1".to_string() },
                Some(n) if (2..=8).contains(&n) => {
                    user::build_for(n, false, self.exercise, self.ci);
                    env.insert(
                        "APP_ASM",
                        TARGET
                            .join("debug")
                            .join("app.asm")
                            .as_os_str()
                            .to_os_string(),
                    );
                    package = format!("tg-ch{}", n);
                }
                _ => unreachable!(),
            }
        }
        // 生成
        let mut all_features = Vec::new();
        if let Some(features) = &self.features {
            all_features.extend(features.split_whitespace());
        }
        if self.nobios {
            all_features.push("nobios");
        }

        let mut build = Cargo::build();
        // If we have a package directory (manifest), use --manifest-path to build that crate
        // prefer explicit `--dir` over legacy `--pkg-dir`
        let chosen_dir = if let Some(d) = &self.dir {
            Some(Path::new(d).to_path_buf())
        } else if let Some(d) = &self.pkg_dir {
            Some(Path::new(d).to_path_buf())
        } else if let Some(d) = &inferred_pkg_dir {
            Some(d.clone())
        } else {
            None
        };

        // If user provided a directory via --dir/--pkg-dir, use its Cargo.toml as manifest
        let manifest_path = chosen_dir.as_ref().map(|p| p.join("Cargo.toml"));

        if let Some(manifest) = &manifest_path {
            build.arg("--manifest-path").arg(manifest);
        } else {
            build.package(&package);
        }

        // If the user provided a directory, instruct cargo to use that directory's target
        if let Some(cd) = &chosen_dir {
            build.arg("--target-dir").arg(cd.join("target"));
        }

        build.conditional(!all_features.is_empty(), |cargo| {
            cargo.features(false, all_features.iter().copied());
        });
        build.optional(&self.log, |cargo, log| {
            cargo.env("LOG", log);
        });
        build.conditional(self.release, |cargo| {
            cargo.release();
        });
        build.target(TARGET_ARCH);
        for (key, value) in env {
            build.env(key, value);
        }
        if self.print_cmd {
            println!("[xtask] cargo: {}", build.to_command_string());
        }
        build.invoke();
        // If prefer_local_target/local_target semantics are requested, we will prefer
        // a package-local target if it exists. The returned ELF path will be chosen below.
        // compute default ELF path. If user specified --dir, use its `target/` as the base,
        // otherwise use workspace-level `target/`.
        let base_target_dir = if let Some(cd) = &chosen_dir {
            cd.join("target")
        } else {
            PROJECT.join("target")
        };

        let global_elf = base_target_dir
            .join(TARGET_ARCH)
            .join(if self.release { "release" } else { "debug" })
            .join(package.clone());

        // determine package directory (explicit --dir/--pkg-dir override, else inferred, else ch-based)
        let pkg_dir = if let Some(d) = &self.dir {
            std::path::PathBuf::from(d)
        } else if let Some(d) = &self.pkg_dir {
            std::path::PathBuf::from(d)
        } else if let Some(d) = inferred_pkg_dir {
            d
        } else {
            match self.ch {
                Some(1) => PROJECT.join(if self.lab { "ch1-lab" } else { "ch1" }),
                Some(n) if (2..=8).contains(&n) => PROJECT.join(format!("ch{}", n)),
                _ => PROJECT.to_path_buf(),
            }
        };

        let local_candidate = pkg_dir
            .join("target")
            .join(TARGET_ARCH)
            .join(if self.release { "release" } else { "debug" })
            .join(package);

        if self.local_target {
            if local_candidate.exists() {
                return local_candidate;
            } else {
                panic!("--local-target specified but local target not found: {}", local_candidate.display());
            }
        }

        if self.prefer_local_target && local_candidate.exists() {
            local_candidate
        } else {
            global_elf
        }
    }
}

#[derive(Args)]
struct AsmArgs {
    #[clap(flatten)]
    build: BuildArgs,
    /// Output file.
    #[clap(short, long)]
    console: Option<String>,
}

impl AsmArgs {
    fn dump(self) {
        let elf = self.build.make();
        let out = Path::new(std::env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(self.console.unwrap_or(format!("ch{}.asm", self.build.ch.unwrap_or(1))));
        println!("Asm file dumps to '{}'.", out.display());
        fs::write(out, BinUtil::objdump().arg(elf).arg("-d").output().stdout).unwrap();
    }
}

#[derive(Args)]
struct QemuArgs {
    #[clap(flatten)]
    build: BuildArgs,
    /// Path of executable qemu-system-x.
    #[clap(long)]
    qemu_dir: Option<String>,
    /// Number of hart (SMP for Symmetrical Multiple Processor).
    #[clap(long)]
    smp: Option<u8>,
    /// Port for gdb to connect. If set, qemu will block and wait gdb to connect.
    #[clap(long)]
    gdb: Option<u16>,
}

impl QemuArgs {
    fn run(self) {
        let elf = self.build.make();
        if let Some(p) = &self.qemu_dir {
            // optional search location for qemu binary
            let _ = p; // search not implemented in bundled qemu builder
        }
        let mut qemu = Qemu::system("riscv64");
        qemu.args(["-machine", "virt"]).arg("-nographic");

        if self.build.nobios {
            // No BIOS mode: kernel is loaded at 0x80000000
            qemu.args(["-bios", "none"]);
        } else {
            // SBI mode: use RustSBI, kernel is loaded at 0x80200000
            qemu.arg("-bios").arg(PROJECT.join("rustsbi-qemu.bin"));
        }

        let bin = objcopy(elf, true, self.build.print_cmd);
        qemu.arg("-kernel").arg(&bin)
            .args(["-smp", &self.smp.unwrap_or(1).to_string()])
            .args(["-m", "64M"])
            .args(["-serial", "mon:stdio"]);
        if self.build.ch.unwrap_or(1) > 5 {
            // Add VirtIO Device
            qemu.args([
                "-drive",
                format!(
                    "file={},if=none,format=raw,id=x0",
                    TARGET
                        .join(if self.build.release {
                            "release"
                        } else {
                            "debug"
                        })
                        .join("fs.img")
                        .into_os_string()
                        .into_string()
                        .unwrap()
                )
                .as_str(),
            ])
            .args([
                "-device",
                "virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0",
            ]);
        }
        qemu.optional(&self.gdb, |qemu, gdb| {
            qemu.args(["-S", "-gdb", &format!("tcp::{gdb}")]);
        });

        if self.build.print_cmd {
            println!("[xtask] qemu: {}", qemu.to_command_string());
        }

        qemu.invoke();
    }
}
fn objcopy(elf: impl AsRef<Path>, binary: bool, print_cmd: bool) -> PathBuf {
    let elf = elf.as_ref();
    let bin = elf.with_extension("bin");
    let mut b = BinUtil::objcopy();
    b.arg(elf).arg("--strip-all");
    b.conditional(binary, |binutil| {
        binutil.args(["-O", "binary"]);
    })
    .arg(&bin);
    if print_cmd {
        println!("[xtask] objcopy: {}", b.to_command_string());
    }
    b.invoke();
    bin
}
