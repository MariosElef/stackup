use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

use crate::ui::{
    stores::app::App,
    theme::{CYAN, pane_border},
};

/// What the selected action does, and what it will ask for.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered()
        .border_style(pane_border(false))
        .title(" what it does ");
    let Some(action) = app.actions.get(app.action) else {
        let text = if app.facts.as_ref().is_some_and(|f| !f.exists) {
            " This project's folder is missing."
        } else {
            " Nothing to do here."
        };
        return f.render_widget(Paragraph::new(text).block(block), area);
    };
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" {}", action.about()),
            Style::new().fg(CYAN).bold(),
        )),
        Line::from(""),
        Line::from(format!(" {}", action.details())),
    ];
    if !action.params().is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " It asks before it runs.",
            Style::new().dim(),
        )));
    }
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
