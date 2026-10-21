use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::models::Action;
use crate::ui::{
    components::output,
    features::projects::{action_about, action_list, urls_strip},
    stores::app::App,
    types::Page,
};

/// The open project's actions on the left; on the right what the selected one does, or, once it
/// runs, its output. The list stays where it was, as in mc2.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    // lazygit needs room: while it runs it has the whole page.
    if app.page == Page::Run && matches!(app.after, Some((Action::GitOpen, _))) {
        return output::render(f, area, app);
    }
    let [left, right] =
        Layout::horizontal([Constraint::Length(24), Constraint::Min(40)]).areas(area);
    action_list::render(f, left, app);
    // Under what the right side shows, where the project can be reached.
    let strip = urls_strip::height(app, right.width);
    let [top, bottom] =
        Layout::vertical([Constraint::Min(5), Constraint::Length(strip)]).areas(right);
    if app.page == Page::Run {
        output::render(f, top, app);
    } else {
        action_about::render(f, top, app);
    }
    if strip > 0 {
        urls_strip::render(f, bottom, app);
    }
}
