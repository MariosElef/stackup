//! The open project's whole stack, a row per layer: what the frontend and the backend are made
//! with, what reaches the databases, what runs in Docker, and the tooling.
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

use super::project_about::{GRAY, pill, row, tech_color};
use crate::{
    services::projects::stack::Stack,
    ui::{stores::app::App, theme::pane_border},
};

/// The rows: each layer that has something, the first name as a badge and the rest after it.
pub fn lines(stack: &Stack) -> Vec<Line<'static>> {
    [
        ("frontend", &stack.frontend),
        ("backend", &stack.backend),
        ("ORM/drivers", &stack.data),
        ("services", &stack.services),
        ("tools", &stack.tools),
    ]
    .into_iter()
    .filter(|(_, names)| !names.is_empty())
    .map(|(layer, names)| {
        let mut spans = vec![];
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ", Style::new().fg(GRAY)));
            }
            // A layer's main piece (the framework) stands out; services each get their color.
            if i == 0 && layer != "tools" || layer == "services" {
                spans.push(pill(name, tech_color(name)));
            } else {
                spans.push(Span::raw(name.clone()));
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
    let block = Block::bordered()
        .border_style(pane_border(false))
        .title(Line::from(Span::styled(" stack ", Style::new().fg(GRAY))));
    f.render_widget(
        Paragraph::new(lines(&app.stack))
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
