//! Nuxt: scaffolded with `nuxi`, linted with oxlint + oxfmt + @nuxt/eslint.
use std::{fs, path::Path};

use anyhow::Result;

use crate::{
    clients::run::{capture, run},
    models::{FeLib, Plan, Tool},
};

use super::shared::*;

pub(super) fn generate(root: &Path, plan: &Plan) -> Result<()> {
    println!("frontend: Nuxt");
    run(
        root,
        "pnpm",
        &[
            "dlx",
            "nuxi@latest",
            "init",
            "frontend",
            "--template",
            "minimal",
            "--packageManager",
            "pnpm",
            "--no-install",
            "--no-gitInit",
            "--force",
        ],
    )?;
    let dir = root.join("frontend");
    install_libs(&dir, plan, true)?;
    fs::write(dir.join("nuxt.config.ts"), nuxt_config(plan))?;

    // An empty pages/ folder turns on page routing with no `/` route (a 404), so it ships with
    // a starter page; the other layers are empty folders kept by .gitkeep.
    fs::write(
        dir.join("app/app.vue"),
        include_str!("../../../assets/frontend/nuxt/app.vue"),
    )?;
    fs::create_dir_all(dir.join("app/pages"))?;
    fs::write(
        dir.join("app/pages/index.vue"),
        include_str!("../../../assets/frontend/nuxt/index.vue"),
    )?;
    crate::services::backend::keep(&dir.join("app"), &plan.frontend_layers())?;

    if plan.lib(FeLib::I18n) {
        fs::create_dir_all(dir.join("i18n/locales"))?;
        fs::write(
            dir.join("i18n/locales/en.json"),
            "{\n  \"welcome\": \"It works\"\n}\n",
        )?;
    }
    if plan.lib(FeLib::Tailwind) {
        fs::create_dir_all(dir.join("app/assets/css"))?;
        fs::write(
            dir.join("app/assets/css/main.css"),
            "@import \"tailwindcss\";\n",
        )?;
    }
    if plan.lib(FeLib::Openapi) {
        openapi_config(&dir, plan, "app/contracts/api")?;
    }
    if plan.tool(Tool::Lint) {
        lint_nuxt(&dir)?;
    }
    let pnpm = capture(root, "pnpm", &["--version"]).unwrap_or_else(|| "latest".into());
    fs::write(
        dir.join("Dockerfile"),
        include_str!("../../../assets/frontend/nuxt/Dockerfile").replace("{{PNPM}}", &pnpm),
    )?;
    Ok(())
}

fn lint_nuxt(dir: &Path) -> Result<()> {
    first_add(dir, &["add", "-D", "oxlint", "oxfmt"])?;
    fs::write(
        dir.join(".oxlintrc.json"),
        include_str!("../../../assets/frontend/shared/oxlintrc.json"),
    )?;
    fs::write(
        dir.join(".oxfmtrc.jsonc"),
        include_str!("../../../assets/frontend/shared/oxfmtrc.jsonc"),
    )?;
    set_scripts(dir, &[("fmt", "oxfmt"), ("fmt:check", "oxfmt --check")])?;
    first_add(
        dir,
        &[
            "add",
            "-D",
            "@nuxt/eslint",
            "eslint",
            "eslint-config-prettier",
            "eslint-plugin-oxlint",
        ],
    )?;
    // Held at 6: @typescript-eslint (via @nuxt/eslint) does not support TypeScript 7 yet.
    // Unpin once `pnpm peers check` is clean with the latest.
    first_add(
        dir,
        &["add", "-D", "typescript@6", "vue-tsc", "@types/node"],
    )?;
    set_scripts(dir, &[("typecheck", "nuxt typecheck")])?;
    fs::write(
        dir.join("eslint.config.mjs"),
        include_str!("../../../assets/frontend/nuxt/eslint.config.mjs"),
    )?;
    set_scripts(dir, &[("lint", "oxlint && eslint .")])?;
    run(dir, "pnpm", &["fmt"])
}

fn nuxt_config(plan: &Plan) -> String {
    let mut modules = Vec::new();
    if plan.tool(Tool::Lint) {
        modules.push("'@nuxt/eslint'");
    }
    if plan.lib(FeLib::Pinia) {
        modules.push("'@pinia/nuxt'");
    }
    if plan.lib(FeLib::Colada) {
        modules.push("'@pinia/colada-nuxt'");
    }
    if plan.lib(FeLib::I18n) {
        modules.push("'@nuxtjs/i18n'");
    }
    let tailwind = plan.lib(FeLib::Tailwind);
    let mut out = String::new();
    if tailwind {
        out.push_str("import tailwindcss from '@tailwindcss/vite'\n\n");
    }
    out.push_str("export default defineNuxtConfig({\n");
    out.push_str("  compatibilityDate: '2025-07-15',\n  devtools: { enabled: true },\n");
    out.push_str(&format!("  modules: [{}],\n", modules.join(", ")));
    if tailwind {
        out.push_str("  css: ['~/assets/css/main.css'],\n  vite: { plugins: [tailwindcss()] },\n");
    }
    if plan.lib(FeLib::I18n) {
        out.push_str("  i18n: {\n    locales: [{ code: 'en', name: 'English', file: 'en.json' }],\n    defaultLocale: 'en',\n    strategy: 'no_prefix'\n  },\n");
    }
    out.push_str("  // Override with NUXT_PUBLIC_API_BASE.\n  runtimeConfig: { public: { apiBase: 'http://localhost:8000' } }\n})\n");
    out
}
