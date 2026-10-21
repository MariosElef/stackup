use super::choice::choice;

choice!(Backend { Axum => "axum (Rust)", Fastapi => "FastAPI (Python)", None => "none" });

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
