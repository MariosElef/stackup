//! What to run to get a tool this machine lacks: brew, winget, npm, rustup, or cargo. Only what
//! is a few clear commands is installed from here; the rest (Rust itself, Docker, git) is left
//! to the person, with the instructions beside it.
use std::path::Path;

use super::{Machine, TOPCOAT_CLI_REV, TOPCOAT_RUST, rust_version};
use crate::{models::Step, services::toolchain::versions::Picks};

pub struct InstallPlan {
    /// What can be installed from here, in the order asked.
    pub steps: Vec<Step>,
    /// The tools that have to be installed by hand.
    pub by_hand: Vec<&'static str>,
}

/// `main` is the newest Topcoat; anything else is a git revision.
fn cargo_install(dir: &Path, toolchain: Option<&str>, rev: &str) -> Step {
    let plus = toolchain.map(|t| format!("+{t}"));
    let mut args: Vec<&str> = plus.as_deref().into_iter().collect();
    args.extend(["install", "--git", "https://github.com/tokio-rs/topcoat"]);
    if rev != "main" {
        args.extend(["--rev", rev]);
    }
    args.push("topcoat-cli");
    Step::new("install topcoat", dir, "cargo", &args)
}

/// How to get `toolchain`: nothing if it is installed, `rustup update` for the stable channel,
/// else a toolchain install.
pub fn rust_steps(m: &dyn Machine, dir: &Path, toolchain: &str, label: &str) -> Vec<Step> {
    if m.toolchains().iter().any(|t| t == toolchain) {
        vec![]
    } else if toolchain == "stable" {
        vec![Step::new(label, dir, "rustup", &["update", "stable"])]
    } else {
        vec![Step::new(
            label,
            dir,
            "rustup",
            &["toolchain", "install", toolchain],
        )]
    }
}

/// `topcoat new` is not in the crates.io release yet, so the CLI comes from git, and it needs a
/// recent Rust. What was picked decides which; unpicked, the Rust in use if it is new enough,
/// else an installed one that is, else the default is updated first.
fn topcoat(m: &dyn Machine, dir: &Path, picks: &Picks) -> Option<Vec<Step>> {
    if !m.has("cargo") {
        return None;
    }
    let rev = picks.topcoat.as_deref().unwrap_or(TOPCOAT_CLI_REV);
    if let Some(toolchain) = picks.rust.as_deref() {
        if !m.has("rustup") {
            return None;
        }
        let mut steps = rust_steps(m, dir, toolchain, "install topcoat: get Rust first");
        steps.push(cargo_install(dir, Some(toolchain), rev));
        return Some(steps);
    }
    if m.active_rust().is_some_and(|v| v >= TOPCOAT_RUST) {
        return Some(vec![cargo_install(dir, None, rev)]);
    }
    if !m.has("rustup") {
        return None;
    }
    if let Some(newer) = m
        .toolchains()
        .into_iter()
        .find(|t| rust_version(t).is_some_and(|v| v >= TOPCOAT_RUST))
    {
        return Some(vec![cargo_install(dir, Some(&newer), rev)]);
    }
    Some(vec![
        Step::new(
            "install topcoat: get Rust first",
            dir,
            "rustup",
            &["update", "stable"],
        ),
        cargo_install(dir, Some("stable"), rev),
    ])
}

/// sqlx-cli, with only what a Postgres project needs.
const SQLX_CLI: &[&str] = &[
    "install",
    "sqlx-cli",
    "--no-default-features",
    "--features",
    "rustls,postgres",
    "--locked",
];

/// The winget package for a tool, on a Windows machine.
fn winget_id(bin: &str) -> Option<&'static str> {
    Some(match bin {
        "mkcert" => "FiloSottile.mkcert",
        "uv" => "astral-sh.uv",
        "pnpm" => "pnpm.pnpm",
        "gh" => "GitHub.cli",
        "lazygit" => "JesseDuffield.lazygit",
        // Git for Windows: also the shell the generated scripts run in.
        "git" => "Git.Git",
        _ => return None,
    })
}

fn command(bin: &str, m: &dyn Machine, dir: &Path, picks: &Picks) -> Option<Vec<Step>> {
    let one = |prog: &str, args: &[&str]| {
        Some(vec![Step::new(&format!("install {bin}"), dir, prog, args)])
    };
    // A Windows machine installs with winget, unless a pnpm version was picked: that is npm's.
    let pnpm_version = bin == "pnpm" && picks.pnpm.as_deref().is_some_and(|v| v != "latest");
    if m.has("winget")
        && !m.has("brew")
        && !pnpm_version
        && let Some(id) = winget_id(bin)
    {
        return one(
            "winget",
            &[
                "install",
                "--id",
                id,
                "--exact",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        );
    }
    match bin {
        "topcoat" => topcoat(m, dir, picks),
        "sqlx" if m.has("cargo") => one("cargo", SQLX_CLI),
        "mkcert" if m.has("brew") => one("brew", &["install", "mkcert"]),
        "gh" if m.has("brew") => one("brew", &["install", "gh"]),
        "lazygit" if m.has("brew") => one("brew", &["install", "lazygit"]),
        "uv" if m.has("brew") => one("brew", &["install", "uv"]),
        // A version other than the latest is an npm package version; brew only has the latest.
        "pnpm" if m.has("npm") && picks.pnpm.as_deref().is_some_and(|v| v != "latest") => {
            let spec = format!("pnpm@{}", picks.pnpm.as_deref().unwrap_or("latest"));
            one("npm", &["install", "--global", &spec])
        }
        "pnpm" if m.has("brew") => one("brew", &["install", "pnpm"]),
        "pnpm" if m.has("npm") => one("npm", &["install", "--global", "pnpm"]),
        _ => None,
    }
}

pub fn install_plan(
    missing: &[&'static str],
    dir: &Path,
    m: &dyn Machine,
    picks: &Picks,
) -> InstallPlan {
    let mut plan = InstallPlan {
        steps: vec![],
        by_hand: vec![],
    };
    for &bin in missing {
        match command(bin, m, dir, picks) {
            Some(steps) => plan.steps.extend(steps),
            None => plan.by_hand.push(bin),
        }
    }
    plan
}
