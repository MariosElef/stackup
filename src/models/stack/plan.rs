use super::{
    backend::{Backend, BeLib},
    frontend::{FeLib, Frontend},
    service::Service,
    tool::Tool,
};

pub struct Plan {
    /// The lowercase name for what needs one: crates, images, the compose project.
    pub name: String,
    /// The name as it was typed, for the folder, the list of projects, the README and GitHub.
    pub title: String,
    pub frontend: Frontend,
    pub backend: Backend,
    pub libs: Vec<FeLib>,
    pub be_libs: Vec<BeLib>,
    pub services: Vec<Service>,
    pub tools: Vec<Tool>,
    pub git: bool,
    /// Overrides the SSH origin that would otherwise be built from the GitHub username.
    pub remote: Option<String>,
    /// Create the GitHub repo with `gh` and push; `Some(true)` makes it public.
    pub publish: Option<bool>,
    /// Versions chosen for the project: `rust`, `topcoat`, `postgres`, `redis`, `mongodb`,
    /// `keycloak`. What is not here takes stackup's default.
    pub pins: Vec<(String, String)>,
}

/// The versions a project can be pinned to.
pub const PIN_KEYS: [&str; 6] = [
    "rust", "topcoat", "postgres", "redis", "mongodb", "keycloak",
];

/// A pin as `key=value`; the value ends up in a compose file or a toolchain file, so only the
/// characters a version or a tag is made of are let through.
pub fn parse_pin(text: &str) -> Result<(String, String), String> {
    let (key, value) = text
        .split_once('=')
        .ok_or_else(|| format!("`{text}` is not key=value, like postgres=17"))?;
    if !PIN_KEYS.contains(&key) {
        return Err(format!(
            "`{key}` cannot be pinned: use {}",
            PIN_KEYS.join(", ")
        ));
    }
    let ok = !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !ok {
        return Err(format!(
            "`{value}` is not a version: use letters, digits and . _ -"
        ));
    }
    Ok((key.to_string(), value.to_string()))
}

/// The rustup channel for a toolchain name: `1.99.0-aarch64-apple-darwin` is `1.99.0`.
pub fn rust_channel(toolchain: &str) -> String {
    if toolchain.starts_with(|c: char| c.is_ascii_digit()) {
        toolchain.split('-').next().unwrap_or(toolchain).to_string()
    } else {
        toolchain.to_string()
    }
}

impl Plan {
    /// The file that pins the Rust toolchain, with the chosen channel.
    pub fn rust_toolchain_file(&self) -> String {
        let channel = rust_channel(self.pin("rust").unwrap_or("stable"));
        include_str!("../../../assets/backend/axum/rust-toolchain.toml")
            .replace("channel = \"stable\"", &format!("channel = \"{channel}\""))
    }

    /// The image for a database: `latest` is the small Alpine one, a number is that major version.
    pub fn image(&self, key: &str, repo: &str, alpine: bool, default: &str) -> String {
        match self.pin(key).unwrap_or(default) {
            "latest" if alpine => format!("{repo}:alpine"),
            "latest" => format!("{repo}:latest"),
            v if alpine => format!("{repo}:{v}-alpine"),
            v => format!("{repo}:{v}"),
        }
    }

    pub fn has(&self, s: Service) -> bool {
        self.services.contains(&s)
    }

    pub fn pin(&self, key: &str) -> Option<&str> {
        self.pins
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn tool(&self, t: Tool) -> bool {
        self.tools.contains(&t)
    }

    pub fn lib(&self, l: FeLib) -> bool {
        self.libs.contains(&l)
    }

    /// Topcoat's UI components need Tailwind and its icons, so they come with both.
    pub fn topcoat_ui(&self) -> bool {
        self.lib(FeLib::Tailwind) && self.lib(FeLib::Lucide)
    }

    /// Topcoat and axum are both Rust: one Cargo workspace, one Dockerfile, one lockfile.
    pub fn rust_workspace(&self) -> bool {
        self.frontend == Frontend::Topcoat && self.backend == Backend::Axum
    }

    pub fn be_lib(&self, l: BeLib) -> bool {
        self.be_libs.contains(&l)
    }

    /// The folders the frontend gets. Some come with a library: Pinia brings `stores/`,
    /// Pinia Colada `queries/`, and Zod or the OpenAPI client `contracts/`. `pages/` (Nuxt) is
    /// written separately because it carries the starter page, and Topcoat's `app`, `features`
    /// and `components` because `topcoat new` writes them.
    pub fn frontend_layers(&self) -> Vec<&'static str> {
        let mut v = match self.frontend {
            Frontend::Nuxt => vec![
                "assets",
                "components",
                "composables",
                "layouts",
                "middleware",
                "plugins",
                "services",
                "types",
                "utils",
            ],
            Frontend::Vue => vec![
                "views",
                "components",
                "composables",
                "layouts",
                "plugins",
                "services",
                "types",
                "utils",
            ],
            // `app` and `features` come from `topcoat new`, and so does `components` when it adds
            // the UI components; these are the rest.
            Frontend::Topcoat => {
                let mut v = vec!["composables", "services", "middleware", "types", "utils"];
                if !self.topcoat_ui() {
                    v.push("components");
                }
                v
            }
            Frontend::None => vec![],
        };
        if self.lib(FeLib::Pinia) {
            v.push("stores");
        }
        if self.lib(FeLib::Colada) {
            v.push("queries");
        }
        if self.lib(FeLib::Zod) || self.lib(FeLib::Openapi) {
            v.push("contracts");
        }
        v
    }
}

