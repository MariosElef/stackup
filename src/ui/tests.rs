use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use super::{
    app::draw_wizard,
    features::create::{
        selection::Selection, use_menu::selectable, use_requirements::TOOLS_TO_CHECK,
        wizard::Wizard,
    },
    types::{Focus, Mode, Outcome, Row, Tab},
};
use crate::models::{Backend, BeLib, FeLib, Frontend, Service, Tool};

fn all_installed() -> Vec<(&'static str, bool)> {
    TOOLS_TO_CHECK.iter().map(|b| (*b, true)).collect()
}

fn app() -> Wizard {
    Wizard::new("demo", Selection::recommended(), all_installed())
}

fn press(app: &mut Wizard, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

/// Types a string as keys; `\n` is Enter, `\x04` is Ctrl-d, and so on.
fn keys(app: &mut Wizard, text: &str) {
    for ch in text.chars() {
        let key = match ch {
            '\n' => KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            '\x04' => KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            '\x08' => KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
            '\x0c' => KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            c => KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        };
        app.handle_key(key);
    }
}

fn screen(app: &Wizard) -> String {
    let mut t = Terminal::new(TestBackend::new(120, 50)).unwrap();
    t.draw(|f| draw_wizard(f, app)).unwrap();
    t.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn cursor_skips_headers_and_gaps() {
    let mut a = app();
    assert!(matches!(a.rows[a.cursor], Row::Frontend(Frontend::Nuxt)));
    for _ in 0..30 {
        press(&mut a, KeyCode::Down);
        assert!(selectable(&a.rows[a.cursor]));
    }
    assert!(matches!(a.rows[a.cursor], Row::Create));
}

#[test]
fn space_selects_radios_and_toggles_checkboxes() {
    let mut a = app();
    press(&mut a, KeyCode::Down); // Vue
    press(&mut a, KeyCode::Enter);
    assert_eq!(a.sel.frontend, Frontend::Vue);
    let svc = a
        .rows
        .iter()
        .position(|r| *r == Row::Service(Service::Mongodb))
        .unwrap();
    a.cursor = svc;
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.services.contains(&Service::Mongodb));
    press(&mut a, KeyCode::Enter);
    assert!(!a.sel.services.contains(&Service::Mongodb));
}

#[test]
fn preview_follows_the_selection() {
    let mut a = app();
    assert!(screen(&a).contains("pages"), "Nuxt shows pages/");
    a.sel.frontend = Frontend::Vue;
    assert!(screen(&a).contains("views"), "Vue shows views/");
    a.sel.backend = Backend::Fastapi;
    let s = screen(&a);
    assert!(s.contains("repositories") && s.contains("pyproject.toml"));
    a.tab = Tab::Compose;
    assert!(screen(&a).contains("postgres:alpine"));
    a.sel.services.retain(|s| *s != Service::Postgres);
    assert!(!screen(&a).contains("postgres:alpine"));
}

#[test]
fn create_is_blocked_while_a_tool_is_missing_and_says_which() {
    let mut a = Wizard::new("demo", Selection::recommended(), vec![("cargo", false)]);
    keys(&mut a, ":wq\n");
    assert_eq!(a.outcome, Outcome::Running);
    let blocked = a.blocked.expect("the app is asked to offer installing it");
    assert!(blocked.contains(&"cargo"), "{blocked:?}");

    // Only what this stack needs is asked for: a Topcoat project does not need pnpm.
    let mut t = Wizard::new("demo", Selection::recommended(), vec![("topcoat", false)]);
    t.sel.frontend = Frontend::Topcoat;
    keys(&mut t, ":wq\n");
    let blocked = t.blocked.expect("blocked");
    assert!(
        blocked.contains(&"topcoat") && !blocked.contains(&"pnpm"),
        "{blocked:?}"
    );

    let mut b = app();
    keys(&mut b, ":wq\n");
    assert_eq!(b.outcome, Outcome::Create);
    assert!(b.blocked.is_none());
}

#[test]
fn q_and_ctrl_c_cancel() {
    let mut a = app();
    press(&mut a, KeyCode::Char('q'));
    assert_eq!(a.outcome, Outcome::Cancel);
    let mut b = app();
    b.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(b.outcome, Outcome::Cancel);
}

fn row_of(a: &Wizard, lib: FeLib) -> usize {
    a.rows.iter().position(|r| *r == Row::Lib(lib)).unwrap()
}

#[test]
fn colada_and_pinia_pull_each_other() {
    let mut a = app();
    a.sel.libs.clear();
    a.cursor = row_of(&a, FeLib::Colada);
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.libs.contains(&FeLib::Colada) && a.sel.libs.contains(&FeLib::Pinia));
    a.cursor = row_of(&a, FeLib::Pinia);
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.libs.is_empty(), "turning Pinia off turns Colada off");
}

