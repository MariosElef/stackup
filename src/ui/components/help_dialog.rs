use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};

use super::modal;
use crate::ui::theme::CYAN;

const WIZARD: &[(&str, &str)] = &[
    ("j k / ↑ ↓", "move (5j moves five)"),
    ("<C-h> <C-l>", "focus the menu / the preview"),
    ("h l", "same, one pane left / right"),
    (
        "<Space> <CR> x",
        "select a radio option, tick a box, or create",
    ),
    ("gg G", "top / bottom"),
    ("<C-d> <C-u>", "half a page"),
    (
        "{ }",
        "previous / next section (<C-k> <C-j> where supported)",
    ),
    ("H L  <Tab>", "switch the preview tab"),
    ("/ n N", "search the menu, next, previous"),
    (":", ":q back · :wq create · :help"),
    ("q  <C-c>", "back without creating anything"),
];

const APP: &[(&str, &str)] = &[
    ("j k / ↑ ↓", "move"),
    ("gg G", "top / bottom"),
    (
        ": <C-p>",
        "the command palette: every command, filtered as you type",
    ),
    ("<CR> l <Space>", "open, or run the selected action"),
    ("h q <Esc>", "back one page; on the home page, quit"),
    ("s n", "start a new project"),
    (
        "a",
        "add projects that already exist: a folder, or a folder of them",
    ),
    ("p", "my projects (from the home page)"),
    ("d", "stop listing a project (its files stay)"),
    ("r", "remove a project: its containers, data and folder"),
    (
        "o",
        "open the project's app in the browser (see its addresses beside it)",
    ),
    (
        "e",
        "switch environment (local, dev, prod...) or make a new one",
    ),
    ("<C-]>", "leave a running action: it is stopped"),
    ("<C-c>", "quit (a running action gets it instead)"),
];

/// The wizard's keymap, centered over the whole screen.
pub fn render_wizard(f: &mut Frame, screen: Rect) {
    render(f, screen, WIZARD);
}

/// The menu's keymap.
pub fn render_app(f: &mut Frame, screen: Rect) {
    render(f, screen, APP);
}

fn render(f: &mut Frame, screen: Rect, entries: &[(&str, &str)]) {
    let lines: Vec<Line> = entries
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<16}"), Style::new().fg(CYAN).bold()),
                Span::raw((*d).to_string()),
            ])
        })
        .collect();
    modal::render(f, screen, "keymaps", lines, "any key closes", Style::new());
}
