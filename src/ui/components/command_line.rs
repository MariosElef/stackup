use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::ui::{
    theme::RED,
    types::{Footer, FooterLine},
};

/// What is being typed, else the last message, else a hint.
pub fn render(f: &mut Frame, area: Rect, footer: &Footer) {
    let line = match &footer.line {
        FooterLine::Input(text) => Line::from(text.clone()),
        FooterLine::Status(text) => Line::from(Span::styled(
            format!(" {text}"),
            Style::new().fg(RED).bold(),
        )),
        FooterLine::Hint(text) => Line::from(Span::styled(*text, Style::new().dim())),
    };
    f.render_widget(Paragraph::new(line), area);
}
