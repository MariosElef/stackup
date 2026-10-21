//! The command line: every command and flag.
use std::path::PathBuf;

use clap::{Args as ClapArgs, Parser, Subcommand};

use crate::models::{self, Backend, BeLib, FeLib, Frontend, Service, Tool};

#[derive(Parser)]
#[command(
    version,
    about = "Scaffold a full-stack project from the official tools, then run it",
    long_about = "Scaffold a full-stack project from the official tools, then run it.\n\nWith no command it opens a menu of the projects you built with stackup."
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

/// Which project a command means. Without either, the one you are inside.
#[derive(ClapArgs)]
pub struct Target {
    /// A project built with stackup, by name (`stackup projects` lists them).
    #[arg(short, long)]
    pub project: Option<String>,
    /// A project folder.
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Print the steps instead of running them.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum VolumeCmd {
    /// Store data that outlives the containers: mount a volume (or a ./folder) in a service.
    Add {
        #[command(flatten)]
        target: Target,
        /// A volume name like uploads, or ./folder to share a folder of the project.
        name: String,
        /// The service that gets it.
        #[arg(long)]
        service: String,
        /// Where it appears inside the container.
        #[arg(long, default_value = "/data")]
        mount: String,
    },
    /// Save a volume to a file in backups/.
    Backup {
        #[command(flatten)]
        target: Target,
        name: String,
    },
    /// Put a saved file back into a volume, replacing what is in it.
    Restore {
        #[command(flatten)]
        target: Target,
        name: String,
        /// The file in backups/.
        file: String,
        /// Do not ask for the project's name to be typed back.
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum GitCmd {
    /// Open lazygit on the project.
    Open(Target),
    /// Start a repository here.
    Init(Target),
    /// Create the GitHub repository with `gh` and push to it.
    Repo {
        #[command(flatten)]
        target: Target,
        /// Make it public (private by default).
        #[arg(long)]
        public: bool,
    },
}

/// The small jobs a step asks stackup itself to do, the same on every system.
#[derive(Subcommand)]
pub enum InternalCmd {
    /// Copy a file over another.
    Copy { from: PathBuf, to: PathBuf },
    /// Delete a folder and everything in it.
    Remove { path: PathBuf },
    /// Make a folder, and the ones above it.
    MakeDir { path: PathBuf },
    /// Mount a volume or a ./folder in a service of the compose file in this folder.
    AddVolume {
        service: String,
        source: String,
        target: String,
    },
    /// Print which parts of the project in a folder start, and where they answer.
    Banner { dir: PathBuf, which: String },
    /// Give a project the HTTPS proxy for development.
    HttpsDev { dir: PathBuf },
    /// Print the volumes of the project in a folder as a table.
    Volumes { dir: PathBuf },
}

#[derive(Subcommand)]
pub enum Cmd {
    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        cmd: InternalCmd,
    },
    /// Create a new project. Anything not given as a flag is asked for.
    New {
        name: String,
        #[arg(long, value_enum)]
        frontend: Option<Frontend>,
        #[arg(long, value_enum)]
        backend: Option<Backend>,
        #[arg(long, value_enum, value_delimiter = ',', num_args = 0..)]
        libs: Option<Vec<FeLib>>,
        /// Libraries for the backend (only utoipa for now, which needs axum).
        #[arg(long, value_enum, value_delimiter = ',', num_args = 0..)]
        backend_libs: Option<Vec<BeLib>>,
        #[arg(long, value_enum, value_delimiter = ',', num_args = 0..)]
        services: Option<Vec<Service>>,
        #[arg(long, value_enum, value_delimiter = ',', num_args = 0..)]
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
        /// Pin a version, like `--pin postgres=17` or `--pin rust=1.99.0`. Repeat it. What can be
        /// pinned: rust, topcoat, postgres, redis, mongodb, keycloak.
        #[arg(long = "pin", value_parser = models::parse_pin)]
        pins: Vec<(String, String)>,
    },
    /// List the projects you built with stackup.
    Projects,
    /// Add projects that already exist: a project folder, or a folder that holds several.
    Add {
        /// Where to look. Default: this folder.
        path: Option<PathBuf>,
    },
    /// Build and start everything in Docker.
    Up {
        #[command(flatten)]
        target: Target,
        /// Run in the background.
        #[arg(short, long)]
        detach: bool,
    },
    /// Stop everything.
    Down(Target),
    /// Show the containers.
    Ps(Target),
    /// Work with the project's git repository.
    Git {
        #[command(subcommand)]
        cmd: GitCmd,
    },
    /// Delete the Rust build files (target/), which can be many gigabytes.
    CargoClean(Target),
    /// List the volumes and which service mounts each.
    Volumes(Target),
    /// Add, back up and restore volumes: where the containers keep what must outlive them.
    Volume {
        #[command(subcommand)]
        cmd: VolumeCmd,
    },
    /// Stop the containers and keep them stopped, also after a restart.
    Stop(Target),
    /// List the Docker images this project built.
    Images(Target),
    /// Delete the containers and the images this project built.
    Clean(Target),
    /// Follow the logs of every service, or of one.
    Logs {
        #[command(flatten)]
        target: Target,
        service: Option<String>,
    },
    /// Start the databases in Docker and run the backend with reload.
    Dev(Target),
    /// Run the backend, the frontend, or both together, for development.
    Start {
        /// backend, frontend or fullstack.
        part: String,
        #[command(flatten)]
        target: Target,
    },
    /// Write the IDE run configurations (JetBrains, VS Code) that start each part, and the
    /// extensions VS Code is asked to use.
    Ide {
        #[command(flatten)]
        target: Target,
        /// Also install the recommended VS Code extensions (needs the `code` command).
        #[arg(long)]
        install_extensions: bool,
    },
    /// Build the Docker images, for this machine or for another platform.
    Build {
        #[command(flatten)]
        target: Target,
        /// For example linux/amd64.
        #[arg(long)]
        platform: Option<String>,
    },
    /// Build for linux, send the images to a server over SSH (no registry), and start them.
    Deploy {
        #[command(flatten)]
        target: Target,
        /// The server, for example deploy@example.com. Remembered for next time.
        #[arg(long)]
        host: Option<String>,
        /// The folder on the server. Default: /srv/<project>.
        #[arg(long)]
        dir: Option<String>,
        /// Which .env.<tier> becomes the server's .env.
        #[arg(long, default_value = "prod")]
        tier: String,
        #[arg(long, default_value = "linux/amd64")]
        platform: String,
        /// Do not ask for the project's name to be typed back.
        #[arg(long)]
        yes: bool,
    },
    /// Run the same checks CI runs.
    Ci(Target),
    /// Format the backend and the frontend.
    Fmt(Target),
    /// Switch environment: copies .env.<tier> to .env. With no tier, lists them.
    Env {
        #[command(flatten)]
        target: Target,
        tier: Option<String>,
        /// Make the tier from .env.example, with new random passwords, instead of switching.
        #[arg(long, requires = "tier")]
        create: bool,
    },
    /// Create trusted local HTTPS certificates (mkcert).
    Certs(Target),
    /// Turn on the gitleaks pre-commit hook.
    Hooks(Target),
    /// Show where the project is reachable on this machine: its web addresses, and its data.
    Urls(Target),
    /// Open the project's app in the browser.
    Open(Target),
    /// Check this machine can run the project, naming the fix for each failure.
    Doctor(Target),
    /// Delete a project: its containers, their data and its folder. This cannot be undone.
    Remove {
        #[command(flatten)]
        target: Target,
        /// Do not ask for the project's name to be typed back.
        #[arg(long)]
        yes: bool,
        /// Also delete the project's repository on GitHub. This cannot be undone.
        #[arg(long, conflicts_with = "gitlab")]
        github: bool,
        /// Also delete the project's repository on GitLab. This cannot be undone.
        #[arg(long)]
        gitlab: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Tool;
    use crate::ui;

    /// What `stackup new` reads back from the flags the menu builds for it.
    fn parsed(sel: &ui::Selection) -> Cmd {
        let mut argv = vec!["stackup".to_string(), "new".into(), "demo".into()];
        argv.extend(sel.cli_args());
        Cli::try_parse_from(argv)
            .expect("the flags parse")
            .cmd
            .expect("a command")
    }

    #[test]
    fn the_menus_choices_survive_the_trip_through_the_command_line() {
        let sel = ui::Selection::recommended();
        let Cmd::New {
            frontend,
            backend,
            libs,
            services,
            tools,
            backend_libs,
            ..
        } = parsed(&sel)
        else {
            panic!("not `new`");
        };
        assert_eq!(frontend, Some(sel.frontend));
        assert_eq!(backend, Some(sel.backend));
        assert_eq!(libs, Some(sel.libs.clone()));
        assert_eq!(services, Some(sel.services.clone()));
        assert_eq!(tools, Some(sel.tools.clone()));
        assert_eq!(backend_libs, Some(vec![]));
    }

    #[test]
    fn nothing_chosen_is_an_empty_list_and_not_a_missing_flag() {
        // A missing flag would open the wizard again inside the app.
        let sel = ui::Selection {
            frontend: Frontend::Topcoat,
            backend: Backend::None,
            libs: vec![],
            be_libs: vec![],
            services: vec![],
            tools: vec![Tool::Git],
        };
        let Cmd::New {
            frontend,
            backend,
            libs,
            services,
            tools,
            ..
        } = parsed(&sel)
        else {
            panic!("not `new`");
        };
        assert_eq!(
            (frontend, backend),
            (Some(Frontend::Topcoat), Some(Backend::None))
        );
        assert_eq!(libs, Some(vec![]));
        assert_eq!(services, Some(vec![]));
        assert_eq!(tools, Some(vec![Tool::Git]));
    }

    #[test]
    fn a_list_given_by_hand_still_works() {
        let cli = Cli::try_parse_from([
            "stackup",
            "new",
            "demo",
            "--services",
            "postgres,redis",
            "--tools",
            "git",
        ])
        .unwrap();
        let Some(Cmd::New { services, .. }) = cli.cmd else {
            panic!()
        };
        assert_eq!(services.map(|s| s.len()), Some(2));
    }

    /// Every command parses: a flag that clashes with another makes clap panic when it is used.
    #[test]
    fn the_whole_command_line_is_well_formed() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
        let cli = Cli::try_parse_from([
            "stackup",
            "volume",
            "add",
            "uploads",
            "--service",
            "backend",
            "--mount",
            "/uploads",
            "--path",
            ".",
        ]);
        assert!(cli.is_ok(), "{:?}", cli.err());
    }
}
