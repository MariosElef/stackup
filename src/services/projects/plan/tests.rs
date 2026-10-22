use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::project::check_removable;
use super::*;
use crate::services::projects::{https_dev, ide::Part, urls};

/// A project folder with `files` in it, and facts read from them.
fn project(name: &str, files: &[&str]) -> (Project, Facts) {
    // One folder deeper than the temp dir, which is as shallow as `/tmp` on Linux: a project
    // folder this near the top of the disk would be refused for removal.
    let dir = env::temp_dir()
        .join("stackup-plan")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for f in files.iter().chain(&["docker-compose.yml"]) {
        let path = dir.join(f);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }
    let p = Project {
        name: "demo".into(),
        path: dir.clone(),
        host: None,
    };
    (p, Facts::read(&dir))
}

/// `dev` reads the compose file with `docker compose config`: tests of it need Docker, which a
/// machine running the tests may not have. They are skipped there.
fn docker_here() -> bool {
    let here = crate::clients::run::have("docker");
    if !here {
        eprintln!("skipped: docker is not installed");
    }
    here
}

fn lines(steps: &[Step]) -> Vec<String> {
    steps.iter().map(Step::command_line).collect()
}

#[test]
fn up_makes_certificates_first_only_when_missing() {
    let (p, f) = project("up", &["Caddyfile", "scripts/certs.sh"]);
    let steps = plan(Action::Up, &p, &f, &Args::default()).unwrap();
    let l = lines(&steps);
    assert_eq!(l.len(), 3, "{l:?}");
    assert_eq!(l[0], "mkcert -install", "trust first, with no shell script");
    assert!(
        l[1].starts_with("mkcert -cert-file certs/local.pem"),
        "{l:?}"
    );
    assert_eq!(l[2], "docker compose up --build");
    fs::create_dir_all(p.path.join("certs")).unwrap();
    fs::write(p.path.join("certs/local.pem"), "").unwrap();
    let steps = plan(
        Action::Up,
        &p,
        &f,
        &Args {
            detach: true,
            ..Args::default()
        },
    )
    .unwrap();
    assert_eq!(lines(&steps), ["docker compose up --build -d"]);
}

#[test]
fn actions_only_apply_where_the_project_has_what_they_work_on() {
    let (p, f) = project("none", &[]);
    for a in [Action::Dev, Action::Ci, Action::Certs, Action::Hooks] {
        assert!(plan(a, &p, &f, &Args::default()).is_err(), "{}", a.name());
    }
    let (p, f) = project(
        "some",
        &["ci.sh", ".githooks/pre-commit", "backend/Cargo.toml"],
    );
    assert_eq!(
        lines(&plan(Action::Ci, &p, &f, &Args::default()).unwrap()),
        ["sh ci.sh"]
    );
    assert_eq!(
        lines(&plan(Action::Hooks, &p, &f, &Args::default()).unwrap()),
        ["git config core.hooksPath .githooks"]
    );
    assert_eq!(
        lines(&plan(Action::Fmt, &p, &f, &Args::default()).unwrap()),
        ["cargo fmt --all"]
    );
}

#[test]
fn fmt_uses_each_stacks_own_formatter() {
    let (p, f) = project(
        "fmt",
        &["backend/pyproject.toml", "frontend/nuxt.config.ts"],
    );
    assert_eq!(
        lines(&plan(Action::Fmt, &p, &f, &Args::default()).unwrap()),
        ["uv run ruff format .", "pnpm fmt"]
    );
}

#[test]
fn env_checks_the_tier_and_the_file_before_copying() {
    let (p, f) = project("env", &[".env.dev"]);
    let args = |t: &str| Args {
        tier: Some(t.into()),
        ..Args::default()
    };
    let steps = plan(Action::Env, &p, &f, &args("dev")).unwrap();
    assert_eq!(steps[0].args, ["internal", "copy", ".env.dev", ".env"]);
    assert!(lines(&steps)[0].ends_with("internal copy .env.dev .env"));
    assert!(
        plan(Action::Env, &p, &f, &args("prod")).is_err(),
        "no such file"
    );
    assert!(
        plan(Action::Env, &p, &f, &args("../x")).is_err(),
        "not a tier name"
    );
}

