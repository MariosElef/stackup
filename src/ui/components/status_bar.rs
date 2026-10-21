use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::ui::{
    theme::{CYAN, GREEN, INK, RED},
    types::Footer,
};

/// The lualine-style bar: mode, where you are, pending keys, and which tools are installed.
pub fn render(f: &mut Frame, area: Rect, footer: &Footer) {
    let mut spans = vec![
        Span::styled(
            footer.mode,
            Style::new().fg(INK).bg(footer.mode_color).bold(),
        ),
        Span::raw(format!(" {} ", footer.focus)),
    ];
    if !footer.pending.is_empty() {
        spans.push(Span::styled(
            format!(" {} ", footer.pending),
            Style::new().fg(INK).bg(CYAN),
        ));
    }
    if !footer.needs.is_empty() {
        spans.push(Span::styled("  needs: ", Style::new().dim()));
    }
    for (bin, installed) in &footer.needs {
        let (mark, color) = if *installed {
            ("✓", GREEN)
        } else {
            ("✗", RED)
        };
        spans.push(Span::raw(format!("{bin} ")));
        spans.push(Span::styled(format!("{mark}  "), Style::new().fg(color)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
