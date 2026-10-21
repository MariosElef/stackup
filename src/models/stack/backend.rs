use super::choice::choice;

choice!(Backend { Axum => "axum (Rust)", Fastapi => "FastAPI (Python)", None => "none" });
choice!(BeLib { Utoipa => "utoipa (OpenAPI spec)" });

impl BeLib {
    /// FastAPI writes its OpenAPI document itself, so these only add something to axum.
    pub fn works_with(self, backend: Backend) -> bool {
        match self {
            BeLib::Utoipa => backend == Backend::Axum,
        }
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
