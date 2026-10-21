mod caddy;

use crate::models::{Backend, Frontend, Plan, Service};

use caddy::caddy;
pub use caddy::caddyfile;

pub fn compose(plan: &Plan) -> String {
    let mut out = String::from("services:\n");
    let mut volumes = Vec::new();

    if plan.has(Service::Postgres) {
        out.push_str(
            &r#"  postgres:
    image: @IMAGE@
    environment:
      POSTGRES_USER: ${POSTGRES_USER:-app}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-change-me}
      POSTGRES_DB: ${POSTGRES_DB:-app}
    ports: ["5432:5432"]
    volumes: [pgdata:/var/lib/postgresql]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $${POSTGRES_USER} -d $${POSTGRES_DB}"]
      interval: 3s
      timeout: 3s
      retries: 20

"#
            .replace(
                "@IMAGE@",
                &plan.image("postgres", "postgres", true, "latest"),
            ),
        );
        volumes.push("pgdata");
    }
    if plan.has(Service::Redis) {
        out.push_str(
            &r#"  redis:
    image: @IMAGE@
    ports: ["6379:6379"]
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 3s
      timeout: 3s
      retries: 20

"#
            .replace("@IMAGE@", &plan.image("redis", "redis", true, "latest")),
        );
    }
    if plan.has(Service::Mongodb) {
        out.push_str(
            &r#"  mongo:
    image: @IMAGE@
    ports: ["27017:27017"]
    volumes: [mongodata:/data/db]
    healthcheck:
      test: ["CMD", "mongosh", "--quiet", "--eval", "db.adminCommand('ping')"]
      interval: 5s
      timeout: 5s
      retries: 20

"#
            .replace("@IMAGE@", &plan.image("mongodb", "mongo", false, "8")),
        );
        volumes.push("mongodata");
    }
    if plan.has(Service::Keycloak) {
        let keycloak = format!(
            "quay.io/keycloak/keycloak:{}",
            plan.pin("keycloak").unwrap_or("latest")
        );
        out.push_str(&format!(
            "  keycloak:\n    image: {keycloak}\n    command: start-dev --import-realm\n    environment:\n      KC_BOOTSTRAP_ADMIN_USERNAME: ${{KEYCLOAK_ADMIN:-admin}}\n      KC_BOOTSTRAP_ADMIN_PASSWORD: ${{KEYCLOAK_ADMIN_PASSWORD:-admin}}\n"
        ));
        if plan.has(Service::Https) {
            // Behind Caddy: Keycloak builds its URLs from this host and the forwarded headers.
            out.push_str(&format!(
                "      KC_HOSTNAME: {}\n      KC_HTTP_ENABLED: \"true\"\n      KC_PROXY_HEADERS: xforwarded\n",
                plan.auth_url()
            ));
        }
        out.push_str(
            // No data volume: the realm is re-imported from keycloak/app-realm.json on every
            // start, so that file is the source of truth for local development.
            "    ports: [\"8080:8080\"]\n    volumes:\n      - ./keycloak:/opt/keycloak/data/import:ro\n\n",
        );
    }

    if plan.backend != Backend::None {
        // In the Rust workspace one Dockerfile at the root builds both: this is its `backend` target.
        let build = if plan.rust_workspace() {
            "build:\n      context: .\n      target: backend"
        } else {
            "build: ./backend"
        };
        out.push_str(&format!(
            "  backend:\n    {build}\n    image: {}-backend\n",
            plan.name
        ));
        let mut deps = Vec::new();
        let mut env = vec!["      BIND_ADDR: 0.0.0.0:8000".to_string()];
        if plan.has(Service::Postgres) {
            deps.push("postgres");
            env.push(
                "      DATABASE_URL: postgres://${POSTGRES_USER:-app}:${POSTGRES_PASSWORD:-change-me}@postgres:5432/${POSTGRES_DB:-app}"
                    .into(),
            );
        }
        if plan.has(Service::Redis) {
            deps.push("redis");
            env.push("      REDIS_URL: redis://redis:6379".into());
        }
        if plan.has(Service::Mongodb) {
            deps.push("mongo");
            env.push("      MONGODB_URL: mongodb://mongo:27017".into());
        }
        if plan.has(Service::Keycloak) {
            // The issuer is the browser-facing URL (it is what a token's `iss` says); the keys
            // are fetched over the Docker network.
            env.push(format!(
                "      OIDC_ISSUER_URL: {}/realms/app",
                plan.auth_url()
            ));
            env.push(
                "      OIDC_JWKS_URL: http://keycloak:8080/realms/app/protocol/openid-connect/certs"
                    .into(),
            );
        }
        if plan.has(Service::Mailtrap) {
            env.push("      SMTP_HOST: ${MAILTRAP_HOST:-sandbox.smtp.mailtrap.io}".into());
            env.push("      SMTP_PORT: ${MAILTRAP_PORT:-2525}".into());
            env.push("      SMTP_USER: ${MAILTRAP_USER:-}".into());
            env.push("      SMTP_PASSWORD: ${MAILTRAP_PASSWORD:-}".into());
        }
        if !deps.is_empty() {
            out.push_str("    depends_on:\n");
            for d in deps {
                out.push_str(&format!("      {d}: {{ condition: service_healthy }}\n"));
            }
        }
        out.push_str("    environment:\n");
        out.push_str(&env.join("\n"));
        out.push_str("\n    ports: [\"8000:8000\"]\n\n");
    }

    match plan.frontend {
        Frontend::Nuxt => out.push_str(&format!(
            "  frontend:\n    build: ./frontend\n    image: {}-frontend\n    environment:\n      NUXT_PUBLIC_API_BASE: ${{NUXT_PUBLIC_API_BASE:-{}}}\n    ports: [\"3000:3000\"]\n\n",
            plan.name,
            plan.api_url()
        )),
        Frontend::Vue => out.push_str(&format!(
            "  frontend:\n    build: ./frontend\n    image: {}-frontend\n    ports: [\"3000:80\"]\n\n",
            plan.name
        )),
        Frontend::Topcoat => {
            let build = if plan.rust_workspace() {
                "build:\n      context: .\n      target: frontend"
            } else {
                "build: ./frontend"
            };
            out.push_str(&format!(
                "  frontend:\n    {build}\n    image: {}-frontend\n",
                plan.name
            ));
            if plan.backend != Backend::None {
                // Topcoat renders on the server, so it reaches the API over the Docker network.
                out.push_str("    environment:\n      API_URL: http://backend:8000\n");
            }
            out.push_str("    ports: [\"3000:3000\"]\n\n");
        }
        Frontend::None => {}
    }

    if plan.has(Service::Https) {
        out.push_str(&caddy(plan));
    }

    if !volumes.is_empty() {
        out.push_str("volumes:\n");
        for v in volumes {
            out.push_str(&format!("  {v}:\n"));
        }
    }
    out.trim_end().to_string() + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FeLib, Tool};

    fn plan(frontend: Frontend, backend: Backend, services: Vec<Service>) -> Plan {
        Plan {
            name: "demo".into(),
            title: "demo".into(),
            frontend,
            backend,
            libs: Vec::<FeLib>::new(),
            be_libs: vec![],
            services,
            tools: Vec::<Tool>::new(),
            git: false,
            remote: None,
            publish: None,
            pins: vec![],
        }
    }

    #[test]
    fn topcoat_is_a_service_that_reaches_the_backend_by_name() {
        let out = compose(&plan(Frontend::Topcoat, Backend::Axum, vec![]));
        // Topcoat + axum are one Rust workspace: both build from the root, each its own target.
        assert!(out.contains("  frontend:\n    build:\n      context: .\n      target: frontend"));
        assert!(out.contains("  backend:\n    build:\n      context: .\n      target: backend"));
        assert!(out.contains("API_URL: http://backend:8000"));
        assert!(out.contains("3000:3000"));
        let alone = compose(&plan(Frontend::Topcoat, Backend::None, vec![]));
        assert!(
            !alone.contains("API_URL"),
            "no backend, nothing to point at"
        );
    }

    #[test]
    fn topcoat_sits_behind_caddy_on_its_own_port() {
        let p = plan(Frontend::Topcoat, Backend::Axum, vec![Service::Https]);
        assert!(caddyfile(&p).contains("reverse_proxy frontend:3000"));
    }

    #[test]
    fn built_services_get_explicit_image_names_for_deploy() {
        for frontend in [Frontend::Nuxt, Frontend::Vue, Frontend::Topcoat] {
            let out = compose(&plan(frontend, Backend::Axum, vec![]));
            assert!(out.contains("image: demo-backend"), "{frontend}");
            assert!(out.contains("image: demo-frontend"), "{frontend}");
        }
        let out = compose(&plan(
            Frontend::None,
            Backend::Fastapi,
            vec![Service::Redis],
        ));
        assert!(out.contains("image: demo-backend") && !out.contains("demo-frontend"));
    }

    #[test]
    fn postgres_data_is_mounted_where_postgres_18_wants_it() {
        // `postgres:alpine` is 18, which exits with an error when a volume is mounted at the old
        // `/var/lib/postgresql/data`; the mount one level up works for it and for older ones.
        let out = compose(&plan(
            Frontend::None,
            Backend::None,
            vec![Service::Postgres],
        ));
        assert!(out.contains("pgdata:/var/lib/postgresql]"), "{out}");
        assert!(!out.contains("/var/lib/postgresql/data"));
    }

    #[test]
    fn pinned_versions_become_the_images_tags() {
        let mut p = plan(
            Frontend::None,
            Backend::None,
            vec![
                Service::Postgres,
                Service::Redis,
                Service::Mongodb,
                Service::Keycloak,
            ],
        );
        let out = compose(&p);
        // Nothing pinned: the small Alpine images, Mongo 8, Keycloak latest.
        for image in [
            "postgres:alpine",
            "redis:alpine",
            "mongo:8",
            "keycloak:latest",
        ] {
            assert!(out.contains(image), "{image}\n{out}");
        }
        p.pins = vec![
            ("postgres".into(), "17".into()),
            ("redis".into(), "7".into()),
            ("mongodb".into(), "latest".into()),
            ("keycloak".into(), "26.0".into()),
        ];
        let out = compose(&p);
        for image in [
            "postgres:17-alpine",
            "redis:7-alpine",
            "mongo:latest",
            "keycloak:26.0",
        ] {
            assert!(out.contains(image), "{image}\n{out}");
        }
    }
}
