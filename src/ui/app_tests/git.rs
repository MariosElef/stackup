use super::*;

/// A project that is a git repository, selected, with `machine` standing in for this computer.
fn app_with_git(test: &str, machine: Fake) -> (App, PathBuf) {
    let (mut app, root) = app_with(test, &[("demo", &[])]);
    let dir = root.join("demo");
    let ok = std::process::Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    app.machine = Box::new(machine);
    app.open_project();
    (app, dir)
}

#[test]
fn without_lazygit_it_says_how_to_get_it_and_stays_on_the_project() {
    let (mut app, _) = app_with_git("nolazygit", fake(&["git"]));
    app.begin_action(Action::GitOpen);
    assert_eq!(app.page, Page::Project);
    assert!(app.run.is_none());
    assert!(
        app.status.contains("lazygit is not installed")
            && app.status.contains("brew install lazygit"),
        "{}",
        app.status
    );
}

#[test]
fn lazygit_runs_inside_the_app_on_the_whole_page_and_quitting_it_comes_back() {
    // A program that stands in for lazygit: it shows something and quits.
    let bin = scratch("lazygit-bin");
    let stand_in = bin.join("lazygit");
    fs::write(
        &stand_in,
        "#!/bin/sh\nprintf 'lazygit is open here'\nsleep 1\n",
    )
    .unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&stand_in)
        .status()
        .unwrap();
    unsafe { env::set_var("STACKUP_LAZYGIT", &stand_in) };

    let (mut app, _) = app_with_git("lazygit", fake(&["git", "lazygit"]));
    app.begin_action(Action::GitOpen);
    assert_eq!(app.page, Page::Run, "it runs in the app's own terminal");
    let shown = screen(&app);
    assert!(
        !shown.contains("▶"),
        "the action list is out of the way: lazygit has the whole page\n{shown}"
    );

    // Quitting it needs no key: the project is back, with git read again.
    for _ in 0..300 {
        app.pump();
        if app.page == Page::Project {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        app.page,
        Page::Project,
        "lazygit quit, so the project is back"
    );
    assert!(app.run.is_none());
    unsafe { env::remove_var("STACKUP_LAZYGIT") };
}

#[test]
fn lazygit_is_in_the_list_under_git_and_the_groups_are_not_split() {
    let (mut app, _) = app_with_git("gitlist", fake(&["git", "lazygit"]));
    assert!(app.actions.contains(&Action::GitOpen));
    // The list is longer than the screen now: it scrolls to what is selected.
    app.action = app
        .actions
        .iter()
        .position(|a| *a == Action::GitOpen)
        .unwrap();
    let shown = screen(&app);
    assert!(
        shown.contains("Git") && shown.contains("lazygit"),
        "{shown}"
    );
    // Each section header appears once.
    for header in [
        "Run",
        "Docker",
        "Build & ship",
        "Environment",
        "Git",
        "Project",
    ] {
        let count = shown.matches(&format!("│{header}")).count();
        assert!(count <= 1, "{header} appears {count} times\n{shown}");
    }
}
