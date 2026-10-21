use super::*;

/// A project with the template and some environments, `.env` a copy of `current`.
pub(super) fn app_with_envs(
    test: &str,
    tiers: &[(&str, &str)],
    current: Option<&str>,
) -> (App, PathBuf) {
    let (mut app, root) = app_with(test, &[("shop", &[".env.example"])]);
    let dir = root.join("shop");
    fs::write(
        dir.join(".env.example"),
        "# Template\nPOSTGRES_PASSWORD=change-me\nDATABASE_URL=postgres://app:change-me@localhost:5432/app\nAPI_URL=http://localhost:8000\n",
    )
    .unwrap();
    for (tier, body) in tiers {
        fs::write(dir.join(format!(".env.{tier}")), body).unwrap();
        if current == Some(*tier) {
            fs::write(dir.join(".env"), body).unwrap();
        }
    }
    app.machine = Box::new(fake(&[]));
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    (app, dir)
}

#[test]
fn the_project_page_shows_which_environment_is_active_in_its_color() {
    let (app, _) = app_with_envs(
        "envshow",
        &[("local", "A=1\n"), ("prod", "A=2\n")],
        Some("prod"),
    );
    assert_eq!(app.env.as_deref(), Some("prod"));
    let s = screen(&app);
    assert!(
        s.contains("shop · prod"),
        "the list's title says which: {s}"
    );
    // The color itself: prod is red, as everywhere.
    let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    t.draw(|f| draw(f, &app)).unwrap();
    let buf = t.backend().buffer().clone();
    let cell = buf
        .content()
        .iter()
        .enumerate()
        .find(|(i, c)| {
            c.symbol() == "p"
                && buf.content()[i + 1].symbol() == "r"
                && buf.content()[i + 2].symbol() == "o"
        })
        .map(|(_, c)| c.fg);
    assert_eq!(cell, Some(crate::ui::theme::RED), "prod is red");
}

#[test]
fn e_lists_the_environments_with_the_active_one_selected_and_a_way_to_make_another() {
    let (mut app, _) = app_with_envs(
        "envpick",
        &[("local", "A=1\n"), ("dev", "A=2\n"), ("prod", "A=3\n")],
        Some("dev"),
    );
    press(&mut app, KeyCode::Char('e'));
    let choice = app.choice.as_ref().expect("the picker is open");
    let values: Vec<&str> = choice.options.iter().map(|o| o.value.as_str()).collect();
    assert_eq!(values, ["local", "dev", "prod", "+new"]);
    assert_eq!(
        choice.selected, 1,
        "the active one is where the cursor starts"
    );
    let s = screen(&app);
    assert!(s.contains("Active now:") && s.contains("● dev"), "{s}");
    assert!(
        s.contains("Docker Compose reads .env, which is a copy of .env.dev"),
        "it says what Docker reads: {s}"
    );
    // One row for each environment, the active one marked, with its file and size.
    assert!(
        s.contains("○ local") && s.contains(".env.local · 1 variable"),
        "{s}"
    );
    assert!(
        s.contains("● dev") && s.contains("active") && s.contains(".env.dev · 1 variable"),
        "{s}"
    );
    assert!(s.contains("○ prod") && s.contains(".env.prod"), "{s}");
    assert!(s.contains("+ a new environment") && s.contains("new random passwords"));
    assert!(
        !s.contains("recommended"),
        "an environment is not a recommended version"
    );
}

#[test]
fn picking_an_environment_switches_to_it_on_the_terminal() {
    let (mut app, dir) = app_with_envs(
        "envswitch",
        &[("local", "A=1\n"), ("prod", "A=2\n")],
        Some("local"),
    );
    press(&mut app, KeyCode::Char('e'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Run);
    assert_eq!(finish(&mut app), "done");
    assert_eq!(fs::read_to_string(dir.join(".env")).unwrap(), "A=2\n");
    press(&mut app, KeyCode::Char('q'));
    assert_eq!(
        app.env.as_deref(),
        Some("prod"),
        "the page knows it changed"
    );
}

#[test]
fn a_new_environment_is_made_from_the_template_with_fresh_secrets() {
    let (mut app, dir) = app_with_envs("envnew", &[("local", "A=1\n")], Some("local"));
    press(&mut app, KeyCode::Char('e'));
    press(&mut app, KeyCode::Down);
    assert_eq!(
        app.choice.as_ref().unwrap().options[app.choice.as_ref().unwrap().selected].value,
        "+new"
    );
    press(&mut app, KeyCode::Enter);
    assert!(
        screen(&app).contains("Name of the new environment"),
        "{}",
        screen(&app)
    );
    type_text(&mut app, "prod");
    press(&mut app, KeyCode::Enter);

    let made = fs::read_to_string(dir.join(".env.prod")).unwrap();
    assert!(
        made.contains("# prod tier") && !made.contains("change-me"),
        "{made}"
    );
    let password = made
        .lines()
        .find(|l| l.starts_with("POSTGRES_PASSWORD="))
        .unwrap();
    assert!(
        made.contains(&format!(":{}@", &password["POSTGRES_PASSWORD=".len()..])),
        "{made}"
    );

    let s = screen(&app);
    assert!(
        s.contains("Made .env.prod") && s.contains("POSTGRES_PASSWORD"),
        "{s}"
    );
    assert!(
        s.contains("API_URL") && s.contains("Look at"),
        "what still needs a look: {s}"
    );
    assert!(s.contains("Switch to it") && s.contains("NEW ENV"), "{s}");
    assert_eq!(
        app.env.as_deref(),
        Some("local"),
        "making it does not switch to it"
    );
}

#[test]
fn a_made_environment_can_be_switched_to_from_the_notice_or_left() {
    let (mut app, dir) = app_with_envs("envnew2", &[("local", "A=1\n")], Some("local"));
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "new environment");
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "staging");
    press(&mut app, KeyCode::Enter);
    assert!(app.tier_modal.is_some());
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Enter);
    assert!(
        app.tier_modal.is_none() && app.run.is_none(),
        "Close leaves it for later"
    );
    assert!(dir.join(".env.staging").exists());

    // Again, this time switching.
    press(&mut app, KeyCode::Char('e'));
    let staging = app
        .choice
        .as_ref()
        .unwrap()
        .options
        .iter()
        .position(|o| o.value == "staging")
        .unwrap();
    assert!(staging > 0);
    app.choice = None;
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "new environment");
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "qa");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.page, Page::Run);
    assert_eq!(finish(&mut app), "done");
    assert_eq!(
        fs::read_to_string(dir.join(".env")).unwrap(),
        fs::read_to_string(dir.join(".env.qa")).unwrap(),
        ".env is now a copy of the new one"
    );
}

