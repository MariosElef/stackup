mod axum;
mod fastapi;

use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::need,
    models::{Backend, Plan},
};

pub fn preflight(plan: &Plan) -> Result<()> {
    match plan.backend {
        Backend::Axum => need("cargo", "the axum backend"),
        Backend::Fastapi => need("uv", "the FastAPI backend"),
        Backend::None => Ok(()),
    }
}

pub fn generate(root: &Path, plan: &Plan) -> Result<()> {
    match plan.backend {
        Backend::Axum => axum::generate(root, plan),
        Backend::Fastapi => fastapi::generate(root, plan),
        Backend::None => Ok(()),
    }
}

/// Empty folders git would drop: each gets a `.gitkeep`.
pub fn keep(parent: &Path, names: &[&str]) -> Result<()> {
    for name in names {
        let d = parent.join(name);
        fs::create_dir_all(&d)?;
        fs::write(d.join(".gitkeep"), "")?;
    }
    Ok(())
}
