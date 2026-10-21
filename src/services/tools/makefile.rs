//! The generated Makefile: short names for the long commands.
use crate::models::{Backend, Frontend, Plan, Service};

pub(super) fn generate(plan: &Plan) -> String {
    let mut s =
        String::from(".PHONY: help up down dev ci fmt hooks env certs\n\nTIER ?= local\n\n");
    s.push_str("# Every target with a `## text` comment is listed by `make help`.\n");
    s.push_str("help: ## Show this list\n\t@grep -hE '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf \"  make %-8s %s\\n\", $$1, $$2}'\n\n");
    if plan.has(Service::Https) {
        s.push_str("up: certs ## Build and start everything in Docker\n");
    } else {
        s.push_str("up: ## Build and start everything in Docker\n");
    }
    s.push_str(
        "\tdocker compose up --build\n\ndown: ## Stop everything\n\tdocker compose down\n\n",
    );
    s.push_str("env: ## Switch environment: make env TIER=dev (copies .env.dev to .env)\n\t@test -f .env.$(TIER) || { echo \".env.$(TIER) not found: copy .env.example to .env.$(TIER)\"; exit 1; }\n\tcp .env.$(TIER) .env\n\n");
    if plan.has(Service::Https) {
        s.push_str(
            "certs: ## Create trusted local HTTPS certificates (mkcert)\n\t./scripts/certs.sh\n\n",
        );
    }

    let infra: Vec<&str> = [
        (Service::Postgres, "postgres"),
        (Service::Redis, "redis"),
        (Service::Mongodb, "mongo"),
    ]
    .iter()
    .filter(|(svc, _)| plan.has(*svc))
    .map(|(_, n)| *n)
    .collect();
    s.push_str("dev: ## Start the databases in Docker and run the backend with reload\n");
    if !infra.is_empty() {
        s.push_str(&format!("\tdocker compose up -d {}\n", infra.join(" ")));
    }
    match plan.backend {
        Backend::Axum => s.push_str("\tcd backend && cargo watch -x run\n"),
        Backend::Fastapi => {
            s.push_str("\tcd backend && uv run uvicorn app.main:app --reload --port 8000\n")
        }
        Backend::None => {}
    }
    s.push('\n');

    s.push_str("ci: ## Run the same checks CI runs (format, lint, tests)\n\t./ci.sh\n\n");
    s.push_str("fmt: ## Format backend and frontend code\n");
    match plan.backend {
        Backend::Axum => s.push_str("\tcd backend && cargo fmt --all\n"),
        Backend::Fastapi => s.push_str("\tcd backend && uv run ruff format .\n"),
        Backend::None => {}
    }
    match plan.frontend {
        Frontend::Nuxt => s.push_str("\tcd frontend && pnpm fmt\n"),
        Frontend::Vue => s.push_str("\tcd frontend && pnpm exec vp check --fix\n"),
        Frontend::None => {}
    }
    s.push('\n');
    s.push_str(
        "hooks: ## Turn on the gitleaks pre-commit hook\n\tgit config core.hooksPath .githooks\n",
    );
    s
}
