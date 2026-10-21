//! ci.sh: the checks CI runs, also behind `stackup ci`.
use crate::models::{Backend, Frontend, Plan};

pub(super) fn generate(plan: &Plan) -> String {
    let mut s = String::from("#!/bin/sh\nset -eu\n\n");
    // Topcoat and axum are one workspace: formatted, linted and tested once, from the root.
    if plan.rust_workspace() {
        s.push_str(
            "cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked\n\
             if grep -rnE 'sqlx::|redis::|mongodb::' backend/src/routes backend/src/services; then exit 1; fi\n\
             if grep -rn 'crate::clients::' backend/src/routes; then exit 1; fi\n",
        );
    }
    match plan.backend {
        Backend::Axum if plan.rust_workspace() => {}
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
        // Downward only: app -> features -> components -> composables -> services -> types, utils.
        // Components are pure UI, so they never reach for services; composables are the `cx`
        // helpers that do.
        Frontend::Topcoat => {
            // Alone it is its own project; with axum the workspace line above already covered it.
            if !plan.rust_workspace() {
                s.push_str(
                    "(cd frontend && cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked)\n",
                );
            }
            s.push_str(
            "if grep -rnEs 'crate::(app|features|services|middleware)' frontend/src/components*; then exit 1; fi\n\
             if grep -rnEs 'crate::(app|features|components|middleware)' frontend/src/composables*; then exit 1; fi\n\
             if grep -rnEs 'crate::(app|features|components|composables|middleware)' frontend/src/services*; then exit 1; fi\n\
             if grep -rnEs 'crate::app' frontend/src/features*; then exit 1; fi\n\
             if grep -rnEs 'crate::(app|features|components|composables|services)' frontend/src/middleware* frontend/src/types* frontend/src/utils*; then exit 1; fi\n",
            );
        }
        Frontend::None => {}
    }
    s
}
