mod nuxt;
mod shared;
mod topcoat;
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
        Frontend::Topcoat => {
            need("cargo", "the Topcoat frontend")?;
            need("topcoat", "the Topcoat frontend")?;
            topcoat::preflight()
        }
        Frontend::Nuxt | Frontend::Vue => need("pnpm", "the frontend"),
    }
}

pub fn generate(root: &Path, plan: &Plan) -> Result<()> {
    match plan.frontend {
        Frontend::Nuxt => nuxt::generate(root, plan)?,
        Frontend::Vue => vue::generate(root, plan)?,
        Frontend::Topcoat => topcoat::generate(root, plan)?,
        Frontend::None => return Ok(()),
    }
    let ignored = match plan.frontend {
        Frontend::Topcoat => "target\n.git\n.env\n",
        _ => "node_modules\n.nuxt\n.output\ndist\n.git\n.env\n",
    };
    fs::write(root.join("frontend/.dockerignore"), ignored)?;
    Ok(())
}
