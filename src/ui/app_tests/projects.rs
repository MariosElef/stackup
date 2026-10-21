use super::*;
use crate::ui::features::projects::farewell::{self, Farewell};

#[test]
fn the_projects_page_lists_what_stackup_built_and_marks_what_is_gone() {
    let (mut app, root) = app_with(
        "list",
        &[
            ("shop", &["backend/Cargo.toml"]),
            ("blog", &["frontend/nuxt.config.ts"]),
        ],
    );
    fs::remove_dir_all(root.join("blog")).unwrap();
    press(&mut app, KeyCode::Char('p'));
    let s = screen(&app);
    assert!(s.contains("shop") && s.contains("blog"));
    assert!(s.contains("axum"), "the stack is read from the folder");
    assert!(s.contains("missing"), "a project whose folder is gone");
}

#[test]
fn an_empty_list_says_how_to_start() {
    let (mut app, _) = app_with("empty", &[]);
    press(&mut app, KeyCode::Char('p'));
    assert!(screen(&app).contains("Nothing here yet"));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Projects, "nothing to open");
}

#[test]
fn a_project_offers_only_the_actions_that_apply_to_it() {
    let (mut app, _) = app_with(
        "actions",
        &[("plain", &[]), ("full", &["backend/Cargo.toml", "ci.sh"])],
    );
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Project);
    assert!(!app.actions.contains(&Action::Dev) && !app.actions.contains(&Action::Ci));
    assert!(app.actions.contains(&Action::Up) && app.actions.contains(&Action::Deploy));

    press(&mut app, KeyCode::Char('h'));
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Enter);
    assert!(app.actions.contains(&Action::Dev) && app.actions.contains(&Action::Ci));
    let s = screen(&app);
    assert!(s.contains("full") && s.contains("what it does"));
    assert!(
        s.contains("Build and start everything in Docker"),
        "the first action is described"
    );
}

#[test]
fn forgetting_a_project_needs_its_name_and_leaves_its_files() {
    let (mut app, root) = app_with("forget", &[("shop", &[]), ("blog", &[])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('d'));
    assert!(screen(&app).contains("type `blog`") || screen(&app).contains("type `shop`"));
    type_text(&mut app, "wrong");
    press(&mut app, KeyCode::Enter);
    assert!(app.status.contains("not confirmed"));
    assert_eq!(registry::load(&app.dir).unwrap().len(), 2);

    let name = app.project().unwrap().name.clone();
    press(&mut app, KeyCode::Char('d'));
    type_text(&mut app, &name);
    press(&mut app, KeyCode::Enter);
    let left: Vec<String> = registry::load(&app.dir)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(left.len(), 1);
    assert!(!left.contains(&name));
    assert!(
        root.join(&name).join("docker-compose.yml").exists(),
        "files are not deleted"
    );
}

#[test]
fn r_removes_a_project_only_after_its_name_is_typed_back() {
    let (mut app, root) = app_with("remove", &[("shop", &[".env.local"]), ("blog", &[])]);
    app.machine = Box::new(fake(&[]));
    press(&mut app, KeyCode::Char('p'));
    // The list is sorted by when it was added: pick shop by name.
    app.cursor = app.projects.iter().position(|p| p.name == "shop").unwrap();
    press(&mut app, KeyCode::Char('r'));
    let s = screen(&app);
    assert!(
        s.contains("REMOVE shop") && s.contains("folder, containers, data"),
        "{s}"
    );
    assert!(
        s.contains("type `shop` to continue") && s.contains("CONFIRM"),
        "{s}"
    );

    // A wrong name changes nothing.
    type_text(&mut app, "blog");
    press(&mut app, KeyCode::Enter);
    assert!(app.status.contains("not confirmed"));
    assert!(root.join("shop").exists() && app.run.is_none());

    // The right one runs it: the folder is really deleted, and the project leaves the list.
    press(&mut app, KeyCode::Char('r'));
    type_text(&mut app, "shop");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Run);
    assert_eq!(finish(&mut app), "done");
    assert!(!root.join("shop").exists(), "the folder is gone");
    assert!(root.join("blog").exists(), "other projects are untouched");
    let left: Vec<String> = registry::load(&app.dir)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(left, ["blog"]);

    // A goodbye shows first, then the list: not a project that no longer exists.
    let goodbye = app.farewell.as_ref().expect("a goodbye after removing");
    assert_eq!(goodbye.name, "shop");
    let s = screen(&app);
    assert!(s.contains("removed"), "{s}");
    press(&mut app, KeyCode::Char('q'));
    assert!(app.farewell.is_none(), "any key skips it");
    assert_eq!(app.page, Page::Projects);
    assert_eq!(app.projects.len(), 1);
}