#[test]
fn deploy_needs_a_host_and_the_tiers_env_file() {
    let (mut p, f) = project("deploy", &[]);
    assert!(
        plan(Action::Deploy, &p, &f, &Args::default()).is_err(),
        "no host"
    );
    p.host = Some("ops@example.com".into());
    let err = plan(Action::Deploy, &p, &f, &Args::default()).unwrap_err();
    assert!(err.to_string().contains(".env.prod not found"), "{err}");
}

#[test]
fn a_folder_without_compose_is_refused() {
    let p = Project {
        name: "x".into(),
        path: PathBuf::from("/nonexistent-stackup"),
        host: None,
    };
    let f = Facts::read(&p.path);
    assert!(plan(Action::Up, &p, &f, &Args::default()).is_err());
}

#[test]
fn only_a_project_folder_that_is_safely_below_the_top_can_be_removed() {
    let (p, _) = project("removable", &[]);
    let cwd = Path::new("/somewhere/else");
    assert!(check_removable(&p.path, cwd, None).is_ok());

    // Not a project.
    assert!(check_removable(Path::new("/tmp/nothing-here-stackup"), cwd, None).is_err());
    // Holding the home folder, or standing inside it.
    assert!(check_removable(&p.path, cwd, Some(&p.path.join("home"))).is_err());
    assert!(check_removable(&p.path, &p.path.join("backend"), None).is_err());
}

#[test]
fn a_folder_near_the_top_is_refused_even_with_a_compose_file() {
    // Few enough path parts that deleting it would take most of a disk with it.
    assert!(check_removable(Path::new("/srv"), Path::new("/x/y/z"), None).is_err());
}

#[test]
fn remove_stops_the_containers_then_deletes_the_folder_from_outside_it() {
    let (p, f) = project("remove", &[]);
    let steps = plan(Action::Remove, &p, &f, &Args::default()).unwrap();
    assert_eq!(steps.len(), 2);
    assert!(
        steps[0]
            .command_line()
            .contains("docker compose down --volumes --remove-orphans")
    );
    assert_eq!(steps[0].cwd, p.path);
    assert_eq!(
        steps[1].args,
        [
            "internal".to_string(),
            "remove".to_string(),
            p.path.to_string_lossy().into_owned()
        ]
    );
    assert_eq!(
        steps[1].cwd,
        p.path.parent().unwrap(),
        "run from outside the folder it deletes"
    );
    assert!(Action::Remove.risky(), "it asks for the name first");
}

#[test]
fn start_runs_the_part_that_was_built_or_both_together() {
    if !docker_here() {
        return;
    }
    let (p, f) = project(
        "start",
        &["backend/Cargo.toml", "frontend/nuxt.config.ts", ".env"],
    );
    fs::write(
        p.path.join("docker-compose.yml"),
        "services:\n  backend:\n    image: x\n",
    )
    .unwrap();
    let line = |part| lines(&start(part, &p, &f).unwrap());
    // Each starts with the banner that says where it runs.
    let banner = |part: &str| format!("internal banner {} {part}", p.path.display());
    let front = line(Part::Frontend);
    assert!(front[0].ends_with(&banner("frontend")), "{front:?}");
    assert_eq!(front[1], "pnpm dev");
    assert!(line(Part::Backend)[0].ends_with(&banner("backend")));
    assert!(
        line(Part::Backend)
            .last()
            .unwrap()
            .starts_with("cargo watch")
    );
    let both = line(Part::Fullstack);
    assert!(both[0].ends_with(&banner("both")), "{both:?}");
    assert_eq!(
        both.iter()
            .filter(|l| l.contains("internal banner"))
            .count(),
        1,
        "one banner for both parts"
    );
    let last = both.last().unwrap();
    assert!(last.starts_with("sh -c"), "{last}");
    assert!(
        last.contains("(cd backend && cargo watch -x run)")
            && last.contains("(cd frontend && pnpm dev)")
    );
    assert!(last.contains("wait"));
}