#[test]
fn libraries_bring_their_folders() {
    let mut a = app();
    a.sel.libs = vec![];
    let s = screen(&a);
    assert!(!s.contains("stores/") && !s.contains("queries/") && !s.contains("contracts/"));
    a.sel.libs = vec![FeLib::Pinia, FeLib::Colada, FeLib::Zod, FeLib::I18n];
    let s = screen(&a);
    assert!(s.contains("stores/") && s.contains("queries/") && s.contains("contracts/"));
    assert!(s.contains("locales"), "i18n adds i18n/locales");
}

#[test]
fn libs_need_a_frontend() {
    let mut a = app();
    a.sel.frontend = Frontend::Vue;
    a.cursor = row_of(&a, FeLib::I18n);
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.libs.contains(&FeLib::I18n));
    a.sel.frontend = Frontend::None;
    a.cursor = row_of(&a, FeLib::Zod);
    press(&mut a, KeyCode::Enter);
    assert!(a.status.contains("frontend"));
}

#[test]
fn ctrl_h_and_ctrl_l_switch_panes() {
    let mut a = app();
    assert_eq!(a.focus, Focus::Menu);
    keys(&mut a, "\x0c");
    assert_eq!(a.focus, Focus::Preview);
    keys(&mut a, "\x08");
    assert_eq!(a.focus, Focus::Menu);
    keys(&mut a, "l");
    assert_eq!(a.focus, Focus::Preview);
    keys(&mut a, "h");
    assert_eq!(a.focus, Focus::Menu);
}

#[test]
fn j_and_k_scroll_the_preview_when_it_has_focus() {
    let mut a = app();
    // A draw tells the app how far the preview can scroll: on a short screen, so that the file
    // tree is taller than it whatever the project's files are.
    let mut t = Terminal::new(TestBackend::new(120, 30)).unwrap();
    t.draw(|f| draw_wizard(f, &a)).unwrap();
    let before = a.cursor;
    keys(&mut a, "\x0c");
    keys(&mut a, "5j");
    assert_eq!(a.cursor, before, "the menu cursor stays put");
    assert_eq!(a.scroll, 5.min(a.max_scroll.get()));
    keys(&mut a, "G");
    assert_eq!(a.scroll, a.max_scroll.get());
    keys(&mut a, "gg");
    assert_eq!(a.scroll, 0);
    keys(&mut a, "\x04");
    assert!(a.scroll > 0, "ctrl-d scrolls half a page");
}

#[test]
fn counts_gg_and_big_g_move_the_menu() {
    let mut a = app();
    keys(&mut a, "3j");
    let after_three = a.cursor;
    keys(&mut a, "gg");
    keys(&mut a, "j");
    keys(&mut a, "j");
    keys(&mut a, "j");
    assert_eq!(a.cursor, after_three);
    keys(&mut a, "G");
    assert!(matches!(a.rows[a.cursor], Row::Create));
    keys(&mut a, "gg");
    assert!(matches!(a.rows[a.cursor], Row::Frontend(Frontend::Nuxt)));
}

#[test]
fn braces_jump_between_sections() {
    let mut a = app();
    keys(&mut a, "}");
    assert!(
        matches!(a.rows[a.cursor], Row::Lib(_)),
        "next section is the libraries"
    );
    keys(&mut a, "}");
    assert!(matches!(a.rows[a.cursor], Row::Backend(_)));
    keys(&mut a, "{");
    assert!(matches!(a.rows[a.cursor], Row::Lib(_)));
}

#[test]
fn space_ticks_like_enter() {
    let mut a = app();
    let mongo = a
        .rows
        .iter()
        .position(|r| *r == Row::Service(Service::Mongodb))
        .unwrap();
    a.cursor = mongo;
    keys(&mut a, " ");
    assert!(a.sel.services.contains(&Service::Mongodb));
    assert!(a.seq.is_empty(), "space starts no sequence");
    keys(&mut a, " ");
    assert!(!a.sel.services.contains(&Service::Mongodb));
    keys(&mut a, "x");
    assert!(a.sel.services.contains(&Service::Mongodb), "x ticks too");
}