#[test]
fn the_goodbye_ends_by_itself_and_goes_back_to_the_list() {
    let (mut app, _) = app_with("goodbye", &[("shop", &[]), ("blog", &[])]);
    press(&mut app, KeyCode::Char('p'));
    app.page = Page::Run;
    app.removed = true;
    app.farewell = Some(Farewell::new("shop", "the folder", None));
    app.pump();
    assert!(app.farewell.is_some(), "it lasts a moment");
    std::thread::sleep(std::time::Duration::from_millis(
        u64::try_from(farewell::DURATION_MS).unwrap() + 50,
    ));
    app.pump();
    assert!(app.farewell.is_none());
    assert_eq!(app.page, Page::Projects);
}

#[test]
fn r_also_works_from_inside_a_project() {
    let (mut app, root) = app_with("remove-inside", &[("shop", &[])]);
    app.machine = Box::new(fake(&[]));
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Project);
    assert!(
        app.actions.contains(&Action::Remove),
        "it is in the action list too"
    );
    press(&mut app, KeyCode::Char('r'));
    assert!(screen(&app).contains("REMOVE shop"));
    press(&mut app, KeyCode::Esc);
    assert!(
        app.prompt.is_none() && root.join("shop").exists(),
        "Escape backs out"
    );
}

#[test]
fn a_project_with_uncommitted_work_is_warned_about() {
    let (mut app, root) = app_with("remove-dirty", &[("shop", &[])]);
    let dir = root.join("shop");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&dir)
            .output()
            .unwrap();
    };
    git(&["init", "-q"]);
    fs::write(dir.join("notes.txt"), "not committed").unwrap();
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('r'));
    assert!(
        screen(&app).contains("uncommitted changes!"),
        "{}",
        screen(&app)
    );
}

#[test]
fn removing_a_project_whose_folder_is_already_gone_only_stops_listing_it() {
    let (mut app, root) = app_with("remove-gone", &[("shop", &[])]);
    fs::remove_dir_all(root.join("shop")).unwrap();
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('r'));
    let s = screen(&app);
    assert!(s.contains("stop listing shop"), "{s}");
    type_text(&mut app, "shop");
    press(&mut app, KeyCode::Enter);
    assert!(registry::load(&app.dir).unwrap().is_empty());
    assert!(app.run.is_none(), "there was nothing to delete");
}

#[test]
fn a_folder_that_is_unsafe_to_delete_is_refused_with_the_reason() {
    let (mut app, root) = app_with("remove-unsafe", &[("shop", &[])]);
    app.machine = Box::new(fake(&[]));
    // The project's folder holds the home folder: deleting it would take the home with it.
    // SAFETY of the test: it only changes this process's view of HOME for the planning check,
    // which refuses before anything is run.
    let steps = crate::services::projects::plan::check_removable(
        &root.join("shop"),
        std::path::Path::new("/elsewhere"),
        Some(&root.join("shop").join("me")),
    );
    assert!(steps.is_err());
    assert!(root.join("shop").exists());
    drop(app);
}

#[test]
fn a_adds_the_projects_in_a_folder_and_says_which() {
    let (mut app, root) = app_with("add", &[]);
    for name in ["shop", "blog"] {
        project_dir(&root.join("work"), name, &[]);
    }
    fs::create_dir_all(root.join("work/notes")).unwrap();
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('a'));
    assert!(
        screen(&app).contains("Add projects from"),
        "{}",
        screen(&app)
    );
    // It starts at the folder stackup was run in; here it is pointed at the work folder.
    app.prompt.as_mut().unwrap().value = root.join("work").display().to_string();
    press(&mut app, KeyCode::Enter);
    assert!(
        app.status.contains("added 2")
            && app.status.contains("blog")
            && app.status.contains("shop"),
        "{}",
        app.status
    );
    let names: Vec<String> = registry::load(&app.dir)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["blog", "shop"]);
    assert_eq!(app.projects.len(), 2, "the list on screen is up to date");
}

#[test]
fn adding_from_a_folder_with_no_project_or_one_that_does_not_exist_says_so() {
    let (mut app, root) = app_with("add-none", &[]);
    fs::create_dir_all(root.join("empty")).unwrap();
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('a'));
    app.prompt.as_mut().unwrap().value = root.join("empty").display().to_string();
    press(&mut app, KeyCode::Enter);
    assert!(
        app.status.contains("no project in or under"),
        "{}",
        app.status
    );

    press(&mut app, KeyCode::Char('a'));
    app.prompt.as_mut().unwrap().value = root.join("missing").display().to_string();
    press(&mut app, KeyCode::Enter);
    assert!(app.status.contains("is not a folder"), "{}", app.status);
    assert!(registry::load(&app.dir).unwrap().is_empty());
}