#[test]
fn dev_runs_the_backend_and_the_frontend_together_when_there_are_both() {
    if !docker_here() {
        return;
    }
    let (p, f) = project(
        "devboth",
        &["backend/Cargo.toml", "frontend/nuxt.config.ts", ".env"],
    );
    fs::write(
        p.path.join("docker-compose.yml"),
        "services:\n  backend:\n    image: x\n",
    )
    .unwrap();
    let steps = lines(&plan(Action::Dev, &p, &f, &Args::default()).unwrap());
    assert_eq!(steps, lines(&start(Part::Fullstack, &p, &f).unwrap()));
    let last = steps.last().unwrap();
    assert!(
        last.contains("(cd backend && cargo watch -x run)")
            && last.contains("(cd frontend && pnpm dev)"),
        "{last}"
    );
}

#[test]
fn dev_sets_up_and_starts_the_https_proxy_for_the_servers_it_runs_here() {
    if !docker_here() || urls::listening(https_dev::PORT) {
        return;
    }
    let (p, f) = project(
        "devhttps",
        &["backend/Cargo.toml", "Caddyfile", "certs/local.pem", ".env"],
    );
    fs::write(
        p.path.join("docker-compose.yml"),
        "services:\n  backend:\n    image: x\n\n  caddy:\n    image: caddy:alpine\n",
    )
    .unwrap();
    let steps = lines(&plan(Action::Dev, &p, &f, &Args::default()).unwrap());
    // A project made before the dev proxy gets it, then it starts; the servers come after.
    let at = |steps: &[String], what: &str| steps.iter().position(|l| l.contains(what));
    let setup = at(&steps, "internal https-dev").expect("sets it up");
    let start = at(&steps, "--profile dev up -d caddy-dev").expect("starts it");
    assert!(setup < start && start < steps.len() - 1, "{steps:?}");
    assert!(steps.last().unwrap().starts_with("cargo watch"));
    // Once it has it, only the start remains.
    https_dev::enable(&p.path).unwrap();
    let steps = lines(&plan(Action::Dev, &p, &f, &Args::default()).unwrap());
    assert!(
        !steps.iter().any(|l| l.contains("internal https-dev")),
        "{steps:?}"
    );
    assert!(steps.iter().any(|l| l.ends_with("up -d caddy-dev")));
    // Without a certificate there is nothing to serve HTTPS with.
    fs::remove_file(p.path.join("certs/local.pem")).unwrap();
    let steps = lines(&plan(Action::Dev, &p, &f, &Args::default()).unwrap());
    assert!(!steps.iter().any(|l| l.contains("caddy-dev")), "{steps:?}");
}

#[test]
fn a_part_the_project_does_not_have_cannot_be_started() {
    if !docker_here() {
        return;
    }
    let (p, f) = project("startnone", &["backend/Cargo.toml"]);
    fs::write(
        p.path.join("docker-compose.yml"),
        "services:\n  backend:\n    image: x\n",
    )
    .unwrap();
    assert!(start(Part::Frontend, &p, &f).is_err());
    assert!(start(Part::Fullstack, &p, &f).is_err());
    assert!(start(Part::Backend, &p, &f).is_ok());
}

/// A project folder that is a git repository with this origin.
fn project_with_origin(name: &str, origin: Option<&str>) -> (Project, Facts) {
    let (p, _) = project(name, &[]);
    let git = |args: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .args(args)
                .current_dir(&p.path)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "-q", "-b", "main"]);
    if let Some(url) = origin {
        git(&["remote", "add", "origin", url]);
    }
    let facts = Facts::read(&p.path);
    (p, facts)
}

#[test]
fn removing_with_the_repository_deletes_it_first_and_the_folder_last() {
    let (p, f) = project_with_origin("rmremote", Some("git@github.com:me/shop.git"));
    let args = Args {
        remove_remote: Some("github".into()),
        ..Args::default()
    };
    // The folder is a temp one: the safety check wants a place that is not the top of a disk.
    let steps = plan(Action::Remove, &p, &f, &args).unwrap();
    let l = lines(&steps);
    assert!(
        l[0].contains("gh auth refresh") && l[0].contains("delete_repo"),
        "{l:?}"
    );
    assert_eq!(l[1], "gh repo delete me/shop --yes");
    assert!(l[2].contains("docker compose down"), "{l:?}");
    assert!(l.last().unwrap().contains("internal remove"), "{l:?}");
    // Without it, only the local steps.
    let local = plan(Action::Remove, &p, &f, &Args::default()).unwrap();
    assert!(!lines(&local).iter().any(|s| s.starts_with("gh ")));
}

