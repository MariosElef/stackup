//! The panels under a project's card in the list: its recent commits, its services, the room it
//! takes, and the keys that work here. Each is one bordered box, so the right side reads as
//! separate things at a glance.
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::project_about::{GRAY, pill, tech_color};
use crate::{
    services::projects::{
        disk::{self, Usage},
        overview::{Commit, Overview, Service},
    },
    ui::{
        stores::app::App,
        theme::{BLUE, CYAN, GREEN, RED, YELLOW, pane_border},
    },
};

fn panel(title: Line<'static>) -> Block<'static> {
    Block::bordered()
        .border_style(pane_border(false))
        .title(title)
}

fn plain_title(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {text} "),
        Style::new().fg(BLUE).add_modifier(Modifier::BOLD),
    ))
}

/// A line cut to `width` characters, ending in `…` when it was longer.
fn clip(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let kept: String = text.chars().take(width.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// The branch with what is changed, as the title of the commits panel.
fn git_title(overview: Option<&Overview>) -> Line<'static> {
    let mut spans = vec![Span::styled(" git ", Style::new().fg(BLUE).bold())];
    if let Some(git) = overview.and_then(|o| o.git.as_ref()) {
        spans.push(pill(&git.branch, CYAN));
        if git.dirty > 0 {
            spans.push(Span::styled(
                format!(" ✱{} changed", git.dirty),
                Style::new().fg(YELLOW).bold(),
            ));
        } else if git.commits {
            spans.push(Span::styled(" ✓ clean", Style::new().fg(GREEN)));
        }
        if git.ahead > 0 {
            spans.push(Span::styled(
                format!(" ↑{} to push", git.ahead),
                Style::new().fg(GREEN),
            ));
        }
        if git.behind > 0 {
            spans.push(Span::styled(
                format!(" ↓{} to pull", git.behind),
                Style::new().fg(RED),
            ));
        }
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

fn commit_line(c: &Commit, width: usize) -> Line<'static> {
    // The hash and the age take their room first; the subject gets what is left.
    let room = width.saturating_sub(c.hash.len() + c.when.chars().count() + 5);
    Line::from(vec![
        Span::styled(format!(" {} ", c.hash), Style::new().fg(YELLOW)),
        Span::raw(clip(&c.subject, room)),
        Span::styled(format!("  {}", c.when), Style::new().fg(GRAY)),
    ])
}

/// The latest commits, with the branch and what is changed in the title.
pub fn commits(f: &mut Frame, area: Rect, overview: Option<&Overview>) {
    let block = panel(git_title(overview));
    let width = usize::from(area.width.saturating_sub(2));
    let lines: Vec<Line> = match overview {
        Some(o) if o.git.is_none() => vec![Line::from(Span::styled(
            " Not a git repository yet: choose init.",
            Style::new().fg(GRAY),
        ))],
        Some(o) if o.recent.is_empty() => vec![Line::from(Span::styled(
            " No commits yet.",
            Style::new().fg(GRAY),
        ))],
        Some(o) => {
            let mut lines: Vec<Line> = o.recent.iter().map(|c| commit_line(c, width)).collect();
            if let Some(remote) = o.git.as_ref().and_then(|g| g.remote.as_ref()) {
                lines.push(Line::from(Span::styled(
                    format!(" ⇄ {}", clip(remote, width.saturating_sub(4))),
                    Style::new().fg(GRAY),
                )));
            }
            lines
        }
        None => vec![],
    };
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn service_line(s: &Service) -> Line<'static> {
    let (dot, word, color) = if s.up {
        ("●", "up", GREEN)
    } else if s.port.is_some() {
        ("○", "down", GRAY)
    } else {
        ("·", "", GRAY)
    };
    Line::from(vec![
        Span::styled(format!(" {dot} "), Style::new().fg(color).bold()),
        Span::styled(
            format!("{:<10}", s.name),
            Style::new()
                .fg(tech_color(&s.name))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{:<7}",
                s.port.as_ref().map_or(String::new(), |p| format!(":{p}"))
            ),
            Style::new().fg(GRAY),
        ),
        Span::styled(word.to_string(), Style::new().fg(color)),
    ])
}

/// Every container of the project, and whether something answers on its port.
pub fn services(f: &mut Frame, area: Rect, overview: Option<&Overview>) {
    let (up, of) = overview.map_or((0, 0), Overview::up_count);
    let title = Line::from(vec![
        Span::styled(" services ", Style::new().fg(BLUE).bold()),
        Span::styled(format!("{up}/{of} up "), Style::new().fg(GRAY)),
    ]);
    let lines: Vec<Line> = match overview {
        Some(o) if o.services.is_empty() => vec![Line::from(Span::styled(
            " No services in the compose file.",
            Style::new().fg(GRAY),
        ))],
        Some(o) => o.services.iter().map(service_line).collect(),
        None => vec![],
    };
    f.render_widget(Paragraph::new(lines).block(panel(title)), area);
}

/// A bar `value` long out of `whole`, `width` cells wide.
fn bar(value: u64, whole: u64, width: usize) -> String {
    let filled = if whole == 0 {
        0
    } else {
        ((value as f64 / whole as f64) * width as f64).round() as usize
    };
    let filled = filled.min(width).max(usize::from(value > 0));
    format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}

fn usage_line(
    name: &str,
    bytes: u64,
    whole: u64,
    width: usize,
    color: ratatui::style::Color,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {name:<13}"), Style::new().fg(GRAY)),
        Span::styled(bar(bytes, whole, width), Style::new().fg(color)),
        Span::styled(format!("  {}", disk::human(bytes)), Style::new().bold()),
    ])
}