#[test]
fn command_line_works_like_vim() {
    let mut a = app();
    keys(&mut a, ":q\n");
    assert_eq!(a.outcome, Outcome::Cancel);
    let mut b = app();
    keys(&mut b, ":wq\n");
    assert_eq!(b.outcome, Outcome::Create);
    let mut c = app();
    keys(&mut c, ":bogus\n");
    assert!(c.status.contains("E492"));
    assert_eq!(c.mode, Mode::Normal);
    let mut d = app();
    keys(&mut d, ":q");
    assert!(
        screen(&d).contains(":q"),
        "the command line shows what is typed"
    );
    d.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(d.mode, Mode::Normal);
    assert_eq!(d.outcome, Outcome::Running);
}

#[test]
fn slash_searches_the_menu_and_n_repeats() {
    let mut a = app();
    keys(&mut a, "/mongo\n");
    assert!(matches!(a.rows[a.cursor], Row::Service(Service::Mongodb)));
    keys(&mut a, "gg/pin\n");
    assert!(matches!(a.rows[a.cursor], Row::Lib(FeLib::Pinia)));
    keys(&mut a, "n");
    assert!(matches!(a.rows[a.cursor], Row::Lib(FeLib::Colada)));
    keys(&mut a, "N");
    assert!(matches!(a.rows[a.cursor], Row::Lib(FeLib::Pinia)));
    keys(&mut a, "/nothing-like-this\n");
    assert!(a.status.contains("not found"));
}

#[test]
fn question_mark_opens_help_and_any_key_closes_it() {
    let mut a = app();
    keys(&mut a, "?");
    assert_eq!(a.mode, Mode::Help);
    assert!(screen(&a).contains("keymaps"));
    keys(&mut a, "x");
    assert_eq!(a.mode, Mode::Normal);
    assert_eq!(
        a.outcome,
        Outcome::Running,
        "the closing key is not also acted on"
    );
}

#[test]
fn shift_h_and_l_switch_the_preview_tab() {
    let mut a = app();
    keys(&mut a, "L");
    assert_eq!(a.tab, Tab::Compose);
    keys(&mut a, "H");
    assert_eq!(a.tab, Tab::Files);
}

#[test]
fn the_statusline_shows_the_mode_and_pending_keys() {
    let mut a = app();
    assert!(screen(&a).contains("NORMAL"));
    keys(&mut a, "3");
    assert!(screen(&a).contains('3'));
    a.count.clear();
    keys(&mut a, ":");
    assert!(screen(&a).contains("COMMAND"));
}

#[test]
fn git_is_a_tool_toggle_that_feeds_the_plan() {
    let mut a = app();
    assert!(a.plan().git);
    a.cursor = a
        .rows
        .iter()
        .position(|r| *r == Row::Tool(Tool::Git))
        .unwrap();
    press(&mut a, KeyCode::Enter);
    assert!(!a.plan().git);
}

#[test]
fn services_show_the_driver_of_the_chosen_backend() {
    let mut a = app();
    let s = screen(&a);
    assert!(
        s.contains("· sqlx") && s.contains("· redis"),
        "axum drivers"
    );
    a.sel.backend = Backend::Fastapi;
    let s = screen(&a);
    assert!(
        s.contains("· asyncpg") && s.contains("· pymongo"),
        "FastAPI drivers"
    );
    assert!(!s.contains("· sqlx"));
    a.sel.backend = Backend::None;
    assert!(!screen(&a).contains("· asyncpg"), "no backend, no driver");
}

#[test]
fn topcoat_needs_cargo_and_its_cli_not_pnpm() {
    let mut a = app();
    a.sel.frontend = Frontend::Topcoat;
    a.sel.backend = Backend::Fastapi;
    let need = a.requires();
    assert!(need.contains(&"cargo") && need.contains(&"topcoat") && need.contains(&"uv"));
    assert!(!need.contains(&"pnpm"));
    a.sel.backend = Backend::Axum;
    assert_eq!(
        a.requires().iter().filter(|b| **b == "cargo").count(),
        1,
        "cargo is listed once"
    );
}