#[test]
fn removing_the_repository_needs_one_that_is_on_that_service() {
    let (p, f) = project_with_origin("rmwrong", Some("git@github.com:me/shop.git"));
    let gitlab = Args {
        remove_remote: Some("gitlab".into()),
        ..Args::default()
    };
    let err = plan(Action::Remove, &p, &f, &gitlab)
        .unwrap_err()
        .to_string();
    assert!(err.contains("GitHub, not GitLab"), "{err}");

    let (p, f) = project_with_origin("rmnone", None);
    let github = Args {
        remove_remote: Some("github".into()),
        ..Args::default()
    };
    let err = plan(Action::Remove, &p, &f, &github)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no GitHub or GitLab repository"), "{err}");
}

#[test]
fn switching_to_local_when_there_is_only_an_env_says_it_already_is() {
    let (p, f) = project("envlone", &[".env"]);
    let args = Args {
        tier: Some("local".into()),
        ..Args::default()
    };
    let err = plan(Action::Env, &p, &f, &args).unwrap_err().to_string();
    assert!(err.contains("already the local environment"), "{err}");
    // Another that does not exist is still the old message.
    let args = Args {
        tier: Some("dev".into()),
        ..Args::default()
    };
    assert!(
        plan(Action::Env, &p, &f, &args)
            .unwrap_err()
            .to_string()
            .contains(".env.dev not found")
    );
}

#[test]
fn cargo_clean_runs_where_there_are_build_files_and_says_how_much_it_frees() {
    // A workspace: one target/ at the root.
    let (p, f) = project("clean-ws", &["Cargo.toml", "backend/Cargo.toml"]);
    assert!(!f.rust_build, "no target/ yet: nothing to clean");
    assert!(plan(Action::CargoClean, &p, &f, &Args::default()).is_err());
    fs::create_dir_all(p.path.join("target/debug")).unwrap();
    fs::write(p.path.join("target/debug/big"), vec![0u8; 3000]).unwrap();
    let f = Facts::read(&p.path);
    assert!(f.rust_build);
    let steps = plan(Action::CargoClean, &p, &f, &Args::default()).unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].command_line(), "cargo clean");
    assert_eq!(steps[0].cwd, p.path);
    assert!(steps[0].label.contains("2.9 KB"), "{}", steps[0].label);

    // Crates that stand alone each clean their own.
    let (p, _) = project("clean-solo", &["backend/Cargo.toml", "frontend/Cargo.toml"]);
    fs::create_dir_all(p.path.join("backend/target")).unwrap();
    fs::create_dir_all(p.path.join("frontend/target")).unwrap();
    let f = Facts::read(&p.path);
    let steps = plan(Action::CargoClean, &p, &f, &Args::default()).unwrap();
    assert_eq!(steps.len(), 2);
    assert!(steps[0].label.contains("in backend") && steps[1].label.contains("in frontend"));
    assert_eq!(steps[1].cwd, p.path.join("frontend"));
}

#[test]
fn cargo_clean_is_offered_only_to_a_project_that_has_build_files() {
    let (p, f) = project("clean-offer", &["backend/Cargo.toml"]);
    assert!(!Action::CargoClean.applies(&f));
    fs::create_dir_all(p.path.join("backend/target")).unwrap();
    assert!(Action::CargoClean.applies(&Facts::read(&p.path)));
    // A Python project has no target/ to speak of.
    let (_, py) = project("clean-py", &["backend/pyproject.toml"]);
    assert!(!Action::CargoClean.applies(&py));
}

const SHOP_COMPOSE: &str = "services:\n  postgres:\n    image: postgres\n    volumes: [pgdata:/var/lib/postgresql]\n\n  redis:\n    image: redis\n\n  backend:\n    image: shop-backend\n\nvolumes:\n  pgdata:\n";

