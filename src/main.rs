//! Layers, top down: `ui` (the menu and the command line) -> `services` (what gets generated
//! and what each action runs) -> `clients` (external programs), all speaking in `models`.
//! `commands` is the command line itself.
mod clients;
mod commands;
mod layers;
mod models;
mod services;
mod ui;

fn main() {
    if let Err(e) = commands::dispatch::run() {
        eprintln!("{} {e:#}", ui::cli::error("error:"));
        std::process::exit(1);
    }
}
