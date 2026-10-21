//! Commands on a project that exists: start, ide, and every action.
use std::env;

use anyhow::{Result, bail};

use crate::{
    clients,
    models::{Action, Args, Facts, Project},
    services::{
        self,
        projects::{plan::plan as plan_steps, registry, resolve::resolve},
    },
    ui,
};

use super::args::*;
use crate::services::projects::git::Host;

/// The project a command means, and what its folder holds.
pub fn open(target: &Target) -> Result<(Project, Facts)> {
    let project = resolve(
        &registry::home(),
        target.project.as_deref(),
        target.path.as_deref(),
        &env::current_dir()?,
    )?;
    let facts = Facts::read(&project.path);
    Ok((project, facts))
}

/// Writes the run configurations for the parts the project has; what was written, one per line.
pub fn write_ide(root: &std::path::Path, facts: &Facts) -> Result<Vec<String>> {
    services::projects::ide::write(root, facts, &env::current_exe()?)
}

/// Installs the VS Code extensions the project asks for, with the `code` command.
pub fn install_vscode_extensions(root: &std::path::Path, facts: &Facts) -> Result<()> {
    let Some(installed) = clients::run::capture(root, "code", &["--list-extensions"]) else {
        bail!(
            "VS Code's `code` command was not found: in VS Code, run \"Shell Command: Install 'code' command in PATH\" from the command palette"
        );
    };
    let steps = services::projects::ide::install_steps(facts, root, &installed);
    if steps.is_empty() {
        println!("the extensions are all installed");
    }
    for step in &steps {
        ui::cli::show_step(step);
        clients::run::run_step(step)?;
    }
    Ok(())
}

/// Starts the backend, the frontend or both, in this terminal.
pub fn start(part: &str, target: &Target) -> Result<()> {
    let (project, facts) = open(target)?;
    let part = services::projects::ide::Part::parse(part)?;
    for step in services::projects::plan::start(part, &project, &facts)? {
        ui::cli::show_step(&step);
        if target.dry_run {
            println!();
        } else {
            clients::run::run_step(&step)?;
        }
    }
    Ok(())
}

/// Runs one action on a project: its steps, one after another, in this terminal.
pub fn act(action: Action, target: &Target, args: Args, confirmed: bool) -> Result<()> {
    let (project, facts) = open(target)?;
    let steps = plan_steps(action, &project, &facts, &args)?;
    if target.dry_run {
        for step in &steps {
            ui::cli::show_step(step);
            println!();
        }
        return Ok(());
    }
    if action.risky() && !confirmed {
        let what = if action == Action::Remove {
            let also = match args.remove_remote.as_deref().and_then(Host::parse) {
                Some(h) => format!(" AND its repository on {}", h.label()),
                None => String::new(),
            };
            format!(
                "Removing {} deletes {} with its containers and data{also}. This cannot be undone.",
                project.name,
                project.path.display()
            )
        } else if action == Action::VolumeRestore {
            format!(
                "Restoring {} from {} replaces what is in the volume, and stops what uses it.",
                args.volume.as_deref().unwrap_or("?"),
                args.backup.as_deref().unwrap_or("?")
            )
        } else {
            let tier = args.tier.as_deref().unwrap_or("prod");
            let host = args
                .host
                .as_deref()
                .or(project.host.as_deref())
                .unwrap_or("?");
            format!(
                "Deploying {} to {host} [{}]",
                project.name,
                ui::cli::tier(tier)
            )
        };
        ui::cli::confirm(&what, &project.name)?;
    }
    for step in &steps {
        ui::cli::show_step(step);
        clients::run::run_step(step)?;
    }
    // Where a deploy went is remembered, so the menu can offer it next time.
    if let (Action::Deploy, Some(host)) = (action, &args.host) {
        let _ = registry::set_host(&registry::home(), &project.path, host);
    }
    // A removed project is no longer listed.
    if action == Action::Remove {
        let _ = registry::forget(&registry::home(), &project.path);
        println!("removed {}", project.name);
    }
    Ok(())
}
