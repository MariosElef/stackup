//! `deploy`: build the images for the server, send them over SSH (no registry), and start them.
use std::path::Path;

use anyhow::{Result, bail};

use crate::models::Step;

pub struct Options {
    pub host: String,
    pub dir: String,
    pub tier: String,
    pub platform: String,
}

/// Hosts, folders and image names go into command lines unquoted, so only plain characters are
/// allowed.
fn check_plain(what: &str, value: &str, extra: &str) -> Result<()> {
    let ok = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c) || extra.contains(c));
    if !ok {
        bail!("{what} `{value}` has characters that are not allowed here");
    }
    Ok(())
}

/// Everything a deploy does, in order. `mounts` are the files and folders services mount from
/// next to the compose file.
pub fn steps(root: &Path, o: &Options, images: &[String], mounts: &[String]) -> Result<Vec<Step>> {
    check_plain("host", &o.host, "@:")?;
    check_plain("folder", &o.dir, "/")?;
    check_plain("platform", &o.platform, "/")?;
    check_plain("tier", &o.tier, "")?;
    for image in images.iter().chain(mounts) {
        check_plain("name", image, ":/")?;
    }
    let (host, dir) = (&o.host, &o.dir);
    let at = |rel: &str| format!("{host}:{dir}/{rel}");

    let mut steps = vec![
        Step::new(
            "build the images for the server",
            root,
            "docker",
            &["compose", "build"],
        )
        .with_env("DOCKER_DEFAULT_PLATFORM", &o.platform),
        Step::new(
            "make the folder on the server",
            root,
            "ssh",
            &[host, &format!("mkdir -p {dir}")],
        ),
        Step::new(
            "send the compose file",
            root,
            "scp",
            &["docker-compose.yml", &at("docker-compose.yml")],
        ),
        Step::new(
            "send this tier's settings as .env",
            root,
            "scp",
            &[&format!(".env.{}", o.tier), &at(".env")],
        ),
    ];
    for m in mounts {
        steps.push(Step::new(
            "send a mounted file or folder",
            root,
            "scp",
            &["-r", m, &format!("{host}:{dir}/")],
        ));
    }
    // One shell line, so the images stream from one program into the other.
    steps.push(Step::new(
        "send the images (docker save | docker load, no registry)",
        root,
        "sh",
        &[
            "-c",
            &format!("docker save {} | ssh {host} docker load", images.join(" ")),
        ],
    ));
    steps.push(Step::new(
        "start them",
        root,
        "ssh",
        &[
            host,
            &format!("cd {dir} && docker compose up -d --no-build --remove-orphans"),
        ],
    ));
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options {
            host: "deploy@example.com".into(),
            dir: "/srv/demo".into(),
            tier: "prod".into(),
            platform: "linux/amd64".into(),
        }
    }

    #[test]
    fn a_deploy_builds_sends_and_starts_in_order() {
        let images = vec!["demo-backend".to_string(), "demo-frontend".to_string()];
        let steps = steps(Path::new("/p"), &options(), &images, &["keycloak".into()]).unwrap();
        let lines: Vec<String> = steps.iter().map(Step::command_line).collect();
        let all = lines.join("\n");
        assert!(all.contains("DOCKER_DEFAULT_PLATFORM=linux/amd64 docker compose build"));
        assert!(all.contains("scp .env.prod deploy@example.com:/srv/demo/.env"));
        assert!(all.contains("scp -r keycloak deploy@example.com:/srv/demo/"));
        assert!(all.contains(
            "sh -c 'docker save demo-backend demo-frontend | ssh deploy@example.com docker load'"
        ));
        assert!(all.contains("ssh deploy@example.com 'cd /srv/demo && docker compose up -d"));
        let pos = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap();
        assert!(pos("compose build") < pos("docker save") && pos("docker save") < pos("up -d"));
    }

    #[test]
    fn a_host_or_folder_with_shell_characters_is_refused() {
        let root = Path::new("/p");
        let mut o = options();
        o.host = "me@host; rm -rf /".into();
        assert!(steps(root, &o, &[], &[]).is_err());
        let mut o = options();
        o.dir = "/srv/$(whoami)".into();
        assert!(steps(root, &o, &[], &[]).is_err());
        let images = vec!["x; reboot".to_string()];
        assert!(steps(root, &options(), &images, &[]).is_err());
    }
}
