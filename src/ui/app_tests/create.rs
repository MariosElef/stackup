use super::*;
use crate::services::toolchain::github::Github;

#[test]
fn start_asks_for_a_name_then_opens_the_wizard() {
    let (mut app, root) = app_with("start", &[]);
    press(&mut app, KeyCode::Char('s'));
    assert!(screen(&app).contains("Project name"));
    type_text(&mut app, "my app");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::New);
    let s = screen(&app);
    assert!(
        s.contains("stackup · my app") && s.contains("Frontend"),
        "{s}"
    );

    // Leaving the wizard goes back to the menu, not out of the program.
    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.page, Page::Home);
    assert_eq!(app.outcome, Outcome::Running);
    assert!(app.wizard.is_none());
    drop(root);
}

#[test]
fn a_name_that_is_taken_or_empty_is_refused() {
    let (mut app, _) = app_with("names", &[]);
    press(&mut app, KeyCode::Char('s'));
    type_text(&mut app, "!!");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Home);
    assert!(app.status.contains("letters or digits"));

    press(&mut app, KeyCode::Char('s'));
    type_text(&mut app, "src");
    press(&mut app, KeyCode::Enter);
    assert!(
        app.status.contains("already exists"),
        "src is a folder in the project root"
    );
    press(&mut app, KeyCode::Char('s'));
    press(&mut app, KeyCode::Esc);
    assert!(app.prompt.is_none(), "Escape drops a prompt");
}

/// An app at the wizard whose tools are all installed, with `exe` standing in for stackup.
pub(super) fn app_ready_to_create(test: &str, exe: &str, name: &str) -> App {
    let (mut app, _) = app_with(test, &[]);
    app.exe = PathBuf::from(exe);
    // Not this computer: whether `gh` is signed in here must not change what a test does.
    app.machine = Box::new(fake(&[]));
    press(&mut app, KeyCode::Char('s'));
    type_text(&mut app, name);
    press(&mut app, KeyCode::Enter);
    app.wizard.as_mut().unwrap().installed = TOOLS_TO_CHECK.iter().map(|b| (*b, true)).collect();
    app
}

/// Takes the recommended version for every question Create asks.
pub(super) fn accept_versions(app: &mut App) {
    while app.choice.is_some() {
        press(app, KeyCode::Enter);
    }
}

#[test]
fn create_asks_which_version_of_each_tool_and_service_before_making_anything() {
    let mut app = app_ready_to_create("createversions", "/bin/echo", "made-with-versions");
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.page,
        Page::New,
        "nothing runs before the questions are answered"
    );
    assert!(app.run.is_none());

    // Nuxt + axum + Postgres + Redis: Rust first, then each service.
    assert!(
        screen(&app).contains("Which Rust for the project?"),
        "{}",
        screen(&app)
    );
    press(&mut app, KeyCode::Enter);
    assert!(screen(&app).contains("Which Postgres?"));
    assert!(screen(&app).contains("latest (18)") && screen(&app).contains("17"));
    press(&mut app, KeyCode::Char('2'));
    press(&mut app, KeyCode::Enter);
    assert!(screen(&app).contains("Which Redis?"));
    press(&mut app, KeyCode::Enter);
    assert!(app.choice.is_none());

    assert_eq!(finish(&mut app), "done");
    let said = app.run.as_ref().unwrap().log_text();
    assert!(said.contains("--pin postgres=17"), "{said}");
    assert!(said.contains("--pin redis=latest"), "{said}");
    assert!(said.contains("--pin rust="), "{said}");
}

#[test]
fn closing_the_version_question_goes_back_to_the_wizard_and_makes_nothing() {
    let mut app = app_ready_to_create("createversionsesc", "/bin/echo", "never-made");
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    assert!(app.choice.is_some());
    press(&mut app, KeyCode::Esc);
    assert!(app.choice.is_none() && app.run.is_none());
    assert_eq!(app.page, Page::New);
}

