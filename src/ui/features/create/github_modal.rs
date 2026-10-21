use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::ui::{
    components::modal,
    stores::app::GithubModal,
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

fn mark(done: bool, what: &str) -> Line<'static> {
    let (sign, color) = if done { ("✓", GREEN) } else { ("✗", RED) };
    Line::from(vec![
        Span::styled(format!(" {sign} "), Style::new().fg(color).bold()),
        Span::raw(what.to_string()),
    ])
}

/// Asked when Create is pressed and GitHub is not set up: what is missing, exactly what would be
/// run, and that afterwards every terminal here can push without anything more. Set up or Skip.
pub fn render(f: &mut Frame, screen: Rect, state: &GithubModal) {
    let mut lines = vec![
        Line::from(Span::styled(
            " To put this project on GitHub, this computer needs:",
            Style::new().bold(),
        )),
        Line::from(""),
        mark(state.github.signed_in, "the GitHub CLI signed in (gh)"),
        mark(state.github.key.is_some(), "an SSH key, which GitHub knows"),
        Line::from(""),
        Line::from(Span::styled(" Set up will run:", Style::new().bold())),
    ];
    for (i, step) in state.steps.iter().enumerate() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {}. ", i + 1), Style::new().dim()),
            Span::styled(step.label.clone(), Style::new().fg(CYAN)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " It opens GitHub in your browser to sign in, once. The key is saved in ~/.ssh and ssh is told to use it for github.com, so afterwards `git push` works from any terminal here, as if it had always been set up. Skip, and the project is still made: you can set GitHub up later.",
        Style::new().dim(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw(" "),
        button("Set up", state.selected == 0),
        Span::raw("  "),
        button("Skip", state.selected == 1),
    ]));
    modal::render(
        f,
        screen,
        "set up GitHub",
        lines,
        "←/→ choose · Enter confirm · s set up · k skip",
        Style::new().fg(YELLOW).bold(),
    );
}
