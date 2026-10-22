//! The actions on the containers: start, stop, list, clean, logs, build, and the dev setup (databases in Docker, the code on this machine).
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::{DATABASES, compose_step};
use crate::{
    clients::docker,
    models::{Args, Backend, Facts, Frontend, Project, Step},
    services::projects::{
        certs, compose, envs, https_dev,
        ide::{Part, parts},
        internal, urls,
    },
};

/// What `up` runs.
pub(super) fn up(root: &Path, facts: &Facts, args: &Args) -> Vec<Step> {
    {
        let mut steps = vec![];
        // The certificates come first, once: Caddy cannot start without them.
        if facts.https && !certs::exists(root) {
            steps.extend(certs::steps(root));
        }
        let up: &[&str] = if args.detach {
            &["up", "--build", "-d"]
        } else {
            &["up", "--build"]
        };
        steps.push(compose_step("build and start everything", root, up));
        steps
    }
}

/// What `stop` runs.
pub(super) fn stop(root: &Path) -> Vec<Step> {
    vec![Step::new(
        "stop the containers for good",
        root,
        "sh",
        &[
            "-c",
            // Without a restart policy Docker does not bring them back when it starts again.
            "ids=$(docker compose ps -aq); if [ -n \"$ids\" ]; then docker update --restart=no $ids; fi; docker compose stop",
        ],
    )]
}

/// What `images` runs.
pub(super) fn images(p: &Project) -> Vec<Step> {
    let root = p.path.as_path();
    vec![Step::new(
        "list the images",
        root,
        "docker",
        &[
            "image",
            "ls",
            "--filter",
            &format!("reference={}-*", p.name),
        ],
    )]
}

/// What `clean` runs.
pub(super) fn clean(p: &Project) -> Vec<Step> {
    let root = p.path.as_path();
    vec![
        compose_step("remove the containers", root, &["down", "--remove-orphans"]),
        Step::new(
            "delete the images this project built",
            root,
            "sh",
            &[
                "-c",
                "ids=$(docker image ls -q --filter \"reference=$1-*\"); if [ -n \"$ids\" ]; then docker image rm $ids; fi",
                "sh",
                &p.name,
            ],
        ),
    ]
}

/// What `logs` runs.
pub(super) fn logs(root: &Path, args: &Args) -> Vec<Step> {
    {
        let mut a = vec!["logs", "--follow", "--tail", "100"];
        a.extend(args.service.as_deref());
        vec![compose_step("follow the logs", root, &a)]
    }
}

/// What `build` runs.
pub(super) fn build(root: &Path, args: &Args) -> Vec<Step> {
    {
        let mut step = compose_step("build the images", root, &["build"]);
        if let Some(platform) = &args.platform {
            step = step.with_env("DOCKER_DEFAULT_PLATFORM", platform);
        }
        vec![step]
    }
}

/// The steps that give development its HTTPS: set the proxy up in a project made without it, and
/// start it. Nothing when there is no certificate to serve with, and nothing when something
/// already answers on the proxy's port (the whole stack running in Docker has one, and two cannot
/// share it).
fn https_steps(root: &Path) -> Vec<Step> {
    // Without Docker (or without the file Docker reads) there is no proxy to start, and the
    // frontend alone has never needed Docker: it starts without HTTPS.
    if !https_dev::possible(root)
        || urls::listening(https_dev::PORT)
        || docker::config(root).is_err()
    {
        return vec![];
    }
    let mut steps = vec![];
    if !https_dev::enabled(root) {
        steps.push(internal::https_dev(root));
    }
    steps.push(compose_step(
        "start the HTTPS proxy",
        root,
        &["--profile", "dev", "up", "-d", https_dev::SERVICE],
    ));
    steps
}

/// What `dev` runs: the part asked for (or only the databases), or else the backend and the
/// frontend together when the project has both, and the one it has when not.
pub(super) fn dev(p: &Project, facts: &Facts, args: &Args) -> Result<Vec<Step>> {
    let part = match args.part.as_deref() {
        Some(DATABASES_ONLY) => {
            let steps = databases(&p.path)?;
            if steps.is_empty() {
                bail!("this project has no databases");
            }
            return Ok(steps);
        }
        Some(part) => Part::parse(part)?,
        None => *parts(facts)
            .last()
            .context("this project has nothing to start")?,
    };
    start(part, p, facts)
}

/// What `dev` is given to start only the databases, with no server.
pub const DATABASES_ONLY: &str = "databases";

