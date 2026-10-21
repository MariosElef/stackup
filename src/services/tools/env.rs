//! The per-tier .env files.
use crate::models::{Backend, Frontend, Plan, Service};

pub(super) fn file(plan: &Plan, tier: &str) -> String {
    let mut s = if tier == "example" {
        String::from(
            "# Template. Copy to .env.<tier> (local, dev, prod) and fill it in.\n# The real files are gitignored; `make env TIER=<tier>` copies one to .env.\n",
        )
    } else {
        format!("# {tier} tier (gitignored). `make env TIER={tier}` copies this file to .env.\n")
    };
    if plan.has(Service::Postgres) {
        s.push_str("POSTGRES_USER=app\nPOSTGRES_PASSWORD=change-me\nPOSTGRES_DB=app\n");
        s.push_str("DATABASE_URL=postgres://app:change-me@localhost:5432/app\n");
    }
    if plan.has(Service::Redis) {
        s.push_str("REDIS_URL=redis://localhost:6379\n");
    }
    if plan.has(Service::Mongodb) {
        s.push_str("MONGODB_URL=mongodb://localhost:27017\n");
    }
    if plan.has(Service::Keycloak) {
        s.push_str("KEYCLOAK_ADMIN=admin\nKEYCLOAK_ADMIN_PASSWORD=admin\n");
        s.push_str(&format!("OIDC_ISSUER_URL={}/realms/app\n", plan.auth_url()));
        s.push_str(
            "OIDC_JWKS_URL=http://localhost:8080/realms/app/protocol/openid-connect/certs\n",
        );
    }
    if plan.has(Service::Mailtrap) {
        s.push_str("# From your Mailtrap inbox: Integration -> SMTP.\n");
        s.push_str("MAILTRAP_HOST=sandbox.smtp.mailtrap.io\nMAILTRAP_PORT=2525\nMAILTRAP_USER=\nMAILTRAP_PASSWORD=\n");
    }
    if plan.backend != Backend::None {
        s.push_str("BIND_ADDR=0.0.0.0:8000\n");
    }
    if plan.frontend == Frontend::Nuxt {
        s.push_str(&format!("NUXT_PUBLIC_API_BASE={}\n", plan.api_url()));
    }
    s
}
