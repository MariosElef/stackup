use std::path::Path;

use super::*;
use crate::{models::Step, services::toolchain::versions::Picks};

fn lines(plan: &InstallPlan) -> Vec<String> {
    plan.steps.iter().map(Step::command_line).collect()
}

const INSTALL: &str =
    "install --git https://github.com/tokio-rs/topcoat --rev b7a85c34 topcoat-cli";

fn topcoat_plan(m: &Fake) -> InstallPlan {
    install_plan(&["topcoat"], Path::new("."), m, &Picks::default())
}

#[test]
fn versions_are_read_from_rustc_and_from_toolchain_names() {
    assert_eq!(
        rust_version("rustc 1.97.1 (8bab26f4f 2026-07-14)"),
        Some((1, 97))
    );
    assert_eq!(rust_version("1.99.0-aarch64-apple-darwin"), Some((1, 99)));
    assert_eq!(rust_version("stable-aarch64-apple-darwin"), None);
    assert_eq!(rust_version("nightly"), None);
}

#[test]
fn a_new_enough_default_rust_installs_the_topcoat_cli_directly() {
    let m = Fake {
        rust: Some((1, 99)),
        ..fake(&["cargo", "rustup"])
    };
    assert_eq!(lines(&topcoat_plan(&m)), [format!("cargo {INSTALL}")]);
}

#[test]
fn an_old_default_uses_an_installed_newer_toolchain() {
    // The case of a machine with 1.97 as the default and 1.99 installed beside it.
    let m = Fake {
        rust: Some((1, 97)),
        toolchains: &[
            "stable-aarch64-apple-darwin",
            "1.85-aarch64-apple-darwin",
            "1.99.0-aarch64-apple-darwin",
        ],
        ..fake(&["cargo", "rustup"])
    };
    let plan = topcoat_plan(&m);
    assert_eq!(
        lines(&plan),
        [format!("cargo +1.99.0-aarch64-apple-darwin {INSTALL}")]
    );
    assert!(plan.by_hand.is_empty());
}

#[test]
fn with_no_new_enough_rust_it_is_updated_first_and_that_is_a_visible_step() {
    let m = Fake {
        rust: Some((1, 97)),
        toolchains: &["stable-aarch64-apple-darwin"],
        ..fake(&["cargo", "rustup"])
    };
    let plan = topcoat_plan(&m);
    assert_eq!(
        lines(&plan),
        [
            "rustup update stable".to_string(),
            format!("cargo +stable {INSTALL}")
        ]
    );
    assert!(plan.steps[0].label.starts_with("install topcoat"));
}

#[test]
fn without_rustup_an_old_rust_is_left_to_the_person() {
    let m = Fake {
        rust: Some((1, 97)),
        ..fake(&["cargo"])
    };
    let plan = topcoat_plan(&m);
    assert!(plan.steps.is_empty());
    assert_eq!(plan.by_hand, ["topcoat"]);
    assert!(topcoat_plan(&fake(&[])).steps.is_empty(), "no cargo at all");
}

#[test]
fn tools_use_brew_where_there_is_one_and_npm_for_pnpm_otherwise() {
    let missing = ["mkcert", "uv", "pnpm"];
    let with_brew = install_plan(
        &missing,
        Path::new("."),
        &fake(&["brew"]),
        &Picks::default(),
    );
    assert_eq!(
        lines(&with_brew),
        [
            "brew install mkcert",
            "brew install uv",
            "brew install pnpm"
        ]
    );
    let with_npm = install_plan(
        &["pnpm", "mkcert"],
        Path::new("."),
        &fake(&["npm"]),
        &Picks::default(),
    );
    assert_eq!(lines(&with_npm), ["npm install --global pnpm"]);
    assert_eq!(with_npm.by_hand, ["mkcert"]);
}

#[test]
fn rust_and_docker_are_never_installed_from_here_and_git_only_through_winget() {
    let all = fake(&["cargo", "rustup", "brew", "npm"]);
    let plan = install_plan(
        &["cargo", "docker", "git", "ssh"],
        Path::new("."),
        &all,
        &Picks::default(),
    );
    assert!(
        plan.steps.is_empty(),
        "a Mac has git already, and brew is not how to get it"
    );
    assert_eq!(plan.by_hand, ["cargo", "docker", "git", "ssh"]);
}

fn picked(rust: Option<&str>, topcoat: Option<&str>, pnpm: Option<&str>) -> Picks {
    Picks {
        rust: rust.map(str::to_string),
        topcoat: topcoat.map(str::to_string),
        pnpm: pnpm.map(str::to_string),
        ..Picks::default()
    }
}

