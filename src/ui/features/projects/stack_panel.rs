//! The open project's whole stack, a row per layer: what the frontend and the backend are made
//! with, what reaches the databases, what runs in Docker, and the tooling.
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Padding, Paragraph, Wrap},
};

use super::project_about::{row, tech_color};
use crate::{
    services::projects::stack::Stack,
    ui::{
        stores::app::App,
        theme::{GRAY, RULE, pane, pane_title},
    },
};

/// The rows: each layer that has something. Its main piece (the framework) is in its color, and
/// every service in its own; the rest are plain, with a quiet dot between them.
pub fn lines(stack: &Stack) -> Vec<Line<'static>> {
    [
        ("frontend", &stack.frontend),
        ("backend", &stack.backend),
        ("data", &stack.data),
        ("services", &stack.services),
        ("tools", &stack.tools),
    ]
    .into_iter()
    .filter(|(_, names)| !names.is_empty())
    .map(|(layer, names)| {
        let mut spans = vec![];
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ·  ", Style::new().fg(RULE)));
            }
            if i == 0 && layer != "tools" || layer == "services" {
                spans.push(Span::styled(
                    name.clone(),
                    Style::new().fg(tech_color(name)).bold(),
                ));
            } else {
                spans.push(Span::styled(name.clone(), Style::new().fg(GRAY)));
            }
        }
        row(layer, spans)
    })
    .collect()
}

/// How tall the panel is: a row per layer and the borders.
pub fn height(app: &App) -> u16 {
    let rows = lines(&app.stack).len() as u16;
    if rows == 0 { 0 } else { rows + 2 }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = pane(false)
        .title(pane_title("stack"))
        .padding(Padding::horizontal(1));
    f.render_widget(
        Paragraph::new(lines(&app.stack))
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
