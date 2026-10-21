mod ci;
mod env;
mod git;
mod readme;

pub use git::init_git;

use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    models::{Plan, Service, Tool},
    services::{
        generate::compose::{caddyfile, compose},
        projects::https_dev,
    },
};

pub fn write_root(root: &Path, plan: &Plan) -> Result<()> {
    fs::write(root.join("docker-compose.yml"), compose(plan))?;
    fs::write(
        root.join(".gitignore"),
        include_str!("../../../../assets/project/gitignore"),
    )?;
    fs::write(
        root.join(".gitattributes"),
        include_str!("../../../../assets/project/gitattributes"),
    )?;
    // `.env.example` is the committed template; `.env` holds the real values and is what compose
    // reads. A `.env.<tier>` appears only when a second environment is made.
    fs::write(root.join(".env.example"), env::file(plan, "example"))?;
    fs::write(root.join(".env"), env::file(plan, "env"))?;

    if plan.has(Service::Keycloak) {
        fs::create_dir_all(root.join("keycloak"))?;
        fs::write(
            root.join("keycloak/app-realm.json"),
            include_str!("../../../../assets/project/keycloak.realm.json")
                .replace("{{WEB}}", &plan.web_url()),
        )?;
    }
    if plan.has(Service::Https) {
        let docker = caddyfile(plan);
        fs::write(
            root.join(https_dev::CADDYFILE),
            https_dev::caddyfile(&docker),
        )?;
        fs::write(root.join("Caddyfile"), docker)?;
        fs::create_dir_all(root.join("certs"))?;
        fs::write(root.join("certs/.gitkeep"), "")?;
        fs::create_dir_all(root.join("scripts"))?;
        write_exec(
            &root.join("scripts/certs.sh"),
            include_str!("../../../../assets/project/certs.sh"),
        )?;
    }

    if plan.tool(Tool::Lint) {
        fs::write(
            root.join(".yamlfmt"),
            include_str!("../../../../assets/project/.yamlfmt"),
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
    crate::clients::os::make_executable(path)?;
    Ok(())
}
