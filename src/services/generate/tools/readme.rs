//! The generated project's README.
use crate::models::{Backend, Frontend, Plan, Service};

/// A technology's name in its brand color, the way stackup's own README shows them.
fn tech(name: &str, color: &str) -> String {
    format!("$\\color{{#{color}}}{{\\textbf{{{name}}}}}$")
}

/// The centered header: the project's name, "Built with stackup", and what it is made of.
fn header(plan: &Plan) -> String {
    let mut stack = Vec::new();
    match plan.frontend {
        Frontend::Nuxt => stack.push(tech("Nuxt", "00DC82")),
        Frontend::Vue => stack.push(tech("Vue", "4FC08D")),
        Frontend::Topcoat => stack.push("**Topcoat**".to_string()),
        Frontend::None => {}
    }
    match plan.backend {
        Backend::Axum => stack.push(tech("axum", "DEA584")),
        Backend::Fastapi => stack.push(tech("FastAPI", "009688")),
        Backend::None => {}
    }
    let mut out = format!(
        "<div align=\"center\">\n\n# {}\n\n**Built with [stackup](https://github.com/MariosElef/stackup)**\n\n",
        plan.title
    );
    if !stack.is_empty() {
        out.push_str(&format!("{}\n\n", stack.join(" · ")));
    }
    let services: Vec<String> = [
        (Service::Postgres, "Postgres", "4169E1"),
        (Service::Redis, "Redis", "FF4438"),
        (Service::Mongodb, "MongoDB", "47A248"),
        (Service::Keycloak, "Keycloak", "00B8E3"),
    ]
    .iter()
    .filter(|(svc, ..)| plan.has(*svc))
    .map(|(_, name, color)| tech(name, color))
    .chain(std::iter::once(tech("Docker Compose", "2496ED")))
    .collect();
    out.push_str(&format!("{}\n\n</div>\n\n---\n\n", services.join(" · ")));
    out
}

pub(super) fn generate(plan: &Plan) -> String {
    let mut s = header(plan);
    s.push_str(&format!(
        "- Frontend: {}\n- Backend: {}\n",
        plan.frontend, plan.backend
    ));
    let services: Vec<String> = plan.services.iter().map(ToString::to_string).collect();
    s.push_str(&format!(
        "- Services: {}\n\n",
        if services.is_empty() {
            "none".into()
        } else {
            services.join(", ")
        }
    ));

    s.push_str("## Commands\n\n[`stackup`](https://github.com/MariosElef/stackup) is the command line for this project. Run it with no arguments for a menu of your projects, or from inside this folder:\n\n```sh\nstackup up              # build and start everything in Docker\nstackup dev             # databases in Docker, backend with reload\nstackup doctor          # can this machine run it? each failure names its fix\nstackup build           # build the images\nstackup deploy --host HOST   # build for linux, send the images over SSH, start them\nstackup ci              # the checks CI runs\nstackup fmt             # format the code\n```\n\n`deploy` needs SSH access to a server with Docker. It builds for `linux/amd64`, streams the images with `docker save | ssh docker load` (no registry), copies `docker-compose.yml` and `.env.prod` (as `.env`), and runs `docker compose up -d`. Try it first with `--dry-run`. Without stackup, `docker compose up --build` starts everything.\n\n");
    s.push_str("## Run\n\n```sh\n");
    if plan.has(Service::Https) {
        s.push_str("./scripts/certs.sh      # once: a trusted local certificate (mkcert)\n");
    }
    s.push_str("docker compose up --build\n```\n\n");

    s.push_str("## Where things are\n\n");
    if plan.frontend != Frontend::None {
        s.push_str(&format!("- App: {}\n", plan.web_url()));
    }
    if plan.backend != Backend::None {
        s.push_str(&format!("- API: {}/health\n", plan.api_url()));
    }
    if plan.has(Service::Keycloak) {
        s.push_str(&format!(
            "- Keycloak: {} (admin / admin). Realm `app`, user `demo` / `demo`, clients `app-frontend` and `app-backend`.\n",
            plan.auth_url()
        ));
    }
    if plan.has(Service::Mailtrap) {
        s.push_str("- Mailtrap: set `MAILTRAP_USER` and `MAILTRAP_PASSWORD` from your inbox's SMTP settings.\n");
    }
    s.push_str("\n## Configuration\n\nSettings live in `.env`, which compose reads. `.env.example` is the committed template. For another environment run `stackup env --create dev` (it saves the current `.env` as `.env.local` first) and switch with `stackup env dev`. Everything except `.env.example` is gitignored.\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BeLib, FeLib, Tool};

    fn plan(frontend: Frontend, backend: Backend, services: Vec<Service>) -> Plan {
        Plan {
            name: "demo".into(),
            title: "demo".into(),
            frontend,
            backend,
            libs: Vec::<FeLib>::new(),
            be_libs: Vec::<BeLib>::new(),
            services,
            tools: Vec::<Tool>::new(),
            git: false,
            remote: None,
            publish: None,
            pins: vec![],
        }
    }

    #[test]
    fn the_readme_opens_with_built_with_stackup_and_the_stack() {
        let md = generate(&plan(
            Frontend::Nuxt,
            Backend::Axum,
            vec![Service::Postgres],
        ));
        assert!(md.starts_with("<div align=\"center\">\n\n# demo\n\n**Built with [stackup]"));
        for brand in ["00DC82", "DEA584", "4169E1", "2496ED"] {
            assert!(md.contains(brand), "{brand}");
        }
        assert!(!md.contains("FF4438"), "Redis is not in this project");
    }

    #[test]
    fn a_project_without_a_stack_still_gets_a_header() {
        let md = generate(&plan(Frontend::None, Backend::None, vec![]));
        assert!(md.contains("Built with [stackup]") && md.contains("Docker Compose"));
    }
}
