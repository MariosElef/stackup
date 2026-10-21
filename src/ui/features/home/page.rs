use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    widgets::Paragraph,
};

use crate::ui::{
    composables::use_app_keys::HOME_ITEMS,
    features::home::{home_list, logo},
    stores::app::App,
};

/// The logo above the menu, centered, as in mc2. A narrow or short terminal gets the menu alone.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let [col] = Layout::horizontal([Constraint::Max(96)])
        .flex(Flex::Center)
        .areas(area);
    let list_height = HOME_ITEMS as u16 + 2;
    let with_logo = col.width >= logo::WIDTH && area.height >= logo::HEIGHT + 2 + list_height;
    if !with_logo {
        let [list] = Layout::vertical([Constraint::Length(list_height)])
            .flex(Flex::Center)
            .areas(col);
        return home_list::render(f, list, app);
    }
    let [top, _, list] = Layout::vertical([
        Constraint::Length(logo::HEIGHT),
        Constraint::Length(2),
        Constraint::Length(list_height),
    ])
    .flex(Flex::Center)
    .areas(col);
    let [mark] = Layout::horizontal([Constraint::Length(logo::WIDTH)])
        .flex(Flex::Center)
        .areas(top);
    f.render_widget(Paragraph::new(logo::lines(app.tick)), mark);
    home_list::render(f, list, app);
}
