//! `doctor`: can this machine run the project? One check per line, and each failure names its
//! own fix.
use std::{
    net::{SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

use super::compose;
use crate::{
    clients::{
        docker,
        run::{capture, have, install_hint},
    },
    models::{Backend, Facts, Frontend},
};

pub struct Check {
    pub name: String,
    /// What to do about it; `None` means the check passed.
    pub fix: Option<String>,
}

impl Check {
    fn new(name: &str, ok: bool, fix: &str) -> Self {
        Self {
            name: name.to_string(),
            fix: (!ok).then(|| fix.to_string()),
        }
    }
}

fn tool(bin: &str) -> Check {
    Check::new(bin, have(bin), install_hint(bin))
}

/// Something is already listening on this port on this machine.
/// The command that shows what is using a port, on this system.
fn port_finder(port: u16) -> String {
    if crate::clients::os::WINDOWS {
        format!("netstat -ano | findstr :{port}")
    } else {
        format!("lsof -nP -iTCP:{port} -sTCP:LISTEN")
    }
}

fn listening(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

pub fn checks(root: &Path, facts: &Facts) -> Vec<Check> {
    let mut checks = Vec::new();

    let daemon = capture(root, "docker", &["info"]).is_some();
    checks.push(Check::new(
        "docker running",
        daemon,
        "start Docker Desktop (or the docker service)",
    ));
    let compose_ok = capture(root, "docker", &["compose", "version"]).is_some();
    checks.push(Check::new(
        "docker compose",
        compose_ok,
        "install the Docker Compose plugin",
    ));

    match facts.backend {
        Backend::Axum => checks.push(tool("cargo")),
        Backend::Fastapi => checks.push(tool("uv")),
        Backend::None => {}
    }
    match facts.frontend {
        Frontend::Nuxt | Frontend::Vue => {
            checks.push(tool("pnpm"));
        }
        Frontend::Topcoat => {
            checks.push(tool("cargo"));
            checks.push(tool("topcoat"));
        }
        Frontend::None => {}
    }
    checks.push(tool("git"));
    if crate::clients::os::WINDOWS {
        // `ci.sh` and a few steps run in Git for Windows' shell.
        checks.push(Check::new(
            "sh (from Git for Windows)",
            crate::clients::os::shell_found(),
            crate::clients::os::SHELL_HELP,
        ));
    }
    checks.push(tool("ssh"));
    checks.push(Check::new(
        ".env",
        root.join(".env").exists(),
        if root.join(".env.local").is_file() {
            "run `stackup env local`"
        } else {
            "copy .env.example to .env and fill it in"
        },
    ));
    if facts.https {
        checks.push(tool("mkcert"));
        checks.push(Check::new(
            "local HTTPS certificate",
            root.join("certs/local.pem").exists(),
            "run `stackup certs`",
        ));
    }

    // Ports only mean something while the project is down: when it is up, it holds them.
    let running = capture(root, "docker", &["compose", "ps", "-q"]).is_some_and(|o| !o.is_empty());
    if daemon
        && !running
        && let Ok(config) = docker::config(root)
    {
        for port in compose::published_ports(&config) {
            checks.push(Check::new(
                &format!("port {port} free"),
                !listening(port),
                &format!(
                    "something else is listening on {port}: find it with `{}`",
                    port_finder(port)
                ),
            ));
        }
    }

    // A backend and a Rust frontend both need cargo: say it once.
    let mut seen = Vec::new();
    checks.retain(|c| {
        let new = !seen.contains(&c.name);
        seen.push(c.name.clone());
        new
    });
    checks
}
