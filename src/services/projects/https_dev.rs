//! HTTPS while developing. The proxy that serves `https://app.localhost:8443` is a container, and
//! it forwards to the other containers; the servers started for development run on this machine.
//! So a second proxy, `caddy-dev`, sits in the compose file under the `dev` profile (it starts
//! only when asked, and never with `up`): it has the same certificate and the same addresses, and
//! forwards to this machine through `host.docker.internal`. Docker Desktop knows that name; on
//! Linux the service maps it to the host with `host-gateway`, so the same file works everywhere.
use std::{fs, path::Path};

use anyhow::{Context, Result};

pub const SERVICE: &str = "caddy-dev";
pub const CADDYFILE: &str = "Caddyfile.dev";
/// The proxy's port on this machine: the same as the other one's, so the addresses do not change.
pub const PORT: u16 = 8443;
/// Where Vite serves a Vue project while developing.
pub const VITE_PORT: u16 = 5173;

/// The compose service, indented as a service in the file is.
pub fn service() -> String {
    format!(
        "  {SERVICE}:\n    image: caddy:alpine\n    profiles: [\"dev\"]\n    ports: [\"{PORT}:443\"]\n    extra_hosts: [\"host.docker.internal:host-gateway\"]\n    volumes:\n      - ./{CADDYFILE}:/etc/caddy/Caddyfile:ro\n      - ./certs:/certs:ro\n\n"
    )
}

/// The Caddyfile for development: the project's own, with the places the servers run in
/// development in place of the containers they run in otherwise.
pub fn caddyfile(docker: &str) -> String {
    // A server not started yet would otherwise be a blank 502 page; this says which to start.
    let to = |upstream: &str, part: &str| {
        format!(
            "reverse_proxy host.docker.internal:{upstream}\n\thandle_errors 502 {{\n\t\trespond \"The {part} is not running. Start it: stackup start {part}\" 502\n\t}}"
        )
    };
    docker
        .replace("reverse_proxy frontend:3000", &to("3000", "frontend"))
        // The Vue image is served by nginx on 80; its dev server is Vite's.
        .replace(
            "reverse_proxy frontend:80",
            &to(&VITE_PORT.to_string(), "frontend"),
        )
        .replace("reverse_proxy backend:8000", &to("8000", "backend"))
}

/// The compose file with the dev proxy added, or `None` when it has no proxy to copy or already
/// has this one. It goes before the top-level `volumes:` when there is one, else at the end.
pub fn add_service(compose: &str) -> Option<String> {
    let has_proxy = compose.lines().any(|l| l == "  caddy:");
    let has_dev = compose.lines().any(|l| l == format!("  {SERVICE}:"));
    if !has_proxy || has_dev {
        return None;
    }
    let mut lines: Vec<&str> = compose.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "volumes:")
        .unwrap_or(lines.len());
    // Keep the blank line that separates services.
    let block = service();
    let mut out = String::new();
    for l in lines.drain(..at) {
        out.push_str(l);
        out.push('\n');
    }
    if !out.ends_with("\n\n") {
        out.push('\n');
    }
    out.push_str(&block);
    for l in lines {
        out.push_str(l);
        out.push('\n');
    }
    let end = if compose.ends_with('\n') { "\n" } else { "" };
    Some(out.trim_end_matches('\n').to_string() + end)
}

/// Whether the project has the dev proxy: its service and its Caddyfile.
pub fn enabled(root: &Path) -> bool {
    root.join(CADDYFILE).is_file()
        && fs::read_to_string(root.join("docker-compose.yml"))
            .is_ok_and(|c| c.lines().any(|l| l == format!("  {SERVICE}:")))
}

/// Whether the project has local HTTPS (a proxy and a certificate) that development could use.
pub fn possible(root: &Path) -> bool {
    root.join("Caddyfile").is_file() && root.join("certs/local.pem").is_file()
}

