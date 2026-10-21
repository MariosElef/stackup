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
    url.rsplit(':').next()?.trim_end_matches('/').parse().ok()
}

/// A project's addresses, with whether each web one is answering. Empty when Docker is not
/// installed: the compose file cannot be read without it.
pub fn read(root: &Path) -> Urls {
    let Ok(config) = docker::config(root) else {
        return Urls::default();
    };
    let caddyfile = std::fs::read_to_string(root.join("Caddyfile")).ok();
    let mut urls = links(&config, caddyfile.as_deref());
    for l in &mut urls.web {
        l.up = port_of(&l.url).map(listening);
    }
    urls
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
    fn a_port_is_read_back_from_an_address() {
        assert_eq!(port_of("http://localhost:3000"), Some(3000));
        assert_eq!(port_of("https://api.localhost:8443"), Some(8443));
        assert_eq!(port_of("http://localhost"), None);
    }
}
