use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::modal;
use crate::ui::{
    stores::app::AutofixModal,
    theme::{CYAN, GREEN, RED, YELLOW},
};

fn button(label: &str, selected: bool) -> Span<'static> {
    let text = format!(" {label} ");
    if selected {
        Span::styled(
            text,
            Style::new()
                .fg(GREEN)
                .bold()
                .add_modifier(Modifier::REVERSED),
        )
    } else {
        Span::styled(text, Style::new().dim())
    }
}

/// Offered when a run fails with an error stackup recognises: what went wrong, the exact commands
/// that would fix it, and that the failed steps are then run again. Autofix or Close.
pub fn render(f: &mut Frame, screen: Rect, state: &AutofixModal) {
    let mut lines = vec![
        Line::from(Span::styled(" Something went wrong:", Style::new().bold())),
        Line::from(Span::styled(
            format!(" ✗ {}", state.problem),
            Style::new().fg(RED).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(" Autofix will run:", Style::new().bold())),
    ];
    for step in &state.fix {
        lines.push(Line::from(Span::styled(
            format!("   {}", step.command_line()),
            Style::new().fg(CYAN),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " and then try again.",
        Style::new().dim(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw(" "),
        button("Autofix", state.selected == 0),
        Span::raw("  "),
        button("Close", state.selected == 1),
    ]));
    modal::render(
        f,
        screen,
        "autofix",
        lines,
        "←/→ choose · Enter confirm · a autofix · c close",
        Style::new().fg(YELLOW).bold(),
    );
}