#[test]
fn creating_in_the_wizard_runs_on_the_in_app_terminal_instead_of_leaving_the_app() {
    // `echo` stands in for stackup: it prints what it was asked to make and succeeds.
    let mut app = app_ready_to_create("create", "/bin/echo", "made-by-echo");
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    accept_versions(&mut app);
    assert_eq!(app.outcome, Outcome::Running, "the app is still open");
    assert_eq!(app.page, Page::Install);
    assert_eq!(app.creating.as_deref(), Some("made-by-echo"));

    assert_eq!(finish(&mut app), "done");
    // What the program was asked to make, as it printed it: the screen wraps long lines.
    let said = app.run.as_ref().unwrap().log_text();
    assert!(
        said.contains("new made-by-echo --frontend nuxt --backend axum"),
        "{said}"
    );
    assert!(said.contains("--services postgres,redis"), "{said}");
    assert!(
        screen(&app).contains("create made-by-echo"),
        "the panel is titled by what it makes"
    );
}

#[test]
fn after_a_creation_you_are_on_the_projects_page_with_the_new_project_listed() {
    let mut app = app_ready_to_create("created", "/bin/echo", "made-and-listed");
    // What `stackup new` does at its end: the project exists and is on the list.
    let dir = std::env::current_dir().unwrap().join("made-and-listed");
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    accept_versions(&mut app);
    assert_eq!(finish(&mut app), "done");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("docker-compose.yml"), "").unwrap();
    registry::add(&app.dir, "made-and-listed", &dir.canonicalize().unwrap()).unwrap();

    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.page, Page::Projects);
    assert!(app.wizard.is_none() && app.creating.is_none());
    assert!(
        app.status.contains("made made-and-listed"),
        "{}",
        app.status
    );
    assert_eq!(
        app.project().map(|p| p.name.as_str()),
        Some("made-and-listed"),
        "the new one is selected"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_creation_that_fails_goes_back_to_the_wizard_with_the_choices_kept() {
    let mut app = app_ready_to_create("createfail", "/usr/bin/false", "made-and-failed");
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    accept_versions(&mut app);
    assert_eq!(finish(&mut app), "failed (exit 1)");
    assert!(
        app.autofix.is_none(),
        "a half-made project is not retried by an autofix"
    );
    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.page, Page::New);
    let wizard = app.wizard.as_ref().expect("the wizard is still there");
    assert!(
        wizard.status.contains("made-and-failed was not made"),
        "{}",
        wizard.status
    );
    assert_eq!(
        wizard.sel.frontend,
        Frontend::Nuxt,
        "the choices are as they were"
    );
}

#[test]
fn the_wizard_run_by_stackup_new_still_hands_its_choices_back() {
    let root = scratch("createwizonly");
    let mut app = App::new(root.join("home"), vec![("git", true)]);
    app.wizard_only = true;
    app.open_wizard("solo");
    app.wizard.as_mut().unwrap().installed = TOOLS_TO_CHECK.iter().map(|b| (*b, true)).collect();
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.outcome,
        Outcome::Start {
            name: "solo".into()
        }
    );
    assert!(
        app.run.is_none(),
        "the command line makes it in its own terminal"
    );
}

#[test]
fn the_wizard_alone_leaves_the_app_when_it_is_left() {
    let root = scratch("wizonly");
    let mut app = App::new(root.join("home"), vec![("git", true)]);
    app.wizard_only = true;
    app.open_wizard("solo");
    assert_eq!(app.page, Page::New);
    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.outcome, Outcome::Quit);
}

/// The wizard page of an app, for a project that needs Topcoat, with the CLI "not installed".
pub(super) fn app_at_the_wizard(test: &str) -> App {
    let (mut app, _) = app_with(test, &[]);
    press(&mut app, KeyCode::Char('s'));
    type_text(&mut app, "demo");
    press(&mut app, KeyCode::Enter);
    // Nothing installed here: no version to ask about, and no real command can be planned.
    app.machine = Box::new(fake(&[]));
    let wizard = app.wizard.as_mut().unwrap();
    wizard.sel.frontend = Frontend::Topcoat;
    wizard.installed = vec![
        ("docker", true),
        ("cargo", true),
        ("topcoat", false),
        ("git", true),
        ("gh", true),
    ];
    app
}

/// A modal offering `steps` for topcoat, with nothing really installed.
pub(super) fn offer(app: &mut App, steps: Vec<Step>, by_hand: Vec<&'static str>) {
    let can = !steps.is_empty();
    app.install = Some(InstallModal {
        missing: vec!["topcoat"],
        plan: InstallPlan { steps, by_hand },
        selected: usize::from(!can),
    });
}

pub(super) fn echo(dir: &std::path::Path, script: &str) -> Step {
    Step::new("install topcoat", dir, "sh", &["-c", script])
}

