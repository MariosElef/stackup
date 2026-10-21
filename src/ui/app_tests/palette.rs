use super::*;

pub(super) fn ctrl(app: &mut App, c: char) {
    app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

#[test]
fn colon_or_ctrl_p_opens_the_command_palette_with_every_command() {
    let (mut app, _) = app_with("palette", &[("shop", &[]), ("blog", &[])]);
    press(&mut app, KeyCode::Char(':'));
    assert!(app.palette.is_some());
    let s = screen(&app);
    assert!(s.contains("commands") && s.contains("COMMAND"), "{s}");
    for want in [
        "Start a new project",
        "My projects",
        "Open shop",
        "Open blog",
        "Quit",
    ] {
        assert!(s.contains(want), "{want} is listed: {s}");
    }
    press(&mut app, KeyCode::Esc);
    assert!(app.palette.is_none());
    ctrl(&mut app, 'p');
    assert!(app.palette.is_some(), "Ctrl-P opens it too");
}

#[test]
fn typing_filters_and_enter_runs_what_is_selected() {
    let (mut app, _) = app_with("palette-run", &[("shop", &[])]);
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "quit");
    let shown: Vec<String> = app
        .palette
        .as_ref()
        .unwrap()
        .shown()
        .iter()
        .map(|e| e.label.clone())
        .collect();
    assert_eq!(shown, ["Quit"]);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.outcome, Outcome::Quit);
    assert!(app.palette.is_none());
}

#[test]
fn a_project_can_be_jumped_to_by_typing_part_of_its_name() {
    let (mut app, _) = app_with("palette-jump", &[("shop", &[]), ("blog", &["ci.sh"])]);
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "blg");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Project);
    assert_eq!(
        app.project().unwrap().name,
        "blog",
        "scattered letters still find it"
    );
    assert!(
        app.actions.contains(&Action::Ci),
        "it is opened properly, with its own actions"
    );
}

#[test]
fn from_a_project_every_action_that_applies_is_in_the_palette() {
    let (mut app, _) = app_with("palette-actions", &[("shop", &[".env.local", "ci.sh"])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char(':'));
    let labels: Vec<String> = app
        .palette
        .as_ref()
        .unwrap()
        .shown()
        .iter()
        .map(|e| e.label.clone())
        .collect();
    for want in [
        "shop · up",
        "shop · deploy",
        "shop · ci",
        "shop · remove",
        "shop · stop listing",
    ] {
        assert!(labels.iter().any(|l| l == want), "{want} in {labels:?}");
    }
    assert!(
        !labels.iter().any(|l| l == "shop · certs"),
        "an action that does not apply is not offered"
    );

    // Choosing one does what choosing it from the list does: here `env` asks which tier.
    type_text(&mut app, "env");
    press(&mut app, KeyCode::Enter);
    assert!(screen(&app).contains("Active now:"), "{}", screen(&app));
}

#[test]
fn an_action_can_be_run_on_the_selected_project_without_opening_it_first() {
    let (mut app, _) = app_with("palette-direct", &[("shop", &[".env.local"])]);
    press(&mut app, KeyCode::Char('p'));
    assert_eq!(app.page, Page::Projects);
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "shop env");
    press(&mut app, KeyCode::Enter);
    assert!(app.choice.is_some(), "it asked which environment");
    assert_eq!(app.page, Page::Project, "and opened the project to do it");
}

#[test]
fn the_closest_match_comes_first_and_nothing_matching_says_so() {
    let (mut app, _) = app_with("palette-rank", &[("shop", &[])]);
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "pro");
    let first = app.palette.as_ref().unwrap().shown()[0].label.clone();
    assert_eq!(
        first, "My projects",
        "a word that starts with it beats scattered letters"
    );
    type_text(&mut app, "zzz");
    assert!(screen(&app).contains("nothing matches"));
    press(&mut app, KeyCode::Enter);
    assert!(
        app.palette.is_none() && app.outcome == Outcome::Running,
        "Enter on nothing does nothing"
    );
}

#[test]
fn the_arrows_move_and_backspace_on_an_empty_query_closes() {
    let (mut app, _) = app_with("palette-keys", &[]);
    press(&mut app, KeyCode::Char(':'));
    press(&mut app, KeyCode::Down);
    assert_eq!(app.palette.as_ref().unwrap().selected, 1);
    ctrl(&mut app, 'n');
    assert_eq!(app.palette.as_ref().unwrap().selected, 2);
    ctrl(&mut app, 'p');
    press(&mut app, KeyCode::Up);
    assert_eq!(app.palette.as_ref().unwrap().selected, 0);
    type_text(&mut app, "q");
    press(&mut app, KeyCode::Backspace);
    assert!(app.palette.is_some(), "that removed the letter");
    press(&mut app, KeyCode::Backspace);
    assert!(app.palette.is_none(), "an empty query closes it");
}

#[test]
fn the_palette_stays_out_of_the_way_of_prompts_the_wizard_and_running_programs() {
    let (mut app, _) = app_with("palette-away", &[("shop", &[])]);
    // In a prompt, `:` is a character.
    press(&mut app, KeyCode::Char('s'));
    press(&mut app, KeyCode::Char(':'));
    assert!(app.palette.is_none());
    assert_eq!(app.prompt.as_ref().unwrap().value, ":");
    press(&mut app, KeyCode::Esc);

    // In the wizard, `:` is its own command line.
    let mut app = app_at_the_wizard("palette-wizard");
    press(&mut app, KeyCode::Char(':'));
    assert!(app.palette.is_none());

    // While a program runs, keys go to it.
    let (mut app, _) = app_with("palette-run2", &[("shop", &[])]);
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    let dir = app.project().unwrap().path.clone();
    let mut run = Run::new("test", vec![Step::new("cat", &dir, "cat", &[])], (24, 80));
    run.start_next().unwrap();
    app.run = Some(run);
    app.page = Page::Run;
    press(&mut app, KeyCode::Char(':'));
    assert!(app.palette.is_none(), "the colon went to the program");
    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL));
}

#[test]
fn the_palette_can_open_the_app_too() {
    let (mut app, _) = app_with("urls-palette", &[("shop", &[])]);
    app.opener = record;
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    app.urls = some_urls();
    press(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "browser");
    assert_eq!(
        app.palette.as_ref().unwrap().shown()[0].label,
        "shop · open in the browser"
    );
    press(&mut app, KeyCode::Enter);
    assert!(
        app.status.contains("opened http://localhost:3000"),
        "{}",
        app.status
    );
}
