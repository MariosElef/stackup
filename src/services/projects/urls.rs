//! Where a project's pieces are reachable on this machine, read from its compose file rather
//! than guessed: the published ports, and the hostnames the local HTTPS proxy answers to.
use std::{
    net::{SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

use serde_json::Value;

use crate::clients::docker;

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// What it is: app, api, auth, postgres...
    pub label: String,
    pub url: String,
    /// Whether something is answering on its port right now; `None` when not checked.
    pub up: Option<bool>,
}

/// The addresses of a project: where to open it, and where to connect to its data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Urls {
    pub web: Vec<Link>,
    pub data: Vec<Link>,
    /// How to sign in to each piece that asks, by its label: shown under its address, so a
    /// database client can be pointed at it without opening `.env`.
    pub logins: Vec<Login>,
}

/// What a client's connection form takes for one piece, a line of fields at a time, named as
/// the forms name them: `host localhost  port 5432`, then `user app  password change-me ...`.
/// A field with no value is a note: `no password`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Login {
    pub label: String,
    pub lines: Vec<Vec<(&'static str, String)>>,
}

/// The sign-ins, from the active `.env`, or the compose file's defaults where it says nothing.
/// The Postgres address gets them too, so it can be copied into a client whole.
pub fn add_logins(urls: &mut Urls, env: &[(String, String)]) {
    let var = |key: &str, default: &str| {
        env.iter()
            .find(|(k, _)| k == key)
            .map_or(default.to_string(), |(_, v)| v.clone())
    };
    for l in &mut urls.data {
        let Some(port) = port_of(&l.url) else {
            continue;
        };
        let host = vec![
            ("host", "localhost".to_string()),
            ("port", port.to_string()),
        ];
        let lines = match l.label.as_str() {
            "postgres" => {
                let (user, password, db) = (
                    var("POSTGRES_USER", "app"),
                    var("POSTGRES_PASSWORD", "change-me"),
                    var("POSTGRES_DB", "app"),
                );
                l.url = format!("postgres://{user}:{password}@localhost:{port}/{db}");
                vec![
                    host,
                    vec![("user", user), ("password", password), ("database", db)],
                ]
            }
            "redis" | "mongo" => {
                let mut line = host;
                line.push(("no password", String::new()));
                vec![line]
            }
            _ => continue,
        };
        urls.logins.push(Login {
            label: l.label.clone(),
            lines,
        });
    }
    if urls.web.iter().any(|l| l.label.starts_with("auth")) {
        urls.logins.push(Login {
            label: "auth".into(),
            lines: vec![vec![
                ("admin", var("KEYCLOAK_ADMIN", "admin")),
                ("password", var("KEYCLOAK_ADMIN_PASSWORD", "admin")),
            ]],
        });
    }
}

impl Urls {
    pub fn is_empty(&self) -> bool {
        self.web.is_empty() && self.data.is_empty()
    }

    /// The one to open in a browser: the app if there is one, preferring the secure address.
    pub fn primary(&self) -> Option<&Link> {
        let by = |label: &str| self.web.iter().find(|l| l.label == label);
        by("app (https)")
            .or_else(|| by("app"))
            .or_else(|| self.web.first())
    }
}

fn link(label: &str, url: String) -> Link {
    Link {
        label: label.to_string(),
        url,
        up: None,
    }
}

/// `https://api.localhost {` -> `api.localhost`.
fn caddy_hosts(caddyfile: &str) -> Vec<String> {
    caddyfile
        .lines()
        .filter_map(|l| l.trim().strip_prefix("https://"))
        .filter_map(|rest| rest.split([' ', '{']).next())
        .filter(|h| !h.is_empty())
        .map(str::to_string)
        .collect()
}

/// The host side and container side of each port a service publishes.
fn ports(svc: &Value) -> Vec<(u16, u16)> {
    let number = |v: &Value| match v {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_u64().and_then(|n| u16::try_from(n).ok()),
        _ => None,
    };
    svc["ports"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| Some((number(&p["published"])?, number(&p["target"]).unwrap_or(0))))
        .collect()
}

