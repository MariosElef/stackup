//! Vue: scaffolded with Vite+, which also owns lint, format and type checks.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::{capture, run},
    models::{FeLib, Plan, Tool},
};

use super::shared::*;

/// Vue is scaffolded with Vite+ (`vp`): create-vue underneath, then Vite+ migrates it, so
/// lint, format and type checks live in vite.config.ts and run with `vp check`.
pub(super) fn generate(root: &Path, plan: &Plan) -> Result<()> {
    println!("frontend: Vue (Vite+)");
    let mut args = vec![
        "dlx",
        "--package=vite-plus",
        "vp",
        "create",
        "vue",
        "--no-interactive",
        "--package-manager",
        "pnpm",
        "--",
        "frontend",
        "--ts",
        "--router",
    ];
    if plan.lib(FeLib::Pinia) {
        args.push("--pinia");
    }
    run(root, "pnpm", &args)?;
    let dir = root.join("frontend");
    // Left behind by the ESLint-to-Oxlint migration.
    let _ = fs::remove_file(dir.join(".oxlintrc.json.bak"));
    remove_script(&dir, "prepare")?;
    let _ = fs::remove_dir_all(dir.join(".vite-hooks"));

    install_libs(&dir, plan, false)?;

    if plan.lib(FeLib::Colada) {
        patch(
            &dir.join("src/main.ts"),
            &[
                (
                    "import { createPinia }",
                    "import { PiniaColada } from \"@pinia/colada\";\nimport { createPinia }",
                ),
                (
                    "app.use(createPinia())",
                    "app.use(createPinia());\napp.use(PiniaColada)",
                ),
            ],
        );
    }
    if plan.lib(FeLib::I18n) {
        fs::create_dir_all(dir.join("src/locales"))?;
        fs::write(
            dir.join("src/locales/en.json"),
            "{\n  \"welcome\": \"It works\"\n}\n",
        )?;
        fs::write(
            dir.join("src/i18n.ts"),
            "import { createI18n } from \"vue-i18n\";\n\nimport en from \"./locales/en.json\";\n\nexport default createI18n({\n  legacy: false,\n  locale: \"en\",\n  fallbackLocale: \"en\",\n  messages: { en },\n});\n",
        )?;
        patch(
            &dir.join("src/main.ts"),
            &[
                (
                    "import App from",
                    "import i18n from \"./i18n\";\nimport App from",
                ),
                ("app.mount(", "app.use(i18n);\n\napp.mount("),
            ],
        );
    }
    if plan.lib(FeLib::Tailwind) {
        patch(
            &dir.join("vite.config.ts"),
            &[
                (
                    "import vue from",
                    "import tailwindcss from \"@tailwindcss/vite\";\nimport vue from",
                ),
                ("vueDevTools()", "vueDevTools(), tailwindcss()"),
            ],
        );
        let css = dir.join("src/assets/main.css");
        let existing = fs::read_to_string(&css).unwrap_or_default();
        fs::write(&css, format!("@import \"tailwindcss\";\n{existing}"))?;
    }
    if plan.lib(FeLib::Openapi) {
        openapi_config(&dir, plan, "src/contracts/api")?;
    }
    crate::services::generate::backend::keep(&dir.join("src"), &plan.frontend_layers())?;

    if plan.tool(Tool::Lint) {
        // The layering rules go into Vite+'s own lint block.
        patch(
            &dir.join("vite.config.ts"),
            &[(
                "lint: {",
                &format!(
                    "lint: {{\n{}",
                    include_str!("../../../../assets/frontend/vue/lint-overrides.txt")
                ),
            )],
        );
        run(&dir, "pnpm", &["exec", "vp", "check", "--fix"])?;
    }

    // Vite+ pins the package manager in package.json; the image must use the same version.
    let pinned = pinned_pnpm(&dir);
    let pnpm = pinned
        .or_else(|| capture(root, "pnpm", &["--version"]))
        .unwrap_or_else(|| "latest".into());
    fs::write(
        dir.join("Dockerfile"),
        include_str!("../../../../assets/frontend/vue/Dockerfile").replace("{{PNPM}}", &pnpm),
    )?;
    Ok(())
}
