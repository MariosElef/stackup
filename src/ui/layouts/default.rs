use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::ui::{
    components::{command_line, status_bar},
    types::Footer,
};

/// The page fills the screen; the status bar and command line sit underneath it.
pub fn render(f: &mut Frame, footer: &Footer, page: impl FnOnce(&mut Frame, Rect)) {
    let [body, bottom] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(f.area());
    let [top, line] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(bottom);
    page(f, body);
    status_bar::render(f, top, footer);
    command_line::render(f, line, footer);
}
