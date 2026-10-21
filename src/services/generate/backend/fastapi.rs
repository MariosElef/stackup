//! FastAPI (Python): uv-managed, layered like the mc2 services.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::{capture, run},
    models::{Plan, Service, Tool},
};

pub(super) fn generate(root: &Path, plan: &Plan) -> Result<()> {
    println!("backend: FastAPI");
    run(
        root,
        "uv",
        &[
            "init",
            "--app",
            "--no-workspace",
            "--vcs",
            "none",
            "backend",
        ],
    )?;
    let dir = root.join("backend");

    run(
        &dir,
        "uv",
        &["add", "fastapi", "uvicorn[standard]", "pydantic-settings"],
    )?;
    if plan.has(Service::Postgres) {
        run(&dir, "uv", &["add", "asyncpg"])?;
    }
    if plan.has(Service::Redis) {
        run(&dir, "uv", &["add", "redis"])?;
    }
    if plan.has(Service::Mongodb) {
        run(&dir, "uv", &["add", "pymongo"])?;
    }
    run(
        &dir,
        "uv",
        &[
            "add",
            "--dev",
            "ruff",
            "pyrefly",
            "pytest",
            "pytest-asyncio",
            "httpx",
            "import-linter",
        ],
    )?;

    let mut pyproject = fs::read_to_string(dir.join("pyproject.toml"))?;
    pyproject.push_str(include_str!(
        "../../../../assets/backend/fastapi/pyproject-tail.toml"
    ));
    fs::write(dir.join("pyproject.toml"), pyproject)?;
    // `uv init --app` drops a hello-world main.py at the root; the app lives in a package.
    let _ = fs::remove_file(dir.join("main.py"));
    fs::create_dir_all(dir.join("app"))?;
    fs::write(dir.join("app/__init__.py"), "")?;
    fs::write(
        dir.join("app/main.py"),
        include_str!("../../../../assets/backend/fastapi/main.py"),
    )?;
    for layer in crate::models::FASTAPI_LAYERS {
        fs::create_dir_all(dir.join("app").join(layer))?;
        fs::write(dir.join("app").join(layer).join("__init__.py"), "")?;
    }
    fs::create_dir_all(dir.join("tests"))?;
    fs::write(
        dir.join("tests/test_health.py"),
        include_str!("../../../../assets/backend/fastapi/test_health.py"),
    )?;

    let py = fs::read_to_string(dir.join(".python-version"))
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|_| "3.13".into());
    let uv = capture(&dir, "uv", &["--version"])
        .and_then(|v| v.split_whitespace().nth(1).map(str::to_string))
        .unwrap_or_else(|| "latest".into());
    fs::write(
        dir.join("Dockerfile"),
        include_str!("../../../../assets/backend/fastapi/Dockerfile")
            .replace("{{PY}}", &py)
            .replace("{{UV}}", &uv),
    )?;
    fs::write(
        dir.join(".dockerignore"),
        ".venv\n.git\n.env\n__pycache__\n",
    )?;
    if plan.tool(Tool::Lint) {
        fs::write(
            dir.join("taplo.toml"),
            include_str!("../../../../assets/backend/taplo.toml"),
        )?;
    }
    Ok(())
}
