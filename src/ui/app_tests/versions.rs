use super::*;

/// A machine with an old default Rust, a newer one installed beside it, and npm: the case that
/// asks about Rust, Topcoat and pnpm. Used only to ask: nothing here is ever run.
pub(super) fn old_rust_machine() -> Box<Fake> {
    Box::new(Fake {
        programs: &["cargo", "rustup", "npm"],
        rust: Some((1, 97)),
        toolchains: &["stable-aarch64-apple-darwin", "1.99.0-aarch64-apple-darwin"],
        github: Default::default(),
    })
}

/// The install question open for topcoat, on a machine that has a choice to make.
pub(super) fn app_asking_to_install(test: &str) -> App {
    let mut app = app_at_the_wizard(test);
    app.machine = old_rust_machine();
    app.open_install_modal(vec!["topcoat"]);
    app
}

#[test]
fn install_asks_which_rust_first_when_the_one_in_use_is_too_old() {
    let mut app =
        app_asking_to_install("install_asks_which_rust_first_when_the_one_in_use_is_too_old");
    press(&mut app, KeyCode::Char('i'));
    let choice = app.choice.as_ref().expect("a version question is open");
    assert!(matches!(
        choice.question,
        crate::services::toolchain::versions::Question::Rust { .. }
    ));
    assert_eq!(choice.next.len(), 1, "the Topcoat question follows");
    assert!(
        app.run.is_none(),
        "nothing runs before the questions are answered"
    );

    let s = screen(&app);
    assert!(s.contains("Which Rust do you want?"), "{s}");
    assert!(
        s.contains("1.99.0-aarch64-apple-darwin") && s.contains("already installed"),
        "{s}"
    );
    assert!(s.contains("latest stable") && s.contains("recommended"));
    assert!(
        s.contains("nightly") && s.contains("beta"),
        "the common ones are listed"
    );
    assert!(s.contains("CHOOSE"));
}

#[test]
fn the_choices_are_made_with_the_arrows_or_a_number_and_confirmed_one_at_a_time() {
    let mut app = app_asking_to_install(
        "the_choices_are_made_with_the_arrows_or_a_number_and_confirmed_one_at_a_time",
    );
    press(&mut app, KeyCode::Char('i'));
    assert_eq!(
        app.choice.as_ref().unwrap().selected,
        0,
        "the recommended one is selected"
    );
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(app.choice.as_ref().unwrap().selected, 2);
    press(&mut app, KeyCode::Char('1'));
    assert_eq!(
        app.choice.as_ref().unwrap().selected,
        0,
        "a number picks it"
    );
    press(&mut app, KeyCode::Char('9'));
    assert_eq!(
        app.choice.as_ref().unwrap().selected,
        0,
        "a number past the list does nothing"
    );
    press(&mut app, KeyCode::Up);
    assert_eq!(
        app.choice.as_ref().unwrap().selected,
        0,
        "it stops at the first"
    );

    // Take "latest stable" for Rust: the Topcoat question comes next, with that remembered.
    press(&mut app, KeyCode::Char('2'));
    press(&mut app, KeyCode::Enter);
    let next = app.choice.as_ref().expect("the next question");
    assert_eq!(
        next.question,
        crate::services::toolchain::versions::Question::Topcoat
    );
    assert_eq!(next.picks.rust.as_deref(), Some("stable"));
    let s = screen(&app);
    assert!(
        s.contains("Which Topcoat CLI do you want?") && s.contains("tested with stackup"),
        "{s}"
    );
    assert!(s.contains("latest (main)"));
    assert!(next.next.is_empty());
}

#[test]
fn escape_goes_back_to_install_or_close_without_installing_anything() {
    let mut app =
        app_asking_to_install("escape_goes_back_to_install_or_close_without_installing_anything");
    press(&mut app, KeyCode::Char('i'));
    press(&mut app, KeyCode::Esc);
    assert!(app.choice.is_none());
    assert!(
        app.install.is_some(),
        "the install question is still there to answer"
    );
    assert!(app.run.is_none());
}

#[test]
fn only_tools_with_a_version_to_choose_are_asked_about() {
    let mut app = app_at_the_wizard("nochoice");
    app.machine = Box::new(fake(&[]));
    // mkcert has only a latest: no question, so Install goes straight to the plan it showed.
    let dir = scratch("nochoice-dir");
    app.install = Some(InstallModal {
        missing: vec!["mkcert"],
        plan: InstallPlan {
            steps: vec![echo(&dir, "echo installing-mkcert")],
            by_hand: vec![],
        },
        selected: 0,
    });
    press(&mut app, KeyCode::Char('i'));
    assert!(app.choice.is_none());
    assert_eq!(app.page, Page::Install);
    assert_eq!(finish(&mut app), "done");
}

#[test]
fn an_old_rust_autofix_asks_which_rust_before_it_runs() {
    let mut app = failed_install("fixchoose", OLD_RUST);
    app.machine = old_rust_machine();
    finish(&mut app);
    assert!(app.autofix.as_ref().unwrap().needs_rust.is_some());
    press(&mut app, KeyCode::Char('a'));
    let choice = app.choice.as_ref().expect("which Rust is asked first");
    assert_eq!(choice.then, crate::ui::stores::app::AfterChoices::Autofix);
    assert!(screen(&app).contains("Which Rust do you want?"));
    // Backing out leaves the autofix question as it was.
    press(&mut app, KeyCode::Esc);
    assert!(app.choice.is_none() && app.autofix.is_some());
}
