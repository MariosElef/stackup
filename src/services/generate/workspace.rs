//! Topcoat and axum are both Rust, so they are one Cargo workspace instead of two projects: one
//! lockfile, one toolchain pin, one shared `target/`, and one Dockerfile that builds both.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::run,
    models::{Plan, Tool},
};

/// The settings that only count at the root of a workspace. Cargo ignores a `[profile]` in a
/// member crate, so the two crates' own are collected here.
const ROOT_MANIFEST: &str = r#"[workspace]
resolver = "3"
members = ["backend", "frontend"]

[profile.release]
overflow-checks = true

# Speed up compilation while keeping file and line numbers in error backtraces.
[profile.dev]
debug = "line-tables-only"

# Speed up compilation by skipping debug information for dependencies.
[profile.dev.package."*"]
debug = false
"#;

/// `manifest` without its `[profile...]` tables, and the comment lines right above each.
pub fn without_profiles(manifest: &str) -> String {
    let mut out: Vec<&str> = vec![];
    let mut skipping = false;
    for line in manifest.lines() {
        let header = line.trim_start().starts_with('[');
        if header {
            skipping = line.trim_start().starts_with("[profile");
            if skipping {
                // The comments that described the table go with it.
                while out.last().is_some_and(|l| l.trim_start().starts_with('#')) {
                    out.pop();
                }
                while out.last().is_some_and(|l| l.trim().is_empty()) {
                    out.pop();
                }
            }
        }
        if !skipping {
            out.push(line);
        }
    }
    out.join("\n") + "\n"
}

/// The files that belong to the workspace as a whole and so leave the crates.
const PER_CRATE: [&str; 8] = [
    // `topcoat new` writes its own README and .gitignore: the project has one of each, at the root.
    "README.md",
    ".gitignore",
    "Cargo.lock",
    "rust-toolchain.toml",
    "Dockerfile",
    ".dockerignore",
    "deny.toml",
    "taplo.toml",
];

/// Makes `backend/` and `frontend/` one workspace, once both have been generated.
pub fn fuse(root: &Path, plan: &Plan, backend_bin: &str) -> Result<()> {
    for krate in ["backend", "frontend"] {
        let dir = root.join(krate);
        let manifest = fs::read_to_string(dir.join("Cargo.toml"))?;
        fs::write(dir.join("Cargo.toml"), without_profiles(&manifest))?;
        for file in PER_CRATE {
            let _ = fs::remove_file(dir.join(file));
        }
    }
    fs::write(root.join("Cargo.toml"), ROOT_MANIFEST)?;
    fs::write(root.join("rust-toolchain.toml"), plan.rust_toolchain_file())?;
    fs::write(
        root.join("Dockerfile"),
        include_str!("../../../assets/project/Dockerfile.rust-workspace")
            .replace(
                "{{REV}}",
                plan.pin("topcoat")
                    .unwrap_or(crate::services::toolchain::machine::TOPCOAT_CLI_REV),
            )
            .replace("{{BACKEND_BIN}}", backend_bin)
            .replace("{{FRONTEND_BIN}}", "frontend"),
    )?;
    // The whole folder is the build context now: what must not be sent is listed once.
    fs::write(
        root.join(".dockerignore"),
        "target\n**/target\n.git\n.env\n.env.*\ncerts\nnode_modules\n",
    )?;
    if plan.tool(Tool::Lint) {
        fs::write(
            root.join("deny.toml"),
            include_str!("../../../assets/backend/axum/deny.toml"),
        )?;
        fs::write(
            root.join("taplo.toml"),
            include_str!("../../../assets/backend/taplo.toml"),
        )?;
    }
    run(root, "cargo", &["generate-lockfile"])
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOPCOAT: &str = "[package]\nname = \"frontend\"\n\n[dependencies]\ntokio = \"1\"\n\n# Speed up compilation while keeping file and line numbers in error backtraces.\n[profile.dev]\ndebug = \"line-tables-only\"\n\n# Speed up compilation by skipping debug information for dependencies.\n[profile.dev.package.\"*\"]\ndebug = false\n";
    const AXUM: &str = "[package]\nname = \"x-backend\"\npublish = false\n\n[dependencies]\naxum = \"1\"\n\n[profile.release]\noverflow-checks = true\n\n[lints.clippy]\ndbg_macro = \"warn\"\n";

    #[test]
    fn profiles_and_the_comments_above_them_are_taken_out() {
        let out = without_profiles(TOPCOAT);
        assert_eq!(
            out,
            "[package]\nname = \"frontend\"\n\n[dependencies]\ntokio = \"1\"\n"
        );
        assert!(!out.contains("profile") && !out.contains("Speed up"));
    }

    #[test]
    fn everything_else_in_a_manifest_is_kept() {
        let out = without_profiles(AXUM);
        assert!(
            !out.contains("[profile.release]") && !out.contains("overflow-checks"),
            "{out}"
        );
        assert!(
            out.contains("[lints.clippy]") && out.contains("dbg_macro"),
            "the lints stay: {out}"
        );
        assert!(out.contains("publish = false") && out.contains("axum = \"1\""));
        assert_eq!(
            without_profiles("[package]\nname = \"a\"\n"),
            "[package]\nname = \"a\"\n"
        );
    }

    #[test]
    fn the_root_carries_the_profiles_both_crates_had() {
        assert!(
            ROOT_MANIFEST.contains("[workspace]")
                && ROOT_MANIFEST.contains("\"backend\", \"frontend\"")
        );
        assert!(ROOT_MANIFEST.contains("overflow-checks = true"));
        assert!(ROOT_MANIFEST.contains("debug = \"line-tables-only\""));
        assert!(ROOT_MANIFEST.contains("[profile.dev.package.\"*\"]"));
        // A virtual workspace has to say which resolver it uses.
        assert!(ROOT_MANIFEST.contains("resolver = \"3\""));
    }
}
