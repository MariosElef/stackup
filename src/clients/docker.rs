use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::os;

/// The compose file with its variables filled in from `.env`, as JSON.
pub fn config(root: &Path) -> Result<Value> {
    let out = os::command("docker")
        .args(["compose", "config", "--format", "json"])
        .current_dir(root)
        .output()
        .context("could not start `docker`: is it installed? `stackup doctor` checks")?;
    if !out.status.success() {
        bail!(
            "`docker compose config` failed:\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(serde_json::from_slice(&out.stdout)?)
}
