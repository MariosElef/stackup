//! `stackup new`: the choices made into a project on disk.
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::{
    clients,
    models::{self, Facts, Plan, Tool, normalize_be_libs, normalize_libs, slug},
    services::{self, projects::registry},
    ui,
};

use super::project::write_ide;

/// Makes every `cargo` this run starts use the Rust that was chosen, getting it first if it is
/// not installed. Without it the project would be set up with whatever Rust is the default, and
/// a Topcoat dependency that needs a newer one would fail before the project's own
/// rust-toolchain.toml is even written.
pub fn use_rust(plan: &Plan) -> Result<()> {
    let Some(pick) = plan.pin("rust") else {
        return Ok(());
    };
    let channel = models::rust_channel(pick);
    if !clients::run::have("rustup") {
        return Ok(());
    }
    let list =
        clients::run::capture(Path::new("."), "rustup", &["toolchain", "list"]).unwrap_or_default();
    let installed = list.lines().any(|l| {
        let name = l.split_whitespace().next().unwrap_or("");
        name == channel || name.starts_with(&format!("{channel}-"))
    });
    // `stable`, `beta` and `nightly` move: asking for them gets the newest, not what is on disk.
    let moving = matches!(channel.as_str(), "stable" | "beta" | "nightly");
    if !installed || moving {
        println!("Installing Rust {channel}...");
        clients::run::run(
            Path::new("."),
            "rustup",
            &[
                "toolchain",
                "install",
                &channel,
                "--profile",
                "minimal",
                "-c",
                "rustfmt",
                "-c",
                "clippy",
            ],
        )?;
    }
    // Nothing else is running yet, so changing the environment is safe.
    unsafe { env::set_var("RUSTUP_TOOLCHAIN", &channel) };
    Ok(())
}

/// What `stackup new` is asked to make.
pub struct Create {
    pub name: String,
    pub in_place: bool,
    pub project: String,
    pub sel: ui::Selection,
    pub no_git: bool,
    pub remote: Option<String>,
    pub create_remote: bool,
    pub public: bool,
    pub pins: Vec<(String, String)>,
}

/// `stackup new .` scaffolds into the current directory and names the project after it.
pub fn project_name(name: &str) -> Result<(bool, String)> {
    let in_place = name == ".";
    let project = slug(&if in_place {
        env::current_dir()?
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        name.to_string()
    });
    if project.is_empty() {
        bail!("could not derive a project name from `{name}`");
    }
    Ok((in_place, project))
}

pub fn create(c: Create) -> Result<()> {
    let Create {
        name,
        in_place,
        project,
        sel,
        no_git,
        remote,
        create_remote,
        public,
        pins,
    } = c;
    let root = if in_place {
        env::current_dir()?
    } else {
        PathBuf::from(&name)
    };

    // The name as it was typed (`Portfolio`): the list of projects, the README and GitHub show it.
    // The lowercase `project` is for what must be lowercase: crates, images, the compose project.
    let title = {
        let typed = if in_place {
            root.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            name.clone()
        };
        let t = models::title(&typed);
        if t.is_empty() { project.clone() } else { t }
    };

    let mut libs = sel.libs;
    for note in normalize_libs(sel.frontend, &mut libs) {
        println!("note: {note}");
    }
    let mut be_libs = sel.be_libs;
    for note in normalize_be_libs(sel.backend, &sel.services, &mut be_libs) {
        println!("note: {note}");
    }
    let plan = Plan {
        name: project,
        title,
        frontend: sel.frontend,
        backend: sel.backend,
        libs,
        be_libs,
        services: sel.services,
        git: !no_git && sel.tools.contains(&Tool::Git),
        tools: sel.tools,
        remote,
        publish: create_remote.then_some(public),
        pins,
    };
    use_rust(&plan)?;
    services::generate::frontend::preflight(&plan)?;
    services::generate::backend::preflight(&plan)?;

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
    let root = crate::clients::os::canonical(&root)?;

    services::generate::frontend::generate(&root, &plan)?;
    services::generate::backend::generate(&root, &plan)?;
    // Topcoat and axum are both Rust: one workspace, not two projects.
    if plan.rust_workspace() {
        services::generate::workspace::fuse(&root, &plan, &format!("{}-backend", plan.name))?;
    }
    services::generate::tools::write_root(&root, &plan)?;
    services::generate::tools::init_git(&root, &plan)?;
    // Local HTTPS works from the first `up`: the certificate is made now, while the person is
    // here for the one password prompt that trusts it. A failure only means `up` tries again.
    if plan.has(models::Service::Https) && env::var_os("STACKUP_NO_CERTS").is_none() {
        println!("\nMaking the local HTTPS certificate...");
        let made = services::projects::certs::steps(&root)
            .iter()
            .try_for_each(|step| {
                crate::ui::cli::show_step(step);
                clients::run::run_step(step)
            });
        if made.is_err() {
            println!("  Not made yet: `stackup up` tries again, or run `stackup certs`.");
        }
    }
    // Ready to start the backend, the frontend or both from the IDE, for what was built.
    let _ = write_ide(&root, &Facts::read(&root));
    remember(&plan.title, &root);

    let cd = if in_place {
        String::new()
    } else {
        format!("cd {name} && ")
    };
    // `up` makes the HTTPS certificates itself when they are missing.
    if env::var_os("STACKUP_MENU").is_some() {
        println!("\nDone. Press q to go back to your projects.");
    } else {
        println!("\nDone. {cd}stackup up   (or just `stackup`, for the menu)");
    }
    Ok(())
}

/// Adds the project to the list the menu shows. Not worth failing a finished project for.
pub fn remember(name: &str, root: &Path) {
    if let Err(e) = registry::add(&registry::home(), name, root) {
        eprintln!("note: could not add {name} to your projects: {e:#}");
    }
}