/// Where the project's room goes. Measured on another thread: until it is back, it says so.
pub fn room(f: &mut Frame, area: Rect, app: &App, usage: Option<&Usage>) {
    let block = panel(plain_title("room it takes"));
    let Some(u) = usage else {
        let dots = ".".repeat(usize::try_from(app.tick / 4 % 4).unwrap_or(0));
        let text = Line::from(Span::styled(
            format!(" measuring{dots}"),
            Style::new().fg(GRAY),
        ));
        return f.render_widget(Paragraph::new(text).block(block), area);
    };
    let width = usize::from(area.width.saturating_sub(28)).clamp(8, 40);
    let mut lines = vec![usage_line("the project", u.total, u.total, width, CYAN)];
    for (name, bytes, color) in [
        ("build files", u.target, YELLOW),
        ("node_modules", u.node_modules, YELLOW),
        (".git", u.git, GREEN),
    ] {
        if let Some(bytes) = bytes {
            lines.push(usage_line(name, bytes, u.total, width, color));
        }
    }
    if u.target.is_some_and(|t| t > 0) {
        lines.push(Line::from(Span::styled(
            " Open the project and choose cargo clean to free the build files.",
            Style::new().fg(GRAY),
        )));
    }
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn key(text: &str) -> Span<'static> {
    pill(text, CYAN)
}

fn hint(text: &str) -> Span<'static> {
    Span::styled(format!(" {text}   "), Style::new().fg(GRAY))
}

/// The keys that work on this page, as keycaps.
pub fn keys(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(vec![
            key("Enter"),
            hint("open it: up, dev, lazygit, deploy"),
            key("o"),
            hint("open the app"),
        ]),
        Line::from(vec![
            key("e"),
            hint("environments"),
            key("r"),
            hint("remove"),
            key("d"),
            hint("stop listing"),
        ]),
        Line::from(vec![
            key("n"),
            hint("new project"),
            key("a"),
            hint("add existing"),
            key(":"),
            hint("every command"),
        ]),
    ];
    f.render_widget(
        Paragraph::new(lines).block(panel(plain_title("keys"))),
        area,
    );
}
