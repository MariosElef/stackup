use super::{Backend, FeLib, Frontend, Service, Tool};

pub struct Plan {
    pub name: String,
    pub frontend: Frontend,
    pub backend: Backend,
    pub libs: Vec<FeLib>,
    pub services: Vec<Service>,
    pub tools: Vec<Tool>,
    pub git: bool,
    /// Overrides the SSH origin that would otherwise be built from the GitHub username.
    pub remote: Option<String>,
    /// Create the GitHub repo with `gh` and push; `Some(true)` makes it public.
    pub publish: Option<bool>,
}

impl Plan {
    pub fn has(&self, s: Service) -> bool {
        self.services.contains(&s)
    }

    pub fn tool(&self, t: Tool) -> bool {
        self.tools.contains(&t)
    }

    pub fn lib(&self, l: FeLib) -> bool {
        self.libs.contains(&l)
    }

    /// The folders the frontend gets. Some come with a library: Pinia brings `stores/`,
    /// Pinia Colada `queries/`, and Zod or the OpenAPI client `contracts/`. `pages/` (Nuxt) is
    /// written separately because it carries the starter page.
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
    if libs.contains(&FeLib::Colada) && !libs.contains(&FeLib::Pinia) {
        libs.insert(0, FeLib::Pinia);
        notes.push("Pinia Colada needs Pinia, so Pinia was added".to_string());
    }
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
