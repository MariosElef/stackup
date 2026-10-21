//! The project itself: checking the machine, and removing the project safely.
use std::{env, path::Path};

use anyhow::{Context, Result, bail};

use crate::{
    models::{Args, Step},
    services::projects::git::{self, Host},
};

/// What `doctor` runs.
pub(super) fn doctor(root: &Path) -> Result<Vec<Step>> {
    Ok({
        let me = env::current_exe()?;
        vec![Step::new(
            "check this machine",
            root,
            &me.to_string_lossy(),
            &["doctor", "--path", &root.to_string_lossy()],
        )]
    })
}

/// What `remove` runs.
pub(super) fn remove(root: &Path, args: &Args) -> Result<Vec<Step>> {
    Ok({
        let home = crate::clients::os::home_dir();
        check_removable(root, &env::current_dir()?, home.as_deref())?;
        let parent = root.parent().unwrap_or(root);
        // The repository on GitHub or GitLab goes first: if that fails, nothing local has
        // been deleted yet.
        let mut steps = match args.remove_remote.as_deref() {
            None => vec![],
            Some(key) => {
                let host = Host::parse(key).context("remove from github or gitlab")?;
                let (found, slug) = git::read(root)
                    .and_then(|g| g.remote)
                    .and_then(|url| git::hosted(&url))
                    .context("this project has no GitHub or GitLab repository to delete")?;
                if found != host {
                    bail!(
                        "this project's repository is on {}, not {}",
                        found.label(),
                        host.label()
                    );
                }
                git::delete_repo_steps(root, host, &slug)
            }
        };
        steps.extend(vec![
                // Data volumes go too: they hold the project's databases. If Docker is not
                // running there is nothing to stop, so the delete goes on.
                Step::new(
                    "stop the containers and delete their data",
                    root,
                    "sh",
                    &[
                        "-c",
                        "docker compose down --volumes --remove-orphans || echo 'could not stop the containers: is Docker running?'",
                    ],
                ),
                crate::services::projects::internal::remove("delete the project's folder", parent, root),
            ]);
        steps
    })
}

/// Whether `root` is safe to delete as a project. Deleting a folder cannot be undone, so
/// anything that is not clearly one project's own folder is refused.
pub fn check_removable(root: &Path, cwd: &Path, home: Option<&Path>) -> Result<()> {
    if !root.join("docker-compose.yml").is_file() {
        bail!(
            "{} has no docker-compose.yml: it is not a project",
            root.display()
        );
    }
    if root.components().count() < 4 {
        bail!(
            "{} is too close to the top of the disk to delete",
            root.display()
        );
    }
    if home.is_some_and(|h| h.starts_with(root)) {
        bail!("{} holds your home folder", root.display());
    }
    if cwd.starts_with(root) {
        bail!("you are inside {}: move out of it first", root.display());
    }
    Ok(())
}