/// Libraries depend on each other and on the frontend. Pinia Colada needs Pinia. Returns what was dropped or added, to tell the user.
pub fn normalize_libs(frontend: Frontend, libs: &mut Vec<FeLib>) -> Vec<String> {
    let mut notes = Vec::new();
    if frontend == Frontend::None {
        libs.clear();
        return notes;
    }
    let before = libs.len();
    libs.retain(|l| l.works_with(frontend));
    if libs.len() < before {
        notes.push(format!(
            "{frontend} cannot use some of those libraries, so they were dropped"
        ));
    }
    if libs.contains(&FeLib::Colada) && !libs.contains(&FeLib::Pinia) {
        libs.insert(0, FeLib::Pinia);
        notes.push("Pinia Colada needs Pinia, so Pinia was added".to_string());
    }
    notes
}

/// Backend libraries only apply to the backends they work with. Returns what was dropped, to tell
/// the user.
pub fn normalize_be_libs(
    backend: Backend,
    services: &[Service],
    libs: &mut Vec<BeLib>,
) -> Vec<String> {
    let mut notes = Vec::new();
    let postgres = services.contains(&Service::Postgres);
    libs.retain(|l| {
        let why = l.why_not(backend, postgres);
        if let Some(why) = why {
            notes.push(format!("{l} was dropped: {why}"));
        }
        why.is_none()
    });
    notes
}

/// Hostnames behind the local HTTPS proxy. `*.localhost` resolves to 127.0.0.1 in browsers, and
/// one mkcert certificate covers all three.
pub const WEB_HOST: &str = "localhost";
pub const API_HOST: &str = "api.localhost";
pub const AUTH_HOST: &str = "auth.localhost";
/// Host port for the HTTPS proxy. Not 443: Docker Desktop's Kubernetes (Traefik) often holds
/// 80 and 443 on a developer machine.
pub const HTTPS_PORT: u16 = 8443;

impl Plan {
    /// Where the browser reaches the API.
    pub fn api_url(&self) -> String {
        if self.has(Service::Https) {
            format!("https://{API_HOST}:{HTTPS_PORT}")
        } else {
            "http://localhost:8000".into()
        }
    }

    /// Where the browser reaches Keycloak. This is also the token issuer, so it is what the
    /// backend must expect in the `iss` claim.
    pub fn auth_url(&self) -> String {
        if self.has(Service::Https) {
            format!("https://{AUTH_HOST}:{HTTPS_PORT}")
        } else {
            "http://localhost:8080".into()
        }
    }

    /// Where the browser reaches the frontend.
    pub fn web_url(&self) -> String {
        if self.has(Service::Https) {
            format!("https://{WEB_HOST}:{HTTPS_PORT}")
        } else {
            "http://localhost:3000".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pin_is_a_known_key_and_a_value_made_of_version_characters() {
        assert_eq!(
            parse_pin("postgres=17"),
            Ok(("postgres".into(), "17".into()))
        );
        assert_eq!(
            parse_pin("rust=1.99.0"),
            Ok(("rust".into(), "1.99.0".into()))
        );
        for bad in [
            "postgres",
            "nginx=1",
            "postgres=",
            "postgres=17 && rm",
            "rust=a/b",
            "redis=7\nx",
        ] {
            assert!(parse_pin(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_toolchain_name_becomes_its_channel() {
        assert_eq!(rust_channel("1.99.0-aarch64-apple-darwin"), "1.99.0");
        assert_eq!(rust_channel("1.85-aarch64-apple-darwin"), "1.85");
        assert_eq!(rust_channel("stable"), "stable");
        assert_eq!(rust_channel("nightly"), "nightly");
    }
}
