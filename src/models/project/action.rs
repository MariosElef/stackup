use crate::models::{Backend, Facts, Frontend};

/// What can be done to a project. The same list is the menu and the command line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Up,
    Down,
    Ps,
    Stop,
    Images,
    Clean,
    Logs,
    Dev,
    /// Apply the migrations not applied yet.
    Migrate,
    /// Make the next migration's files.
    MigrationNew,
    /// Undo the last migration.
    MigrationRevert,
    Build,
    Deploy,
    Ci,
    Fmt,
    /// Delete the Rust build files (`target/`).
    CargoClean,
    /// List the volumes and where each is mounted.
    Volumes,
    /// Store data that outlives the containers: mount a volume or a folder in a service.
    VolumeAdd,
    /// Save a volume to a file in `backups/`.
    VolumeBackup,
    /// Put a saved file back into a volume, replacing what is in it.
    VolumeRestore,
    Env,
    Certs,
    Hooks,
    Doctor,
    Remove,
    /// Opens lazygit on the project, inside the menu.
    GitOpen,
    GitInit,
    GitRepo,
}

/// The sections of the action list: related actions together, under a header.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Run,
    Database,
    Ship,
    Docker,
    Environment,
    Git,
    Project,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Run => "Run",
            Group::Database => "Database",
            Group::Ship => "Build & ship",
            Group::Docker => "Docker",
            Group::Environment => "Environment",
            Group::Git => "Git",
            Group::Project => "Project",
        }
    }
}

/// A value an action needs before it can run; the menu asks for it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Param {
    Host,
    Tier,
    /// Whether a new GitHub repository is private or public.
    Visibility,
    /// Whether removing a project also deletes its repository on GitHub or GitLab.
    RemoveFrom,
    /// A volume's name, or a `./folder`: typed.
    VolumeName,
    /// Which service gets the volume: picked.
    VolumeService,
    /// Where the volume appears inside the container: typed.
    MountPath,
    /// Which of the project's volumes: picked.
    VolumePick,
    /// Which backup file: picked.
    BackupFile,
    /// What a new migration is about, which names its files: typed.
    MigrationName,
}

