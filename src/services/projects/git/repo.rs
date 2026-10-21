//! The GitHub repository of a project: its address, and the steps that create it.
use std::path::Path;

use crate::models::Step;

/// `git@github.com:me/app.git` or `https://github.com/me/app.git` -> `me/app`.
pub fn github_slug(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let slug = rest.trim_end_matches('/').trim_end_matches(".git");
    (slug.split('/').count() == 2 && !slug.contains(' ')).then(|| slug.to_string())
}

/// A hosting service whose repository stackup can delete.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Host {
    Github,
    Gitlab,
}

impl Host {
    pub fn label(self) -> &'static str {
        match self {
            Host::Github => "GitHub",
            Host::Gitlab => "GitLab",
        }
    }

    /// What it is called in `--github` and in the arguments.
    pub fn key(self) -> &'static str {
        match self {
            Host::Github => "github",
            Host::Gitlab => "gitlab",
        }
    }

    /// The command line program for it.
    pub fn tool(self) -> &'static str {
        match self {
            Host::Github => "gh",
            Host::Gitlab => "glab",
        }
    }

    pub fn parse(key: &str) -> Option<Host> {
        match key {
            "github" => Some(Host::Github),
            "gitlab" => Some(Host::Gitlab),
            _ => None,
        }
    }
}

/// `git@gitlab.com:group/sub/app.git` or `https://gitlab.com/group/app` -> `group/sub/app`.
pub fn gitlab_slug(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("git@gitlab.com:")
        .or_else(|| url.strip_prefix("https://gitlab.com/"))
        .or_else(|| url.strip_prefix("ssh://git@gitlab.com/"))?;
    let slug = rest.trim_end_matches('/').trim_end_matches(".git");
    (slug.split('/').count() >= 2 && !slug.contains(' ')).then(|| slug.to_string())
}

/// Where a remote's repository is, and what it is called there.
pub fn hosted(url: &str) -> Option<(Host, String)> {
    github_slug(url)
        .map(|s| (Host::Github, s))
        .or_else(|| gitlab_slug(url).map(|s| (Host::Gitlab, s)))
}

/// The steps that delete the repository `slug` on `host`. Deleting needs a permission the usual
/// sign-in does not give, so GitHub is asked for it first. This cannot be undone.
pub fn delete_repo_steps(root: &Path, host: Host, slug: &str) -> Vec<Step> {
    match host {
        Host::Github => vec![
            Step::new(
                "let gh delete repositories",
                root,
                "gh",
                &[
                    "auth",
                    "refresh",
                    "--hostname",
                    "github.com",
                    "--scopes",
                    "delete_repo",
                ],
            ),
            Step::new(
                &format!("delete {slug} on GitHub"),
                root,
                "gh",
                &["repo", "delete", slug, "--yes"],
            ),
        ],
        Host::Gitlab => vec![Step::new(
            &format!("delete {slug} on GitLab"),
            root,
            "glab",
            &["repo", "delete", slug, "--yes"],
        )],
    }
}

/// Create the GitHub repository `slug` and push the project to it. With a first commit made when
/// there is none. If the project already points at a remote, that one is created and pushed to;
/// if not, `gh` adds it.
pub fn repo_steps(
    root: &Path,
    slug: &str,
    visibility: &str,
    has_remote: bool,
    has_commits: bool,
) -> Vec<Step> {
    let mut steps = vec![];
    if !has_commits {
        steps.push(Step::new("stage everything", root, "git", &["add", "-A"]));
        steps.push(Step::new(
            "make the first commit",
            root,
            "git",
            &["commit", "-q", "-m", "Initial commit"],
        ));
    }
    let vis = format!("--{visibility}");
    if has_remote {
        steps.push(Step::new(
            "create the GitHub repository",
            root,
            "gh",
            &["repo", "create", slug, &vis],
        ));
        steps.push(Step::new(
            "push to it",
            root,
            "git",
            &["push", "-u", "origin", "HEAD"],
        ));
    } else {
        steps.push(Step::new(
            "create the GitHub repository and push",
            root,
            "gh",
            &[
                "repo",
                "create",
                slug,
                &vis,
                "--source=.",
                "--remote=origin",
                "--push",
            ],
        ));
    }
    steps
}
