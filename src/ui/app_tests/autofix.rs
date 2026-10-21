use super::*;

/// An app on the install page, whose install has just failed with `message`.
pub(super) fn failed_install(test: &str, message: &str) -> App {
    let mut app = app_at_the_wizard(test);
    // A machine that has rustup, so the Rust fix is possible.
    app.machine = Box::new(fake(&["cargo", "rustup"]));
    let dir = scratch(&format!("{test}-dir"));
    let script = format!("echo '{message}'; exit 101");
    let mut run = Run::new("install topcoat", vec![echo(&dir, &script)], (24, 100));
    run.start_next().unwrap();
    app.run = Some(run);
    app.page = Page::Install;
    app
}

#[test]
fn a_failure_with_a_known_fix_offers_autofix_by_itself() {
    let mut app = failed_install("fixoffer", OLD_RUST);
    assert_eq!(finish(&mut app), "failed (exit 101)");
    let modal = app.autofix.as_ref().expect("the fix is offered");
    assert_eq!(modal.id, "rust-too-old");
    assert_eq!(modal.fix.len(), 1);
    assert_eq!(modal.retry.len(), 1, "what failed is kept to run again");

    let s = screen(&app);
    assert!(
        s.contains("Something went wrong") && s.contains("Your Rust is too old"),
        "{s}"
    );
    assert!(
        s.contains("rustup update stable") && s.contains("and then try again"),
        "{s}"
    );
    assert!(s.contains("Autofix") && s.contains("Close") && s.contains("AUTOFIX"));
}

#[test]
fn a_failure_it_does_not_recognise_offers_nothing() {
    let mut app = failed_install("nofix", "error: something nobody has seen before");
    assert_eq!(finish(&mut app), "failed (exit 101)");
    assert!(app.autofix.is_none());
    assert!(!screen(&app).contains("Autofix"));
}

#[test]
fn closing_autofix_leaves_the_failure_alone_and_does_not_ask_again() {
    let mut app = failed_install("fixclose", OLD_RUST);
    finish(&mut app);
    press(&mut app, KeyCode::Char('l'));
    assert_eq!(app.autofix.as_ref().unwrap().selected, 1);
    press(&mut app, KeyCode::Enter);
    assert!(app.autofix.is_none());
    for _ in 0..5 {
        app.pump();
    }
    assert!(
        app.autofix.is_none(),
        "the same failure is not offered again"
    );
    assert!(
        app.run.as_ref().unwrap().ended.is_some(),
        "the output is still there to read"
    );
}

#[test]
fn autofix_runs_the_fix_then_the_failed_steps_again() {
    let mut app = failed_install("fixrun", OLD_RUST);
    let dir = scratch("fixrun-work");
    // A harmless stand-in for the fix, and for the step that failed: the second one can only
    // succeed if the first ran before it.
    app.autofix = Some(AutofixModal {
        id: "rust-too-old",
        problem: "Your Rust is too old".into(),
        fix: vec![Step::new(
            "fix it",
            &dir,
            "sh",
            &["-c", "echo fixed > fixed.txt"],
        )],
        retry: vec![Step::new(
            "try again",
            &dir,
            "sh",
            &["-c", "test -f fixed.txt && echo retried"],
        )],
        selected: 0,
        needs_rust: None,
    });
    press(&mut app, KeyCode::Enter);
    assert!(app.autofix.is_none());
    assert!(app.run.as_ref().unwrap().title.starts_with("autofix · "));

    assert_eq!(finish(&mut app), "done");
    assert!(dir.join("fixed.txt").exists());
    assert!(screen(&app).contains("retried"));
    assert!(app.fixed.contains(&"rust-too-old"));
}

#[test]
fn the_same_fix_is_never_tried_twice_in_a_row() {
    let mut app = failed_install("fixtwice", OLD_RUST);
    app.fixed.push("rust-too-old");
    assert_eq!(finish(&mut app), "failed (exit 101)");
    assert!(
        app.autofix.is_none(),
        "it was already tried, so it is not offered again"
    );

    // Going back starts clean: a later failure may be offered the fix again.
    press(&mut app, KeyCode::Char('q'));
    assert!(app.fixed.is_empty());
}

#[test]
fn a_failing_project_action_can_be_fixed_too() {
    let (mut app, root) = app_with("fixaction", &[("shop", &[".env.local"])]);
    app.machine = Box::new(fake(&[]));
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Enter);
    let dir = app.project().unwrap().path.clone();
    let log = "env file /x/.env not found: stat /x/.env: no such file or directory";
    let script = format!("echo '{log}'; exit 1");
    let mut run = Run::new(
        "up · shop",
        vec![Step::new("compose up", &dir, "sh", &["-c", &script])],
        (24, 100),
    );
    run.start_next().unwrap();
    app.run = Some(run);
    app.page = Page::Run;
    finish(&mut app);
    let modal = app.autofix.as_ref().expect("a missing .env is recognised");
    assert_eq!(modal.id, "env-missing");
    assert!(screen(&app).contains("internal copy .env.local .env"));
    drop(root);
}