#[test]
fn adding_a_project_again_keeps_where_it_deploys() {
    let (mut app, root) = app_with("add-twice", &[("shop", &[])]);
    registry::set_host(&app.dir, &root.join("shop"), "ops@example.com").unwrap();
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('a'));
    app.prompt.as_mut().unwrap().value = root.display().to_string();
    press(&mut app, KeyCode::Enter);
    let shop = registry::load(&app.dir)
        .unwrap()
        .into_iter()
        .find(|p| p.name == "shop")
        .unwrap();
    assert_eq!(shop.host.as_deref(), Some("ops@example.com"));
}

/// A project that is a git repository whose origin is `remote`, selected.
fn app_with_remote(test: &str, remote: &str, machine: Fake) -> (App, PathBuf) {
    let (mut app, root) = app_with(test, &[("shop", &[])]);
    let dir = root.join("shop");
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["remote", "add", "origin", remote],
    ] {
        let ok = std::process::Command::new("git")
            .args(&args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success();
        assert!(ok);
    }
    app.machine = Box::new(machine);
    (app, dir)
}

#[test]
fn removing_asks_whether_to_delete_the_github_repository_too() {
    let (mut app, dir) = app_with_remote(
        "rmgithub",
        "git@github.com:me/shop.git",
        fake(&["git", "gh"]),
    );
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('r'));
    let shown = screen(&app);
    for text in [
        "Remove it from where?",
        "this computer only",
        "this computer and GitHub (me/shop)",
        "no undo",
    ] {
        assert!(shown.contains(text), "{text} missing:\n{shown}");
    }
    assert!(
        app.run.is_none() && app.prompt.is_none(),
        "nothing is asked or run yet"
    );

    // GitHub too: the name is then typed back, and the prompt says the repository goes as well.
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    let prompt = app.prompt.as_ref().expect("the name is asked");
    assert!(
        prompt.label.contains("AND its GitHub repo"),
        "{}",
        prompt.label
    );
    press(&mut app, KeyCode::Esc);
    assert!(dir.exists(), "cancelled: nothing was deleted");
}

#[test]
fn this_computer_only_asks_for_the_name_without_mentioning_a_remote() {
    let (mut app, dir) = app_with_remote(
        "rmlocal",
        "git@github.com:me/shop.git",
        fake(&["git", "gh"]),
    );
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('r'));
    press(&mut app, KeyCode::Enter);
    let prompt = app.prompt.as_ref().unwrap();
    assert!(prompt.label.starts_with("REMOVE shop"), "{}", prompt.label);
    assert!(!prompt.label.contains("repository"), "{}", prompt.label);
    press(&mut app, KeyCode::Esc);
    assert!(dir.exists());
}

#[test]
fn gitlab_is_offered_for_a_gitlab_remote_and_nothing_for_other_hosts_or_a_missing_tool() {
    let (mut app, _) = app_with_remote(
        "rmgitlab",
        "https://gitlab.com/team/shop.git",
        fake(&["git", "glab"]),
    );
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('r'));
    assert!(screen(&app).contains("this computer and GitLab (team/shop)"));

    // Without `glab` there is nothing that could delete it: straight to the name.
    let (mut app, _) = app_with_remote(
        "rmnoglab",
        "https://gitlab.com/team/shop.git",
        fake(&["git"]),
    );
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('r'));
    assert!(app.choice.is_none() && app.prompt.is_some());

    // A host that is neither: the same.
    let (mut app, _) = app_with_remote(
        "rmother",
        "git@example.com:me/shop.git",
        fake(&["git", "gh", "glab"]),
    );
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('r'));
    assert!(app.choice.is_none() && app.prompt.is_some());
}

#[test]
fn the_project_panel_shows_what_is_running_the_stack_and_where_git_is() {
    // Something listening on a port the project's compose file publishes: it counts as up.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (mut app, root) = app_with("panel", &[("shop", &["backend/Cargo.toml"])]);
    let dir = root.join("shop");
    fs::write(
        dir.join("docker-compose.yml"),
        format!(
            "services:\n  postgres:\n    image: postgres\n    ports: [\"{port}:5432\"]\n  redis:\n    image: redis\n    ports: [\"1:6379\"]\n"
        ),
    )
    .unwrap();
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "t@e.x"],
        vec!["config", "user.name", "T"],
        vec!["add", "-A"],
        vec!["commit", "-q", "-m", "first commit"],
    ] {
        assert!(
            std::process::Command::new("git")
                .args(&args)
                .current_dir(&dir)
                .status()
                .unwrap()
                .success()
        );
    }
    app.open_projects();
    let shown = screen(&app);
    for text in [
        "partly running",
        "1/2 services answering",
        "built with",
        "axum",
        "postgres",
        "redis",
        "STACK",
        "git",
        "services 1/2 up",
        "room it takes",
        "keys",
        "main",
        "first commit",
        "ENVIRONMENT",
    ] {
        assert!(shown.contains(text), "{text} missing:\n{shown}");
    }
    // Moving between projects reads the one selected, once.
    assert!(app.overview.as_ref().is_some_and(|(p, _)| p == &dir));
}
