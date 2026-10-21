//! Where git is for a project: its branch, what changed, how far it is from origin.
use std::path::Path;

use crate::clients::run::capture;

/// What git says about a project right now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitInfo {
    pub branch: String,
    /// Files changed, added or not yet tracked.
    pub dirty: usize,
    pub ahead: u32,
    pub behind: u32,
    /// Where `origin` points, if there is one.
    pub remote: Option<String>,
    /// Whether there is a commit yet.
    pub commits: bool,
}

/// Reads `git status --porcelain=v2 --branch`: the branch, how far ahead and behind its upstream,
/// and how many files differ.
pub fn parse_status(text: &str) -> (String, u32, u32, usize) {
    let (mut branch, mut ahead, mut behind, mut dirty) = (String::new(), 0, 0, 0);
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            branch = head.to_string();
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let mut parts = ab.split_whitespace();
            ahead = parts
                .next()
                .and_then(|a| a.trim_start_matches('+').parse().ok())
                .unwrap_or(0);
            behind = parts
                .next()
                .and_then(|b| b.trim_start_matches('-').parse().ok())
                .unwrap_or(0);
        } else if !line.starts_with('#') && !line.is_empty() {
            dirty += 1;
        }
    }
    (branch, ahead, behind, dirty)
}

/// The state of the repository at `root`, or `None` when it is not one.
pub fn read(root: &Path) -> Option<GitInfo> {
    if !root.join(".git").exists() {
        return None;
    }
    let status = capture(root, "git", &["status", "--porcelain=v2", "--branch"])?;
    let (branch, ahead, behind, dirty) = parse_status(&status);
    Some(GitInfo {
        branch,
        dirty,
        ahead,
        behind,
        remote: capture(root, "git", &["remote", "get-url", "origin"]).filter(|u| !u.is_empty()),
        commits: capture(root, "git", &["rev-parse", "--verify", "-q", "HEAD"]).is_some(),
    })
}
