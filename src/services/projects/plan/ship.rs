//! Getting the project out and keeping it tidy: formatting, deploying, and cleaning build files.
use std::path::Path;

use anyhow::{Result, bail};

use super::valid_tier;
use crate::{
    clients::docker,
    models::{Args, Backend, Facts, Frontend, Project, Step},
    services::projects::{compose, deploy, disk},
};

/// What `deploy` runs.
pub(super) fn deploy(p: &Project, args: &Args) -> Result<Vec<Step>> {
    let root = p.path.as_path();
    Ok({
        let Some(host) = args.host.clone().or_else(|| p.host.clone()) else {
            bail!("deploy needs a server: give --host user@server");
        };
        let tier = args.tier.clone().unwrap_or_else(|| "prod".into());
        if !valid_tier(&tier) {
            bail!("`{tier}` is not a tier name");
        }
        let file = format!(".env.{tier}");
        if !root.join(&file).is_file() {
            bail!("{file} not found: copy .env.example to {file} and fill in the {tier} values");
        }
        let config = docker::config(root)?;
        let images = compose::built_images(&config);
        if images.is_empty() {
            bail!("nothing to deploy: no service in docker-compose.yml is built from this project");
        }
        let options = deploy::Options {
            host,
            dir: args
                .dir
                .clone()
                .unwrap_or_else(|| format!("/srv/{}", p.name)),
            tier,
            platform: args
                .platform
                .clone()
                .unwrap_or_else(|| "linux/amd64".into()),
        };
        deploy::steps(
            root,
            &options,
            &images,
            &compose::bind_sources(&config, root),
        )?
    })
}

pub(super) fn fmt(root: &Path, facts: &Facts) -> Result<Vec<Step>> {
    let (backend, frontend) = (root.join("backend"), root.join("frontend"));
    // Topcoat + axum are one Cargo workspace (it has a Cargo.toml at the root): formatted once.
    if facts.backend == Backend::Axum
        && facts.frontend == Frontend::Topcoat
        && root.join("Cargo.toml").is_file()
    {
        return Ok(vec![Step::new(
            "format the workspace",
            root,
            "cargo",
            &["fmt", "--all"],
        )]);
    }
    let mut steps = vec![];
    match facts.backend {
        Backend::Axum => steps.push(Step::new(
            "format the backend",
            &backend,
            "cargo",
            &["fmt", "--all"],
        )),
        Backend::Fastapi => steps.push(Step::new(
            "format the backend",
            &backend,
            "uv",
            &["run", "ruff", "format", "."],
        )),
        Backend::None => {}
    }
    match facts.frontend {
        Frontend::Nuxt => steps.push(Step::new(
            "format the frontend",
            &frontend,
            "pnpm",
            &["fmt"],
        )),
        Frontend::Vue => steps.push(Step::new(
            "format the frontend",
            &frontend,
            "pnpm",
            &["exec", "vp", "check", "--fix"],
        )),
        Frontend::Topcoat => steps.push(Step::new(
            "format the frontend",
            &frontend,
            "cargo",
            &["fmt", "--all"],
        )),
        Frontend::None => {}
    }
    if steps.is_empty() {
        bail!("nothing to format: this project has no backend or frontend");
    }
    Ok(steps)
}

/// `cargo clean` where there are build files: at the root of a workspace, or in each crate that
/// stands alone. Each step says how much room it frees.
pub(super) fn cargo_clean(p: &Project) -> Result<Vec<Step>> {
    let root = p.path.as_path();
    let dirs = if root.join("Cargo.toml").is_file() {
        vec![root.to_path_buf()]
    } else {
        vec![root.join("backend"), root.join("frontend")]
    };
    let steps: Vec<Step> = dirs
        .into_iter()
        .filter(|d| d.join("Cargo.toml").is_file() && d.join("target").is_dir())
        .map(|d| {
            let freed = disk::human(disk::size(&d.join("target")));
            let name = d
                .strip_prefix(root)
                .ok()
                .filter(|r| !r.as_os_str().is_empty());
            let where_ = name.map_or(String::new(), |n| format!(" in {}", n.display()));
            Step::new(
                &format!("delete the build files{where_} ({freed})"),
                &d,
                "cargo",
                &["clean"],
            )
        })
        .collect();
    if steps.is_empty() {
        bail!("there are no Rust build files to clean");
    }
    Ok(steps)
}
