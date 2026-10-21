//! The words every layer speaks: what is chosen (`stack`) and what a project is and can do
//! (`project`).
pub mod project;
pub mod stack;

pub use project::action::{Action, Group, Param};
pub use project::facts::{Facts, Project, slug, title};
pub use project::step::{Args, Step};
pub use stack::backend::{AXUM_LAYERS, Backend, BeLib, FASTAPI_LAYERS};
pub use stack::frontend::{FeLib, Frontend};
pub use stack::plan::{
    API_HOST, AUTH_HOST, Plan, WEB_HOST, normalize_be_libs, normalize_libs, parse_pin, rust_channel,
};
pub use stack::service::Service;
pub use stack::tool::Tool;
