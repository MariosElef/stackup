//! Git: opening lazygit, starting a repository, creating the GitHub repository, the hooks.
use std::{env, path::Path};

use anyhow::{Result, bail};

use crate::{
    clients::run::capture,
    models::{Args, Facts, Project, Step},
    services::projects::git::{self},
};

/// What `hooks` runs.
pub(super) fn hooks(root: &Path) -> Vec<Step> {
    vec![Step::new(
        "turn on the gitleaks hook",
        root,
        "git",
        &["config", "core.hooksPath", ".githooks"],
    )]
}

/// What `git open` runs.
/// Run as it is: its colors, config and keys are the ones it has in your own terminal.
/// `STACKUP_LAZYGIT` names another program to run in its place: the tests use it.
pub(super) fn git_open(root: &Path) -> Vec<Step> {
    vec![Step::new(
        "open lazygit",
        root,
        &env::var("STACKUP_LAZYGIT").unwrap_or_else(|_| "lazygit".into()),
        &[],
    )]
}

/// What `git init` runs.
pub(super) fn git_init(root: &Path, facts: &Facts) -> Vec<Step> {
    {
        let mut steps = vec![Step::new(
            "start a repository",
            root,
            "git",
            &["init", "-q", "-b", "main"],
        )];
        if facts.hooks {
            steps.push(Step::new(
                "turn on the gitleaks hook",
                root,
                "git",
                &["config", "core.hooksPath", ".githooks"],
            ));
        }
        steps
    }
}

/// What `git repo` runs.
pub(super) fn git_repo(p: &Project, args: &Args) -> Result<Vec<Step>> {
    let root = p.path.as_path();
    Ok({
        let visibility = args.visibility.as_deref().unwrap_or("private");
        if !matches!(visibility, "private" | "public") {
            bail!("a repository is `private` or `public`");
        }
        let Some(login) =
            capture(root, "gh", &["api", "user", "-q", ".login"]).filter(|l| !l.is_empty())
        else {
            bail!("this needs the GitHub CLI, signed in: run `gh auth login`");
        };
        let remote =
            capture(root, "git", &["remote", "get-url", "origin"]).filter(|u| !u.is_empty());
        // A project that already points at a GitHub repository gets that one made.
        let slug = remote
            .as_deref()
            .and_then(git::github_slug)
            .unwrap_or_else(|| format!("{login}/{}", p.name));
        let has_commits = capture(root, "git", &["rev-parse", "--verify", "-q", "HEAD"]).is_some();
        git::repo_steps(root, &slug, visibility, remote.is_some(), has_commits)
    })
}
