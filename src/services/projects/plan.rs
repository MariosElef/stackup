//! What each action runs, as a list of steps. The command line runs them directly and the menu
//! runs them on its in-app terminal, so there is one place that knows what `up` means.
use std::path::Path;

use anyhow::{Result, bail};

use super::certs;
use crate::models::{Action, Args, Facts, Project, Step};

const DATABASES: [&str; 3] = ["postgres", "redis", "mongo"];

fn compose_step(label: &str, root: &Path, args: &[&str]) -> Step {
    let mut all = vec!["compose"];
    all.extend(args);
    Step::new(label, root, "docker", &all)
}

/// A tier is a short name: it becomes part of a file name.
pub fn valid_tier(tier: &str) -> bool {
    !tier.is_empty() && tier.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

mod containers;
mod environment;
mod migrations;
mod project;
mod repo;
mod ship;
#[cfg(test)]
mod tests;
mod volumes;

pub use containers::{DATABASES_ONLY, start};
// Checked by the menu's tests too (which run on Unix).
#[cfg(all(test, unix))]
pub use project::check_removable;

pub fn plan(action: Action, p: &Project, facts: &Facts, args: &Args) -> Result<Vec<Step>> {
    let root = p.path.as_path();
    if !facts.exists {
        bail!("{} has no docker-compose.yml", root.display());
    }
    if !action.applies(facts) {
        bail!("`{}` does not apply to this project", action.name());
    }
    let steps = match action {
        Action::Up => containers::up(root, facts, args),
        Action::Down => vec![compose_step("stop everything", root, &["down"])],
        Action::Ps => vec![compose_step("show the containers", root, &["ps"])],
        Action::Stop => containers::stop(root),
        Action::Images => containers::images(p),
        Action::Clean => containers::clean(p),
        Action::Logs => containers::logs(root, args),
        Action::Build => containers::build(root, args),
        Action::Dev => containers::dev(p, facts, args)?,
        Action::Migrate => migrations::migrate(root, facts)?,
        Action::MigrationNew => migrations::new(root, facts, args)?,
        Action::MigrationRevert => migrations::revert(root, facts)?,
        Action::Ci => vec![Step::new("run the CI checks", root, "sh", &["ci.sh"])],
        Action::Fmt => ship::fmt(root, facts)?,
        Action::CargoClean => ship::cargo_clean(p)?,
        Action::Volumes => volumes::list(p),
        Action::VolumeAdd => volumes::add(p, args)?,
        Action::VolumeBackup => volumes::backup(p, args)?,
        Action::VolumeRestore => volumes::restore(p, args)?,
        Action::Env => environment::env(root, args)?,
        Action::Certs => certs::steps(root),
        Action::Hooks => repo::hooks(root),
        Action::Doctor => project::doctor(root)?,
        Action::Remove => project::remove(root, args)?,
        Action::GitOpen => repo::git_open(root),
        Action::GitInit => repo::git_init(root, facts),
        Action::GitRepo => repo::git_repo(p, args)?,
        Action::Deploy => ship::deploy(p, args)?,
    };
    Ok(steps)
}
