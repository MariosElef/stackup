//! FastAPI (Python): uv-managed, layered like the mc2 services.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::{capture, run},
    models::{BeLib, Plan, Service, Tool},
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
    if plan.be_lib(BeLib::Alembic) {
        // Alembic migrates through SQLAlchemy, which uses asyncpg underneath.
        run(&dir, "uv", &["add", "sqlalchemy[asyncio]", "alembic"])?;
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

    if plan.be_lib(BeLib::Alembic) {
        alembic(&dir)?;
    }

    let py = fs::read_to_string(dir.join(".python-version"))
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|_| "3.13".into());
    let uv = capture(&dir, "uv", &["--version"])
        .and_then(|v| v.split_whitespace().nth(1).map(str::to_string))
        .unwrap_or_else(|| "latest".into());
    let mut dockerfile = include_str!("../../../../assets/backend/fastapi/Dockerfile")
        .replace("{{PY}}", &py)
        .replace("{{UV}}", &uv);
    if plan.be_lib(BeLib::Alembic) {
        dockerfile = dockerfile.replace(SERVE, MIGRATE_THEN_SERVE);
    }
    fs::write(dir.join("Dockerfile"), dockerfile)?;
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

const SERVE: &str = r#"CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]"#;
const MIGRATE_THEN_SERVE: &str = r#"# The database is brought up to date, then served.
CMD ["sh", "-c", "alembic upgrade head && exec uvicorn app.main:app --host 0.0.0.0 --port 8000"]"#;

/// What `env.py` gets: the database from `DATABASE_URL`, as the app reads it, with the driver
/// SQLAlchemy names. `%` is doubled: Alembic's config reads it as the start of a variable.
const ENV_URL: &str = r#"config = context.config

# The database comes from DATABASE_URL, like the app's; SQLAlchemy names the async driver.
database_url = os.environ.get("DATABASE_URL", "")
if database_url:
    database_url = database_url.replace("postgres://", "postgresql+asyncpg://", 1)
    config.set_main_option("sqlalchemy.url", database_url.replace("%", "%%"))
"#;

/// Alembic's own hooks, on: each new revision is fixed and formatted by the project's ruff, so
/// what Alembic writes passes the lint CI runs.
const RUFF_HOOKS: &str = "[post_write_hooks]
# New revisions are fixed and formatted with the project's ruff, so CI passes on them.
hooks = ruff_fix, ruff_format
ruff_fix.type = module
ruff_fix.module = ruff
ruff_fix.options = check --fix REVISION_SCRIPT_FILENAME
ruff_format.type = module
ruff_format.module = ruff
ruff_format.options = format REVISION_SCRIPT_FILENAME
";

/// A revision imports `op` and `sa` for the one writing it, and the first has no parent to name:
/// neither is a mistake there.
const RUFF_IGNORES: &str = "
[tool.ruff.lint.per-file-ignores]
\"migrations/versions/*\" = [\"F401\", \"W291\"]
";

/// Alembic in `migrations/`, with its async template, reading the database from the
/// environment, and a first revision to start from.
fn alembic(dir: &Path) -> Result<()> {
    run(
        dir,
        "uv",
        &["run", "alembic", "init", "-t", "async", "migrations"],
    )?;
    let ini = dir.join("alembic.ini");
    let text = fs::read_to_string(&ini)?;
    if !text.contains("[post_write_hooks]\n") {
        anyhow::bail!("alembic.ini has no [post_write_hooks] section to turn the hooks on in");
    }
    fs::write(&ini, text.replacen("[post_write_hooks]\n", RUFF_HOOKS, 1))?;
    let mut pyproject = fs::read_to_string(dir.join("pyproject.toml"))?;
    pyproject.push_str(RUFF_IGNORES);
    fs::write(dir.join("pyproject.toml"), pyproject)?;
    let env = dir.join("migrations/env.py");
    let text = fs::read_to_string(&env)?;
    let patched = text
        .replacen("import asyncio\n", "import asyncio\nimport os\n", 1)
        .replacen("config = context.config\n", ENV_URL, 1);
    if patched == text {
        anyhow::bail!("migrations/env.py is not the one Alembic's async template writes");
    }
    fs::write(&env, patched)?;
    run(dir, "uv", &["run", "alembic", "revision", "-m", "init"])?;
    // The template's env.py, in the project's style.
    run(dir, "uv", &["run", "ruff", "check", "--fix", "migrations"])?;
    run(dir, "uv", &["run", "ruff", "format", "migrations"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_alembic_the_image_migrates_before_it_serves() {
        let dockerfile = include_str!("../../../../assets/backend/fastapi/Dockerfile");
        assert!(
            dockerfile.contains(SERVE),
            "the command the migration goes before"
        );
    }
}
