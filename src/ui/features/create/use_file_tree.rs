use crate::models::{AXUM_LAYERS, Backend, FASTAPI_LAYERS, FeLib, Frontend, Plan, Service, Tool};

#[derive(Clone)]
pub enum Node {
    Dir(String, Vec<Node>),
    File(String),
}

fn dir(name: &str, kids: Vec<Node>) -> Node {
    Node::Dir(format!("{name}/"), kids)
}

fn file(name: &str) -> Node {
    Node::File(name.to_string())
}

fn files(names: &[&str]) -> Vec<Node> {
    names.iter().map(|n| file(n)).collect()
}

/// Empty layers: shown on one line, since each is just a `.gitkeep`.
fn layers(names: &[&str]) -> Vec<Node> {
    names.iter().map(|n| dir(n, vec![])).collect()
}

/// What `stackup new` will write for this plan, as a tree.
pub fn file_tree(plan: &Plan) -> Vec<Node> {
    let lint = plan.tool(Tool::Lint);
    let mut root = Vec::new();

    // Topcoat + axum: one Cargo workspace. What belongs to the whole project is at the root, once.
    if plan.rust_workspace() {
        root.extend(files(&[
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "Dockerfile",
            ".dockerignore",
        ]));
        if lint {
            root.extend(files(&["deny.toml", "taplo.toml"]));
        }
    }

    match plan.frontend {
        Frontend::Nuxt => {
            let tailwind = plan.lib(FeLib::Tailwind);
            let mut app = vec![file("app.vue"), dir("pages", vec![file("index.vue")])];
            for name in plan.frontend_layers() {
                app.push(if name == "assets" && tailwind {
                    dir("assets", vec![dir("css", vec![file("main.css")])])
                } else {
                    dir(name, vec![])
                });
            }
            let mut kids = vec![dir("app", app)];
            if plan.lib(FeLib::I18n) {
                kids.push(dir("i18n", vec![dir("locales", vec![file("en.json")])]));
            }
            kids.extend(files(&["nuxt.config.ts", "package.json", "Dockerfile"]));
            if plan.lib(FeLib::Openapi) {
                kids.push(file("openapi-ts.config.ts"));
            }
            if lint {
                kids.extend(files(&[
                    ".oxlintrc.json",
                    ".oxfmtrc.jsonc",
                    "eslint.config.mjs",
                ]));
            }
            root.push(dir("frontend", kids));
        }
        // Scaffolded with Vite+: lint, format and type checks live in vite.config.ts.
        Frontend::Vue => {
            let mut src = vec![
                file("main.ts"),
                file("App.vue"),
                dir("assets", vec![file("main.css")]),
                dir("router", vec![file("index.ts")]),
            ];
            if plan.lib(FeLib::I18n) {
                src.push(file("i18n.ts"));
                src.push(dir("locales", vec![file("en.json")]));
            }
            src.extend(layers(&plan.frontend_layers()));
            let mut kids = vec![dir("src", src)];
            kids.extend(files(&[
                "vite.config.ts",
                "package.json",
                "pnpm-workspace.yaml",
                "AGENTS.md",
                "Dockerfile",
            ]));
            if plan.lib(FeLib::Openapi) {
                kids.push(file("openapi-ts.config.ts"));
            }
            root.push(dir("frontend", kids));
        }
        // Scaffolded by `topcoat new`: routes in `app`, then `features` and `components`.
        Frontend::Topcoat => {
            let mut src = vec![file("main.rs"), file("app.rs"), file("features.rs")];
            if plan.topcoat_ui() {
                src.push(file("components.rs"));
                src.push(dir("components", vec![file("card.rs")]));
            }
            src.extend(layers(&plan.frontend_layers()));
            let mut kids = vec![dir("src", src)];
            if plan.lib(FeLib::I18n) {
                kids.push(dir("locales", vec![file("en.yml")]));
            }
            kids.extend(files(&["Cargo.toml", "styles.css"]));
            if !plan.rust_workspace() {
                kids.push(file("Cargo.lock"));
            }
            // Tailwind and the icon sets are fetched and staged by build.rs.
            if plan.lib(FeLib::Tailwind) || plan.lib(FeLib::Lucide) {
                kids.push(file("build.rs"));
            }
            if plan.topcoat_ui() {
                kids.push(file("components.toml"));
            }
            if !plan.rust_workspace() {
                kids.extend(files(&["rust-toolchain.toml", "Dockerfile"]));
            }
            root.push(dir("frontend", kids));
        }
        Frontend::None => {}
    }

    match plan.backend {
        Backend::Axum => {
            let mut kids = vec![dir(
                "src",
                [vec![file("main.rs")], layers(AXUM_LAYERS)].concat(),
            )];
            kids.push(file("Cargo.toml"));
            if !plan.rust_workspace() {
                kids.extend(files(&["rust-toolchain.toml", "Dockerfile"]));
                if lint {
                    kids.extend(files(&["deny.toml", "taplo.toml"]));
                }
            }
            root.push(dir("backend", kids));
        }
        Backend::Fastapi => {
            let pkg = |n: &str| dir(n, vec![file("__init__.py")]);
            let mut app = vec![file("__init__.py"), file("main.py")];
            app.extend(FASTAPI_LAYERS.iter().map(|n| pkg(n)));
            let mut kids = vec![dir("app", app), dir("tests", vec![file("test_health.py")])];
            kids.extend(files(&["pyproject.toml", "Dockerfile"]));
            if lint {
                kids.push(file("taplo.toml"));
            }
            root.push(dir("backend", kids));
        }
        Backend::None => {}
    }

    if plan.tool(Tool::Gitleaks) {
        root.push(dir(".githooks", vec![file("pre-commit")]));
    }
    if plan.has(Service::Keycloak) {
        root.push(dir("keycloak", vec![file("app-realm.json")]));
    }
    if plan.has(Service::Https) {
        root.push(dir("certs", vec![]));
        root.push(dir("scripts", vec![file("certs.sh")]));
        root.push(file("Caddyfile"));
        root.push(file("Caddyfile.dev"));
    }
    root.extend(files(&[
        "docker-compose.yml",
        ".env.example",
        ".env",
        ".gitignore",
        ".gitattributes",
        "README.md",
    ]));
    if plan.tool(Tool::Ci) {
        root.push(file("ci.sh"));
    }
    if lint {
        root.push(file(".yamlfmt"));
    }
    root
}
