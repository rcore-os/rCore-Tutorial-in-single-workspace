use std::process::Command;
use std::ffi::OsStr;

pub struct CargoBuilder {
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

impl CargoBuilder {
    pub fn build() -> Self {
        CargoBuilder {
            program: "cargo".to_string(),
            args: vec!["build".to_string()],
            envs: vec![],
        }
    }

    pub fn package(&mut self, name: &str) -> &mut Self {
        self.args.push("--package".to_string());
        self.args.push(name.to_string());
        self
    }

    pub fn features<I>(&mut self, _flag: bool, features: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        self.args.push("--features".to_string());
        let s = features
            .into_iter()
            .map(|f| f.as_ref().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(",");
        self.args.push(s);
        self
    }

    pub fn conditional(&mut self, cond: bool, f: impl FnOnce(&mut CargoBuilder)) -> &mut Self {
        if cond {
            f(self);
        }
        self
    }

    pub fn optional<T>(&mut self, opt: &Option<T>, f: impl FnOnce(&mut CargoBuilder, &T)) -> &mut Self {
        if let Some(v) = opt {
            f(self, v);
        }
        self
    }

    pub fn env<S: AsRef<OsStr>>(&mut self, key: &str, value: S) -> &mut Self {
        let v = value.as_ref().to_string_lossy().into_owned();
        self.envs.push((key.to_string(), v));
        self
    }

    pub fn release(&mut self) -> &mut Self {
        self.args.push("--release".to_string());
        self
    }

    pub fn target(&mut self, target: &str) -> &mut Self {
        self.args.push("--target".to_string());
        self.args.push(target.to_string());
        self
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

    pub fn invoke(&self) {
        let mut cmd = Command::new(&self.program);
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        cmd.args(&self.args);
        let status = cmd.status().expect("failed to execute cargo");
        if !status.success() {
            panic!("cargo build failed");
        }
    }

    pub fn to_command_string(&self) -> String {
        let mut parts: Vec<String> = vec![];
        for (k, v) in &self.envs {
            parts.push(format!("{}={}", k, v));
        }
        parts.push(self.program.clone());
        parts.extend(self.args.clone());
        parts.join(" ")
    }
}