#[test]
fn topcoat_takes_tailwind_lucide_and_i18n_but_not_npm_libraries() {
    let mut a = app();
    a.sel.frontend = Frontend::Topcoat;
    a.cursor = row_of(&a, FeLib::Pinia);
    press(&mut a, KeyCode::Enter);
    assert!(a.status.contains("not available"), "{}", a.status);
    for lib in [FeLib::Tailwind, FeLib::Lucide, FeLib::I18n] {
        a.cursor = row_of(&a, lib);
        press(&mut a, KeyCode::Enter);
        assert!(a.sel.libs.contains(&lib), "{lib} can be ticked for Topcoat");
    }
    assert_eq!(
        a.plan().libs,
        vec![FeLib::Tailwind, FeLib::Lucide, FeLib::I18n],
        "the plan keeps those and drops Pinia and Colada"
    );
    let s = screen(&a);
    assert!(s.contains("features.rs") && s.contains("Cargo.toml") && s.contains("Dockerfile"));
    assert!(
        s.contains("locales") && s.contains("components.toml"),
        "i18n and UI show up"
    );
    for layer in [
        "composables/",
        "services/",
        "middleware/",
        "types/",
        "utils/",
    ] {
        assert!(s.contains(layer), "Topcoat gets the {layer} layer");
    }
    assert!(!s.contains("pnpm-workspace"), "no Node files for Topcoat");
}

#[test]
fn topcoat_ui_needs_both_tailwind_and_lucide() {
    let mut a = app();
    a.sel.frontend = Frontend::Topcoat;
    a.sel.libs = vec![FeLib::Tailwind];
    assert!(!a.plan().topcoat_ui());
    assert!(
        a.plan().frontend_layers().contains(&"components"),
        "no UI: components is a plain layer"
    );
    a.sel.libs = vec![FeLib::Tailwind, FeLib::Lucide];
    assert!(a.plan().topcoat_ui());
    assert!(
        !a.plan().frontend_layers().contains(&"components"),
        "the UI scaffold owns it"
    );
}

#[test]
fn topcoat_runs_beside_the_chosen_backend() {
    let mut a = app();
    a.sel.frontend = Frontend::Topcoat;
    let s = screen(&a);
    assert!(
        s.contains("frontend/") && s.contains("backend/"),
        "both folders: {s}"
    );
    a.tab = Tab::Compose;
    let s = screen(&a);
    assert!(s.contains("API_URL") && s.contains("3000:3000"));
}

fn be_row(a: &Wizard) -> usize {
    a.rows
        .iter()
        .position(|r| matches!(r, Row::BeLib(_)))
        .unwrap()
}

#[test]
fn utoipa_is_an_axum_only_backend_library() {
    let mut a = app();
    assert!(screen(&a).contains("Backend libraries") && screen(&a).contains("utoipa"));
    a.cursor = be_row(&a);
    press(&mut a, KeyCode::Enter);
    assert_eq!(a.sel.be_libs, vec![BeLib::Utoipa], "ticks with axum");
    assert_eq!(a.plan().be_libs, vec![BeLib::Utoipa]);
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.be_libs.is_empty(), "and unticks");

    a.sel.backend = Backend::Fastapi;
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.be_libs.is_empty());
    assert!(a.status.contains("axum"), "{}", a.status);

    a.sel.be_libs = vec![BeLib::Utoipa];
    assert!(
        a.plan().be_libs.is_empty(),
        "the plan drops it without axum"
    );
    a.sel.backend = Backend::None;
    assert!(a.plan().be_libs.is_empty());
}

#[test]
fn each_backend_ticks_its_own_migrations_and_both_need_postgres() {
    let mut a = app();
    let row = |a: &Wizard, lib: BeLib| a.rows.iter().position(|r| *r == Row::BeLib(lib)).unwrap();
    let s = screen(&a);
    assert!(
        s.contains("sqlx migrations") && s.contains("Alembic migrations"),
        "{s}"
    );

    // axum with Postgres, as recommended: sqlx ticks, Alembic does not.
    a.cursor = row(&a, BeLib::Sqlx);
    press(&mut a, KeyCode::Enter);
    assert_eq!(a.sel.be_libs, vec![BeLib::Sqlx]);
    assert!(a.requires().contains(&"sqlx"), "its actions run sqlx-cli");
    a.cursor = row(&a, BeLib::Alembic);
    press(&mut a, KeyCode::Enter);
    assert_eq!(a.sel.be_libs, vec![BeLib::Sqlx]);
    assert!(a.status.contains("FastAPI"), "{}", a.status);

    // FastAPI: Alembic ticks, and the plan drops sqlx.
    a.sel.backend = Backend::Fastapi;
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.be_libs.contains(&BeLib::Alembic));
    assert_eq!(a.plan().be_libs, vec![BeLib::Alembic]);

    // Without Postgres there is nothing to migrate.
    a.sel.services.retain(|s| *s != Service::Postgres);
    assert!(a.plan().be_libs.is_empty());
    a.sel.be_libs.clear();
    press(&mut a, KeyCode::Enter);
    assert!(a.sel.be_libs.is_empty());
    assert!(a.status.contains("Postgres"), "{}", a.status);
}
