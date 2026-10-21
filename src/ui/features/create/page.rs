use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::ui::features::create::{menu, preview, wizard::Wizard};

/// The only page: the menu beside the live preview.
pub fn render(f: &mut Frame, area: Rect, w: &Wizard) {
    let [left, right] =
        Layout::horizontal([Constraint::Length(42), Constraint::Min(30)]).areas(area);
    menu::render(f, left, w);
    preview::render(f, right, w);
}
