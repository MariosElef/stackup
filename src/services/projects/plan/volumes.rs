//! Volumes: listing them, adding one to a service, saving one to a file, and putting a file back.
use std::{fs, time::SystemTime};

use anyhow::{Context, Result, bail};

use super::compose_step;
use crate::{
    clients::run::capture,
    models::{Args, Project, Step},
    services::projects::{internal, volumes},
};

fn compose_text(p: &Project) -> Result<String> {
    fs::read_to_string(p.path.join("docker-compose.yml"))
        .context("could not read docker-compose.yml")
}

/// The volume asked for, checked against the file: it has to be one the file declares.
fn declared_volume(p: &Project, args: &Args) -> Result<String> {
    let name = args.volume.as_deref().context("which volume?")?;
    if !volumes::named(&compose_text(p)?).iter().any(|v| v == name) {
        bail!("`{name}` is not a volume this project declares: `stackup volumes` lists them");
    }
    Ok(name.to_string())
}

/// What `volumes` runs: a table of what is mounted where.
pub(super) fn list(p: &Project) -> Vec<Step> {
    vec![internal::volumes("list the volumes", &p.path)]
}

/// What `add volume` runs: the entry in the compose file, and the service started again with it.
pub(super) fn add(p: &Project, args: &Args) -> Result<Vec<Step>> {
    let source = args
        .volume
        .as_deref()
        .context("name the volume, or a ./folder")?;
    let service = args.service.as_deref().context("which service gets it?")?;
    let target = args
        .mount
        .as_deref()
        .context("where does it appear in the container?")?;
    // Checked here, so a mistake is said before anything is run.
    volumes::add(&compose_text(p)?, service, source, target)?;
    let mut steps = vec![];
    if source.starts_with("./") {
        // A folder that does not exist would be made by Docker, owned by root.
        steps.push(internal::make_dir(
            &format!("make the folder {source}"),
            &p.path,
            &p.path.join(source),
        ));
    }
    steps.push(internal::add_volume(
        &format!("mount {source} at {target} in {service}"),
        &p.path,
        service,
        source,
        target,
    ));
    steps.push(compose_step(
        &format!("start {service} with it"),
        &p.path,
        &["up", "-d", "--no-deps", service],
    ));
    Ok(steps)
}

/// What `backup volume` runs: a throwaway container packs the volume into `backups/`.
pub(super) fn backup(p: &Project, args: &Args) -> Result<Vec<Step>> {
    let name = declared_volume(p, args)?;
    let dir = p.path.join("backups");
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let file = format!("{name}-{}.tar.gz", volumes::stamp(secs));
    Ok(vec![
        internal::make_dir("make the backups folder", &p.path, &dir),
        Step::new(
            &format!("pack {name} into backups/{file}"),
            &p.path,
            "docker",
            &[
                "run",
                "--rm",
                "--mount",
                &format!(
                    "type=volume,source={},target=/data,readonly",
                    volumes::docker_name(&p.path, &name)
                ),
                "--mount",
                &format!("type=bind,source={},target=/backup", dir.display()),
                "alpine",
                "tar",
                "czf",
                &format!("/backup/{file}"),
                "-C",
                "/data",
                ".",
            ],
        ),
    ])
}

/// The services that use `volume` and are running now: only those are stopped and started again,
/// because starting a service that was never created is an error, and starting one that was
/// stopped on purpose is not what was asked.
fn running_users(p: &Project, volume: &str) -> Result<Vec<String>> {
    let mut users: Vec<String> = volumes::mounts(&compose_text(p)?)
        .into_iter()
        .filter(|m| m.source == volume)
        .map(|m| m.service)
        .collect();
    users.dedup();
    let running = capture(
        &p.path,
        "docker",
        &["compose", "ps", "--status", "running", "--services"],
    )
    .unwrap_or_default();
    users.retain(|u| running.lines().any(|r| r.trim() == u));
    Ok(users)
}

/// What `restore volume` runs: the running services that use the volume are stopped, a throwaway
/// container empties it and unpacks the backup into it, and those services are started again.
pub(super) fn restore(p: &Project, args: &Args) -> Result<Vec<Step>> {
    let name = declared_volume(p, args)?;
    let file = args.backup.as_deref().context("which backup?")?;
    if !volumes::valid_backup(file) || !p.path.join("backups").join(file).is_file() {
        bail!("`{file}` is not a backup in this project's backups/ folder");
    }
    Ok(restore_steps(p, &name, file, &running_users(p, &name)?))
}

/// The steps of a restore, given which services have to be stopped around it.
pub(super) fn restore_steps(p: &Project, name: &str, file: &str, users: &[String]) -> Vec<Step> {
    let with_users = |first: &'static str| -> Vec<&str> {
        let mut a = vec![first];
        a.extend(users.iter().map(String::as_str));
        a
    };
    let mut steps = vec![];
    if !users.is_empty() {
        steps.push(compose_step(
            "stop what uses it",
            &p.path,
            &with_users("stop"),
        ));
    }
    steps.push(Step::new(
        &format!("empty {name} and unpack {file} into it"),
        &p.path,
        "docker",
        &[
            "run",
            "--rm",
            "--mount",
            &format!(
                "type=volume,source={},target=/data",
                volumes::docker_name(&p.path, name)
            ),
            "--mount",
            &format!(
                "type=bind,source={},target=/backup,readonly",
                p.path.join("backups").display()
            ),
            "alpine",
            "sh",
            "-c",
            // The file name reaches the script as an argument, never inside its text.
            "find /data -mindepth 1 -delete && tar xzf \"/backup/$1\" -C /data",
            "sh",
            file,
        ],
    ));
    if !users.is_empty() {
        steps.push(compose_step(
            "start them again",
            &p.path,
            &with_users("start"),
        ));
    }
    steps
}
