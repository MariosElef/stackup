//! axum (Rust): cargo-managed, layered like the mc2 services.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::run,
    models::{BeLib, Plan, Service, Tool},
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
    if plan.be_lib(BeLib::Utoipa) {
        run(&dir, "cargo", &["add", "utoipa"])?;
    }
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
    manifest.push_str(include_str!(
        "../../../../assets/backend/axum/cargo-tail.toml"
    ));
    fs::write(dir.join("Cargo.toml"), manifest)?;
    // With utoipa the starter also documents itself and serves the spec at /openapi.json.
    let main = if plan.be_lib(BeLib::Utoipa) {
        include_str!("../../../../assets/backend/axum/main-utoipa.rs")
    } else {
        include_str!("../../../../assets/backend/axum/main.rs")
    };
    if plan.be_lib(BeLib::Sqlx) {
        fs::write(dir.join("src/main.rs"), with_migrations(main))?;
        fs::create_dir_all(dir.join("migrations"))?;
        for (name, sql) in [
            (
                "0001_init.up.sql",
                include_str!("../../../../assets/backend/axum/migrations/0001_init.up.sql"),
            ),
            (
                "0001_init.down.sql",
                include_str!("../../../../assets/backend/axum/migrations/0001_init.down.sql"),
            ),
        ] {
            fs::write(dir.join("migrations").join(name), sql)?;
        }
        fs::write(
            dir.join("build.rs"),
            include_str!("../../../../assets/backend/axum/build.rs"),
        )?;
    } else {
        fs::write(dir.join("src/main.rs"), main)?;
    }
    keep(&dir.join("src"), crate::models::AXUM_LAYERS)?;
    fs::write(dir.join("rust-toolchain.toml"), plan.rust_toolchain_file())?;
    let mut dockerfile =
        include_str!("../../../../assets/backend/axum/Dockerfile").replace("{{BIN}}", &bin);
    if plan.be_lib(BeLib::Sqlx) {
        // The migrations are compiled in, so the build needs them and the script that watches them.
        dockerfile = dockerfile.replace(
            "COPY src ./src\n",
            "COPY src ./src\nCOPY build.rs ./\nCOPY migrations ./migrations\n",
        );
    }
    fs::write(dir.join("Dockerfile"), dockerfile)?;
    fs::write(dir.join(".dockerignore"), "target\n.git\n.env\n")?;
    if plan.tool(Tool::Lint) {
        fs::write(
            dir.join("deny.toml"),
            include_str!("../../../../assets/backend/axum/deny.toml"),
        )?;
        fs::write(
            dir.join("taplo.toml"),
            include_str!("../../../../assets/backend/taplo.toml"),
        )?;
    }
    Ok(())
}

/// The starter's `main`, connecting to Postgres and applying the migrations before it serves.
fn with_migrations(main: &str) -> String {
    main.replacen(
        "    let addr = std::env::var(\"BIND_ADDR\")",
        "    // The database, brought up to date before anything is served: each file in migrations/,
    // compiled in, is applied once.
    let database_url = std::env::var(\"DATABASE_URL\")?;
    let pool = sqlx::PgPool::connect(&database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let addr = std::env::var(\"BIND_ADDR\")",
        1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_migrations_the_starter_migrates_before_it_listens() {
        for main in [
            include_str!("../../../../assets/backend/axum/main.rs"),
            include_str!("../../../../assets/backend/axum/main-utoipa.rs"),
        ] {
            let out = with_migrations(main);
            let migrate = out.find("sqlx::migrate!().run(&pool)").expect("migrates");
            assert!(migrate < out.find("BIND_ADDR").unwrap(), "{out}");
        }
        let dockerfile = include_str!("../../../../assets/backend/axum/Dockerfile");
        assert!(
            dockerfile.contains("COPY src ./src\n"),
            "the anchor the migrations go after"
        );
    }
}
