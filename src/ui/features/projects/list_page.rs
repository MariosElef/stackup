use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::{
    models::Facts,
    ui::{
        features::projects::{project_about, project_list, project_panels, stack_banner},
        stores::app::App,
    },
};

/// The projects on the left; on the right the selected one's card, and under it panels with its
/// commits, its services, the room it takes and the keys.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let [left, right] =
        Layout::horizontal([Constraint::Length(44), Constraint::Min(30)]).areas(area);
    project_list::render(f, left, app);

    let selected = app.project().map(|p| p.path.clone());
    let overview = app
        .overview
        .as_ref()
        .filter(|(path, _)| Some(path) == selected.as_ref())
        .map(|(_, o)| o);
    let usage = selected.as_ref().and_then(|p| app.usage.get(p));

    // The banner only when there is room for it too: the panels under it matter more.
    let banner = if right.height >= 38 {
        stack_banner::HEIGHT
    } else {
        0
    };
    let [banner_area, columns, keys] = Layout::vertical([
        Constraint::Length(banner),
        Constraint::Min(14),
        Constraint::Length(5),
    ])
    .areas(right);
    if banner > 0 {
        stack_banner::render(
            f,
            banner_area,
            &Facts::read(selected.as_deref().unwrap_or(std::path::Path::new("."))),
            app.tick,
        );
    }
    // Two columns: what the project is and what it did on the left, what it runs and what it
    // takes on the right.
    let [left_col, right_col] =
        Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)]).areas(columns);
    let [card, commits] = Layout::vertical([
        Constraint::Length(project_about::height(app, left_col.width)),
        Constraint::Min(4),
    ])
    .areas(left_col);
    let [services, room] =
        Layout::vertical([Constraint::Length(9), Constraint::Min(5)]).areas(right_col);
    project_about::render(f, card, app);
    project_panels::commits(f, commits, overview);
    project_panels::services(f, services, overview);
    project_panels::room(f, room, app, usage);
    project_panels::keys(f, keys);
}