/// Starts the project's databases in Docker, in the background; nothing when it has none.
fn databases(root: &Path) -> Result<Vec<Step>> {
    let config = docker::config(root)?;
    let infra = compose::present(&config, &DATABASES);
    if infra.is_empty() {
        return Ok(vec![]);
    }
    // `--wait`: the backend connects (and migrates) as it starts, so they must be answering.
    let mut up = vec!["up", "-d", "--wait"];
    up.extend(infra.iter().map(String::as_str));
    Ok(vec![compose_step("start the databases", root, &up)])
}

/// The databases in Docker and the backend with reload.
fn backend_dev(root: &Path, facts: &Facts) -> Result<Vec<Step>> {
    Ok({
        let mut steps = databases(root)?;
        steps.extend(https_steps(root));
        // Alembic runs apart from the app: the database is brought up to date before it starts.
        // (An axum backend applies its migrations itself.)
        if facts.migrations && facts.backend == Backend::Fastapi {
            steps.push(super::migrations::apply(root, facts)?);
        }
        let backend = root.join("backend");
        let mut run = match facts.backend {
            Backend::Axum => Step::new(
                "run the backend with reload",
                &backend,
                "cargo",
                &["watch", "-x", "run"],
            ),
            _ => Step::new(
                "run the backend with reload",
                &backend,
                "uv",
                &[
                    "run",
                    "uvicorn",
                    "app.main:app",
                    "--reload",
                    "--port",
                    "8000",
                ],
            ),
        };
        // The backend runs here, not in Docker, so it needs the active tier's values itself.
        run.env.extend(envs::dotenv(root));
        // The banner comes last before the servers, when the databases and the proxy are up, so
        // the addresses it lists are the ones that answer.
        if facts.backend != Backend::None {
            steps.push(internal::banner(root, "backend"));
        }
        steps.push(run);
        steps
    })
}

/// A step's program and arguments as one shell line. Its environment is left out: it travels
/// as the step's own, so a secret is never written into a command.
fn shell_words(step: &Step) -> String {
    std::iter::once(step.prog.as_str())
        .chain(step.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}

/// How the frontend is run while developing, as a command and its arguments.
fn frontend_dev(facts: &Facts) -> (&'static str, &'static [&'static str]) {
    match facts.frontend {
        // The Topcoat CLI rebuilds and restarts the server when the source changes.
        Frontend::Topcoat => ("topcoat", &["dev"]),
        _ => ("pnpm", &["dev"]),
    }
}

/// Starts one part of the project for development, or both together. The backend part is the
/// databases in Docker and the backend with reload; the frontend runs on this machine.
pub fn start(part: Part, p: &Project, facts: &Facts) -> Result<Vec<Step>> {
    let root = p.path.as_path();
    if !facts.exists {
        bail!("{} has no docker-compose.yml", root.display());
    }
    if !parts(facts).contains(&part) {
        bail!("this project has no {}", part.cli());
    }
    let front = frontend_dev(facts);
    // The proxy is a container: on Linux it reaches this machine through the Docker bridge, not
    // through localhost, so the dev server has to listen on every address. (Docker Desktop on a
    // Mac or Windows machine forwards to localhost itself.) Nuxt reads `HOST` for that.
    let open_host = cfg!(target_os = "linux") && facts.frontend == Frontend::Nuxt;
    let frontend = || {
        let step = Step::new("run the frontend", &root.join("frontend"), front.0, front.1);
        if open_host {
            step.with_env("HOST", "0.0.0.0")
        } else {
            step
        }
    };
    match part {
        Part::Backend => backend_dev(root, facts),
        Part::Frontend => {
            let mut steps = https_steps(root);
            steps.push(internal::banner(root, "frontend"));
            steps.push(frontend());
            Ok(steps)
        }
        Part::Fullstack => {
            let mut steps = backend_dev(root, facts)?;
            // Both servers stay up in one step; stopping it (Ctrl-C, or the IDE's stop) ends both.
            let back = steps.pop().context("no backend step")?;
            // One banner for both, in place of the backend's.
            let at = steps
                .iter()
                .position(|s| s.args.iter().any(|a| a == "banner"))
                .context("no banner step")?;
            steps[at] = internal::banner(root, "both");
            let host = if open_host { "HOST=0.0.0.0 " } else { "" };
            let both = format!(
                "trap 'kill 0' INT TERM EXIT; (cd backend && {back}) & (cd frontend && {host}{front}) & wait",
                back = shell_words(&back),
                front = shell_words(&frontend()),
            );
            let mut run = Step::new(
                "run the backend and the frontend",
                root,
                "sh",
                &["-c", &both],
            );
            run.env = back.env;
            steps.push(run);
            Ok(steps)
        }
    }
}
