//! A quick look at a project for its page in the list: which services it runs and on which ports,
//! where git is, and the last commit. Cheap reads only: no Docker, no build, no network.
use std::{
    fs,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

use super::git::{self, GitInfo};
use crate::clients::run::capture;

/// A service of the compose file, and the port it publishes on this machine, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct Service {
    pub name: String,
    pub port: Option<String>,
    /// Something answers on its port right now. Always false for a service with no port.
    pub up: bool,
}

/// Whether something is listening on this machine's `port`: a connection is made and dropped.
/// Localhost answers at once, one way or the other, so this does not wait.
pub fn answering(port: &str) -> bool {
    let Ok(port) = port.parse::<u16>() else {
        return false;
    };
    TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        Duration::from_millis(80),
    )
    .is_ok()
}

/// How much of a project is running.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Running,
    Partly,
    Stopped,
}

impl Overview {
    /// Running when every service with a port answers, stopped when none does.
    pub fn state(&self) -> State {
        let ported: Vec<&Service> = self.services.iter().filter(|s| s.port.is_some()).collect();
        let up = ported.iter().filter(|s| s.up).count();
        match (up, ported.len()) {
            (0, _) => State::Stopped,
            (u, n) if u == n => State::Running,
            _ => State::Partly,
        }
    }

    /// `3/5`: the services answering out of those that have a port.
    pub fn up_count(&self) -> (usize, usize) {
        let ported = self.services.iter().filter(|s| s.port.is_some());
        (ported.clone().filter(|s| s.up).count(), ported.count())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Overview {
    /// The containers the compose file defines, in its order.
    pub services: Vec<Service>,
    pub git: Option<GitInfo>,
    /// The latest few commits, newest first.
    pub recent: Vec<Commit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Commit {
    pub hash: String,
    pub subject: String,
    pub when: String,
}

/// `git log` lines made with `%h<US>%s<US>%ar`.
pub fn parse_commits(text: &str) -> Vec<Commit> {
    text.lines()
        .filter_map(|l| {
            let mut p = l.split('\u{1f}');
            Some(Commit {
                hash: p.next()?.to_string(),
                subject: p.next()?.to_string(),
                when: p.next()?.to_string(),
            })
        })
        .collect()
}

/// The services in a compose file's text, with the first published port of each. Reads the file
/// the way stackup writes it (two-space indents, `ports: ["3000:3000"]`) and ignores the rest.
pub fn compose_services(text: &str) -> Vec<Service> {
    let mut out: Vec<Service> = vec![];
    let mut in_services = false;
    for line in text.lines() {
        if !line.starts_with(' ') && !line.trim().is_empty() {
            in_services = line.trim_end() == "services:";
            continue;
        }
        if !in_services {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let body = line.trim();
        if indent == 2 && body.ends_with(':') && !body.starts_with('#') {
            out.push(Service {
                name: body.trim_end_matches(':').to_string(),
                port: None,
                up: false,
            });
        } else if body.starts_with("ports:")
            && let Some(service) = out.last_mut()
            && service.port.is_none()
        {
            // `ports: ["8443:443"]`: the part before the colon is the one on this machine.
            service.port = body
                .split('"')
                .nth(1)
                .and_then(|p| p.split(':').next())
                .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
                .map(str::to_string);
        }
    }
    out
}

pub fn read(root: &Path) -> Overview {
    let mut services = fs::read_to_string(root.join("docker-compose.yml"))
        .map(|t| compose_services(&t))
        .unwrap_or_default();
    for s in &mut services {
        s.up = s.port.as_deref().is_some_and(answering);
    }
    let git = git::read(root);
    let recent = git
        .as_ref()
        .filter(|g| g.commits)
        .and_then(|_| capture(root, "git", &["log", "-n6", "--format=%h%x1f%s%x1f%ar"]))
        .map(|t| parse_commits(&t))
        .unwrap_or_default();
    Overview {
        services,
        git,
        recent,
    }
}

#[cfg(test)]
mod tests {
    use std::{env, process::Command};

    use super::*;

    const COMPOSE: &str = "services:\n  postgres:\n    image: postgres:alpine\n    ports: [\"5432:5432\"]\n    volumes: [pgdata:/var/lib/postgresql]\n\n  backend:\n    build:\n      context: .\n    ports: [\"8000:8000\"]\n\n  keycloak:\n    image: kc\n\n  caddy:\n    ports: [\"8443:443\"]\n\nvolumes:\n  pgdata:\n";

    #[test]
    fn services_and_their_published_ports_are_read_from_the_compose_text() {
        let services = compose_services(COMPOSE);
        let names: Vec<&str> = services.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            ["postgres", "backend", "keycloak", "caddy"],
            "the volumes are not services"
        );
        let port = |n: &str| services.iter().find(|s| s.name == n).unwrap().port.clone();
        assert_eq!(port("postgres").as_deref(), Some("5432"));
        assert_eq!(
            port("caddy").as_deref(),
            Some("8443"),
            "the port on this machine, not the container's"
        );
        assert_eq!(port("keycloak"), None);
        assert!(compose_services("").is_empty());
    }

    #[test]
    fn a_project_with_git_has_its_branch_and_last_commit() {
        let dir = env::temp_dir()
            .join("stackup-overview")
            .join(format!("o-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(&dir)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@e.x"]);
        git(&["config", "user.name", "T"]);
        fs::write(dir.join("docker-compose.yml"), COMPOSE).unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "first commit"]);
        let o = read(&dir);
        assert_eq!(o.services.len(), 4);
        assert_eq!(o.git.as_ref().unwrap().branch, "main");
        assert_eq!(o.recent.len(), 1);
        assert_eq!(o.recent[0].subject, "first commit");
        assert!(o.recent[0].when.contains("ago") || o.recent[0].when.contains("now"));
        // No git, no commits: nothing to show about them.
        let none = read(&dir.join("nowhere"));
        assert!(none.git.is_none() && none.recent.is_empty() && none.services.is_empty());
    }

    #[test]
    fn commit_lines_are_read_and_a_broken_line_is_skipped() {
        let text = "abc1234\u{1f}fix the thing\u{1f}2 hours ago\nbroken line\ndef5678\u{1f}add it\u{1f}3 days ago\n";
        let commits = parse_commits(text);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[1].when, "3 days ago");
    }

    #[test]
    fn a_service_is_up_when_its_port_answers_and_the_project_state_follows() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let open = listener.local_addr().unwrap().port().to_string();
        // A port nothing listens on: bind and drop it.
        let closed = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port().to_string()
        };
        assert!(answering(&open));
        assert!(!answering(&closed));
        assert!(!answering("not a port"));

        let svc = |name: &str, port: Option<&str>, up: bool| Service {
            name: name.into(),
            port: port.map(str::to_string),
            up,
        };
        let mut o = Overview {
            services: vec![
                svc("a", Some("1"), true),
                svc("b", Some("2"), true),
                svc("c", None, false),
            ],
            ..Overview::default()
        };
        assert_eq!(
            o.state(),
            State::Running,
            "a service with no port is not counted"
        );
        assert_eq!(o.up_count(), (2, 2));
        o.services[1].up = false;
        assert_eq!(o.state(), State::Partly);
        o.services[0].up = false;
        assert_eq!(o.state(), State::Stopped);
        assert_eq!(Overview::default().state(), State::Stopped);
    }
}