#[test]
fn pressing_create_with_a_tool_missing_asks_to_install_it_or_close() {
    let mut app = app_at_the_wizard("ask");
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    let modal = app.install.as_ref().expect("the question is open");
    assert_eq!(modal.missing, ["topcoat"]);
    assert_eq!(app.outcome, Outcome::Running, "nothing was created");
    let s = screen(&app);
    assert!(
        s.contains("To create this project you need to install"),
        "{s}"
    );
    assert!(s.contains("topcoat") && s.contains("for Topcoat projects"));
    assert!(s.contains("Close"), "{s}");
    assert!(s.contains("INSTALL"), "the status bar says what is open");
}

#[test]
fn close_leaves_everything_as_it_was() {
    let mut app = app_at_the_wizard("close");
    let dir = std::env::temp_dir();
    offer(&mut app, vec![echo(&dir, "echo never")], vec![]);
    let s = screen(&app);
    assert!(
        s.contains("Install") && s.contains("Close") && s.contains("will run:"),
        "{s}"
    );

    // Install is the default; moving to Close and confirming (or `c`) leaves it alone.
    press(&mut app, KeyCode::Right);
    assert_eq!(app.install.as_ref().unwrap().selected, 1);
    press(&mut app, KeyCode::Enter);
    assert!(app.install.is_none() && app.run.is_none());
    assert_eq!(app.page, Page::New, "still in the wizard");

    offer(&mut app, vec![echo(&dir, "echo never")], vec![]);
    press(&mut app, KeyCode::Char('c'));
    assert!(app.install.is_none() && app.run.is_none());
    offer(&mut app, vec![echo(&dir, "echo never")], vec![]);
    press(&mut app, KeyCode::Esc);
    assert!(app.install.is_none() && app.run.is_none());
}

#[test]
fn install_runs_on_the_terminal_then_returns_to_the_wizard() {
    let mut app = app_at_the_wizard("install");
    let dir = scratch("install-dir");
    offer(
        &mut app,
        vec![echo(
            &dir,
            "echo installing-topcoat > installed.txt; echo installing-topcoat",
        )],
        vec![],
    );
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Install);
    assert!(app.install.is_none());

    assert_eq!(finish(&mut app), "done");
    let s = screen(&app);
    assert!(
        s.contains("install topcoat") && s.contains("installing-topcoat"),
        "{s}"
    );
    assert!(dir.join("installed.txt").exists(), "the step really ran");

    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.page, Page::New);
    assert!(app.run.is_none());
    let status = &app.wizard.as_ref().unwrap().status;
    assert!(status.contains("press Create project again"), "{status}");
}

#[test]
fn a_finished_install_celebrates_then_goes_back_on_its_own() {
    let mut app = app_at_the_wizard("celebrate");
    let dir = scratch("celebrate-dir");
    offer(&mut app, vec![echo(&dir, "echo ok")], vec![]);
    press(&mut app, KeyCode::Enter);
    assert_eq!(finish(&mut app), "done");
    app.pump();
    assert!(
        app.celebration.is_some(),
        "a finished install is celebrated"
    );
    // Once the check has drawn itself, the words come in.
    thread::sleep(Duration::from_millis(700));
    let s = screen(&app);
    assert!(
        s.contains("topcoat installed") && s.contains("back to the wizard"),
        "{s}"
    );

    // It ends by itself, and the wizard is back as `q` would bring it.
    for _ in 0..150 {
        app.pump();
        if app.celebration.is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(app.celebration.is_none());
    assert_eq!(app.page, Page::New);
    let status = &app.wizard.as_ref().unwrap().status;
    assert!(status.contains("press Create project again"), "{status}");

    // A failure is not celebrated.
    offer(&mut app, vec![echo(&dir, "exit 3")], vec![]);
    press(&mut app, KeyCode::Enter);
    assert_eq!(finish(&mut app), "failed (exit 3)");
    app.pump();
    assert!(app.celebration.is_none());
}

#[test]
fn a_failed_install_says_so_and_changes_nothing() {
    let mut app = app_at_the_wizard("failed");
    let dir = scratch("failed-dir");
    offer(&mut app, vec![echo(&dir, "exit 2")], vec![]);
    press(&mut app, KeyCode::Char('i'));
    assert_eq!(finish(&mut app), "failed (exit 2)");
    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.page, Page::New);
    assert!(
        app.wizard
            .as_ref()
            .unwrap()
            .status
            .contains("did not finish")
    );
    assert_eq!(app.outcome, Outcome::Running);
}

