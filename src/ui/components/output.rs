use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Block,
};
use tui_term::widget::PseudoTerminal;

use crate::ui::{
    stores::{app::App, run::Run},
    theme::{GREEN, RED, YELLOW, pane_border},
};

fn title(run: &Run) -> Line<'static> {
    let mut spans = vec![Span::raw(format!(" {} ", run.title))];
    match &run.ended {
        Some(how) => {
            let color = if how == "done" { GREEN } else { RED };
            spans.push(Span::styled(
                format!("· {how} · q back "),
                Style::new().fg(color),
            ));
        }
        None => spans.push(Span::styled(
            format!("● running {} · Ctrl-] back to the menu ", run.progress()),
            Style::new().fg(YELLOW),
        )),
    }
    Line::from(spans)
}

/// The running action's terminal, sized to the inside of the pane so a program wraps where the
/// pane does.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(run) = &app.run else { return };
    app.pane
        .set((area.height.saturating_sub(2), area.width.saturating_sub(2)));
    let block = Block::bordered()
        .border_style(pane_border(true))
        .title(title(run));
    f.render_widget(PseudoTerminal::new(run.term.screen()).block(block), area);
}
