use super::*;

#[test]
fn the_home_menu_has_the_logo_start_projects_and_quit() {
    let (app, _) = app_with("home", &[("shop", &[])]);
    let s = screen(&app);
    assert!(s.contains('█'), "the logo");
    assert!(s.contains("Start") && s.contains("My projects") && s.contains("Quit"));
    assert!(s.contains("1 project"), "the count of projects");
    assert!(s.contains("NORMAL") && s.contains("home"));
}

#[test]
fn j_k_and_enter_move_through_the_home_menu() {
    let (mut app, _) = app_with("homekeys", &[("shop", &[])]);
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(app.home, 1);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.page, Page::Projects);
    press(&mut app, KeyCode::Char('h'));
    assert_eq!(app.page, Page::Home);
    for _ in 0..5 {
        press(&mut app, KeyCode::Char('j'));
    }
    assert_eq!(app.home, 2, "stops at the last row");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.outcome, Outcome::Quit);
}

#[test]
fn the_question_mark_shows_the_keymap_and_any_key_closes_it() {
    let (mut app, _) = app_with("help", &[]);
    press(&mut app, KeyCode::Char('?'));
    assert!(screen(&app).contains("keymaps") && screen(&app).contains("stop listing"));
    press(&mut app, KeyCode::Char('q'));
    assert!(!app.help);
    assert_eq!(
        app.outcome,
        Outcome::Running,
        "the closing key is not also acted on"
    );
}
