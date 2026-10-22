//! Which command was asked for, and what it does.
use std::{env, io::IsTerminal};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};

use crate::{
    clients,
    models::{Action, Args},
    services::{
        self,
        projects::{doctor, registry},
    },
    ui,
};

use super::{args::*, create::*, project::*};

pub fn run() -> Result<()> {
    let Some(cmd) = Cli::parse().cmd else {
        if !std::io::stdout().is_terminal() {
            Cli::command().print_help()?;
            return Ok(());
        }
        return ui::menu();
    };
    match cmd {
        Cmd::New {
            name,
            frontend,
            backend,
            libs,
            backend_libs,
            services,
            tools,
            no_git,
            remote,
            create_remote,
            public,
            pins,
        } => {
            let (in_place, project) = project_name(&name)?;
            // Every choice given as a flag skips the wizard; anything missing opens it, pre-filled.
            let base = ui::Selection::recommended();
            let complete =
                frontend.is_some() && backend.is_some() && services.is_some() && tools.is_some();
            let mut sel = ui::Selection {
                frontend: frontend.unwrap_or(base.frontend),
                backend: backend.unwrap_or(base.backend),
                libs: libs.unwrap_or(base.libs),
                be_libs: backend_libs.unwrap_or(base.be_libs),
                services: services.unwrap_or(base.services),
                tools: tools.unwrap_or(base.tools),
            };
            if !complete {
                if !std::io::stdout().is_terminal() {
                    bail!("not a terminal: pass --frontend, --backend, --services and --tools");
                }
                match ui::run(&project, sel)? {
                    Some(chosen) => sel = chosen,
                    None => return Ok(()),
                }
            }
            create(Create {
                name,
                in_place,
                project,
                sel,
                no_git,
                remote,
                create_remote,
                public,
                pins,
            })
        }
        Cmd::Projects => {
            ui::cli::show_projects(&registry::load(&registry::home())?);
            Ok(())
        }
        Cmd::Add { path } => {
            let from = crate::clients::os::canonical(&path.unwrap_or(env::current_dir()?))?;
            let found = registry::add_found(&registry::home(), &from)?;
            if found.added.is_empty() && found.known.is_empty() {
                bail!(
                    "no project found in or under {}: a project has a docker-compose.yml",
                    from.display()
                );
            }
            for name in &found.added {
                println!("added {name}");
            }
            for name in &found.known {
                println!("already listed: {name}");
            }
            Ok(())
        }
        Cmd::Up { target, detach } => act(
            Action::Up,
            &target,
            Args {
                detach,
                ..Args::default()
            },
            true,
        ),
        Cmd::Down(t) => act(Action::Down, &t, Args::default(), true),
        Cmd::Ps(t) => act(Action::Ps, &t, Args::default(), true),
        Cmd::Internal { cmd } => match cmd {
            InternalCmd::Copy { from, to } => {
                std::fs::copy(&from, &to)
                    .with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
                Ok(())
            }
            InternalCmd::MakeDir { path } => {
                std::fs::create_dir_all(&path).with_context(|| format!("making {}", path.display()))
            }
            InternalCmd::AddVolume {
                service,
                source,
                target,
            } => {
                let file = std::path::Path::new("docker-compose.yml");
                let text = std::fs::read_to_string(file).context("reading docker-compose.yml")?;
                let out = services::projects::volumes::add(&text, &service, &source, &target)?;
                std::fs::write(file, out).context("writing docker-compose.yml")?;
                println!("docker-compose.yml: {source} is mounted at {target} in {service}");
                Ok(())
            }
            InternalCmd::Banner { dir, which } => {
                use services::projects::launch::Which;
                let which = match which.as_str() {
                    "backend" => Which::Backend,
                    "frontend" => Which::Frontend,
                    _ => Which::Both,
                };
                let facts = crate::models::Facts::read(&dir);
                print!(
                    "{}",
                    services::projects::launch::banner(&dir, &facts, which)
                );
                Ok(())
            }
            InternalCmd::HttpsDev { dir } => {
                println!("{}", services::projects::https_dev::enable(&dir)?);
                Ok(())
            }
            InternalCmd::Volumes { dir } => {
                let text = std::fs::read_to_string(dir.join("docker-compose.yml"))
                    .context("reading docker-compose.yml")?;
                let existing = clients::run::capture(
                    &dir,
                    "docker",
                    &["volume", "ls", "--format", "{{.Name}}"],
                );
                print!(
                    "{}",
                    services::projects::volumes::table(&dir, &text, existing.as_deref())
                );
                Ok(())
            }
            InternalCmd::Remove { path } => match std::fs::remove_dir_all(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    Err(e).with_context(|| format!("deleting {}", path.display()))
                }
                _ => Ok(()),
            },
        },
        Cmd::Git { cmd } => match cmd {
            GitCmd::Open(t) => act(Action::GitOpen, &t, Args::default(), true),
            GitCmd::Init(t) => act(Action::GitInit, &t, Args::default(), true),
            GitCmd::Repo { target, public } => act(
                Action::GitRepo,
                &target,
                Args {
                    visibility: Some(if public { "public" } else { "private" }.into()),
                    ..Args::default()
                },
                true,
            ),
        },
        Cmd::Volumes(t) => act(Action::Volumes, &t, Args::default(), true),
        Cmd::Volume { cmd } => match cmd {
            VolumeCmd::Add {
                target,
                name,
                service,
                mount,
            } => act(
                Action::VolumeAdd,
                &target,
                Args {
                    volume: Some(name),
                    service: Some(service),
                    mount: Some(mount),
                    ..Args::default()
                },
                true,
            ),
            VolumeCmd::Backup { target, name } => act(
                Action::VolumeBackup,
                &target,
                Args {
                    volume: Some(name),
                    ..Args::default()
                },
                true,
            ),
            VolumeCmd::Restore {
                target,
                name,
                file,
                yes,
            } => act(
                Action::VolumeRestore,
                &target,
                Args {
                    volume: Some(name),
                    backup: Some(file),
                    ..Args::default()
                },
                yes,
            ),
        },
        Cmd::CargoClean(t) => act(Action::CargoClean, &t, Args::default(), true),
        Cmd::Stop(t) => act(Action::Stop, &t, Args::default(), true),
        Cmd::Images(t) => act(Action::Images, &t, Args::default(), true),
        Cmd::Clean(t) => act(Action::Clean, &t, Args::default(), true),
        Cmd::Logs { target, service } => act(
            Action::Logs,
            &target,
            Args {
                service,
                ..Args::default()
            },
            true,
        ),
        Cmd::Dev(t) => act(Action::Dev, &t, Args::default(), true),
        Cmd::Migrate(t) => act(Action::Migrate, &t, Args::default(), true),
        Cmd::Migration { cmd } => match cmd {
            MigrationCmd::New { target, name } => act(
                Action::MigrationNew,
                &target,
                Args {
                    name: Some(name),
                    ..Args::default()
                },
                true,
            ),
            MigrationCmd::Revert { target, yes } => {
                act(Action::MigrationRevert, &target, Args::default(), yes)
            }
        },
        Cmd::Start { part, target } => start(&part, &target),
        Cmd::Ide {
            target,
            install_extensions,
        } => {
            let (project, facts) = open(&target)?;
            for line in write_ide(&project.path, &facts)? {
                println!("{line}");
            }
            if install_extensions {
                install_vscode_extensions(&project.path, &facts)?;
            }
            Ok(())
        }
        Cmd::Build { target, platform } => act(
            Action::Build,
            &target,
            Args {
                platform,
                ..Args::default()
            },
            true,
        ),
        Cmd::Deploy {
            target,
            host,
            dir,
            tier,
            platform,
            yes,
        } => act(
            Action::Deploy,
            &target,
            Args {
                host,
                dir,
                tier: Some(tier),
                platform: Some(platform),
                ..Args::default()
            },
            yes,
        ),
        Cmd::Ci(t) => act(Action::Ci, &t, Args::default(), true),
        Cmd::Fmt(t) => act(Action::Fmt, &t, Args::default(), true),
        Cmd::Env {
            target,
            tier,
            create,
        } => {
            let (project, _) = open(&target)?;
            match tier {
                None => {
                    let tiers = services::projects::envs::tiers(&project.path);
                    let current = services::projects::envs::current(&project.path);
                    ui::cli::show_envs(&tiers, current.as_deref());
                    Ok(())
                }
                Some(tier) if create => {
                    let made = services::projects::envs::create(&project.path, &tier)?;
                    ui::cli::show_created(&tier, &made);
                    Ok(())
                }
                Some(tier) => act(
                    Action::Env,
                    &target,
                    Args {
                        tier: Some(tier),
                        ..Args::default()
                    },
                    true,
                ),
            }
        }
        Cmd::Certs(t) => act(Action::Certs, &t, Args::default(), true),
        Cmd::Hooks(t) => act(Action::Hooks, &t, Args::default(), true),
        Cmd::Urls(t) => {
            let (project, facts) = open(&t)?;
            if !facts.exists {
                bail!("{} has no docker-compose.yml", project.path.display());
            }
            let urls = services::projects::urls::read(&project.path);
            if urls.is_empty() {
                bail!("no published ports: nothing is reachable from this machine");
            }
            ui::cli::show_urls(&urls);
            Ok(())
        }
        Cmd::Open(t) => {
            let (project, facts) = open(&t)?;
            if !facts.exists {
                bail!("{} has no docker-compose.yml", project.path.display());
            }
            let urls = services::projects::urls::read(&project.path);
            let Some(link) = urls.primary() else {
                bail!("this project publishes no web port: there is nothing to open");
            };
            clients::run::open_url(&link.url)?;
            println!("opened {}", link.url);
            Ok(())
        }
        Cmd::Remove {
            target,
            yes,
            github,
            gitlab,
        } => {
            let remote = if github {
                Some("github")
            } else if gitlab {
                Some("gitlab")
            } else {
                None
            };
            let args = Args {
                remove_remote: remote.map(str::to_string),
                ..Args::default()
            };
            act(Action::Remove, &target, args, yes)
        }
        Cmd::Doctor(t) => {
            let (project, facts) = open(&t)?;
            if !facts.exists {
                bail!("{} has no docker-compose.yml", project.path.display());
            }
            let checks = doctor::checks(&project.path, &facts);
            let failed = ui::cli::show_checks(&checks);
            if failed > 0 {
                bail!("{failed} of {} checks failed", checks.len());
            }
            Ok(())
        }
    }
}