/// A project folder named `shop` with that compose file and, when asked, a saved backup.
fn shop(test: &str, backup: Option<&str>) -> (Project, Facts) {
    let (p, _) = project(test, &[]);
    let dir = p
        .path
        .parent()
        .unwrap()
        .join(format!("shop-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("docker-compose.yml"), SHOP_COMPOSE).unwrap();
    if let Some(file) = backup {
        fs::create_dir_all(dir.join("backups")).unwrap();
        fs::write(dir.join("backups").join(file), "x").unwrap();
    }
    let project = Project {
        name: "shop".into(),
        path: dir.clone(),
        host: None,
    };
    (project, Facts::read(&dir))
}

#[test]
fn adding_a_volume_edits_the_compose_file_then_starts_the_service_with_it() {
    let (p, f) = shop("vadd", None);
    let args = Args {
        volume: Some("redisdata".into()),
        service: Some("redis".into()),
        mount: Some("/data".into()),
        ..Args::default()
    };
    let steps = plan(Action::VolumeAdd, &p, &f, &args).unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(
        steps[0].args,
        ["internal", "add-volume", "redis", "redisdata", "/data"]
    );
    assert_eq!(
        steps[1].command_line(),
        "docker compose up -d --no-deps redis"
    );

    // A folder of the project is made first, so Docker does not make it as root.
    let folder = Args {
        volume: Some("./uploads".into()),
        service: Some("backend".into()),
        mount: Some("/uploads".into()),
        ..Args::default()
    };
    let steps = plan(Action::VolumeAdd, &p, &f, &folder).unwrap();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].args[1], "make-dir");
    assert!(steps[0].args[2].ends_with("uploads"));

    // A mistake is said before anything runs.
    let bad = |a: Args| plan(Action::VolumeAdd, &p, &f, &a).unwrap_err().to_string();
    assert!(
        bad(Args {
            service: Some("nope".into()),
            ..args.clone()
        })
        .contains("no service `nope`")
    );
    assert!(
        bad(Args {
            mount: Some("relative".into()),
            ..args.clone()
        })
        .contains("not a path")
    );
    assert!(
        bad(Args {
            volume: Some("Bad Name".into()),
            ..args.clone()
        })
        .contains("not a volume name")
    );
    assert!(
        bad(Args {
            volume: None,
            ..args
        })
        .contains("name the volume")
    );
}

#[test]
fn a_backup_packs_a_declared_volume_into_a_dated_file_without_stopping_anything() {
    let (p, f) = shop("vback", None);
    let args = Args {
        volume: Some("pgdata".into()),
        ..Args::default()
    };
    let steps = plan(Action::VolumeBackup, &p, &f, &args).unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].args[1], "make-dir");
    let run = &steps[1];
    assert_eq!(run.prog, "docker");
    let joined = run.args.join(" ");
    let project = crate::services::projects::volumes::compose_project(&p.path);
    assert!(
        joined.contains(&format!(
            "type=volume,source={project}_pgdata,target=/data,readonly"
        )),
        "{joined}"
    );
    assert!(joined.contains("target=/backup"), "{joined}");
    assert!(
        run.args
            .iter()
            .any(|a| a.starts_with("/backup/pgdata-") && a.ends_with(".tar.gz")),
        "{joined}"
    );
    assert!(!lines(&steps).iter().any(|l| l.contains("compose stop")));
    // Only a volume the file declares can be backed up.
    let err = plan(
        Action::VolumeBackup,
        &p,
        &f,
        &Args {
            volume: Some("other".into()),
            ..Args::default()
        },
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("not a volume this project declares"),
        "{err}"
    );
}

