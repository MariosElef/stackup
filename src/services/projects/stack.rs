//! What a project is made of, layer by layer, read from its own files: the frameworks, the
//! libraries each side uses (the ones that talk to the databases among them), the services in
//! Docker, and the tooling. What the files say, so a library added or removed later shows.
use std::{fs, path::Path};

use serde_json::Value;

use crate::models::{Backend, Facts, Frontend};

/// One layer of the stack and what is in it, by the names people know them by.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stack {
    pub frontend: Vec<String>,
    pub backend: Vec<String>,
    /// The libraries the backend reaches its databases with.
    pub data: Vec<String>,
    /// What runs in Docker beside the code: databases, auth, mail, the HTTPS proxy.
    pub services: Vec<String>,
    pub tools: Vec<String>,
}

/// The Rust crates worth naming, and how.
const RUST_LIBS: &[(&str, &str)] = &[
    ("tower-http", "tower-http"),
    ("utoipa", "utoipa (OpenAPI)"),
    ("tracing", "tracing"),
    ("rust-i18n", "rust-i18n"),
];
const RUST_DATA: &[(&str, &str)] = &[
    ("sqlx", "sqlx (Postgres)"),
    ("sea-orm", "SeaORM"),
    ("diesel", "Diesel"),
    ("redis", "redis-rs"),
    ("mongodb", "mongodb driver"),
];
const PYTHON_LIBS: &[(&str, &str)] = &[
    ("uvicorn", "uvicorn"),
    ("pydantic-settings", "pydantic-settings"),
];
const PYTHON_DATA: &[(&str, &str)] = &[
    ("sqlalchemy", "SQLAlchemy"),
    ("sqlmodel", "SQLModel"),
    ("asyncpg", "asyncpg (Postgres)"),
    ("redis", "redis-py"),
    ("pymongo", "PyMongo"),
    ("motor", "Motor (MongoDB)"),
    ("alembic", "Alembic (migrations)"),
    ("beanie", "Beanie (MongoDB)"),
];
const JS_LIBS: &[(&str, &str)] = &[
    ("tailwindcss", "Tailwind"),
    ("pinia", "Pinia"),
    ("@pinia/colada", "Pinia Colada"),
    ("vue-router", "Vue Router"),
    ("vue-i18n", "vue-i18n"),
    ("@nuxtjs/i18n", "Nuxt i18n"),
    ("zod", "Zod"),
    ("@hey-api/openapi-ts", "openapi-ts"),
    ("lucide-vue-next", "Lucide"),
    ("@lucide/vue", "Lucide"),
    ("@vueuse/core", "VueUse"),
    ("@vueuse/nuxt", "VueUse"),
];

/// The dependency names in a Cargo.toml: each `name = ...` line under a dependencies table.
fn cargo_deps(text: &str) -> Vec<String> {
    let mut deps = vec![];
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line.ends_with("dependencies]");
            continue;
        }
        if inside && let Some((name, _)) = line.split_once('=') {
            deps.push(name.trim().to_string());
        }
    }
    deps
}

