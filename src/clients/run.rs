use std::{path::Path, process::Command};

use anyhow::{Context, Result, bail};

pub fn have(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Fails early, with a name, when a tool a choice depends on is missing.
pub fn need(bin: &str, why: &str) -> Result<()> {
    let found = Command::new(bin)
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
    let status = Command::new(prog)
        .args(args)
        .current_dir(dir)
        .status()
        .with_context(|| format!("failed to start `{prog}`"))?;
    if !status.success() {
        bail!("`{prog} {}` exited with {status}", args.join(" "));
    }
    Ok(())
}

pub fn capture(dir: &Path, prog: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(prog)
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}
