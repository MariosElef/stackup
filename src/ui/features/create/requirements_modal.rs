use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::ui::components::modal;
use crate::{
    clients::run::install_hint,
    ui::{
        features::create::use_requirements::needed_for,
        stores::app::InstallModal,
        theme::{CYAN, GREEN, RED, YELLOW},
    },
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

/// Asked when Create is pressed and something the stack needs is missing: what, what it is for,
/// and what would be run to install it. Install or Close, so nothing is installed by surprise.
pub fn render(f: &mut Frame, screen: Rect, modal_state: &InstallModal) {
    let mut lines = vec![
        Line::from(Span::styled(
            " To create this project you need to install:",
            Style::new().bold(),
        )),
        Line::from(""),
    ];
    for &bin in &modal_state.missing {
        lines.push(Line::from(vec![
            Span::styled(" ✗ ", Style::new().fg(RED).bold()),
            Span::styled(format!("{bin:<9}"), Style::new().fg(RED).bold()),
            Span::styled(format!("for {}", needed_for(bin)), Style::new().dim()),
        ]));
        let step = modal_state
            .plan
            .steps
            .iter()
            .find(|s| s.label == format!("install {bin}"));
        let how = match step {
            Some(step) => format!("     will run: {}", step.command_line()),
            None => format!("     by hand: {}", install_hint(bin)),
        };
        lines.push(Line::from(Span::styled(how, Style::new().fg(CYAN))));
        lines.push(Line::from(""));
    }
    if modal_state.missing.contains(&"topcoat") {
        lines.push(Line::from(Span::styled(
            " The Topcoat CLI needs Rust 1.99 or newer: if the install says otherwise, run `rustup update`.",
            Style::new().dim(),
        )));
        lines.push(Line::from(""));
    }
    let can = modal_state.can_install();
    let mut buttons = vec![Span::raw(" ")];
    if can {
        buttons.push(button("Install", modal_state.selected == 0));
        buttons.push(Span::raw("  "));
    }
    buttons.push(button("Close", modal_state.selected == 1 || !can));
    lines.push(Line::from(buttons));
    let keys = if can {
        "←/→ choose · Enter confirm · i install · c close"
    } else {
        "Enter close"
    };
    modal::render(
        f,
        screen,
        "missing tools",
        lines,
        keys,
        Style::new().fg(YELLOW).bold(),
    );
}
