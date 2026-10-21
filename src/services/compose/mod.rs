mod caddy;

use crate::models::{Backend, Frontend, Plan, Service};

use caddy::caddy;
pub use caddy::caddyfile;

const KEYCLOAK_IMAGE: &str = "quay.io/keycloak/keycloak:latest";

pub fn compose(plan: &Plan) -> String {
    let mut out = String::from("services:\n");
    let mut volumes = Vec::new();

    if plan.has(Service::Postgres) {
        out.push_str(
            r#"  postgres:
    image: postgres:alpine
    environment:
      POSTGRES_USER: ${POSTGRES_USER:-app}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-change-me}
      POSTGRES_DB: ${POSTGRES_DB:-app}
    ports: ["5432:5432"]
    volumes: [pgdata:/var/lib/postgresql/data]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $${POSTGRES_USER} -d $${POSTGRES_DB}"]
      interval: 3s
      timeout: 3s
      retries: 20

"#,
        );
        volumes.push("pgdata");
    }
    if plan.has(Service::Redis) {
        out.push_str(
            r#"  redis:
    image: redis:alpine
    ports: ["6379:6379"]
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 3s
      timeout: 3s
      retries: 20

"#,
        );
    }
    if plan.has(Service::Mongodb) {
        out.push_str(
            r#"  mongo:
    image: mongo:8
    ports: ["27017:27017"]
    volumes: [mongodata:/data/db]
    healthcheck:
      test: ["CMD", "mongosh", "--quiet", "--eval", "db.adminCommand('ping')"]
      interval: 5s
      timeout: 5s
      retries: 20

"#,
        );
        volumes.push("mongodata");
    }
    if plan.has(Service::Keycloak) {
        out.push_str(&format!(
            "  keycloak:\n    image: {KEYCLOAK_IMAGE}\n    command: start-dev --import-realm\n    environment:\n      KC_BOOTSTRAP_ADMIN_USERNAME: ${{KEYCLOAK_ADMIN:-admin}}\n      KC_BOOTSTRAP_ADMIN_PASSWORD: ${{KEYCLOAK_ADMIN_PASSWORD:-admin}}\n"
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
        out.push_str("  backend:\n    build: ./backend\n");
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
            "  frontend:\n    build: ./frontend\n    environment:\n      NUXT_PUBLIC_API_BASE: ${{NUXT_PUBLIC_API_BASE:-{}}}\n    ports: [\"3000:3000\"]\n\n",
            plan.api_url()
        )),
        Frontend::Vue => out.push_str("  frontend:\n    build: ./frontend\n    ports: [\"3000:80\"]\n\n"),
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
