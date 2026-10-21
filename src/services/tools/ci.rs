//! ci.sh: the checks CI runs, also behind `make ci`.
use crate::models::{Backend, Frontend, Plan};

pub(super) fn generate(plan: &Plan) -> String {
    let mut s = String::from("#!/bin/sh\nset -eu\n\n");
    match plan.backend {
        Backend::Axum => s.push_str(
            "(cd backend && cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked)\n\
             if grep -rnE 'sqlx::|redis::|mongodb::' backend/src/routes backend/src/services; then exit 1; fi\n\
             if grep -rn 'crate::clients::' backend/src/routes; then exit 1; fi\n",
        ),
        Backend::Fastapi => s.push_str(
            "(cd backend && uv sync --locked && uv run ruff check . && uv run ruff format --check . && uv run lint-imports && uv run pytest -q)\n",
        ),
        Backend::None => {}
    }
    match plan.frontend {
        Frontend::Nuxt => s.push_str(
            "(cd frontend && pnpm install --frozen-lockfile && pnpm fmt:check && pnpm lint && pnpm typecheck)\n",
        ),
        // Vite+ runs format, lint and type checks together.
        Frontend::Vue => s.push_str(
            "(cd frontend && pnpm install --frozen-lockfile && pnpm exec vp check)\n",
        ),
        Frontend::None => {}
    }
    s
}
