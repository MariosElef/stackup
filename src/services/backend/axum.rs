//! axum (Rust): cargo-managed, layered like the mc2 services.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::run,
    models::{Plan, Service, Tool},
};

use super::keep;

pub(super) fn generate(root: &Path, plan: &Plan) -> Result<()> {
    let bin = format!("{}-backend", plan.name);
    println!("backend: axum");
    run(
        root,
        "cargo",
        &["new", "backend", "--name", &bin, "--vcs", "none"],
    )?;
    let dir = root.join("backend");

    run(
        &dir,
        "cargo",
        &["add", "axum", "anyhow", "tracing", "serde_json"],
    )?;
    run(
        &dir,
        "cargo",
        &[
            "add",
            "tokio",
            "--features",
            "rt-multi-thread,macros,net,signal,time",
        ],
    )?;
    run(
        &dir,
        "cargo",
        &["add", "tracing-subscriber", "--features", "env-filter,json"],
    )?;
    run(&dir, "cargo", &["add", "serde", "--features", "derive"])?;
    run(
        &dir,
        "cargo",
        &["add", "tower-http", "--features", "trace,cors"],
    )?;

    if plan.has(Service::Postgres) {
        run(
            &dir,
            "cargo",
            &[
                "add",
                "sqlx",
                "--no-default-features",
                "--features",
                "runtime-tokio,tls-rustls-ring,postgres,macros,migrate,uuid,chrono",
            ],
        )?;
        run(&dir, "cargo", &["add", "uuid", "--features", "v7,serde"])?;
        run(
            &dir,
            "cargo",
            &[
                "add",
                "chrono",
                "--no-default-features",
                "--features",
                "now,serde",
            ],
        )?;
    }
    if plan.has(Service::Redis) {
        run(
            &dir,
            "cargo",
            &[
                "add",
                "redis",
                "--no-default-features",
                "--features",
                "tokio-comp,connection-manager",
            ],
        )?;
    }
    if plan.has(Service::Mongodb) {
        run(&dir, "cargo", &["add", "mongodb"])?;
    }
    run(
        &dir,
        "cargo",
        &["add", "--dev", "tower", "http-body-util", "axum-test"],
    )?;

    let mut manifest = fs::read_to_string(dir.join("Cargo.toml"))?;
    manifest = manifest.replacen("\n[dependencies]", "publish = false\n\n[dependencies]", 1);
    manifest.push_str(include_str!("../../../assets/backend/axum/cargo-tail.toml"));
    fs::write(dir.join("Cargo.toml"), manifest)?;
    fs::write(
        dir.join("src/main.rs"),
        include_str!("../../../assets/backend/axum/main.rs"),
    )?;
    keep(&dir.join("src"), crate::models::AXUM_LAYERS)?;
    fs::write(
        dir.join("rust-toolchain.toml"),
        include_str!("../../../assets/backend/axum/rust-toolchain.toml"),
    )?;
    fs::write(
        dir.join("Dockerfile"),
        include_str!("../../../assets/backend/axum/Dockerfile").replace("{{BIN}}", &bin),
    )?;
    fs::write(dir.join(".dockerignore"), "target\n.git\n.env\n")?;
    if plan.tool(Tool::Lint) {
        fs::write(
            dir.join("deny.toml"),
            include_str!("../../../assets/backend/axum/deny.toml"),
        )?;
        fs::write(
            dir.join("taplo.toml"),
            include_str!("../../../assets/backend/taplo.toml"),
        )?;
    }
    Ok(())
}
