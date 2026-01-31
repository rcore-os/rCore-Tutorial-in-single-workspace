use std::process::Command;
use std::ffi::OsStr;

pub struct BinUtil {
    pub program: String,
    pub args: Vec<String>,
}

impl BinUtil {
    pub fn objcopy() -> Self {
        BinUtil {
            program: "rust-objcopy".to_string(),
            args: vec![],
        }
    }

    pub fn objdump() -> Self {
        BinUtil {
            program: "rust-objdump".to_string(),
            args: vec![],
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, a: S) -> &mut Self {
        let s = a.as_ref().to_string_lossy().into_owned();
        self.args.push(s);
        self
    }

    pub fn args<I, S>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for s in iter {
            self.args.push(s.as_ref().to_string());
        }
        self
    }

    pub fn conditional(&mut self, cond: bool, f: impl FnOnce(&mut BinUtil)) -> &mut Self {
        if cond {
            f(self);
        }
        self
    }

    pub fn invoke(&self) {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        let status = cmd.status().expect("failed to execute objcopy");
        if !status.success() {
            panic!("objcopy failed");
        }
    }

    pub fn output(&self) -> std::process::Output {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        cmd.output().expect("failed to execute command")
    }

    pub fn to_command_string(&self) -> String {
        let mut parts: Vec<String> = vec![self.program.clone()];
        parts.extend(self.args.clone());
        parts.join(" ")
    }
}