#[test]
fn an_environment_that_exists_or_has_a_bad_name_is_refused_and_nothing_is_overwritten() {
    let (mut app, dir) = app_with_envs(
        "envrefuse",
        &[("local", "A=1\n"), ("prod", "mine=1\n")],
        Some("local"),
    );
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "new environment");
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "prod");
    press(&mut app, KeyCode::Enter);
    assert!(app.status.contains("already exists"), "{}", app.status);
    assert_eq!(
        fs::read_to_string(dir.join(".env.prod")).unwrap(),
        "mine=1\n"
    );

    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "new environment");
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "../x");
    press(&mut app, KeyCode::Enter);
    assert!(app.status.contains("not a tier name"), "{}", app.status);
    assert!(app.tier_modal.is_none());
}

#[test]
fn the_about_panel_names_the_active_environment() {
    let (mut app, _) = app_with_envs("envabout", &[("local", "A=1\n")], Some("local"));
    press(&mut app, KeyCode::Char('h'));
    let s = screen(&app);
    assert!(s.contains("environment") && s.contains("local"), "{s}");
}

#[test]
fn a_hand_edited_env_is_called_out_and_not_passed_off_as_a_tier() {
    let (mut app, dir) = app_with_envs(
        "envcustom",
        &[("local", "A=1\n"), ("dev", "A=2\n")],
        Some("local"),
    );
    fs::write(dir.join(".env"), "A=1\nEXTRA=mine\n").unwrap();
    press(&mut app, KeyCode::Char('e'));
    let s = screen(&app);
    assert!(s.contains("Active now:") && s.contains("custom"), "{s}");
    assert!(
        s.contains(".env was edited by hand") && s.contains("Switching replaces it"),
        "{s}"
    );
    assert!(
        !s.contains("● local") && !s.contains("● dev"),
        "none of them is marked active: {s}"
    );
}

#[test]
fn with_no_env_yet_it_says_so_and_offers_to_make_one() {
    let (mut app, dir) = app_with_envs("envnone", &[("local", "A=1\n")], None);
    assert!(!dir.join(".env").exists());
    press(&mut app, KeyCode::Char('e'));
    let s = screen(&app);
    assert!(
        s.contains("nothing yet") && s.contains("There is no .env yet"),
        "{s}"
    );
    assert!(s.contains("○ local"), "{s}");
}

#[test]
fn n_in_the_environments_list_goes_straight_to_making_a_new_one() {
    let (mut app, _) = app_with_envs(
        "envn",
        &[("local", "A=1\n"), ("dev", "A=2\n")],
        Some("local"),
    );
    press(&mut app, KeyCode::Char('e'));
    press(&mut app, KeyCode::Char('n'));
    assert!(app.choice.is_none());
    assert!(
        screen(&app).contains("Name of the new environment"),
        "{}",
        screen(&app)
    );
}

#[test]
fn the_about_panel_lists_the_other_environments_beside_the_active_one() {
    let (mut app, _) = app_with_envs(
        "envalso",
        &[("local", "A=1\n"), ("dev", "A=2\n"), ("prod", "A=3\n")],
        Some("local"),
    );
    press(&mut app, KeyCode::Char('h'));
    let s = screen(&app);
    assert!(s.contains("environment") && s.contains("local"), "{s}");
    assert!(s.contains("also") && s.contains("dev, prod"), "{s}");
}

#[test]
fn the_status_bar_says_environments_for_the_environments_view_and_choose_for_versions() {
    let (mut app, _) = app_with_envs("envbar", &[("local", "A=1\n")], Some("local"));
    press(&mut app, KeyCode::Char('e'));
    let s = screen(&app);
    assert!(
        s.contains("ENVIRONMENTS") && s.contains("which one is active"),
        "{s}"
    );
    assert!(s.contains("n new environment"), "{s}");
    assert!(!s.contains("CHOOSE"));
}