/// The addresses in `config` (`docker compose config --format json`), and the proxy's hosts
/// from the Caddyfile when there is one.
pub fn links(config: &Value, caddyfile: Option<&str>) -> Urls {
    let mut urls = Urls::default();
    let services = config["services"].as_object();
    for (name, svc) in services.into_iter().flatten() {
        let published = ports(svc);
        let Some(&(port, _)) = published.first() else {
            continue;
        };
        match name.as_str() {
            "caddy" => {
                // Behind the proxy each piece has its own hostname, on the proxy's port.
                let tls = published
                    .iter()
                    .find(|(_, target)| *target == 443)
                    .map_or(port, |(p, _)| *p);
                for host in caddyfile.map(caddy_hosts).unwrap_or_default() {
                    let label = match host.split('.').next().unwrap_or("") {
                        "api" => "api (https)",
                        "auth" => "auth (https)",
                        _ => "app (https)",
                    };
                    urls.web.push(link(label, format!("https://{host}:{tls}")));
                }
            }
            "frontend" => urls
                .web
                .push(link("app", format!("http://localhost:{port}"))),
            "backend" => urls
                .web
                .push(link("api", format!("http://localhost:{port}"))),
            "keycloak" => urls
                .web
                .push(link("auth", format!("http://localhost:{port}"))),
            "postgres" => urls
                .data
                .push(link("postgres", format!("postgres://localhost:{port}"))),
            "redis" => urls
                .data
                .push(link("redis", format!("redis://localhost:{port}"))),
            "mongo" => urls
                .data
                .push(link("mongo", format!("mongodb://localhost:{port}"))),
            other => urls
                .web
                .push(link(other, format!("http://localhost:{port}"))),
        }
    }
    // The app first, then the api, then the rest, secure before plain.
    let rank = |label: &str| match label {
        "app (https)" => 0,
        "app" => 1,
        "api (https)" => 2,
        "api" => 3,
        "auth (https)" => 4,
        "auth" => 5,
        _ => 6,
    };
    urls.web.sort_by_key(|l| rank(&l.label));
    urls.data.sort_by(|a, b| a.label.cmp(&b.label));
    urls
}

