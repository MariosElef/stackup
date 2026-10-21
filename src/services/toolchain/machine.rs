//! Setting up this machine: what to run to get a missing tool. Only what is a few clear commands
//! is installed from here; the rest (Rust itself, Docker, git) is left to the person, with the
//! instructions beside it.
mod install;
#[cfg(test)]
mod tests;

pub use install::{InstallPlan, install_plan, rust_steps};

use super::github::Github;

/// What the install plan needs to know about this machine. Behind a trait so it can be tested
/// without installing anything.
pub trait Machine {
    fn has(&self, bin: &str) -> bool;
    /// The Rust that `cargo` uses by default: (1, 97) for 1.97.1.
    fn active_rust(&self) -> Option<(u32, u32)>;
    /// The toolchains rustup has installed, by name.
    fn toolchains(&self) -> Vec<String>;
    /// Whether `gh` is signed in, and which SSH key there is.
    fn github(&self) -> Github;
}

/// This machine, asked by running programs.
pub struct System;

fn output(prog: &str, args: &[&str]) -> Option<String> {
    let out = crate::clients::os::command(prog).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `1.99.0-aarch64-apple-darwin` or `rustc 1.97.1 (...)` -> (1, 99).
pub fn rust_version(text: &str) -> Option<(u32, u32)> {
    let digits = text
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?;
    let mut parts = digits.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

impl Machine for System {
    fn has(&self, bin: &str) -> bool {
        crate::clients::run::have(bin)
    }

    fn active_rust(&self) -> Option<(u32, u32)> {
        rust_version(&output("rustc", &["--version"])?)
    }

    fn github(&self) -> Github {
        super::github::read(&crate::clients::os::home_dir().unwrap_or_default())
    }

    fn toolchains(&self) -> Vec<String> {
        output("rustup", &["toolchain", "list"])
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .map(str::to_string)
            .collect()
    }
}

/// The Topcoat CLI needs at least this Rust.
pub const TOPCOAT_RUST: (u32, u32) = (1, 99);

/// The Topcoat revision stackup is tested with: its projects and Dockerfiles are made with it.
pub const TOPCOAT_CLI_REV: &str = "b7a85c34";

/// A machine for tests: it has some programs, runs some Rust, and has some toolchains installed.
#[cfg(test)]
pub struct Fake {
    pub programs: &'static [&'static str],
    pub rust: Option<(u32, u32)>,
    pub toolchains: &'static [&'static str],
    pub github: Github,
}

#[cfg(test)]
pub fn fake(programs: &'static [&'static str]) -> Fake {
    Fake {
        programs,
        rust: None,
        toolchains: &[],
        github: Github::default(),
    }
}

#[cfg(test)]
impl Machine for Fake {
    fn has(&self, bin: &str) -> bool {
        self.programs.contains(&bin)
    }
    fn active_rust(&self) -> Option<(u32, u32)> {
        self.rust
    }
    fn github(&self) -> Github {
        self.github.clone()
    }
    fn toolchains(&self) -> Vec<String> {
        self.toolchains.iter().map(|t| (*t).to_string()).collect()
    }
}
