mod nuxt;
mod shared;
mod vue;

use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::need,
    models::{Frontend, Plan},
};

pub fn preflight(plan: &Plan) -> Result<()> {
    match plan.frontend {
        Frontend::None => Ok(()),
        _ => need("pnpm", "the frontend"),
    }
}

pub fn generate(root: &Path, plan: &Plan) -> Result<()> {
    match plan.frontend {
        Frontend::Nuxt => nuxt::generate(root, plan)?,
        Frontend::Vue => vue::generate(root, plan)?,
        Frontend::None => return Ok(()),
    }
    fs::write(
        root.join("frontend/.dockerignore"),
        "node_modules\n.nuxt\n.output\ndist\n.git\n.env\n",
    )?;
    Ok(())
}
