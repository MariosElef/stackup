use std::path::Path;

use anyhow::{Context, Result, bail};

use super::os;

/// Whether `bin` is installed: it answers `--version`, or `-V` (ssh only knows that one).
pub fn have(bin: &str) -> bool {
    ["--version", "-V"].iter().any(|flag| {
        os::command(bin)
            .arg(flag)
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

/// Why a program could not be started: and, when it is the shell on a Windows machine without
/// Git for Windows, how to get it.
pub fn start_failure(prog: &str) -> String {
    if prog == "sh" && !os::shell_found() {
        format!("failed to start `sh`: {}", os::SHELL_HELP)
    } else {
        format!("failed to start `{prog}`")
    }
}

/// Docker is Docker Desktop on a Mac or Windows machine, and Docker Engine on Linux. Either way
/// it has to be running, which `doctor` checks.
fn docker_hint(os: &str) -> &'static str {
    match os {
        "macos" => {
            "install Docker Desktop for Mac and start it: https://docs.docker.com/desktop/setup/install/mac-install/"
        }
        "windows" => {
            "install Docker Desktop for Windows and start it: https://docs.docker.com/desktop/setup/install/windows-install/"
        }
        _ => {
            "install Docker Engine and the Compose plugin: https://docs.docker.com/engine/install/"
        }
    }
}

/// How to get a tool on Windows, where `winget` ships with the system.
fn windows_hint(bin: &str) -> Option<&'static str> {
    Some(match bin {
        "cargo" => "winget install Rustlang.Rustup, then open a new terminal",
        "uv" => "winget install astral-sh.uv",
        "gh" => "winget install GitHub.cli",
        "lazygit" => "winget install JesseDuffield.lazygit",
        "pnpm" => "winget install pnpm.pnpm",
        "mkcert" => "winget install FiloSottile.mkcert",
        "git" => "winget install Git.Git (it also brings the shell stackup's scripts run in)",
        "ssh" => "turn on the OpenSSH Client: Settings > Apps > Optional features",
        _ => return None,
    })
}

/// How to get a tool, for the messages that say it is missing.
pub fn install_hint(bin: &str) -> &'static str {
    if super::os::WINDOWS
        && let Some(hint) = windows_hint(bin)
    {
        return hint;
    }
    match bin {
        "cargo" => "install Rust: https://rustup.rs",
        "uv" => "install uv: https://docs.astral.sh/uv",
        "gh" => "install the GitHub CLI: https://cli.github.com",
        "lazygit" => {
            "brew install lazygit (macOS, Linux), scoop install lazygit (Windows), or https://github.com/jesseduffield/lazygit#installation"
        }
        "pnpm" => "install pnpm: https://pnpm.io/installation",
        // `topcoat new` is not in the crates.io release yet, and the CLI needs a recent Rust.
        "topcoat" => {
            "cargo install --git https://github.com/tokio-rs/topcoat topcoat-cli (needs Rust 1.99+: rustup update)"
        }
        "mkcert" => "install mkcert: https://github.com/FiloSottile/mkcert",
        "docker" => docker_hint(std::env::consts::OS),
        "git" => "install git",
        "ssh" => "install an SSH client",
        _ => "install it",
    }
}

/// Fails early, with a name, when a tool a choice depends on is missing.
pub fn need(bin: &str, why: &str) -> Result<()> {
    let found = os::command(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success());
    if !matches!(found, Ok(true)) {
        bail!("`{bin}` is required for {why} but was not found on PATH");
    }
    Ok(())
}

pub fn run(dir: &Path, prog: &str, args: &[&str]) -> Result<()> {
    println!("  $ {prog} {}", args.join(" "));
    let status = os::command(prog)
        .args(args)
        .current_dir(dir)
        .status()
        .with_context(|| start_failure(prog))?;
    if !status.success() {
        bail!("`{prog} {}` exited with {status}", args.join(" "));
    }
    Ok(())
}

pub fn capture(dir: &Path, prog: &str, args: &[&str]) -> Option<String> {
    let out = os::command(prog)
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Runs a step of a project's action, its output going straight to this terminal.
pub fn run_step(step: &crate::models::Step) -> Result<()> {
    let status = os::command(&step.prog)
        .args(&step.args)
        .envs(step.env.iter().map(|(k, v)| (k, v)))
        .current_dir(&step.cwd)
        .status()
        .with_context(|| start_failure(&step.prog))?;
    if !status.success() {
        bail!("`{}` exited with {status}", step.command_line());
    }
    Ok(())
}

/// Opens an address in the browser, with whatever this system opens addresses with.
pub fn open_url(url: &str) -> Result<()> {
    let (opener, before) = os::opener();
    os::command(opener)
        .args(before)
        .arg(url)
        .spawn()
        .with_context(|| format!("could not start `{opener}`"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_is_docker_desktop_on_a_mac_or_windows_and_the_engine_on_linux() {
        assert!(docker_hint("macos").contains("Docker Desktop for Mac"));
        assert!(docker_hint("windows").contains("Docker Desktop for Windows"));
        assert!(docker_hint("linux").contains("Docker Engine"));
        assert!(install_hint("docker").contains("docker.com"));
    }
}
