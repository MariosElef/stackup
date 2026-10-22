use crate::{
    models::{Backend, Frontend, Service, Tool},
    ui::{features::create::wizard::Wizard, types::Outcome},
};

pub const TOOLS_TO_CHECK: [&str; 9] = [
    "docker", "cargo", "uv", "pnpm", "topcoat", "git", "gh", "mkcert", "ssh",
];

/// What a tool is for, in the message that says it is missing.
pub fn needed_for(bin: &str) -> &'static str {
    match bin {
        "docker" => {
            "running, building and deploying every project (Docker Desktop must be running)"
        }
        "cargo" => "axum and Topcoat projects",
        "uv" => "FastAPI projects",
        "pnpm" => "Nuxt and Vue projects",
        "topcoat" => "Topcoat projects",
        "git" => "creating a project's git repository and its hooks",
        "gh" => "creating the GitHub repository and signing in with an SSH key",
        "mkcert" => "local HTTPS",
        "ssh" => "deploying to a server",
        _ => "some projects",
    }
}

impl Wizard {
    /// The programs the current choices need installed.
    pub fn requires(&self) -> Vec<&'static str> {
        // Every project runs on Docker Compose.
        let mut need = vec!["docker"];
        match self.sel.frontend {
            Frontend::Nuxt | Frontend::Vue => need.push("pnpm"),
            Frontend::Topcoat => need.extend(["cargo", "topcoat"]),
            Frontend::None => {}
        }
        match self.sel.backend {
            Backend::Axum if !need.contains(&"cargo") => need.push("cargo"),
            Backend::Axum => {}
            Backend::Fastapi => need.push("uv"),
            Backend::None => {}
        }
        if self.sel.services.contains(&Service::Https) {
            need.push("mkcert");
        }
        // The migrate, new migration and revert actions run sqlx-cli.
        if self.sel.backend == Backend::Axum
            && self.sel.be_libs.contains(&crate::models::BeLib::Sqlx)
        {
            need.push("sqlx");
        }
        need.push("git");
        // The project's repository goes to GitHub: that needs the GitHub CLI, signed in.
        if self.sel.tools.contains(&Tool::Git) {
            need.push("gh");
        }
        need
    }

    pub fn is_installed(&self, bin: &str) -> bool {
        self.installed.iter().any(|(b, ok)| *b == bin && *ok)
    }

    /// Finishes the wizard, unless something the choices need is missing: then the app is asked
    /// to offer installing it.
    pub fn try_create(&mut self) {
        let missing: Vec<&'static str> = self
            .requires()
            .into_iter()
            .filter(|b| !self.is_installed(b))
            .collect();
        if missing.is_empty() {
            self.outcome = Outcome::Create;
        } else {
            self.blocked = Some(missing);
        }
    }
}
