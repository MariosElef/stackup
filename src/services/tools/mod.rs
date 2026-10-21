mod ci;
mod env;
mod git;
mod makefile;
mod readme;

pub use git::init_git;

use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use anyhow::Result;

use crate::{
    models::{Plan, Service, Tool},
    services::compose::{caddyfile, compose},
};

pub fn write_root(root: &Path, plan: &Plan) -> Result<()> {
    fs::write(root.join("docker-compose.yml"), compose(plan))?;
    fs::write(
        root.join(".gitignore"),
        include_str!("../../../assets/project/gitignore"),
    )?;
    // Configuration lives in per-tier .env files. `.env.example` is the committed template;
    // `.env.local` works as generated and is copied to `.env`, which compose reads.
    fs::write(root.join(".env.example"), env::file(plan, "example"))?;
    let local = env::file(plan, "local");
    fs::write(root.join(".env.local"), &local)?;
    fs::write(root.join(".env"), local)?;

    if plan.has(Service::Keycloak) {
        fs::create_dir_all(root.join("keycloak"))?;
        fs::write(
            root.join("keycloak/app-realm.json"),
            include_str!("../../../assets/project/keycloak.realm.json")
                .replace("{{WEB}}", &plan.web_url()),
        )?;
    }
    if plan.has(Service::Https) {
        fs::write(root.join("Caddyfile"), caddyfile(plan))?;
        fs::create_dir_all(root.join("certs"))?;
        fs::write(root.join("certs/.gitkeep"), "")?;
        fs::create_dir_all(root.join("scripts"))?;
        write_exec(
            &root.join("scripts/certs.sh"),
            include_str!("../../../assets/project/certs.sh"),
        )?;
    }

    if plan.tool(Tool::Makefile) {
        fs::write(root.join("Makefile"), makefile::generate(plan))?;
    }
    if plan.tool(Tool::Lint) {
        fs::write(
            root.join(".yamlfmt"),
            include_str!("../../../assets/project/.yamlfmt"),
        )?;
    }
    if plan.tool(Tool::Ci) {
        write_exec(&root.join("ci.sh"), &ci::generate(plan))?;
    }
    if plan.tool(Tool::Gitleaks) {
        fs::create_dir_all(root.join(".githooks"))?;
        write_exec(
            &root.join(".githooks/pre-commit"),
            "#!/bin/sh\nexec gitleaks protect --staged\n",
        )?;
    }
    fs::write(root.join("README.md"), readme::generate(plan))?;
    Ok(())
}

fn write_exec(path: &Path, body: &str) -> Result<()> {
    fs::write(path, body)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}