#[test]
fn what_cannot_be_installed_from_here_is_explained_and_only_offers_close() {
    let mut app = app_at_the_wizard("byhand");
    offer(&mut app, vec![], vec!["topcoat"]);
    let s = screen(&app);
    assert!(s.contains("by hand:") && s.contains("Close"), "{s}");
    assert!(
        !s.contains("Install "),
        "no Install button when there is nothing to run"
    );
    press(&mut app, KeyCode::Char('i'));
    assert!(app.install.is_some(), "`i` does nothing without an install");
    press(&mut app, KeyCode::Enter);
    assert!(app.install.is_none() && app.run.is_none());
}

#[test]
fn while_the_question_is_open_the_wizard_gets_no_keys() {
    let mut app = app_at_the_wizard("swallow");
    let before = app.wizard.as_ref().unwrap().cursor;
    offer(
        &mut app,
        vec![echo(&std::env::temp_dir(), "echo x")],
        vec![],
    );
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(app.wizard.as_ref().unwrap().cursor, before);
    assert!(app.install.is_some());
}

pub(super) const OLD_RUST: &str = "error: cannot install package `topcoat-cli 0.10.0`, it requires rustc 1.99 or newer, while the currently active rustc version is 1.97.1";

/// An app at the wizard, ready to create, on a machine that has `gh` but nothing set up in it.
fn app_needing_github(test: &str, github: Github) -> App {
    let mut app = app_ready_to_create(test, "/bin/echo", "to-github");
    app.machine = Box::new(Fake {
        github,
        ..fake(&["gh", "git", "ssh-keygen"])
    });
    app
}

#[test]
fn create_offers_to_set_up_github_when_gh_is_not_signed_in_and_there_is_no_key() {
    let mut app = app_needing_github("ghoffer", Github::default());
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    assert!(
        app.github.is_some(),
        "the modal opens before anything is made"
    );
    assert!(app.run.is_none() && app.choice.is_none());
    let shown = screen(&app);
    for text in [
        "set up GitHub",
        "the GitHub CLI signed in",
        "an SSH key",
        "create an SSH key",
        "sign in to GitHub",
        "give GitHub the key",
        "Set up",
        "Skip",
    ] {
        assert!(shown.contains(text), "{text} missing:\n{shown}");
    }
}

#[test]
fn nothing_is_asked_when_github_is_already_set_up_or_gh_is_not_chosen() {
    let ready = Github {
        signed_in: true,
        key: Some(PathBuf::from("/h/.ssh/id_ed25519")),
    };
    let mut app = app_needing_github("ghready", ready);
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    assert!(app.github.is_none(), "already set up: straight on");
    assert!(app.choice.is_some(), "to the version questions");

    // No git tool, no GitHub.
    let mut app = app_needing_github("ghnogit", Github::default());
    app.wizard
        .as_mut()
        .unwrap()
        .sel
        .tools
        .retain(|t| *t != crate::models::Tool::Git);
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    assert!(app.github.is_none());
}

#[test]
fn skipping_goes_on_to_make_the_project_and_is_not_asked_again() {
    let mut app = app_needing_github("ghskip", Github::default());
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('l'));
    press(&mut app, KeyCode::Enter);
    assert!(app.github.is_none() && app.github_skipped);
    assert!(app.choice.is_some(), "carries on to the version questions");
    accept_versions(&mut app);
    assert_eq!(finish(&mut app), "done", "the project is made without it");
}

#[test]
fn set_up_runs_on_the_terminal_here_then_returns_to_the_wizard_to_create() {
    let mut app = app_needing_github("ghrun", Github::default());
    type_text(&mut app, ":wq");
    press(&mut app, KeyCode::Enter);
    // Stand-ins for the real steps: nothing here may touch ~/.ssh, ssh-keygen or GitHub.
    let dir = std::env::temp_dir();
    if let Some(modal) = &mut app.github {
        modal.steps = vec![echo(&dir, "echo setting up")];
    }
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Install, "it runs in the app's own terminal");
    assert!(app.github_run);
    assert_eq!(finish(&mut app), "done");
    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.page, Page::New, "back to the wizard");
    assert!(
        app.wizard
            .as_ref()
            .unwrap()
            .status
            .contains("GitHub is set up"),
        "{}",
        app.wizard.as_ref().unwrap().status
    );
    assert!(!app.github_run);
}
