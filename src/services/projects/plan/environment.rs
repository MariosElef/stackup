//! Environments: which `.env.<tier>` is the active `.env`.
use std::path::Path;

use anyhow::{Result, bail};

use super::valid_tier;
use crate::models::{Args, Step};

/// What `env` runs.
pub(super) fn env(root: &Path, args: &Args) -> Result<Vec<Step>> {
    Ok({
        let tier = args.tier.as_deref().unwrap_or("local");
        if !valid_tier(tier) {
            bail!(
                "`{tier}` is not a tier name: use letters, digits and dashes, like local, dev or prod"
            );
        }
        let file = format!(".env.{tier}");
        // A project with only a `.env` already runs on its one environment, the local one.
        if tier == "local" && !root.join(&file).is_file() && root.join(".env").is_file() {
            bail!(".env is already the local environment: there is nothing to switch");
        }
        if !root.join(&file).is_file() {
            bail!("{file} not found: copy .env.example to {file} and fill it in");
        }
        vec![crate::services::projects::internal::copy(
            &format!("make {tier} the current tier"),
            root,
            &file,
            ".env",
        )]
    })
}
