//! Topcoat: a Rust framework that renders on the server. It is only the frontend here; the
//! backend and the services stay as chosen, so it needs no database of its own.
use std::{fs, path::Path};

use anyhow::{Result, bail};

use crate::{
    clients::run::{capture, run},
    models::{FeLib, Plan},
};

use super::shared::patch;

/// The `topcoat new` command is not in the crates.io release of the CLI yet, so the CLI is
/// installed from git. The same revision goes into the Dockerfile.
pub(super) const CLI_REV: &str = crate::services::toolchain::machine::TOPCOAT_CLI_REV;
const INSTALL_HINT: &str = "cargo install --git https://github.com/tokio-rs/topcoat topcoat-cli";

pub(super) fn preflight() -> Result<()> {
    if capture(Path::new("."), "topcoat", &["new", "--help"]).is_none() {
        bail!(
            "the frontend needs a Topcoat CLI with `topcoat new`, which the crates.io release \
             does not have yet. Install it with: {INSTALL_HINT}"
        );
    }
    Ok(())
}

/// The crate and binary are both called `frontend`, like the folder.
const BIN: &str = "frontend";

/// The `topcoat new` flags for the chosen libraries. Tailwind and Lucide (Iconify icons) map to
/// its own features, and its UI components need both, so they come only with both. Neither gets
/// a database: the backend owns the data.
fn preset(plan: &Plan) -> Vec<&'static str> {
    let tailwind = plan.lib(FeLib::Tailwind);
    let lucide = plan.lib(FeLib::Lucide);
    match (tailwind, lucide) {
        (true, true) => vec!["--recommended", "--database", "none", "--no-example"],
        (true, false) => vec![
            "--recommended",
            "--database",
            "none",
            "--no-example",
            "--no-ui",
            "--icons",
            "none",
        ],
        (false, true) => vec!["--minimal", "--icons", "iconify", "--icon-set", "lucide"],
        (false, false) => vec!["--minimal"],
    }
}

pub(super) fn generate(root: &Path, plan: &Plan) -> Result<()> {
    println!("frontend: Topcoat");
    let mut args = vec![
        "new",
        "frontend",
        "--name",
        BIN,
        "--no-git",
        "--no-interactive",
    ];
    args.extend(preset(plan));
    run(root, "topcoat", &args)?;
    let dir = root.join("frontend");

    if plan.lib(FeLib::I18n) {
        // rust-i18n reads `locales/<lang>.yml` at compile time; `t!("welcome")` looks a key up.
        // The locale is global, so a server handling many languages passes `locale = ...`.
        run(&dir, "cargo", &["add", "rust-i18n"])?;
        fs::create_dir_all(dir.join("locales"))?;
        fs::write(
            dir.join("locales/en.yml"),
            "_version: 1\nwelcome: It works\n",
        )?;
        patch(
            &dir.join("src/main.rs"),
            &[(
                "mod features;\n",
                "mod features;\n\nrust_i18n::i18n!(\"locales\", fallback = \"en\");\n",
            )],
        );
    }

    // Empty layers, kept by .gitkeep like the backend's; the CI check enforces how they depend
    // on each other.
    crate::services::generate::backend::keep(&dir.join("src"), &plan.frontend_layers())?;

    // The Dockerfile builds with --locked.
    run(&dir, "cargo", &["generate-lockfile"])?;
    fs::write(dir.join("rust-toolchain.toml"), plan.rust_toolchain_file())?;
    fs::write(
        dir.join("Dockerfile"),
        include_str!("../../../../assets/frontend/topcoat/Dockerfile")
            .replace("{{REV}}", plan.pin("topcoat").unwrap_or(CLI_REV))
            .replace("{{BIN}}", BIN),
    )?;
    Ok(())
}