#[test]
fn a_restore_stops_what_uses_the_volume_replaces_its_data_and_starts_it_again() {
    let (p, f) = shop("vrest", Some("pgdata-20261022-215900.tar.gz"));
    let file = "pgdata-20261022-215900.tar.gz";
    // The steps, when postgres (which mounts it) is running.
    let steps = super::volumes::restore_steps(&p, "pgdata", file, &["postgres".to_string()]);
    let l = lines(&steps);
    assert_eq!(l.len(), 3, "{l:?}");
    assert_eq!(
        l[0], "docker compose stop postgres",
        "only the service that mounts it"
    );
    assert_eq!(l[2], "docker compose start postgres");
    let run = &steps[1].args;
    // The file name is an argument of the script, not part of its text.
    assert_eq!(run[run.len() - 1], file);
    assert!(
        run.iter()
            .any(|a| a.contains("find /data -mindepth 1 -delete && tar xzf \"/backup/$1\""))
    );
    // Nothing running: nothing to stop or start, just the restore.
    assert_eq!(
        super::volumes::restore_steps(&p, "pgdata", file, &[]).len(),
        1
    );
    assert!(
        Action::VolumeRestore.risky(),
        "the project's name is typed back first"
    );

    let args = Args {
        volume: Some("pgdata".into()),
        backup: Some(file.into()),
        ..Args::default()
    };
    let planned = plan(Action::VolumeRestore, &p, &f, &args).unwrap();
    assert!(
        planned
            .iter()
            .any(|s| s.args.iter().any(|a| a.contains("tar xzf")))
    );
    let missing = Args {
        backup: Some("pgdata-nothing.tar.gz".into()),
        ..args.clone()
    };
    assert!(
        plan(Action::VolumeRestore, &p, &f, &missing)
            .unwrap_err()
            .to_string()
            .contains("not a backup")
    );
    let path = Args {
        backup: Some("../x.tar.gz".into()),
        ..args
    };
    assert!(plan(Action::VolumeRestore, &p, &f, &path).is_err());
}

#[test]
fn volumes_are_offered_only_where_there_is_something_to_back_up() {
    let (p, f) = shop("voffer", None);
    assert!(f.has_volumes);
    assert!(Action::VolumeBackup.applies(&f) && Action::VolumeRestore.applies(&f));
    assert!(Action::Volumes.applies(&f) && Action::VolumeAdd.applies(&f));
    fs::write(
        p.path.join("docker-compose.yml"),
        "services:\n  a:\n    image: x\n",
    )
    .unwrap();
    let none = Facts::read(&p.path);
    assert!(!Action::VolumeBackup.applies(&none) && !Action::VolumeRestore.applies(&none));
    assert!(
        Action::VolumeAdd.applies(&none),
        "a first volume can always be added"
    );
}

#[test]
fn migrations_run_with_the_backends_own_tool_against_postgres() {
    let (p, f) = project(
        "migrate",
        &["backend/Cargo.toml", "backend/migrations/0001_init.up.sql"],
    );
    assert!(f.migrations);
    let migrate = lines(&plan(Action::Migrate, &p, &f, &Args::default()).unwrap());
    assert_eq!(
        migrate,
        ["docker compose up -d --wait postgres", "sqlx migrate run"]
    );
    let args = Args {
        name: Some("add_users".into()),
        ..Args::default()
    };
    let new = lines(&plan(Action::MigrationNew, &p, &f, &args).unwrap());
    assert_eq!(
        new,
        ["sqlx migrate add -r --sequential add_users"],
        "no database needed"
    );
    assert!(plan(Action::MigrationNew, &p, &f, &Args::default()).is_err());
    let revert = lines(&plan(Action::MigrationRevert, &p, &f, &Args::default()).unwrap());
    assert_eq!(revert.last().unwrap(), "sqlx migrate revert");
    assert!(Action::MigrationRevert.risky());

    let (p, f) = project(
        "alembic",
        &["backend/pyproject.toml", "backend/migrations/env.py"],
    );
    let migrate = lines(&plan(Action::Migrate, &p, &f, &Args::default()).unwrap());
    assert_eq!(migrate[1], "uv run alembic upgrade head");
    let new = lines(&plan(Action::MigrationNew, &p, &f, &args).unwrap());
    assert_eq!(new, ["uv run alembic revision -m add_users"]);
    let revert = lines(&plan(Action::MigrationRevert, &p, &f, &Args::default()).unwrap());
    assert_eq!(revert[1], "uv run alembic downgrade -1");

    // Without migrations the actions are not offered.
    let (p, f) = project("nomigrate", &["backend/Cargo.toml"]);
    assert!(!f.migrations);
    assert!(plan(Action::Migrate, &p, &f, &Args::default()).is_err());
}
