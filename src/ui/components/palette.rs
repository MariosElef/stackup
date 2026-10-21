use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use crate::ui::{
    stores::app::Palette,
    theme::{CYAN, GREEN, YELLOW},
};

/// How many entries are shown at once.
const ROWS: usize = 10;

/// The palette, near the top: what is typed, then the entries that match, the closest first.
pub fn render(f: &mut Frame, screen: Rect, palette: &Palette) {
    let width = 80.min(screen.width);
    let shown = palette.shown();
    // The window follows the selection, so a long list scrolls.
    let start = palette.selected.saturating_sub(ROWS - 1);
    let rows = shown.len().clamp(1, ROWS);
    let area = Rect {
        x: screen.x + screen.width.saturating_sub(width) / 2,
        y: screen.y + 1.min(screen.height.saturating_sub(1)),
        width,
        height: (rows as u16 + 3).min(screen.height),
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(" : ", Style::new().fg(YELLOW).bold()),
        Span::raw(format!("{}█", palette.query)),
    ])];
    if shown.is_empty() {
        lines.push(Line::from(Span::styled(
            "   nothing matches",
            Style::new().dim(),
        )));
    }
    for (i, e) in shown.iter().enumerate().skip(start).take(ROWS) {
        let selected = i == palette.selected;
        let label = format!(" {:<34}", e.label);
        let style = if selected {
            Style::new()
                .fg(GREEN)
                .bold()
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
        };
        lines.push(Line::from(vec![
            Span::styled(label, style),
            Span::styled(format!(" {}", e.note), Style::new().fg(CYAN).dim()),
        ]));
    }
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .border_style(Style::new().fg(YELLOW).bold())
                .title(" commands ")
                .title_bottom(format!(" {} ", shown.len())),
        ),
        area,
    );
}
