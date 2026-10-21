use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::modal;
use crate::{
    services::toolchain::versions::Question,
    ui::{
        stores::app::ChoiceModal,
        theme::{CYAN, GREEN, YELLOW, tier_color},
    },
};

/// "Which version?": one option per line with a note under it, the recommended one first. The
/// numbers are shortcuts.
pub fn render(f: &mut Frame, screen: Rect, state: &ChoiceModal) {
    let is_tier = state.question == Question::Tier;
    if is_tier {
        return render_environments(f, screen, state);
    }
    let mut lines = vec![
        Line::from(Span::styled(
            format!(" {}", state.question.title()),
            Style::new().bold(),
        )),
        Line::from(Span::styled(
            format!(" {}", state.question.why()),
            Style::new().dim(),
        )),
        Line::from(""),
    ];
    for (i, option) in state.options.iter().enumerate() {
        let selected = i == state.selected;
        let marker = if selected { "(•)" } else { "( )" };
        let head = format!(" {marker} {}  {}", i + 1, option.label);
        let head_style = if selected {
            Style::new()
                .fg(GREEN)
                .bold()
                .add_modifier(Modifier::REVERSED)
        } else if is_tier && option.value != "+new" {
            // An environment shows in its own color, so prod reads as prod.
            Style::new().fg(tier_color(&option.value))
        } else {
            Style::new()
        };
        // For versions the first is the recommended one; environments and branches have no such
        // thing: the first branch is only where you are.
        let recommended = if i == 0 { "  recommended" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(head, head_style),
            Span::styled(recommended, Style::new().fg(CYAN)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("        {}", option.note),
            Style::new().dim(),
        )));
    }
    if !state.next.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {} more question to answer after this.", state.next.len()),
            Style::new().dim(),
        )));
    }
    modal::render(
        f,
        screen,
        match state.question {
            Question::Tier => "choose an environment",
            Question::Visibility => "new repository",
            Question::RemoveFrom => "remove from",
            Question::VolumeService => "service",
            Question::Volume => "volume",
            Question::Backup => "backup",
            _ => "choose a version",
        },
        lines,
        "↑/↓ choose · 1-9 pick · Enter confirm · Esc back",
        Style::new().fg(YELLOW).bold(),
    );
}

/// The environments view: what is active at the top, what Docker reads, then every environment on
/// its own row with its file, and a row to make another.
fn render_environments(f: &mut Frame, screen: Rect, state: &ChoiceModal) {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let dim = Style::new().add_modifier(Modifier::DIM);
    let active = state.active.as_deref();

    let mut lines = vec![Line::from(match active {
        Some("custom") => vec![
            Span::styled(" Active now:  ", bold),
            Span::styled(
                "custom",
                Style::new()
                    .fg(tier_color("custom"))
                    .add_modifier(Modifier::BOLD),
            ),
        ],
        Some(t) => vec![
            Span::styled(" Active now:  ", bold),
            Span::styled("● ", Style::new().fg(tier_color(t))),
            Span::styled(
                t.to_string(),
                Style::new().fg(tier_color(t)).add_modifier(Modifier::BOLD),
            ),
        ],
        None => vec![
            Span::styled(" Active now:  ", bold),
            Span::styled("nothing yet", dim),
        ],
    })];
    lines.push(Line::from(Span::styled(
        match active {
            Some("custom") => {
                " .env was edited by hand: it matches none of these. Switching replaces it."
                    .to_string()
            }
            Some(t) => format!(" Docker Compose reads .env, which is a copy of .env.{t}."),
            None => " There is no .env yet: pick one to make it.".to_string(),
        },
        dim,
    )));
    lines.push(Line::from(""));

    let last = state.options.len().saturating_sub(1);
    for (i, option) in state.options.iter().enumerate() {
        let selected = i == state.selected;
        let cursor = if selected { "▶" } else { " " };
        let row = Style::new().add_modifier(if selected {
            Modifier::REVERSED
        } else {
            Modifier::empty()
        });
        if option.value == "+new" && i == last {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw(format!(" {cursor} ")),
                Span::styled(format!("+ {}", option.label), row.fg(GREEN)),
                Span::styled(format!("  {}", option.note), dim),
            ]));
            continue;
        }
        let is_active = active == Some(option.value.as_str());
        let color = tier_color(&option.value);
        lines.push(Line::from(vec![
            Span::raw(format!(" {cursor} ")),
            Span::styled(if is_active { "● " } else { "○ " }, Style::new().fg(color)),
            Span::styled(
                format!("{:<10}", option.label),
                row.fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if is_active { " active " } else { "        " },
                Style::new().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(option.note.clone(), dim),
        ]));
    }
    modal::render(
        f,
        screen,
        "environments",
        lines,
        "↑/↓ choose · Enter switch · n new · Esc close",
        Style::new().fg(YELLOW).add_modifier(Modifier::BOLD),
    );
}
