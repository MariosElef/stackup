use super::{backend::Backend, choice::choice};

choice!(Service {
    Postgres => "Postgres",
    Redis => "Redis",
    Mongodb => "MongoDB",
    Keycloak => "Keycloak (auth)",
    Mailtrap => "Mailtrap (email sandbox)",
    Https => "Local HTTPS (Caddy + mkcert)",
});

impl Service {
    /// The package the backend uses to talk to this service, if it needs one. Keep in step with
    /// what `services::generate::backend` installs.
    pub fn driver(self, backend: Backend) -> Option<&'static str> {
        match (self, backend) {
            (Service::Postgres, Backend::Axum) => Some("sqlx"),
            (Service::Postgres, Backend::Fastapi) => Some("asyncpg"),
            // Same package name on crates.io and PyPI, so say which one.
            (Service::Redis, Backend::Axum) => Some("redis crate"),
            (Service::Redis, Backend::Fastapi) => Some("redis (py)"),
            (Service::Mongodb, Backend::Axum) => Some("mongodb"),
            (Service::Mongodb, Backend::Fastapi) => Some("pymongo"),
            _ => None,
        }
    }
}
