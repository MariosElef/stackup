//! Steps both frontends use: package lists, pnpm helpers, patching generated files.
use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::{
    clients::run::run,
    models::{Backend, BeLib, FeLib, Plan},
};

/// Runtime and dev packages for the chosen libraries. The two frameworks differ only in how
/// Pinia, Pinia Colada and i18n are packaged.
pub(super) fn packages(plan: &Plan, nuxt: bool) -> (Vec<&'static str>, Vec<&'static str>) {
    let (mut deps, mut dev) = (Vec::new(), Vec::new());
    if plan.lib(FeLib::Pinia) {
        deps.push("pinia");
        if nuxt {
            deps.push("@pinia/nuxt");
        }
    }
    if plan.lib(FeLib::Colada) {
        deps.push("@pinia/colada");
        if nuxt {
            deps.push("@pinia/colada-nuxt");
        }
    }
    if plan.lib(FeLib::I18n) {
        deps.push(if nuxt { "@nuxtjs/i18n" } else { "vue-i18n" });
    }
    if plan.lib(FeLib::Zod) {
        deps.push("zod");
    }
    if plan.lib(FeLib::Lucide) {
        deps.push("@lucide/vue");
    }
    if plan.lib(FeLib::Vueuse) {
        deps.push("@vueuse/core");
    }
    if plan.lib(FeLib::Tailwind) {
        deps.extend(["tailwindcss", "@tailwindcss/vite"]);
    }
    if plan.lib(FeLib::Openapi) {
        dev.push("@hey-api/openapi-ts");
    }
    (deps, dev)
}

pub(super) fn install_libs(dir: &Path, plan: &Plan, nuxt: bool) -> Result<()> {
    let (deps, dev) = packages(plan, nuxt);
    if deps.is_empty() {
        // Nuxt is scaffolded with --no-install, so there is always an install to do.
        if nuxt {
            first_add(dir, &["install"])?;
        }
    } else {
        let mut args = vec!["add"];
        args.extend(deps);
        first_add(dir, &args)?;
    }
    if !dev.is_empty() {
        let mut args = vec!["add", "-D"];
        args.extend(dev);
        first_add(dir, &args)?;
    }
    Ok(())
}

/// A hey-api config pointing at the backend's OpenAPI document. FastAPI serves
/// `/openapi.json`; for axum, point `input` at your own spec.
pub(super) fn openapi_config(dir: &Path, plan: &Plan, output: &str) -> Result<()> {
    let note = if plan.backend == Backend::Fastapi {
        "FastAPI serves this at /openapi.json."
    } else if plan.be_lib(BeLib::Utoipa) {
        "The axum backend serves this at /openapi.json (utoipa)."
    } else {
        "Point this at your backend's OpenAPI document."
    };
    fs::write(
        dir.join("openapi-ts.config.ts"),
        format!(
            "import {{ defineConfig }} from '@hey-api/openapi-ts'\n\n// Run `pnpm generate:api` with the backend up. {note}\nexport default defineConfig({{\n  input: 'http://localhost:8000/openapi.json',\n  output: '{output}'\n}})\n"
        ),
    )?;
    set_scripts(dir, &[("generate:api", "openapi-ts")])
}

/// String replacements on a generated file. A miss is reported and skipped, never fatal: the
/// project still works, it just lacks that wiring.
pub(super) fn patch(path: &Path, edits: &[(&str, &str)]) {
    let Ok(mut text) = fs::read_to_string(path) else {
        println!("  ! could not read {}", path.display());
        return;
    };
    for (from, to) in edits {
        if text.contains(from) {
            text = text.replacen(from, to, 1);
        } else {
            println!("  ! `{from}` not found in {}, skipped", path.display());
        }
    }
    let _ = fs::write(path, text);
}

/// pnpm 11 exits non-zero when a dependency's build script (esbuild) has not been approved,
/// after having installed everything. Approve them all, once, and finish the install.
pub(super) fn first_add(dir: &Path, args: &[&str]) -> Result<()> {
    if run(dir, "pnpm", args).is_ok() {
        return Ok(());
    }
    run(dir, "pnpm", &["approve-builds", "--all"])?;
    run(dir, "pnpm", &["install"])
}

/// Sets scripts in package.json directly. `npm pkg set` refuses to run once Vite+ has pinned
/// the package manager to pnpm, and `pnpm pkg` rejects names containing a colon.
pub(super) fn set_scripts(dir: &Path, scripts: &[(&str, &str)]) -> Result<()> {
    let path = dir.join("package.json");
    let mut pkg: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    let entry = pkg
        .as_object_mut()
        .context("package.json is not an object")?
        .entry("scripts")
        .or_insert_with(|| serde_json::json!({}));
    let table = entry.as_object_mut().context("scripts is not an object")?;
    for (name, command) in scripts {
        table.insert((*name).to_string(), serde_json::json!(command));
    }
    fs::write(&path, serde_json::to_string_pretty(&pkg)? + "\n")?;
    Ok(())
}

/// Drops a script. Vite+ adds `prepare: vp config` to install its own git hooks, which would
/// repoint `core.hooksPath` away from the root gitleaks hook and fails inside Docker.
pub(super) fn remove_script(dir: &Path, name: &str) -> Result<()> {
    let path = dir.join("package.json");
    let mut pkg: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    if let Some(table) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        table.shift_remove(name);
    }
    fs::write(&path, serde_json::to_string_pretty(&pkg)? + "\n")?;
    Ok(())
}

/// The package manager version Vite+ pins in package.json, if any.
pub(super) fn pinned_pnpm(dir: &Path) -> Option<String> {
    let pkg: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("package.json")).ok()?).ok()?;
    pkg["devEngines"]["packageManager"]["version"]
        .as_str()
        .map(str::to_string)
}
