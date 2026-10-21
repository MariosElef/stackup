//! The HTTPS reverse proxy: its compose service and its Caddyfile.
use crate::models::{API_HOST, AUTH_HOST, Backend, Frontend, Plan, Service, WEB_HOST};

pub(super) fn caddy(plan: &Plan) -> String {
    let mut deps = Vec::new();
    if plan.frontend != Frontend::None {
        deps.push("frontend");
    }
    if plan.backend != Backend::None {
        deps.push("backend");
    }
    if plan.has(Service::Keycloak) {
        deps.push("keycloak");
    }
    let mut s = String::from(
        "  caddy:\n    image: caddy:alpine\n    ports: [\"8443:443\"]\n    volumes:\n      - ./Caddyfile:/etc/caddy/Caddyfile:ro\n      - ./certs:/certs:ro\n",
    );
    if !deps.is_empty() {
        s.push_str(&format!("    depends_on: [{}]\n", deps.join(", ")));
    }
    s.push('\n');
    s
}

/// The reverse proxy: one TLS site per component, all using the mkcert certificate.
pub fn caddyfile(plan: &Plan) -> String {
    let mut s = String::from("{\n\tauto_https off\n}\n");
    let mut site = |host: &str, upstream: &str| {
        s.push_str(&format!(
            "\nhttps://{host} {{\n\ttls /certs/local.pem /certs/local-key.pem\n\treverse_proxy {upstream}\n}}\n"
        ));
    };
    match plan.frontend {
        Frontend::Nuxt => site(WEB_HOST, "frontend:3000"),
        Frontend::Vue => site(WEB_HOST, "frontend:80"),
        Frontend::None => {}
    }
    if plan.backend != Backend::None {
        site(API_HOST, "backend:8000");
    }
    if plan.has(Service::Keycloak) {
        site(AUTH_HOST, "keycloak:8080");
    }
    s
}
