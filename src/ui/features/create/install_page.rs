use ratatui::{Frame, layout::Rect};

use crate::ui::{components::output, stores::app::App};

/// A tool being installed: the whole screen is its output, since there is nothing to keep beside it.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    output::render(f, area, app);
}