impl Action {
    pub const ALL: &'static [Action] = &[
        Action::Up,
        Action::Down,
        Action::Logs,
        Action::Dev,
        Action::Migrate,
        Action::MigrationNew,
        Action::MigrationRevert,
        Action::Ps,
        Action::Stop,
        Action::Images,
        Action::Clean,
        Action::Volumes,
        Action::VolumeAdd,
        Action::VolumeBackup,
        Action::VolumeRestore,
        Action::Build,
        Action::Deploy,
        Action::Ci,
        Action::Fmt,
        Action::CargoClean,
        Action::Env,
        Action::Certs,
        Action::GitOpen,
        Action::GitInit,
        Action::GitRepo,
        Action::Hooks,
        Action::Doctor,
        Action::Remove,
    ];

    /// The command-line name: `stackup up`.
    pub fn name(self) -> &'static str {
        match self {
            Action::Up => "up",
            Action::Down => "down",
            Action::Ps => "ps",
            Action::Stop => "stop",
            Action::Images => "images",
            Action::Clean => "clean",
            Action::Logs => "logs",
            Action::Dev => "dev",
            Action::Migrate => "migrate",
            Action::MigrationNew => "new-migration",
            Action::MigrationRevert => "revert-migration",
            Action::Build => "build",
            Action::Deploy => "deploy",
            Action::Ci => "ci",
            Action::Fmt => "fmt",
            Action::CargoClean => "cargo-clean",
            Action::Volumes => "volumes",
            Action::VolumeAdd => "add-volume",
            Action::VolumeBackup => "backup-volume",
            Action::VolumeRestore => "restore-volume",
            Action::Env => "env",
            Action::Certs => "certs",
            Action::Hooks => "hooks",
            Action::Doctor => "doctor",
            Action::Remove => "remove",
            Action::GitOpen => "git",
            Action::GitInit => "git init",
            Action::GitRepo => "git repo",
        }
    }

    /// What the action list shows: under the Git header the word git is not repeated.
    pub fn label(self) -> &'static str {
        match self {
            Action::CargoClean => "cargo clean",
            Action::MigrationNew => "new migration",
            Action::MigrationRevert => "revert",
            Action::VolumeAdd => "add volume",
            Action::VolumeBackup => "backup volume",
            Action::VolumeRestore => "restore volume",
            Action::GitOpen => "lazygit",
            Action::GitInit => "init",
            Action::GitRepo => "create repo",
            other => other.name(),
        }
    }

    pub fn group(self) -> Group {
        match self {
            Action::Up | Action::Down | Action::Logs | Action::Dev => Group::Run,
            Action::Migrate | Action::MigrationNew | Action::MigrationRevert => Group::Database,
            Action::Ps
            | Action::Stop
            | Action::Images
            | Action::Clean
            | Action::Volumes
            | Action::VolumeAdd
            | Action::VolumeBackup
            | Action::VolumeRestore => Group::Docker,
            Action::Build | Action::Deploy | Action::Ci | Action::Fmt | Action::CargoClean => {
                Group::Ship
            }
            Action::Env | Action::Certs => Group::Environment,
            Action::GitOpen | Action::GitInit | Action::GitRepo => Group::Git,
            Action::Hooks | Action::Doctor | Action::Remove => Group::Project,
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Action::Up => "Build and start everything in Docker",
            Action::Down => "Stop everything",
            Action::Ps => "Show the containers",
            Action::Stop => "Stop the containers, and keep them stopped after a restart",
            Action::Images => "The images this project built, and their size",
            Action::Clean => "Delete the containers and the images this project built",
            Action::Logs => "Follow the logs of every service",
            Action::Dev => "Databases in Docker, the backend and the frontend with reload",
            Action::Migrate => "Apply the migrations not applied yet",
            Action::MigrationNew => "Make the next migration",
            Action::MigrationRevert => "Undo the last migration",
            Action::Build => "Build the Docker images",
            Action::Deploy => "Build for linux, send the images over SSH, start them",
            Action::Ci => "Run the checks CI runs",
            Action::Fmt => "Format the backend and the frontend",
            Action::CargoClean => "Delete the Rust build files, which can be gigabytes",
            Action::Volumes => "The volumes, and which service mounts each",
            Action::VolumeAdd => "Store data that outlives the containers",
            Action::VolumeBackup => "Save a volume to a file in backups/",
            Action::VolumeRestore => "Put a saved file back into a volume",
            Action::Env => "Switch environment, or make a new one",
            Action::Certs => "Create trusted local HTTPS certificates",
            Action::Hooks => "Turn on the gitleaks pre-commit hook",
            Action::Doctor => "Can this machine run it? Each failure names its fix",
            Action::Remove => "Delete the project: its containers, its data and its folder",
            Action::GitOpen => "lazygit, in this window: stage, commit, branches, push",
            Action::GitInit => "Start a git repository here",
            Action::GitRepo => "Create the GitHub repository and push to it",
        }
    }

    /// When to reach for it, and what it changes: shown beside the list.
    pub fn details(self) -> &'static str {
        match self {
            Action::Up => {
                "docker compose up --build. Makes the local HTTPS certificates first when they are missing. Ctrl-C stops it."
            }
            Action::Down => {
                "docker compose down. Containers and networks go; volumes (your data) stay."
            }
            Action::Ps => "docker compose ps.",
            Action::Stop => {
                "Sets the restart policy of every container to no, then stops them: they stay stopped when Docker or the computer restarts. Nothing is deleted, and `up` starts them again."
            }
            Action::Images => "docker image ls for the images this project built.",
            Action::Clean => {
                "docker compose down, then deletes the images this project built (they are rebuilt by up or build). Volumes, your data, stay. Databases' images are kept: other projects may use them."
            }
            Action::Logs => "docker compose logs --follow --tail 100, until stopped.",
            Action::Dev => {
                "Asks what to start: both, the backend or the frontend. The databases run in Docker; the backend (cargo watch, or uvicorn) and the frontend run on this machine with reload, so you can debug them. Ctrl-C stops them."
            }
            Action::Migrate => {
                "Starts Postgres in Docker, then applies what is new in backend/migrations: sqlx migrate run (axum) or alembic upgrade head (FastAPI). The backend also does this each time it starts, in dev and in Docker."
            }
            Action::MigrationNew => {
                "Asks what it is about (add_users), then writes its files in backend/migrations: an up and a down .sql (sqlx migrate add -r --sequential), or a revision with upgrade and downgrade (alembic revision). Fill them in; the next start applies it."
            }
            Action::MigrationRevert => {
                "Runs the last migration's down part: sqlx migrate revert (axum) or alembic downgrade -1 (FastAPI). It can drop tables and their data, so the project's name is typed back first."
            }
            Action::Build => {
                "docker compose build, for this machine. To build for a server, use deploy."
            }
            Action::Deploy => {
                "Builds the images for linux/amd64, streams them with docker save | ssh docker load (no registry), copies docker-compose.yml, the tier's .env and any mounted folders, then docker compose up -d on the server. Asks for the host; the project's name is typed back first."
            }
            Action::Ci => "Runs ci.sh: the same checks CI runs (format, lint, tests, layering).",
            Action::Fmt => "Formats the backend, then the frontend, with each one's own formatter.",
            Action::Volumes => {
                "Every volume the compose file mounts: its name or folder, the service, the path inside the container, and whether Docker has created it yet."
            }
            Action::VolumeAdd => {
                "Mount storage in a service so what it writes survives restarts and rebuilds. A name makes a Docker volume (kept by Docker); a ./folder shares a folder of the project. The compose file gets the entry, and the service is started again with it."
            }
            Action::VolumeBackup => {
                "Packs a volume into a .tar.gz in the project's backups/ folder, with a throwaway container, so it works for a database too. Nothing is stopped."
            }
            Action::VolumeRestore => {
                "Replaces what is in a volume with a saved backup. The services that use it are stopped first and started again after. This overwrites the volume's data, so the project's name is typed back first."
            }
            Action::CargoClean => {
                "cargo clean: deletes the target/ folder of the Rust crates, which grows with every build. The next build compiles everything again, so it takes longer. The label shows how much room it frees."
            }
            Action::Env => {
                "Copies .env.<tier> over .env, which compose reads. Pick one you have, or make a new one from .env.example with new random passwords."
            }
            Action::Certs => {
                "Runs scripts/certs.sh: one mkcert certificate for localhost, api.localhost and auth.localhost."
            }
            Action::Hooks => {
                "git config core.hooksPath .githooks, so the gitleaks hook runs before every commit."
            }
            Action::Doctor => {
                "Checks Docker, the tools this stack needs, .env, the certificate and whether the ports are free."
            }
            Action::GitOpen => {
                "Opens lazygit on the project, inside this window: stage, commit, branches, push and pull, the way lazygit does them. Quit it with q and you are back here. Needs lazygit installed (brew install lazygit)."
            }
            Action::GitInit => {
                "git init -b main, and turns on the gitleaks hook if the project has one. Only offered when this folder is not a repository yet."
            }
            Action::GitRepo => {
                "Asks private or public, then gh repo create, and pushes. It needs the GitHub CLI, signed in (gh auth login). If the project already points at a remote, that repository is created and pushed to."
            }
            Action::Remove => {
                "Stops the project's containers and deletes their data, deletes the project's folder, and stops listing it. It cannot be undone, so the project's name is typed back first. Press r on a project to start it; to only stop listing it, press d."
            }
        }
    }

    /// Whether this project has what the action works on.
    pub fn applies(self, f: &Facts) -> bool {
        f.exists
            && match self {
                Action::Dev => f.backend != Backend::None || f.frontend != Frontend::None,
                Action::Migrate | Action::MigrationNew | Action::MigrationRevert => f.migrations,
                Action::Ci => f.ci,
                Action::Certs => f.https,
                Action::Hooks => f.hooks,
                Action::CargoClean => f.rust_build,
                Action::VolumeBackup | Action::VolumeRestore => f.has_volumes,
                // A repository is made once, then worked with.
                Action::GitInit => !f.git,
                Action::GitOpen | Action::GitRepo => f.git,
                _ => true,
            }
    }

    pub fn params(self) -> &'static [Param] {
        match self {
            Action::Deploy => &[Param::Host],
            Action::MigrationNew => &[Param::MigrationName],
            Action::Env => &[Param::Tier],
            Action::GitRepo => &[Param::Visibility],
            Action::Remove => &[Param::RemoveFrom],
            Action::VolumeAdd => &[Param::VolumeName, Param::VolumeService, Param::MountPath],
            Action::VolumeBackup => &[Param::VolumePick],
            Action::VolumeRestore => &[Param::VolumePick, Param::BackupFile],
            _ => &[],
        }
    }

    /// Changes something that is not this machine, or cannot be undone: the project's name is
    /// typed back first.
    pub fn risky(self) -> bool {
        matches!(
            self,
            Action::Deploy | Action::Remove | Action::VolumeRestore | Action::MigrationRevert
        )
    }
}
