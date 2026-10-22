use super::choice::choice;

choice!(Backend { Axum => "axum (Rust)", Fastapi => "FastAPI (Python)", None => "none" });
choice!(BeLib {
    Utoipa => "utoipa (OpenAPI spec)",
    Sqlx => "sqlx migrations",
    Alembic => "Alembic migrations",
});

impl BeLib {
    /// FastAPI writes its OpenAPI document itself, so utoipa only adds something to axum. Each
    /// backend migrates its database with its own tool.
    pub fn works_with(self, backend: Backend) -> bool {
        match self {
            BeLib::Utoipa | BeLib::Sqlx => backend == Backend::Axum,
            BeLib::Alembic => backend == Backend::Fastapi,
        }
    }

    /// Migrations change a Postgres schema: without Postgres there is nothing to migrate.
    pub fn needs_postgres(self) -> bool {
        matches!(self, BeLib::Sqlx | BeLib::Alembic)
    }

    /// Why it cannot be ticked with these choices, if it cannot.
    pub fn why_not(self, backend: Backend, postgres: bool) -> Option<&'static str> {
        if !self.works_with(backend) {
            return Some(match self {
                BeLib::Utoipa | BeLib::Sqlx => "that needs the axum backend",
                BeLib::Alembic => "that needs the FastAPI backend",
            });
        }
        (self.needs_postgres() && !postgres)
            .then_some("migrations need Postgres: tick it in Services")
    }
}

/// The folders every generated project gets, taken from the mc2 repos. One list per stack,
/// shared by the generator and the wizard preview so the two cannot drift.
pub const AXUM_LAYERS: &[&str] = &[
    "routes",
    "services",
    "repositories",
    "models",
    "clients",
    "middleware",
];
pub const FASTAPI_LAYERS: &[&str] = &[
    "routes",
    "services",
    "repositories",
    "models",
    "schemas",
    "clients",
    "auth",
];
