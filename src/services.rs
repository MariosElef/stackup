//! What the app does, with no screen in it. One folder per concern:
//!
//! - `generate`: what `stackup new` writes: the backend, the frontend, the compose file, the
//!   tools (CI, env, readme, git) and the Rust workspace
//! - `projects`: working with a project that exists: its actions, git, environments, addresses,
//!   the list of projects
//! - `toolchain`: what this machine has, which versions can be installed, and the fixes offered
//!   when something fails
pub mod generate;
pub mod projects;
pub mod toolchain;
