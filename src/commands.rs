//! The command line, one file per concern.
//!
//! - `args`: every command and flag
//! - `dispatch`: which command was asked for, and what it does
//! - `project`: commands on a project that exists
//! - `create`: `stackup new`
pub mod args;
pub mod create;
pub mod dispatch;
pub mod project;