/// Whether something answers on this port on this machine.
pub fn listening(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

pub fn port_of(url: &str) -> Option<u16> {
    // `postgres://user:pass@localhost:5432/app`: the port is the end of the part before the path.
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split('/').next()?;
    host.rsplit(':').next()?.parse().ok()
}

/// A project's addresses, with whether each web one is answering. Empty when Docker is not
/// installed: the compose file cannot be read without it.
pub fn read(root: &Path) -> Urls {
    let Ok(config) = docker::config(root) else {
        return Urls::default();
    };
    let caddyfile = std::fs::read_to_string(root.join("Caddyfile")).ok();
    let mut urls = links(&config, caddyfile.as_deref());
    add_logins(&mut urls, &super::envs::dotenv(root));
    probe(&mut urls);
    urls
}

/// Asks each web address again whether something answers: cheap, so it can be done while the
/// servers start, without reading the compose file again.
pub fn probe(urls: &mut Urls) {
    let answering = |url: &str| port_of(url).is_some_and(listening);
    let plain: Vec<(String, bool)> = urls
        .web
        .iter()
        .map(|l| (l.label.clone(), answering(&l.url)))
        .collect();
    for l in &mut urls.web {
        l.up = port_of(&l.url).map(listening);
        // The proxy answers even when the server behind it does not, with a blank 502: a secure
        // address is only up when its plain one is too.
        if let Some(behind) = l.label.strip_suffix(" (https)") {
            let behind_up = plain.iter().any(|(label, up)| label == behind && *up)
                || (behind == "app" && listening(super::https_dev::VITE_PORT));
            l.up = l.up.map(|up| up && behind_up);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Value {
        serde_json::json!({ "services": {
            "frontend": { "ports": [{ "published": "3000", "target": 3000 }] },
            "backend": { "ports": [{ "published": "8000", "target": 8000 }] },
            "postgres": { "ports": [{ "published": "5432", "target": 5432 }] },
            "redis": { "ports": [{ "published": 6379, "target": 6379 }] },
            "keycloak": { "ports": [{ "published": "8080", "target": 8080 }] },
            "worker": {}
        }})
    }

    fn labels(links: &[Link]) -> Vec<(&str, &str)> {
        links
            .iter()
            .map(|l| (l.label.as_str(), l.url.as_str()))
            .collect()
    }

    #[test]
    fn web_pieces_and_data_pieces_are_told_apart() {
        let urls = links(&config(), None);
        let web = labels(&urls.web);
        assert_eq!(
            web,
            [
                ("app", "http://localhost:3000"),
                ("api", "http://localhost:8000"),
                ("auth", "http://localhost:8080"),
            ]
        );
        let data = labels(&urls.data);
        assert_eq!(
            data,
            [
                ("postgres", "postgres://localhost:5432"),
                ("redis", "redis://localhost:6379")
            ]
        );
    }

    #[test]
    fn a_service_with_no_published_port_has_no_address() {
        let urls = links(&config(), None);
        assert!(
            !urls
                .web
                .iter()
                .chain(&urls.data)
                .any(|l| l.label == "worker")
        );
    }

    #[test]
    fn behind_the_proxy_each_piece_has_a_hostname_on_the_proxys_port() {
        let config = serde_json::json!({ "services": {
            "caddy": { "ports": [{ "published": "8443", "target": 443 }] },
            "frontend": { "ports": [{ "published": "3000", "target": 3000 }] },
        }});
        let caddyfile = "{\n\tauto_https off\n}\n\nhttps://localhost {\n\ttls x\n}\n\nhttps://api.localhost {\n}\n\nhttps://auth.localhost {\n}\n";
        let urls = links(&config, Some(caddyfile));
        let web = labels(&urls.web);
        assert!(
            web.contains(&("app (https)", "https://localhost:8443")),
            "{web:?}"
        );
        assert!(web.contains(&("api (https)", "https://api.localhost:8443")));
        assert!(web.contains(&("auth (https)", "https://auth.localhost:8443")));
        assert!(
            web.contains(&("app", "http://localhost:3000")),
            "the direct address is still listed"
        );
        assert_eq!(
            urls.primary().unwrap().url,
            "https://localhost:8443",
            "secure is preferred"
        );
    }

    #[test]
    fn the_app_is_what_opens_in_the_browser() {
        let urls = links(&config(), None);
        assert_eq!(urls.primary().unwrap().label, "app");
        assert!(Urls::default().primary().is_none());
        assert!(Urls::default().is_empty());
    }

    #[test]
    fn the_databases_say_how_to_sign_in_from_the_env_or_the_defaults() {
        let login = |urls: &Urls, label: &str| {
            urls.logins
                .iter()
                .find(|l| l.label == label)
                .cloned()
                .unwrap()
        };
        let mut urls = links(&config(), None);
        add_logins(
            &mut urls,
            &[("POSTGRES_PASSWORD".to_string(), "s3cret".to_string())],
        );
        assert_eq!(
            urls.data[0].url, "postgres://app:s3cret@localhost:5432/app",
            "the address can be copied whole"
        );
        let pg = login(&urls, "postgres");
        assert_eq!(
            pg.lines,
            [
                vec![
                    ("host", "localhost".to_string()),
                    ("port", "5432".to_string())
                ],
                vec![
                    ("user", "app".to_string()),
                    ("password", "s3cret".to_string()),
                    ("database", "app".to_string())
                ]
            ]
        );
        assert_eq!(
            login(&urls, "redis").lines,
            [vec![
                ("host", "localhost".to_string()),
                ("port", "6379".to_string()),
                ("no password", String::new())
            ]]
        );
        assert_eq!(
            login(&urls, "auth").lines,
            [vec![
                ("admin", "admin".to_string()),
                ("password", "admin".to_string())
            ]]
        );
    }

    #[test]
    fn a_port_is_read_back_from_an_address() {
        assert_eq!(port_of("http://localhost:3000"), Some(3000));
        assert_eq!(port_of("https://api.localhost:8443"), Some(8443));
        assert_eq!(port_of("http://localhost"), None);
        assert_eq!(port_of("postgres://app:pw@localhost:5432/app"), Some(5432));
        assert_eq!(port_of("http://localhost:8000/health"), Some(8000));
    }
}