/// Gives a project made without it the dev proxy: the service in its compose file and a
/// `Caddyfile.dev` made from its `Caddyfile`. Says what it did.
pub fn enable(root: &Path) -> Result<String> {
    if enabled(root) {
        return Ok("the development HTTPS proxy is already set up".into());
    }
    let compose_path = root.join("docker-compose.yml");
    let compose = fs::read_to_string(&compose_path).context("reading docker-compose.yml")?;
    let docker = fs::read_to_string(root.join("Caddyfile")).context("reading Caddyfile")?;
    fs::write(root.join(CADDYFILE), caddyfile(&docker))
        .with_context(|| format!("writing {CADDYFILE}"))?;
    if let Some(with) = add_service(&compose) {
        fs::write(&compose_path, with).context("writing docker-compose.yml")?;
    }
    Ok(format!(
        "added {SERVICE} to docker-compose.yml and {CADDYFILE}: HTTPS now works while developing"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPOSE: &str = "services:\n  backend:\n    image: x\n\n  caddy:\n    image: caddy:alpine\n    ports: [\"8443:443\"]\n\nvolumes:\n  pgdata:\n";

    #[test]
    fn the_dev_proxy_is_added_before_the_volumes_and_only_once() {
        let with = add_service(COMPOSE).unwrap();
        assert!(with.contains("  caddy-dev:\n    image: caddy:alpine\n    profiles: [\"dev\"]"));
        assert!(with.contains("host.docker.internal:host-gateway"));
        assert!(with.find("caddy-dev").unwrap() < with.find("\nvolumes:").unwrap());
        assert!(with.ends_with("volumes:\n  pgdata:\n"), "{with}");
        assert_eq!(add_service(&with), None, "a second time changes nothing");
        // Without a proxy there is nothing to copy.
        assert_eq!(add_service("services:\n  backend:\n    image: x\n"), None);
    }

    #[test]
    fn without_a_volumes_block_it_goes_at_the_end() {
        let with = add_service("services:\n  caddy:\n    image: caddy:alpine\n").unwrap();
        assert!(with.ends_with("      - ./certs:/certs:ro\n"), "{with}");
        assert!(with.contains("  caddy-dev:"));
    }

    #[test]
    fn the_dev_caddyfile_points_at_this_machine_not_the_containers() {
        let docker = "https://app.localhost {\n\treverse_proxy frontend:3000\n}\nhttps://api.localhost {\n\treverse_proxy backend:8000\n}\nhttps://auth.localhost {\n\treverse_proxy keycloak:8080\n}\n";
        let dev = caddyfile(docker);
        assert!(dev.contains("reverse_proxy host.docker.internal:3000"));
        assert!(dev.contains("reverse_proxy host.docker.internal:8000"));
        // Keycloak is a container that stays one.
        assert!(dev.contains("reverse_proxy keycloak:8080"));
        assert!(caddyfile("reverse_proxy frontend:80").contains("host.docker.internal:5173"));
    }

    #[test]
    fn enabling_it_in_a_project_writes_both_files_and_is_safe_to_repeat() {
        let dir = std::env::temp_dir().join(format!("stackup-httpsdev-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("certs")).unwrap();
        fs::write(dir.join("docker-compose.yml"), COMPOSE).unwrap();
        fs::write(dir.join("Caddyfile"), "reverse_proxy backend:8000\n").unwrap();
        assert!(!enabled(&dir));
        enable(&dir).unwrap();
        assert!(enabled(&dir));
        assert!(
            fs::read_to_string(dir.join(CADDYFILE))
                .unwrap()
                .contains("host.docker.internal:8000")
        );
        let once = fs::read_to_string(dir.join("docker-compose.yml")).unwrap();
        assert!(enable(&dir).unwrap().contains("already"));
        assert_eq!(
            fs::read_to_string(dir.join("docker-compose.yml")).unwrap(),
            once
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
