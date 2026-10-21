//! VS Code: the play button, the tasks, and the extensions that give each language its server.
use std::{fs, path::Path};

use anyhow::Result;
use serde_json::{Value, json};

use super::Part;
use crate::{
    clients::os,
    models::{Backend, Facts, Frontend, Step},
};

/// An entry for VS Code's Run and Debug list, which is what its play button runs. It starts the
/// command in the integrated terminal, so it needs no extension and stopping it stops the tree.
/// On Windows that terminal is PowerShell, where a quoted program needs the `&` call operator.
pub(super) fn vscode_launch(part: Part, exe: &str, windows: bool) -> Value {
    let call = if windows { "& " } else { "" };
    json!({
        "name": format!("stackup: {}", part.label().to_lowercase()),
        "type": "node-terminal",
        "request": "launch",
        "command": format!("{call}\"{exe}\" start {} --path \"${{workspaceFolder}}\"", part.cli()),
    })
}

pub(super) fn vscode_task(part: Part, exe: &str) -> Value {
    json!({
        "label": format!("stackup: {}", part.label().to_lowercase()),
        "type": "process",
        "command": exe,
        "args": ["start", part.cli(), "--path", "${workspaceFolder}"],
        "isBackground": true,
        "problemMatcher": [],
    })
}

/// The VS Code extensions that give a project's languages their language servers: VS Code has none
/// of its own for Rust or Vue. Only what the project is built with.
pub fn extensions(f: &Facts) -> Vec<&'static str> {
    let mut out = vec![];
    let rust = f.backend == Backend::Axum || f.frontend == Frontend::Topcoat;
    if rust {
        // rust-analyzer brings its own server; it needs a Rust toolchain on the machine.
        out.extend(["rust-lang.rust-analyzer", "tamasfe.even-better-toml"]);
    }
    if matches!(f.frontend, Frontend::Nuxt | Frontend::Vue) {
        // Volar is the Vue language server (it drives TypeScript inside .vue files too).
        out.extend(["Vue.volar", "oxc.oxc-vscode", "dbaeumer.vscode-eslint"]);
    }
    if f.backend == Backend::Fastapi {
        out.extend([
            "ms-python.python",
            "ms-python.vscode-pylance",
            "charliermarsh.ruff",
        ]);
    }
    if f.exists {
        out.extend(["ms-azuretools.vscode-docker", "redhat.vscode-yaml"]);
    }
    if f.https {
        out.push("matthewpi.caddyfile-support");
    }
    out
}

/// The steps that install the recommended extensions this machine does not have yet, with VS
/// Code's own `code` command. `installed` is what `code --list-extensions` printed: ids are
/// compared without regard to case, as VS Code does.
pub fn install_steps(f: &Facts, root: &Path, installed: &str) -> Vec<Step> {
    let have: Vec<String> = installed
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect();
    extensions(f)
        .into_iter()
        .filter(|id| !have.contains(&id.to_lowercase()))
        .map(|id| {
            Step::new(
                &format!("install the {id} extension"),
                root,
                "code",
                &["--install-extension", id],
            )
        })
        .collect()
}

/// Settings that make those extensions behave: Rust checked with clippy and formatted on save,
/// Python with Ruff, and TypeScript asked to use the project's own.
pub(super) fn vscode_settings(f: &Facts, windows: bool) -> Value {
    let mut s = serde_json::Map::new();
    if f.backend == Backend::Axum || f.frontend == Frontend::Topcoat {
        s.insert("rust-analyzer.check.command".into(), json!("clippy"));
        s.insert(
            "[rust]".into(),
            json!({"editor.defaultFormatter": "rust-lang.rust-analyzer", "editor.formatOnSave": true}),
        );
        s.insert(
            "[toml]".into(),
            json!({"editor.defaultFormatter": "tamasfe.even-better-toml"}),
        );
    }
    if matches!(f.frontend, Frontend::Nuxt | Frontend::Vue) {
        s.insert(
            "typescript.enablePromptUseWorkspaceTsdk".into(),
            json!(true),
        );
    }
    if f.backend == Backend::Fastapi {
        s.insert(
            "python.defaultInterpreterPath".into(),
            // A virtual environment keeps its programs in `Scripts` on Windows, `bin` elsewhere.
            json!(if windows {
                "${workspaceFolder}/backend/.venv/Scripts/python.exe"
            } else {
                "${workspaceFolder}/backend/.venv/bin/python"
            }),
        );
        s.insert(
            "[python]".into(),
            json!({
                "editor.defaultFormatter": "charliermarsh.ruff",
                "editor.formatOnSave": true,
                "editor.codeActionsOnSave": {"source.fixAll": "explicit", "source.organizeImports": "explicit"}
            }),
        );
    }
    Value::Object(s)
}

/// `.vscode/extensions.json` and `settings.json`, each only when there is none: a file of yours is
/// never touched, and these two are meant to be committed, so the whole team is asked for the
/// same extensions.
pub(super) fn write_editor_support(root: &Path, f: &Facts, wrote: &mut Vec<String>) -> Result<()> {
    let list = extensions(f);
    if list.is_empty() {
        return Ok(());
    }
    let dir = root.join(".vscode");
    let extensions = dir.join("extensions.json");
    if !extensions.exists() {
        fs::create_dir_all(&dir)?;
        let body = json!({"recommendations": list});
        fs::write(&extensions, serde_json::to_string_pretty(&body)? + "\n")?;
        wrote.push("extensions (VS Code)".to_string());
    }
    let settings = dir.join("settings.json");
    if !settings.exists() {
        let body = vscode_settings(f, os::WINDOWS);
        if body.as_object().is_some_and(|m| !m.is_empty()) {
            fs::create_dir_all(&dir)?;
            fs::write(&settings, serde_json::to_string_pretty(&body)? + "\n")?;
            wrote.push("settings (VS Code)".to_string());
        }
    }
    Ok(())
}
