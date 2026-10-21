//! A project's git state, and the steps for working with it.
mod repo;
mod status;
#[cfg(test)]
mod tests;

pub use repo::*;
pub use status::*;