/// The dependency names in a pyproject.toml: the quoted requirements, without their versions.
fn python_deps(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(|req| {
            req.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .next()
                .unwrap_or("")
                .to_lowercase()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// The dependency names in a package.json.
fn js_deps(text: &str) -> Vec<String> {
    let json: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    ["dependencies", "devDependencies"]
        .iter()
        .filter_map(|k| json[k].as_object())
        .flat_map(|o| o.keys().cloned())
        .collect()
}

/// The names in `known` that `deps` has, once each, in `known`'s order.
fn named(deps: &[String], known: &[(&str, &str)]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for (dep, name) in known {
        if deps.iter().any(|d| d == dep) && !out.iter().any(|o| o == name) {
            out.push((*name).to_string());
        }
    }
    out
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_default()
}

pub fn read_stack(root: &Path, facts: &Facts) -> Stack {
    let mut stack = Stack::default();

    match facts.frontend {
        Frontend::None => {}
        Frontend::Topcoat => {
            stack.frontend.push("Topcoat".into());
            let cargo = read(root, "frontend/Cargo.toml");
            // Topcoat's own pieces are features of the one crate.
            for (feature, name) in [
                ("\"tailwind\"", "Tailwind"),
                ("\"icon-iconify\"", "Iconify icons"),
                ("\"ui\"", "Topcoat UI"),
            ] {
                if cargo.contains(feature) {
                    stack.frontend.push(name.into());
                }
            }
            stack.frontend.extend(named(&cargo_deps(&cargo), RUST_LIBS));
        }
        f => {
            stack.frontend.push(f.to_string());
            let deps = js_deps(&read(root, "frontend/package.json"));
            stack.frontend.extend(named(&deps, JS_LIBS));
        }
    }

    match facts.backend {
        Backend::None => {}
        Backend::Axum => {
            stack.backend.push("axum".into());
            let deps = cargo_deps(&read(root, "backend/Cargo.toml"));
            stack.backend.extend(named(&deps, RUST_LIBS));
            stack.data = named(&deps, RUST_DATA);
            if root.join("backend/migrations").is_dir() {
                stack.data.push("sqlx migrations".into());
            }
        }
        Backend::Fastapi => {
            stack.backend.push("FastAPI".into());
            let deps = python_deps(&read(root, "backend/pyproject.toml"));
            stack.backend.extend(named(&deps, PYTHON_LIBS));
            stack.data = named(&deps, PYTHON_DATA);
        }
    }

    // The services by the image they run, so a hand-added one shows too.
    let compose = read(root, "docker-compose.yml");
    for (image, name) in [
        ("postgres", "Postgres"),
        ("redis", "Redis"),
        ("mongo", "MongoDB"),
        ("keycloak", "Keycloak"),
        ("mailpit", "Mailpit"),
        ("mailtrap", "Mailtrap"),
        ("caddy", "Caddy (HTTPS)"),
    ] {
        let runs = compose.lines().any(|l| {
            l.trim()
                .strip_prefix("image:")
                .is_some_and(|i| i.trim().rsplit('/').next().unwrap_or("").starts_with(image))
        });
        if runs {
            stack.services.push(name.into());
        }
    }
    stack.services.push("Docker Compose".into());

    if facts.ci {
        stack.tools.push("CI (ci.sh)".into());
    }
    if root.join(".githooks/pre-commit").is_file() {
        stack.tools.push("gitleaks hook".into());
    }
    if root.join("frontend/.oxlintrc.json").is_file() {
        stack.tools.push("oxlint".into());
    }
    if root.join("frontend/eslint.config.mjs").is_file() {
        stack.tools.push("ESLint".into());
    }
    if facts.git {
        stack.tools.push("git".into());
    }
    stack
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_dependencies_are_read_from_every_dependencies_table() {
        let deps = cargo_deps(
            "[package]\nname = \"backend\"\n\n[dependencies]\naxum = \"0.8\"\nsqlx = { version = \"0.9\" }\n\n[dev-dependencies]\naxum-test = \"21\"\n\n[lints.clippy]\ntodo = \"warn\"\n",
        );
        assert_eq!(deps, ["axum", "sqlx", "axum-test"]);
        assert_eq!(named(&deps, RUST_DATA), ["sqlx (Postgres)"]);
    }

    #[test]
    fn python_requirements_lose_their_versions_and_extras() {
        let deps = python_deps(
            "[project]\nname = \"backend\"\ndependencies = [\n    \"fastapi>=0.120\",\n    \"uvicorn[standard]>=0.38\",\n    \"asyncpg>=0.30\",\n    \"pymongo>=4\",\n]\n",
        );
        assert!(deps.contains(&"uvicorn".to_string()), "{deps:?}");
        assert_eq!(named(&deps, PYTHON_DATA), ["asyncpg (Postgres)", "PyMongo"]);
    }

    #[test]
    fn a_library_with_two_package_names_is_named_once() {
        let deps = js_deps(
            r#"{"dependencies":{"@vueuse/core":"1","@vueuse/nuxt":"1"},"devDependencies":{"tailwindcss":"4"}}"#,
        );
        assert_eq!(named(&deps, JS_LIBS), ["Tailwind", "VueUse"]);
    }
}