fn plan_with(m: &Fake, missing: &[&'static str], picks: &Picks) -> Vec<String> {
    lines(&install_plan(missing, Path::new("."), m, picks))
}

#[test]
fn the_topcoat_revision_picked_is_the_one_installed() {
    let m = Fake {
        rust: Some((1, 99)),
        ..fake(&["cargo", "rustup"])
    };
    let latest = plan_with(&m, &["topcoat"], &picked(None, Some("main"), None));
    assert_eq!(
        latest,
        ["cargo install --git https://github.com/tokio-rs/topcoat topcoat-cli"],
        "latest has no revision pinned"
    );
    let other = plan_with(&m, &["topcoat"], &picked(None, Some("abc1234"), None));
    assert!(other[0].contains("--rev abc1234"), "{other:?}");
}

#[test]
fn the_rust_picked_is_fetched_when_needed_and_used_for_the_install() {
    let m = Fake {
        rust: Some((1, 97)),
        toolchains: &["stable-aarch64-apple-darwin", "1.99.0-aarch64-apple-darwin"],
        ..fake(&["cargo", "rustup"])
    };
    // Latest stable: updated, then used.
    assert_eq!(
        plan_with(&m, &["topcoat"], &picked(Some("stable"), None, None)),
        [
            "rustup update stable".to_string(),
            format!("cargo +stable {INSTALL}")
        ]
    );
    // One already installed: nothing to fetch.
    assert_eq!(
        plan_with(
            &m,
            &["topcoat"],
            &picked(Some("1.99.0-aarch64-apple-darwin"), None, None)
        ),
        [format!("cargo +1.99.0-aarch64-apple-darwin {INSTALL}")]
    );
    // One that is not installed yet: installed first, then used.
    assert_eq!(
        plan_with(&m, &["topcoat"], &picked(Some("nightly"), None, None)),
        [
            "rustup toolchain install nightly".to_string(),
            format!("cargo +nightly {INSTALL}")
        ]
    );
}

#[test]
fn a_pnpm_version_comes_from_npm_and_latest_from_brew() {
    let both = fake(&["brew", "npm"]);
    assert_eq!(
        plan_with(&both, &["pnpm"], &picked(None, None, Some("10"))),
        ["npm install --global pnpm@10"]
    );
    assert_eq!(
        plan_with(&both, &["pnpm"], &picked(None, None, Some("latest"))),
        ["brew install pnpm"]
    );
    assert_eq!(
        plan_with(&both, &["pnpm"], &Picks::default()),
        ["brew install pnpm"]
    );
}

/// Not run by default: prints what would be installed on the machine running the tests.
/// `cargo test real_machine -- --ignored --nocapture`
#[test]
#[ignore]
fn real_machine_plan_for_topcoat() {
    let plan = install_plan(&["topcoat"], Path::new("."), &System, &Picks::default());
    println!("active rust: {:?}", System.active_rust());
    println!("toolchains: {:?}", System.toolchains());
    for line in lines(&plan) {
        println!("would run: {line}");
    }
    println!("by hand: {:?}", plan.by_hand);
}

#[test]
fn a_windows_machine_installs_with_winget_and_names_the_package_exactly() {
    let m = fake(&["winget", "npm"]);
    let plan = install_plan(
        &["mkcert", "gh", "uv"],
        Path::new("."),
        &m,
        &Picks::default(),
    );
    let lines = lines(&plan);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(
        lines[0].starts_with("winget install --id FiloSottile.mkcert --exact"),
        "{}",
        lines[0]
    );
    assert!(lines[1].contains("GitHub.cli") && lines[2].contains("astral-sh.uv"));
    assert!(
        lines[0].contains("--accept-package-agreements"),
        "no prompt to stop on"
    );
    // Git for Windows is offered with winget: it is also the shell the scripts run in.
    let git = install_plan(&["git"], Path::new("."), &m, &Picks::default());
    assert!(
        lines_of(&git)[0].contains("Git.Git"),
        "{:?}",
        lines_of(&git)
    );
    // Rust and Docker are still the person's to install.
    let by_hand = install_plan(&["cargo", "docker"], Path::new("."), &m, &Picks::default());
    assert!(by_hand.steps.is_empty());
    // A pnpm version that was picked is npm's, not winget's.
    let pinned = Picks {
        pnpm: Some("9".into()),
        ..Picks::default()
    };
    let plan = install_plan(&["pnpm"], Path::new("."), &m, &pinned);
    assert_eq!(lines_of(&plan), ["npm install --global pnpm@9"]);
}

fn lines_of(plan: &InstallPlan) -> Vec<String> {
    plan.steps.iter().map(Step::command_line).collect()
}
