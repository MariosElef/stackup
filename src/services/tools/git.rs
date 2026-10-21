//! `git init`, the origin remote, and optionally creating the GitHub repo with `gh`.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::{capture, run},
    models::{Plan, Tool},
};

pub fn init_git(root: &Path, plan: &Plan) -> Result<()> {
    if !plan.git {
        return Ok(());
    }
    run(root, "git", &["init", "-q", "-b", "main"])?;
    let login = gh_login(root);
    let owner = login
        .clone()
        .or_else(|| capture(root, "git", &["config", "github.user"]));

    // Nothing here may fail the scaffold: the project is already generated, and git, gh and
    // the network are all things a user may not have set up.
    if let Some(public) = plan.publish {
        match &login {
            None => println!(
                "note: --create-remote needs the GitHub CLI, logged in. Run `gh auth login`, then `gh repo create {} --source=. --push`.",
                plan.name
            ),
            Some(login) if !commit_all(root) => println!(
                "note: could not make the first commit (is git user.name / user.email set?), so {login}/{} was not created.",
                plan.name
            ),
            Some(login) => {
                let slug = format!("{login}/{}", plan.name);
                let vis = if public { "--public" } else { "--private" };
                let args = [
                    "repo",
                    "create",
                    &slug,
                    vis,
                    "--source=.",
                    "--remote=origin",
                    "--push",
                ];
                if run(root, "gh", &args).is_err() {
                    println!(
                        "note: `gh repo create {slug}` failed (does it already exist?). The project is committed locally."
                    );
                }
            }
        }
    }

    if capture(root, "git", &["remote", "get-url", "origin"]).is_none() {
        let url = plan.remote.clone().or_else(|| {
            let ssh = match capture(root, "gh", &["config", "get", "git_protocol"]) {
                Some(p) if login.is_some() => p == "ssh",
                _ => has_ssh_key(root),
            };
            owner.map(|o| origin_url(&o, &plan.name, ssh))
        });
        match url {
            Some(url) => {
                run(root, "git", &["remote", "add", "origin", &url])?;
                println!("git: origin -> {url} (create that repo, then `git push -u origin main`)");
            }
            None => println!(
                "note: no GitHub username known, so no remote was added. Later: `git remote add origin <url>`, or `gh repo create`."
            ),
        }
    }
    if plan.tool(Tool::Gitleaks) {
        run(root, "git", &["config", "core.hooksPath", ".githooks"])?;
    }
    Ok(())
}

/// The GitHub login behind `gh`, when it is installed and signed in.
fn gh_login(dir: &Path) -> Option<String> {
    capture(dir, "gh", &["api", "user", "-q", ".login"]).filter(|l| !l.is_empty())
}

fn commit_all(dir: &Path) -> bool {
    run(dir, "git", &["add", "-A"]).is_ok()
        && run(dir, "git", &["commit", "-q", "-m", "Initial commit"]).is_ok()
}

/// SSH when a key is loaded in the agent or sits in ~/.ssh, https otherwise.
fn has_ssh_key(dir: &Path) -> bool {
    if capture(dir, "ssh-add", &["-L"]).is_some_and(|k| k.contains("ssh-") || k.contains("ecdsa-"))
    {
        return true;
    }
    std::env::var_os("HOME")
        .and_then(|h| fs::read_dir(Path::new(&h).join(".ssh")).ok())
        .is_some_and(|d| {
            d.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.starts_with("id_") && n.ends_with(".pub")
            })
        })
}

fn origin_url(owner: &str, name: &str, ssh: bool) -> String {
    if ssh {
        format!("git@github.com:{owner}/{name}.git")
    } else {
        format!("https://github.com/{owner}/{name}.git")
    }
}

#[cfg(test)]
mod tests {
    use super::origin_url;

    #[test]
    fn origin_is_ssh_with_a_key_and_https_without() {
        assert_eq!(origin_url("me", "app", true), "git@github.com:me/app.git");
        assert_eq!(
            origin_url("me", "app", false),
            "https://github.com/me/app.git"
        );
    }
}
