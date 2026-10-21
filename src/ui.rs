//! The terminal app: a menu of projects, and the wizard that makes one. Laid out by feature, with
//! what every screen shares in Nuxt-style folders:
//!
//! - `features`: one folder per part (`home`, `projects`, `create`, `git`), each with its pages,
//!   widgets, keys and state
//! - `app`: the entry point: terminal, event loop, top-level draw
//! - `layouts`: the frame around a page (status bar, command line)
//! - `components`: widgets shared by every screen (modals, output, palette)
//! - `composables`: logic shared by every screen (navigation, keys, footer)
//! - `stores`: state shared by every screen (`App`, `Run`)
//! - `cli`: how the same actions print in a terminal
//! - `types`, `theme`, `utils`: shared enums, colors and helpers
mod app;
pub mod cli;
mod components;
mod composables;
mod features;
mod layouts;
mod stores;
mod theme;
mod types;
mod utils;

// These run real programs (`echo`, `sh`) in the app's terminal: Unix tools.
#[cfg(all(test, unix))]
mod app_tests;
#[cfg(test)]
mod tests;

pub use app::{menu, run};
pub use features::create::selection::Selection;
