use std::path::{Path, PathBuf};

use crate::models::{Backend, Frontend};

/// A project built with stackup, as remembered between runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    /// Where `deploy` last went: the menu asks with this filled in.
    pub host: Option<String>,
}

/// What a project folder holds, read from its files rather than remembered. The actions offered
/// depend on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Facts {
    pub exists: bool,
    pub frontend: Frontend,
    pub backend: Backend,
    /// Local HTTPS: a Caddyfile and mkcert certificates.
    pub https: bool,
    pub ci: bool,
    pub hooks: bool,
    /// The folder is a git repository.
    pub git: bool,
    /// It has an `origin` remote.
    pub remote: bool,
    /// A Rust project with build files lying around (a `target/` folder).
    pub rust_build: bool,
    /// The compose file declares named volumes: something to back up.
    pub has_volumes: bool,
    /// The backend keeps migrations in `backend/migrations`: sqlx's or Alembic's.
    pub migrations: bool,
}

impl Facts {
    pub fn read(root: &Path) -> Self {
        let has = |rel: &str| root.join(rel).exists();
        let frontend = if has("frontend/nuxt.config.ts") {
            Frontend::Nuxt
        } else if has("frontend/vite.config.ts") {
            Frontend::Vue
        } else if has("frontend/Cargo.toml") {
            Frontend::Topcoat
        } else {
            Frontend::None
        };
        let backend = if has("backend/Cargo.toml") {
            Backend::Axum
        } else if has("backend/pyproject.toml") {
            Backend::Fastapi
        } else {
            Backend::None
        };
        Self {
            exists: root.join("docker-compose.yml").is_file(),
            frontend,
            backend,
            https: has("Caddyfile"),
            ci: has("ci.sh"),
            hooks: has(".githooks"),
            git: has(".git"),
            remote: std::fs::read_to_string(root.join(".git/config"))
                .is_ok_and(|c| c.contains("[remote \"origin\"]")),
            // A top-level `volumes:` is where a compose file declares its named volumes.
            migrations: has("backend/migrations"),
            has_volumes: std::fs::read_to_string(root.join("docker-compose.yml"))
                .is_ok_and(|t| t.lines().any(|l| l.trim_end() == "volumes:")),
            // A workspace has one `target/` at the root; separate crates have their own.
            rust_build: ["", "backend", "frontend"].iter().any(|d| {
                let dir = root.join(d);
                dir.join("Cargo.toml").is_file() && dir.join("target").is_dir()
            }),
        }
    }

    /// "Nuxt + axum", or a dash when there is neither. Short: it sits in a narrow list.
    pub fn summary(&self) -> String {
        let parts: Vec<String> = [
            (self.frontend != Frontend::None).then(|| short(&self.frontend.to_string())),
            (self.backend != Backend::None).then(|| short(&self.backend.to_string())),
        ]
        .into_iter()
        .flatten()
        .collect();
        if parts.is_empty() {
            "—".into()
        } else {
            parts.join(" + ")
        }
    }
}

/// `Topcoat (Rust)` as `Topcoat`.
fn short(name: &str) -> String {
    name.split(" (").next().unwrap_or(name).to_string()
}

/// The name as it was typed, made safe for a folder and a GitHub repository: letters (any case),
/// digits and `. _ -`. `Portfolio` stays `Portfolio`; the lowercase `slug` is for what needs it.
pub fn title(raw: &str) -> String {
    let mapped: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    mapped.trim_matches(['-', '.']).to_string()
}

/// Lowercase letters, digits and dashes: valid as a cargo, npm and Python project name.
pub fn slug(raw: &str) -> String {
    let mapped: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    mapped.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_keeps_the_case_it_was_typed_with() {
        assert_eq!(title("Portfolio"), "Portfolio");
        assert_eq!(title("My App!"), "My-App");
        assert_eq!(title("my_site.v2"), "my_site.v2");
        assert_eq!(title(".."), "");
        assert_eq!(
            slug("Portfolio"),
            "portfolio",
            "the package name is still lowercase"
        );
    }

    #[test]
    fn a_name_becomes_a_valid_package_name() {
        assert_eq!(slug("My App!"), "my-app");
        assert_eq!(slug("--x--"), "x");
        assert_eq!(slug("!!"), "");
    }
}
