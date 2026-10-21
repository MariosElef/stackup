//! The menu: its pages, its prompts, and a real action running on the in-app terminal.
use std::{env, fs, path::PathBuf, thread, time::Duration};

use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

use super::{
    app::draw,
    features::create::use_requirements::TOOLS_TO_CHECK,
    stores::{
        app::{App, AutofixModal, InstallModal, Outcome},
        run::Run,
    },
    types::Page,
};
use crate::services::projects::urls::{Link, Urls};
use crate::{
    models::{Action, Frontend, Step},
    services::{
        projects::registry,
        toolchain::machine::{Fake, InstallPlan, fake},
    },
};

/// A folder of its own for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("stackup-app-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

/// A project folder holding `files`, with a compose file.
fn project_dir(root: &std::path::Path, name: &str, files: &[&str]) -> PathBuf {
    let dir = root.join(name);
    for f in files.iter().chain(&["docker-compose.yml"]) {
        let path = dir.join(f);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }
    dir
}

/// The steps that run stackup itself (`internal copy`) need the real program, not the test
/// harness this runs as: it is the one cargo built beside it, in `target/debug`.
fn use_the_built_program() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let me = env::current_exe().unwrap();
        let built = me
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(if cfg!(windows) {
                "stackup.exe"
            } else {
                "stackup"
            });
        // `cargo test` on a fresh checkout builds the test harness, not the program itself.
        if !built.is_file() {
            let ok = std::process::Command::new(env!("CARGO"))
                .args(["build", "--quiet", "--locked"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .status()
                .is_ok_and(|s| s.success());
            assert!(ok, "could not build the program the tests run");
        }
        assert!(built.is_file(), "{} is not built", built.display());
        unsafe { env::set_var("STACKUP_BIN", built) };
    });
}

/// An app whose list holds `projects`: (name, files in its folder). Nothing is installed.
fn app_with(test: &str, projects: &[(&str, &[&str])]) -> (App, PathBuf) {
    use_the_built_program();
    let root = scratch(test);
    let home = root.join("home");
    for (name, files) in projects {
        let dir = project_dir(&root, name, files);
        registry::add(&home, name, &dir).unwrap();
    }
    (App::new(home, vec![("git", true)]), root)
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

fn screen(app: &App) -> String {
    let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
    t.draw(|f| draw(f, app)).unwrap();
    t.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// Lets a running action work until it has ended.
fn finish(app: &mut App) -> String {
    for _ in 0..200 {
        app.pump();
        if let Some(how) = app.run.as_ref().and_then(|r| r.ended.clone()) {
            return how;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("the action did not end");
}

#[allow(unused_imports)]
use actions::*;
#[allow(unused_imports)]
use autofix::*;
#[allow(unused_imports)]
use create::*;
#[allow(unused_imports)]
use environments::*;
#[allow(unused_imports)]
use git::*;
#[allow(unused_imports)]
use home::*;
#[allow(unused_imports)]
use palette::*;
#[allow(unused_imports)]
use projects::*;
#[allow(unused_imports)]
use urls::*;
#[allow(unused_imports)]
use versions::*;

mod actions;
mod autofix;
mod create;
mod environments;
mod git;
mod home;
mod palette;
mod projects;
mod urls;
mod versions;
