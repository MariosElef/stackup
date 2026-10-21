use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::ui::components::modal;
use crate::ui::{
    stores::app::TierModal,
    theme::{CYAN, GREEN, YELLOW, tier_color},
};

fn button(label: &str, selected: bool) -> Span<'static> {
    let text = format!(" {label} ");
    if selected {
        Span::styled(
            text,
            Style::new()
                .fg(GREEN)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
    } else {
        Span::styled(text, Style::new().add_modifier(Modifier::DIM))
    }
}

/// What making an environment did: which passwords are new, what is still the template's and
/// needs a look, and the choice to switch to it.
pub fn render(f: &mut Frame, screen: Rect, state: &TierModal) {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let dim = Style::new().add_modifier(Modifier::DIM);
    let mut lines = vec![Line::from(vec![
        Span::styled(" Made ", bold),
        Span::styled(
            format!(".env.{}", state.tier),
            Style::new()
                .fg(tier_color(&state.tier))
                .add_modifier(Modifier::BOLD),
        ),
    ])];
    lines.push(Line::from(""));
    if !state.created.generated.is_empty() {
        lines.push(Line::from(Span::styled(" New random values for:", bold)));
        for key in &state.created.generated {
            lines.push(Line::from(Span::styled(
                format!("   {key}"),
                Style::new().fg(GREEN),
            )));
        }
        lines.push(Line::from(""));
    }
    if !state.created.review.is_empty() {
        lines.push(Line::from(Span::styled(
            " Still the template's: still says localhost, or is empty. Look at:",
            Style::new().fg(YELLOW).add_modifier(Modifier::BOLD),
        )));
        for key in &state.created.review {
            lines.push(Line::from(Span::styled(
                format!("   {key}"),
                Style::new().fg(CYAN),
            )));
        }
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        " The file is readable only by you, and gitignored.",
        dim,
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw(" "),
        button("Switch to it", state.selected == 0),
        Span::raw("  "),
        button("Close", state.selected == 1),
    ]));
    modal::render(
        f,
        screen,
        "new environment",
        lines,
        "←/→ choose · Enter confirm · s switch · c close",
        Style::new().fg(GREEN).add_modifier(Modifier::BOLD),
    );
}
