use super::*;

#[test]
fn deploy_asks_for_the_host_then_for_the_name_before_it_runs() {
    let (mut app, root) = app_with("deploy", &[("shop", &[])]);
    registry::set_host(&app.dir, &root.join("shop"), "ops@example.com").unwrap();
    app.reload();
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.action = app
        .actions
        .iter()
        .position(|a| *a == Action::Deploy)
        .unwrap();
    press(&mut app, KeyCode::Enter);
    assert!(
        screen(&app).contains("Server (user@host): ops@example.com"),
        "the last host is filled in"
    );

    press(&mut app, KeyCode::Enter);
    let s = screen(&app);
    assert!(
        s.contains("deploy shop to ops@example.com [prod]") && s.contains("type `shop`"),
        "{s}"
    );
    assert!(s.contains("CONFIRM"));

    type_text(&mut app, "nope");
    press(&mut app, KeyCode::Enter);
    assert!(
        app.status.contains("not confirmed") && app.run.is_none(),
        "nothing ran"
    );

    // Confirmed, it still checks the tier's settings exist before touching anything.
    press(&mut app, KeyCode::Char('l'));
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "shop");
    press(&mut app, KeyCode::Enter);
    assert!(app.status.contains(".env.prod not found"), "{}", app.status);
    assert_eq!(app.page, Page::Project);
}

#[test]
fn an_action_runs_on_the_in_app_terminal_and_the_output_replaces_the_description() {
    let (mut app, root) = app_with("run", &[("shop", &[".env.local"])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.action = app.actions.iter().position(|a| *a == Action::Env).unwrap();
    press(&mut app, KeyCode::Enter);
    let picker = screen(&app);
    assert!(
        picker.contains("environments") && picker.contains("+ a new environment"),
        "{picker}"
    );
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Run);

    assert_eq!(finish(&mut app), "done");
    let s = screen(&app);
    assert!(
        s.contains("make local the current tier") && s.contains("internal copy .env.local .env"),
        "{s}"
    );
    assert!(
        s.contains("done") && s.contains("shop"),
        "the list stays beside the output"
    );
    assert!(root.join("shop/.env").exists(), "the step really ran");

    press(&mut app, KeyCode::Char('q'));
    assert_eq!(app.page, Page::Project);
    assert!(app.run.is_none());
}

#[test]
fn a_failing_step_is_reported_and_stops_the_rest() {
    let (mut app, _) = app_with("fail", &[("shop", &[])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    let dir = app.project().unwrap().path.clone();
    let steps = vec![
        Step::new("fails", &dir, "sh", &["-c", "exit 3"]),
        Step::new("never runs", &dir, "touch", &["ran-anyway"]),
    ];
    let mut run = Run::new("test", steps, (24, 80));
    run.start_next().unwrap();
    app.run = Some(run);
    app.page = Page::Run;
    assert_eq!(finish(&mut app), "failed (exit 3)");
    assert!(!dir.join("ran-anyway").exists());
}

#[test]
fn keys_go_to_a_running_program_and_ctrl_close_bracket_comes_back() {
    let (mut app, _) = app_with("live", &[("shop", &[])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    let dir = app.project().unwrap().path.clone();
    // `cat` echoes what it is typed, so the typing must have reached it.
    let mut run = Run::new(
        "test",
        vec![Step::new("echo keys", &dir, "cat", &[])],
        (24, 80),
    );
    run.start_next().unwrap();
    app.run = Some(run);
    app.page = Page::Run;
    type_text(&mut app, "hello");
    press(&mut app, KeyCode::Enter);
    for _ in 0..50 {
        app.pump();
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        screen(&app).contains("hello"),
        "the program received the keys"
    );
    assert!(app.run.as_ref().unwrap().live(), "still running");

    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL));
    assert_eq!(app.page, Page::Project);
    assert!(app.run.is_none(), "stopped");
}

#[test]
fn add_volume_asks_for_a_name_then_a_service_then_a_path_and_refuses_a_bad_name() {
    let (mut app, root) = app_with("vol", &[("shop", &[])]);
    fs::write(
        root.join("shop/docker-compose.yml"),
        "services:\n  redis:\n    image: redis\n  backend:\n    image: b\n",
    )
    .unwrap();
    app.open_project();
    app.begin_action(Action::VolumeAdd);
    assert!(app.prompt.as_ref().unwrap().label.contains("Volume name"));

    // A name that is not one is refused, and asked again.
    type_text(&mut app, "Bad Name");
    press(&mut app, KeyCode::Enter);
    assert!(app.status.contains("not a volume name"), "{}", app.status);

    app.begin_action(Action::VolumeAdd);
    type_text(&mut app, "redisdata");
    press(&mut app, KeyCode::Enter);
    // Then the service, from the compose file's own.
    let shown = screen(&app);
    assert!(shown.contains("Which service stores it?"), "{shown}");
    assert!(
        shown.contains("redis") && shown.contains("backend"),
        "{shown}"
    );
    press(&mut app, KeyCode::Enter);
    // Then the path, with a usual one filled in.
    let prompt = app.prompt.as_ref().expect("the path is asked");
    assert!(prompt.label.contains("Path inside the container"));
    assert_eq!(prompt.value, "/data");
    press(&mut app, KeyCode::Esc);
    assert!(
        app.run.is_none(),
        "nothing ran: the last question was not answered"
    );
}

#[test]
fn backup_volume_offers_the_declared_volumes_and_says_where_each_is_used() {
    let (mut app, root) = app_with("volpick", &[("shop", &[])]);
    fs::write(
        root.join("shop/docker-compose.yml"),
        "services:\n  postgres:\n    image: p\n    volumes: [pgdata:/var/lib/postgresql]\n\nvolumes:\n  pgdata:\n  idle:\n",
    )
    .unwrap();
    app.open_project();
    assert!(
        app.actions.contains(&Action::VolumeBackup),
        "it has volumes to back up"
    );
    app.begin_action(Action::VolumeBackup);
    let shown = screen(&app);
    assert!(shown.contains("Which volume?"), "{shown}");
    assert!(
        shown.contains("pgdata") && shown.contains("postgres /var/lib/postgresql"),
        "{shown}"
    );
    assert!(
        shown.contains("idle") && shown.contains("not mounted by any service"),
        "{shown}"
    );
}

#[test]
fn dev_asks_in_its_panel_which_part_to_start() {
    let (mut app, _) = app_with(
        "devpick",
        &[("shop", &["backend/Cargo.toml", "frontend/nuxt.config.ts"])],
    );
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.action = app.actions.iter().position(|a| *a == Action::Dev).unwrap();
    press(&mut app, KeyCode::Enter);
    let s = screen(&app);
    assert!(
        s.contains("What should it start?") && s.contains("both") && s.contains("frontend"),
        "{s}"
    );
    assert!(
        app.choice.is_none() && app.run.is_none(),
        "no popup, nothing runs yet"
    );
    assert_eq!(app.dev_parts()[0], "fullstack", "both comes first");
    assert!(
        !app.dev_parts().contains(&"databases"),
        "no databases, so no databases-only start"
    );

    // j moves the pick, not the action list; Esc puts the panel back.
    let action = app.action;
    press(&mut app, KeyCode::Char('j'));
    assert_eq!((app.dev_part, app.action), (Some(1), action));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.dev_part, None);
    assert_eq!(app.page, Page::Project);
    assert!(!screen(&app).contains("What should it start?"));
}
