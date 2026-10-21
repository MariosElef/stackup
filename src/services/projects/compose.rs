//! What a project's compose file says, read from `docker compose config --format json`.
use std::path::Path;

use serde_json::Value;

fn services(config: &Value) -> impl Iterator<Item = (&String, &Value)> {
    config["services"].as_object().into_iter().flatten()
}

/// The images compose builds from this project's own folders (not the ones it pulls).
pub fn built_images(config: &Value) -> Vec<String> {
    services(config)
        .filter(|(_, svc)| svc.get("build").is_some())
        .filter_map(|(_, svc)| svc["image"].as_str().map(str::to_string))
        .collect()
}

/// Which of `names` this project has as services, in the order asked.
pub fn present(config: &Value, names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter(|n| config["services"].get(**n).is_some())
        .map(|n| (*n).to_string())
        .collect()
}

/// Host ports the services publish.
pub fn published_ports(config: &Value) -> Vec<u16> {
    let mut ports: Vec<u16> = services(config)
        .flat_map(|(_, svc)| svc["ports"].as_array().into_iter().flatten())
        .filter_map(|p| match &p["published"] {
            Value::String(s) => s.parse().ok(),
            Value::Number(n) => n.as_u64().and_then(|n| u16::try_from(n).ok()),
            _ => None,
        })
        .collect();
    ports.sort_unstable();
    ports.dedup();
    ports
}

/// Files and folders next to the compose file that services mount: they travel with a deploy.
pub fn bind_sources(config: &Value, root: &Path) -> Vec<String> {
    let mut found: Vec<String> = services(config)
        .flat_map(|(_, svc)| svc["volumes"].as_array().into_iter().flatten())
        .filter(|v| v["type"] == "bind")
        .filter_map(|v| v["source"].as_str())
        .filter_map(|src| Path::new(src).strip_prefix(root).ok())
        .filter_map(|rel| rel.components().next())
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        serde_json::json!({
            "services": {
                "postgres": { "image": "postgres:alpine", "ports": [{ "published": "5432" }] },
                "backend": { "image": "demo-backend", "build": { "context": "/p/backend" },
                             "ports": [{ "published": "8000" }] },
                "frontend": { "image": "demo-frontend", "build": { "context": "/p/frontend" },
                              "ports": [{ "published": 3000 }] },
                "caddy": { "image": "caddy:alpine", "volumes": [
                    { "type": "bind", "source": "/p/Caddyfile" },
                    { "type": "bind", "source": "/p/certs" },
                    { "type": "volume", "source": "pgdata" }] }
            }
        })
    }

    #[test]
    fn only_the_images_compose_builds_are_shipped() {
        let mut built = built_images(&sample());
        built.sort();
        assert_eq!(built, ["demo-backend", "demo-frontend"]);
    }

    #[test]
    fn services_ports_and_mounts_are_read_from_the_config() {
        let c = sample();
        assert_eq!(present(&c, &["postgres", "redis", "mongo"]), ["postgres"]);
        assert_eq!(published_ports(&c), [3000, 5432, 8000]);
        assert_eq!(bind_sources(&c, Path::new("/p")), ["Caddyfile", "certs"]);
    }
}
