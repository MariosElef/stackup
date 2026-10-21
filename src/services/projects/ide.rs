//! The run configurations that make "start backend", "start frontend" and "start fullstack" one
//! click in an IDE, for what the project actually has. Modelled on mc2's: a configuration that
//! runs this program by its full path (an IDE started from the dock has no shell PATH), so it
//! holds no values of its own.
mod jetbrains;
#[cfg(test)]
mod tests;
mod vscode;

pub use vscode::install_steps;
use vscode::{vscode_launch, vscode_task, write_editor_support};

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::json;

use crate::{
    clients::os,
    models::{Backend, Facts, Frontend},
};

/// Which part of a project can be started on its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Backend,
    Frontend,
    Fullstack,
}

impl Part {
    /// What it is called on the command line: `stackup start backend`.
    pub fn cli(self) -> &'static str {
        match self {
            Part::Backend => "backend",
            Part::Frontend => "frontend",
            Part::Fullstack => "fullstack",
        }
    }

    pub fn parse(text: &str) -> Result<Part> {
        match text {
            "backend" => Ok(Part::Backend),
            "frontend" => Ok(Part::Frontend),
            "fullstack" => Ok(Part::Fullstack),
            other => anyhow::bail!("`{other}` is not a part: use backend, frontend or fullstack"),
        }
    }

    pub fn label(self) -> String {
        format!("Start {}", self.cli())
    }
}

/// The parts this project has: each of the two, and both together when it has both.
pub fn parts(f: &Facts) -> Vec<Part> {
    let (back, front) = (f.backend != Backend::None, f.frontend != Frontend::None);
    let mut out = vec![];
    if back {
        out.push(Part::Backend);
    }
    if front {
        out.push(Part::Frontend);
    }
    if back && front {
        out.push(Part::Fullstack);
    }
    out
}

/// Writes the run configurations for the parts `f` has. Returns what was written, to say so.
pub fn write(root: &Path, f: &Facts, exe: &Path) -> Result<Vec<String>> {
    let exe = exe.display().to_string();
    let parts = parts(f);
    if parts.is_empty() {
        return Ok(vec![]);
    }
    let mut wrote = vec![];
    write_editor_support(root, f, &mut wrote)?;
    // The shell JetBrains runs the command through: `sh`, or Git for Windows' on Windows.
    let shell = if os::WINDOWS {
        os::shell().display().to_string()
    } else {
        "/bin/sh".to_string()
    };

    let runs = root.join(".idea/runConfigurations");
    fs::create_dir_all(&runs)?;
    // The script the configurations run: the IDE runs script files, not programs.
    let launcher = root.join(jetbrains::LAUNCHER);
    fs::write(&launcher, jetbrains::launcher(&exe))
        .with_context(|| format!("writing {}", launcher.display()))?;
    os::make_executable(&launcher)?;
    for part in &parts {
        let file = runs.join(format!("Start_{}.xml", part.cli()));
        fs::write(&file, jetbrains::config(*part, &shell))
            .with_context(|| format!("writing {}", file.display()))?;
        wrote.push(format!("{} (JetBrains)", part.label()));
    }

    // VS Code keeps all tasks in one file. A file that is the person's own is left alone.
    let tasks = root.join(".vscode/tasks.json");
    let ours = fs::read_to_string(&tasks).map_or(true, |t| t.contains("stackup: start"));
    if ours {
        fs::create_dir_all(root.join(".vscode"))?;
        let body = json!({
            "version": "2.0.0",
            "tasks": parts.iter().map(|p| vscode_task(*p, &exe)).collect::<Vec<_>>(),
        });
        fs::write(&tasks, serde_json::to_string_pretty(&body)? + "\n")?;
        wrote.push("tasks (VS Code)".to_string());
    }
    // The play button: Run and Debug's list. Same rule: one of your own is left alone.
    let launch = root.join(".vscode/launch.json");
    let ours = fs::read_to_string(&launch).map_or(true, |t| t.contains("stackup: start"));
    if ours {
        fs::create_dir_all(root.join(".vscode"))?;
        let body = json!({
            "version": "0.2.0",
            "configurations": parts.iter().map(|p| vscode_launch(*p, &exe, os::WINDOWS)).collect::<Vec<_>>(),
        });
        fs::write(&launch, serde_json::to_string_pretty(&body)? + "\n")?;
        wrote.push("play button (VS Code)".to_string());
    }
    Ok(wrote)
}
