//! One folder per part of the app, each holding everything for it: its pages, its widgets, its
//! keys and its state. Look here first for what a screen does.
//!
//! - `home`: the logo and the Start / My projects / Quit menu
//! - `projects`: the list of projects, and one project's actions, description and addresses
//! - `create`: the wizard that makes a project (choices, preview, install, tool checks)
pub mod create;
pub mod home;
pub mod projects;
