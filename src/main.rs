//! Layers, top down: `ui` (the wizard) -> `services` (what gets generated) -> `clients`
//! (external programs), all speaking in `models` (the plan). This file is the CLI itself.
mod clients;
mod models;
mod services;
mod ui;

use std::{fs, io::IsTerminal, path::PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use models::{Backend, FeLib, Frontend, Plan, Service, Tool, normalize_libs};

#[derive(Parser)]
#[command(
    version,
    about = "Scaffold a full-stack project from the official tools"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new project. Anything not given as a flag is asked for.
    New {
        name: String,
        #[arg(long, value_enum)]
        frontend: Option<Frontend>,
        #[arg(long, value_enum)]
        backend: Option<Backend>,
        #[arg(long, value_enum, value_delimiter = ',')]
        libs: Option<Vec<FeLib>>,
        #[arg(long, value_enum, value_delimiter = ',')]
        services: Option<Vec<Service>>,
        #[arg(long, value_enum, value_delimiter = ',')]
        tools: Option<Vec<Tool>>,
        /// Skip `git init`.
        #[arg(long)]
        no_git: bool,
        /// Remote for origin, e.g. git@github.com:me/app.git. Default: built from your `gh` login
        /// or the `github.user` git setting; no remote is added if neither is known.
        #[arg(long)]
        remote: Option<String>,
        /// Also commit, create the GitHub repo with `gh` (private) and push.
        #[arg(long)]
        create_remote: bool,
        /// With --create-remote, make the repo public.
        #[arg(long, requires = "create_remote")]
        public: bool,
    },
}

fn main() -> Result<()> {
    let Cmd::New {
        name,
        frontend,
        backend,
        libs,
        services,
        tools,
        no_git,
        remote,
        create_remote,
        public,
    } = Cli::parse().cmd;

    // `stackup new .` scaffolds into the current directory and names the project after it.
    let in_place = name == ".";
    let root = if in_place {
        std::env::current_dir()?
    } else {
        PathBuf::from(&name)
    };
    let project = slug(&if in_place {
        root.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        name.clone()
    });
    if project.is_empty() {
        bail!("could not derive a project name from `{name}`");
    }

    // Every choice given as a flag skips the wizard; anything missing opens it, pre-filled.
    let base = ui::tui::Selection::recommended();
    let complete = frontend.is_some() && backend.is_some() && services.is_some() && tools.is_some();
    let mut sel = ui::tui::Selection {
        frontend: frontend.unwrap_or(base.frontend),
        backend: backend.unwrap_or(base.backend),
        libs: libs.unwrap_or(base.libs),
        services: services.unwrap_or(base.services),
        tools: tools.unwrap_or(base.tools),
    };
    if !complete {
        if !std::io::stdout().is_terminal() {
            bail!("not a terminal: pass --frontend, --backend, --services and --tools");
        }
        match ui::tui::run(&project, sel)? {
            Some(chosen) => sel = chosen,
            None => return Ok(()),
        }
    }

    let mut libs = sel.libs;
    for note in normalize_libs(sel.frontend, &mut libs) {
        println!("note: {note}");
    }
    let plan = Plan {
        name: project,
        frontend: sel.frontend,
        backend: sel.backend,
        libs,
        services: sel.services,
        git: !no_git && sel.tools.contains(&Tool::Git),
        tools: sel.tools,
        remote,
        publish: create_remote.then_some(public),
    };
    services::frontend::preflight(&plan)?;
    services::backend::preflight(&plan)?;

    if in_place {
        if fs::read_dir(&root)?.any(|e| e.is_ok_and(|e| e.file_name() != ".git")) {
            bail!("the current directory is not empty");
        }
    } else {
        if root.exists() {
            bail!("`{name}` already exists");
        }
        fs::create_dir_all(&root)?;
    }
    let root = root.canonicalize()?;

    services::frontend::generate(&root, &plan)?;
    services::backend::generate(&root, &plan)?;
    services::tools::write_root(&root, &plan)?;
    services::tools::init_git(&root, &plan)?;

    let cd = if in_place {
        String::new()
    } else {
        format!("cd {name} && ")
    };
    let certs = if plan.has(Service::Https) {
        "./scripts/certs.sh && "
    } else {
        ""
    };
    println!("\nDone. {cd}{certs}docker compose up --build");
    Ok(())
}

/// Lowercase letters, digits and dashes: valid as a cargo, npm and Python project name.
fn slug(raw: &str) -> String {
    let mapped: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    mapped.trim_matches('-').to_string()
}
