use super::*;

pub(super) fn link(label: &str, url: &str, up: Option<bool>) -> Link {
    Link {
        label: label.into(),
        url: url.into(),
        up,
    }
}

/// The urls a running project would have, set directly so no test needs Docker.
pub(super) fn some_urls() -> Urls {
    Urls {
        web: vec![
            link("app", "http://localhost:3000", Some(true)),
            link("api", "http://localhost:8000", Some(false)),
        ],
        data: vec![link("postgres", "postgres://localhost:5432", None)],
        logins: vec![],
    }
}

/// Every address "opened" in a test, instead of a browser opening.
pub(super) static OPENED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub(super) fn record(url: &str) -> anyhow::Result<()> {
    OPENED.lock().unwrap().push(url.to_string());
    Ok(())
}

#[test]
fn an_open_project_shows_where_it_is_reachable_and_whether_it_is_answering() {
    let (mut app, _) = app_with("urls", &[("shop", &[])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.urls = some_urls();
    let s = screen(&app);
    assert!(
        s.contains("open in a browser") && s.contains("connect to"),
        "{s}"
    );
    assert!(
        s.contains("● app") && s.contains("http://localhost:3000"),
        "{s}"
    );
    assert!(
        s.contains("○ api") && s.contains("http://localhost:8000"),
        "a stopped one has an empty dot: {s}"
    );
    assert!(
        s.contains("postgres://localhost:5432"),
        "and where to connect to its data"
    );
    assert!(
        s.contains("what it does"),
        "beside the description, not instead of it"
    );
}

#[test]
fn the_addresses_stay_under_the_output_while_something_runs() {
    let (mut app, _) = app_with("urls-run", &[("shop", &[])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.urls = some_urls();
    let dir = app.project().unwrap().path.clone();
    let mut run = Run::new(
        "up · shop",
        vec![Step::new("echo", &dir, "echo", &["building"])],
        (24, 80),
    );
    run.start_next().unwrap();
    app.run = Some(run);
    app.page = Page::Run;
    assert_eq!(finish(&mut app), "done");
    let s = screen(&app);
    assert!(
        s.contains("building") && s.contains("http://localhost:3000"),
        "{s}"
    );
}

#[test]
fn a_project_with_no_published_ports_shows_no_strip() {
    let (mut app, _) = app_with("urls-none", &[("shop", &[])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.urls = Urls::default();
    assert!(!screen(&app).contains("open in a browser"));
}

#[test]
fn o_opens_the_app_in_the_browser_and_says_what_it_opened() {
    let (mut app, _) = app_with("urls-open", &[("shop", &[])]);
    app.opener = record;
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.urls = some_urls();
    press(&mut app, KeyCode::Char('o'));
    assert!(
        app.status.contains("opened http://localhost:3000"),
        "{}",
        app.status
    );
    assert!(
        OPENED
            .lock()
            .unwrap()
            .contains(&"http://localhost:3000".to_string())
    );

    app.urls = Urls::default();
    press(&mut app, KeyCode::Char('o'));
    assert!(app.status.contains("nothing to open"), "{}", app.status);
}
