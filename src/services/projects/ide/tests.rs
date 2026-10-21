use std::{env, path::PathBuf};

use serde_json::{Value, json};

use super::{
    jetbrains::{self, shell_quote},
    vscode::{extensions, vscode_launch, vscode_settings},
    *,
};
use crate::models::Facts;

fn facts(frontend: Frontend, backend: Backend) -> Facts {
    Facts {
        exists: true,
        frontend,
        backend,
        https: false,
        ci: false,
        hooks: false,
        git: false,
        remote: false,
        rust_build: false,
        has_volumes: false,
    }
}

fn scratch(test: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("stackup-ide-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn what_can_be_started_depends_on_what_was_built() {
    assert_eq!(
        parts(&facts(Frontend::Nuxt, Backend::Axum)),
        [Part::Backend, Part::Frontend, Part::Fullstack]
    );
    assert_eq!(
        parts(&facts(Frontend::None, Backend::Fastapi)),
        [Part::Backend]
    );
    assert_eq!(
        parts(&facts(Frontend::Vue, Backend::None)),
        [Part::Frontend]
    );
    assert!(parts(&facts(Frontend::None, Backend::None)).is_empty());
}

#[test]
fn a_jetbrains_configuration_runs_the_launcher_script_with_an_interpreter() {
    // The shape of mc2's configurations, which run in RustRover: a script file, an interpreter.
    let xml = jetbrains::config(Part::Fullstack, "/bin/sh");
    assert!(xml.contains(r#"name="Start fullstack""#), "{xml}");
    assert!(
        xml.contains(r#"name="SCRIPT_PATH" value="$PROJECT_DIR$/.idea/stackup-start.sh""#),
        "{xml}"
    );
    assert!(xml.contains(r#"name="SCRIPT_OPTIONS" value="start fullstack --path .""#));
    assert!(xml.contains(r#"name="INTERPRETER_PATH" value="/bin/sh""#));
    assert!(xml.contains(r#"name="EXECUTE_SCRIPT_FILE" value="true""#));
    assert!(
        xml.contains(r#"name="SCRIPT_TEXT" value="""#),
        "no inline script text"
    );
    assert!(
        xml.contains(r#"name="EXECUTE_IN_TERMINAL" value="false""#),
        "the Run window, so stop ends the tree"
    );
    assert!(xml.contains(r#"name="SCRIPT_WORKING_DIRECTORY" value="$PROJECT_DIR$""#));
}

#[test]
fn a_shell_path_with_special_characters_cannot_break_the_xml() {
    let xml = jetbrains::config(Part::Backend, r#"/tmp/a&b "c" <d>/sh"#);
    assert!(xml.contains("a&amp;b &quot;c&quot; &lt;d&gt;"), "{xml}");
    assert!(!xml.contains("a&b"));
}

#[test]
fn the_launcher_quotes_the_program_path_for_the_shell() {
    // A quote that would end the shell word is closed, escaped and reopened.
    assert_eq!(shell_quote("it's"), r"'it'\''s'");
    let script = jetbrains::launcher("/Users/me/my apps/stackup");
    assert!(script.starts_with("#!/bin/sh\n"));
    assert!(
        script.contains("exec '/Users/me/my apps/stackup' \"$@\""),
        "{script}"
    );
}

#[test]
fn only_the_parts_that_exist_get_a_configuration_in_both_ides() {
    let root = scratch("write");
    let wrote = write(
        &root,
        &facts(Frontend::Topcoat, Backend::None),
        Path::new("/x/stackup"),
    )
    .unwrap();
    assert!(
        root.join(".idea/runConfigurations/Start_frontend.xml")
            .exists()
    );
    assert!(
        !root
            .join(".idea/runConfigurations/Start_backend.xml")
            .exists()
    );
    assert!(
        !root
            .join(".idea/runConfigurations/Start_fullstack.xml")
            .exists()
    );
    let tasks: Value =
        serde_json::from_str(&fs::read_to_string(root.join(".vscode/tasks.json")).unwrap())
            .unwrap();
    assert_eq!(tasks["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(tasks["tasks"][0]["label"], "stackup: start frontend");
    assert_eq!(
        tasks["tasks"][0]["args"],
        json!(["start", "frontend", "--path", "${workspaceFolder}"])
    );
    assert_eq!(
        wrote,
        [
            "extensions (VS Code)",
            "settings (VS Code)",
            "Start frontend (JetBrains)",
            "tasks (VS Code)",
            "play button (VS Code)"
        ]
    );
    let launch: Value =
        serde_json::from_str(&fs::read_to_string(root.join(".vscode/launch.json")).unwrap())
            .unwrap();
    let entry = &launch["configurations"][0];
    assert_eq!(entry["name"], "stackup: start frontend");
    assert_eq!(entry["type"], "node-terminal");
    assert_eq!(
        entry["command"],
        "\"/x/stackup\" start frontend --path \"${workspaceFolder}\""
    );
}

#[test]
fn a_project_with_nothing_to_start_gets_nothing_written() {
    let root = scratch("none");
    assert!(
        write(
            &root,
            &facts(Frontend::None, Backend::None),
            Path::new("/x")
        )
        .unwrap()
        .is_empty()
    );
    assert!(!root.join(".idea").exists() && !root.join(".vscode").exists());
}

#[test]
fn a_vscode_tasks_file_of_your_own_is_left_alone_but_ours_is_refreshed() {
    let root = scratch("own");
    fs::create_dir_all(root.join(".vscode")).unwrap();
    fs::write(
        root.join(".vscode/tasks.json"),
        r#"{"tasks":[{"label":"mine"}]}"#,
    )
    .unwrap();
    let wrote = write(
        &root,
        &facts(Frontend::None, Backend::Axum),
        Path::new("/x/stackup"),
    )
    .unwrap();
    assert!(
        fs::read_to_string(root.join(".vscode/tasks.json"))
            .unwrap()
            .contains("mine")
    );
    assert!(!wrote.contains(&"tasks (VS Code)".to_string()));
    assert!(
        wrote.contains(&"play button (VS Code)".to_string()),
        "no launch.json of yours"
    );

    // A launch.json of your own is left alone too.
    fs::write(
        root.join(".vscode/launch.json"),
        r#"{"configurations":[{"name":"mine"}]}"#,
    )
    .unwrap();
    let wrote = write(
        &root,
        &facts(Frontend::None, Backend::Axum),
        Path::new("/x/stackup"),
    )
    .unwrap();
    assert!(!wrote.iter().any(|w| w.contains("VS Code")), "{wrote:?}");
    assert!(
        fs::read_to_string(root.join(".vscode/launch.json"))
            .unwrap()
            .contains("mine")
    );

    // Ours is rewritten, so a moved stackup or a new part is picked up.
    let other = scratch("ours");
    write(
        &other,
        &facts(Frontend::None, Backend::Axum),
        Path::new("/old/stackup"),
    )
    .unwrap();
    write(
        &other,
        &facts(Frontend::Nuxt, Backend::Axum),
        Path::new("/new/stackup"),
    )
    .unwrap();
    let t = fs::read_to_string(other.join(".vscode/tasks.json")).unwrap();
    assert!(
        t.contains("/new/stackup") && !t.contains("/old/stackup") && t.contains("start fullstack")
    );
}

#[cfg(unix)]
#[test]
fn the_launcher_really_runs_the_program_with_its_arguments_even_when_the_path_has_a_space() {
    let dir = scratch("runs").join("a folder");
    fs::create_dir_all(dir.join(".idea")).unwrap();
    let exe = dir.join("stackup");
    let out = dir.join("args.txt");
    fs::write(
        &exe,
        format!("#!/bin/sh\nprintf '%s|' \"$@\" > '{}'\n", out.display()),
    )
    .unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&exe)
        .status()
        .unwrap();
    // What `write` makes, run the way the IDE runs it: the interpreter, the script, the options.
    let wrote = write(&dir, &facts(Frontend::None, Backend::Axum), &exe).unwrap();
    assert!(
        wrote.iter().any(|w| w.contains("Start backend")),
        "{wrote:?}"
    );
    let launcher = dir.join(".idea/stackup-start.sh");
    let ok = std::process::Command::new("/bin/sh")
        .arg(&launcher)
        .args(["start", "backend", "--path", "."])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    assert_eq!(fs::read_to_string(out).unwrap(), "start|backend|--path|.|");
    // And the file really is runnable by itself.
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        fs::metadata(&launcher).unwrap().permissions().mode() & 0o111,
        0o111
    );
}

#[test]
fn each_stack_gets_the_language_servers_it_needs_and_nothing_for_what_it_does_not_use() {
    let nuxt_python = extensions(&facts(Frontend::Nuxt, Backend::Fastapi));
    for id in [
        "Vue.volar",
        "oxc.oxc-vscode",
        "ms-python.python",
        "charliermarsh.ruff",
    ] {
        assert!(nuxt_python.contains(&id), "{id}: {nuxt_python:?}");
    }
    assert!(
        !nuxt_python.contains(&"rust-lang.rust-analyzer"),
        "no Rust here"
    );

    let rust = extensions(&facts(Frontend::Topcoat, Backend::Axum));
    assert!(
        rust.contains(&"rust-lang.rust-analyzer") && rust.contains(&"tamasfe.even-better-toml")
    );
    assert!(!rust.contains(&"Vue.volar") && !rust.contains(&"ms-python.python"));
    assert!(
        rust.contains(&"ms-azuretools.vscode-docker"),
        "every project has a compose file"
    );
}

#[test]
fn extensions_and_settings_are_written_once_and_never_over_yours() {
    let root = scratch("editor");
    let f = facts(Frontend::Topcoat, Backend::Axum);
    write(&root, &f, Path::new("/x/stackup")).unwrap();
    let ext: Value =
        serde_json::from_str(&fs::read_to_string(root.join(".vscode/extensions.json")).unwrap())
            .unwrap();
    assert!(
        ext["recommendations"]
            .as_array()
            .unwrap()
            .contains(&json!("rust-lang.rust-analyzer"))
    );
    let set: Value =
        serde_json::from_str(&fs::read_to_string(root.join(".vscode/settings.json")).unwrap())
            .unwrap();
    assert_eq!(set["rust-analyzer.check.command"], "clippy");

    // Yours are left alone, on a second run too.
    fs::write(
        root.join(".vscode/extensions.json"),
        r#"{"recommendations":["mine.ext"]}"#,
    )
    .unwrap();
    fs::write(root.join(".vscode/settings.json"), r#"{"mine":true}"#).unwrap();
    let wrote = write(&root, &f, Path::new("/x/stackup")).unwrap();
    assert!(
        !wrote
            .iter()
            .any(|w| w.contains("extensions") || w.contains("settings")),
        "{wrote:?}"
    );
    assert!(
        fs::read_to_string(root.join(".vscode/extensions.json"))
            .unwrap()
            .contains("mine.ext")
    );
    assert!(
        fs::read_to_string(root.join(".vscode/settings.json"))
            .unwrap()
            .contains("mine")
    );
}

#[test]
fn python_settings_point_at_the_projects_own_interpreter_and_ruff() {
    let s = vscode_settings(&facts(Frontend::None, Backend::Fastapi), false);
    assert_eq!(
        s["python.defaultInterpreterPath"],
        "${workspaceFolder}/backend/.venv/bin/python"
    );
    assert_eq!(
        s["[python]"]["editor.defaultFormatter"],
        "charliermarsh.ruff"
    );
    assert!(
        vscode_settings(&facts(Frontend::None, Backend::None), false)
            .as_object()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn only_the_extensions_not_yet_installed_are_installed_and_case_does_not_matter() {
    let f = facts(Frontend::Topcoat, Backend::Axum);
    let all = install_steps(&f, Path::new("."), "");
    let ids: Vec<&str> = all.iter().map(|s| s.args[1].as_str()).collect();
    assert!(ids.contains(&"rust-lang.rust-analyzer"), "{ids:?}");
    assert!(
        all.iter()
            .all(|s| s.prog == "code" && s.args[0] == "--install-extension")
    );

    // `code --list-extensions` prints lowercase ids.
    let some = install_steps(
        &f,
        Path::new("."),
        "rust-lang.rust-analyzer\nms-azuretools.vscode-docker\n",
    );
    let ids: Vec<&str> = some.iter().map(|s| s.args[1].as_str()).collect();
    assert!(
        !ids.contains(&"rust-lang.rust-analyzer") && !ids.contains(&"ms-azuretools.vscode-docker")
    );
    assert!(ids.contains(&"tamasfe.even-better-toml"));
    assert!(install_steps(&f, Path::new("."), &extensions(&f).join("\n")).is_empty());
}

#[test]
fn on_windows_the_play_button_calls_the_program_the_powershell_way() {
    let win = vscode_launch(Part::Fullstack, r"C:\Users\me\.cargo\bin\stackup.exe", true);
    assert_eq!(
        win["command"],
        r#"& "C:\Users\me\.cargo\bin\stackup.exe" start fullstack --path "${workspaceFolder}""#
    );
    let unix = vscode_launch(Part::Fullstack, "/x/stackup", false);
    assert!(!unix["command"].as_str().unwrap().starts_with('&'));
    let py = vscode_settings(&facts(Frontend::None, Backend::Fastapi), true);
    assert_eq!(
        py["python.defaultInterpreterPath"],
        "${workspaceFolder}/backend/.venv/Scripts/python.exe"
    );
}

#[test]
fn jetbrains_is_told_which_shell_runs_the_launcher() {
    let xml = jetbrains::config(Part::Backend, r"C:\Program Files\Git\usr\bin\sh.exe");
    assert!(
        xml.contains(r#"name="INTERPRETER_PATH" value="C:\Program Files\Git\usr\bin\sh.exe""#),
        "{xml}"
    );
    // A Windows program path is single-quoted in the launcher, so its backslashes stay as they are.
    let script = jetbrains::launcher(r"C:\Users\me\stackup.exe");
    assert!(
        script.contains(r"exec 'C:\Users\me\stackup.exe' "),
        "{script}"
    );
}
