use std::process::Command;
use std::ffi::OsStr;

pub struct QemuBuilder {
    pub program: String,
    pub args: Vec<String>,
}

impl QemuBuilder {
    pub fn system(arch: &str) -> Self {
        QemuBuilder {
            program: format!("qemu-system-{}", arch),
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

    pub fn optional<T>(&mut self, opt: &Option<T>, f: impl FnOnce(&mut QemuBuilder, &T)) -> &mut Self {
        if let Some(v) = opt {
            f(self, v);
        }
        self
    }

    pub fn invoke(&self) {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        let status = cmd.status().expect("failed to execute qemu");
        if !status.success() {
            panic!("qemu failed");
        }
    }

    pub fn to_command_string(&self) -> String {
        let mut parts: Vec<String> = vec![self.program.clone()];
        parts.extend(self.args.clone());
        parts.join(" ")
    }
}
