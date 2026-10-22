//! The migrations in `backend/migrations`, applied, made and undone with the backend's own tool:
//! sqlx-cli for axum, Alembic for FastAPI. They run on this machine against the Postgres in
//! Docker, with the active `.env`, as the backend does in development.
use std::path::Path;

use anyhow::{Result, bail};

use super::compose_step;
use crate::{
    models::{Args, Backend, Facts, Step},
    services::projects::envs,
};

/// One run of the tool, in `backend/`, with the database address from `.env`.
fn tool(root: &Path, facts: &Facts, label: &str, sqlx: &[&str], alembic: &[&str]) -> Result<Step> {
    let dir = root.join("backend");
    let mut step = match facts.backend {
        Backend::Axum => Step::new(label, &dir, "sqlx", sqlx),
        Backend::Fastapi => {
            let mut args = vec!["run", "alembic"];
            args.extend(alembic);
            Step::new(label, &dir, "uv", &args)
        }
        Backend::None => bail!("this project has no backend"),
    };
    step.env.extend(envs::dotenv(root));
    Ok(step)
}

/// Postgres up and answering, before anything is done to it.
fn postgres(root: &Path) -> Step {
    compose_step("start Postgres", root, &["up", "-d", "--wait", "postgres"])
}

/// The steps that bring the database up to date.
pub(super) fn apply(root: &Path, facts: &Facts) -> Result<Step> {
    tool(
        root,
        facts,
        "apply the migrations",
        &["migrate", "run"],
        &["upgrade", "head"],
    )
}

pub(super) fn migrate(root: &Path, facts: &Facts) -> Result<Vec<Step>> {
    Ok(vec![postgres(root), apply(root, facts)?])
}

/// The next migration's files. Neither tool needs the database for this.
pub(super) fn new(root: &Path, facts: &Facts, args: &Args) -> Result<Vec<Step>> {
    let Some(name) = args.name.as_deref() else {
        bail!("a new migration needs a name, like add_users");
    };
    Ok(vec![tool(
        root,
        facts,
        "make the migration",
        // Numbered after the starter's 0001_init, not by the clock.
        &["migrate", "add", "-r", "--sequential", name],
        &["revision", "-m", name],
    )?])
}

pub(super) fn revert(root: &Path, facts: &Facts) -> Result<Vec<Step>> {
    Ok(vec![
        postgres(root),
        tool(
            root,
            facts,
            "undo the last migration",
            &["migrate", "revert"],
            &["downgrade", "-1"],
        )?,
    ])
}
