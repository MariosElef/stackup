use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

use crate::{
    models::{Backend, Facts, Frontend},
    services::projects::{
        envs,
        overview::{Overview, State},
    },
    ui::{
        stores::app::App,
        theme::{BLUE, CYAN, GREEN, INK, PINK, RED, YELLOW, pane_border, tier_color},
    },
};

pub(super) const GRAY: Color = Color::Indexed(244);

/// A word on a colored background, like a badge.
pub(super) fn pill(text: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::new().fg(INK).bg(color).add_modifier(Modifier::BOLD),
    )
}

/// A badge that is on, in color, or off, dim.
pub(super) fn flag(text: &str, on: bool) -> Span<'static> {
    if on {
        pill(&format!("{text} ✓"), GREEN)
    } else {
        Span::styled(format!(" {text} ✗ "), Style::new().fg(GRAY))
    }
}

/// The color a technology is known by, so a stack reads at a glance.
pub(super) fn tech_color(name: &str) -> Color {
    match name
        .split(" (")
        .next()
        .unwrap_or(name)
        .to_lowercase()
        .as_str()
    {
        "nuxt" | "vue" | "mongo" | "mongodb" => Color::Indexed(41),
        "topcoat" => PINK,
        "axum" => Color::Indexed(208),
        "fastapi" => Color::Indexed(37),
        "postgres" => Color::Indexed(33),
        "redis" => Color::Indexed(160),
        "keycloak" => Color::Indexed(99),
        "caddy" => BLUE,
        "mailtrap" => Color::Indexed(172),
        _ => CYAN,
    }
}

/// A path with the home folder as `~`, so it fits on a line.
pub(super) fn tilde(path: &std::path::Path) -> String {
    match crate::clients::os::home_dir().and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

pub(super) fn heading(text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(" ▍", Style::new().fg(BLUE)),
        Span::styled(
            text.to_uppercase(),
            Style::new().fg(BLUE).add_modifier(Modifier::BOLD),
        ),
    ])
}

pub(super) fn label(text: &str) -> Span<'static> {
    Span::styled(format!("  {text:<12}"), Style::new().fg(GRAY))
}

pub(super) fn row(name: &str, value: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = vec![label(name)];
    spans.extend(value);
    Line::from(spans)
}

/// The state of the whole project, as one colored line at the top.
pub(super) fn state_line(overview: Option<&Overview>) -> Line<'static> {
    let Some(o) = overview else {
        return Line::from("");
    };
    let (up, of) = o.up_count();
    let (dot, word, color) = match o.state() {
        State::Running => ("●", "running", GREEN),
        State::Partly => ("◐", "partly running", YELLOW),
        State::Stopped => ("○", "stopped", GRAY),
    };
    let mut spans = vec![
        Span::styled(format!(" {dot} "), Style::new().fg(color).bold()),
        Span::styled(word.to_string(), Style::new().fg(color).bold()),
    ];
    if of > 0 {
        spans.push(Span::styled(
            format!("   {up}/{of} services answering"),
            Style::new().fg(GRAY),
        ));
    }
    Line::from(spans)
}

/// What the card says: its lines, and the color of its frame and title.
struct Card {
    name: String,
    border: Color,
    lines: Vec<Line<'static>>,
}

/// The selected project's card, or nothing when there is none selected.
fn card(app: &App) -> Option<Card> {
    let p = app.project()?;
    let facts = Facts::read(&p.path);
    let overview = app
        .overview
        .as_ref()
        .filter(|(path, _)| *path == p.path)
        .map(|(_, o)| o);
    // The frame takes the color of the project's state, so it shows from across the room.
    let border = match overview.map(Overview::state) {
        Some(State::Running) if facts.exists => GREEN,
        Some(State::Partly) if facts.exists => YELLOW,
        _ if !facts.exists => RED,
        _ => GRAY,
    };
    let mut lines = vec![state_line(overview.filter(|_| facts.exists))];
    lines.push(row(
        "folder",
        vec![Span::styled(tilde(&p.path), Style::new().fg(GRAY))],
    ));
    if !facts.exists {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " This folder is gone, or has no docker-compose.yml. Press d to stop listing it.",
            Style::new().fg(RED).bold(),
        )));
    } else {
        lines.push(Line::from(""));
        lines.push(heading("Stack"));
        let mut stack = vec![];
        if facts.frontend != Frontend::None {
            let name = facts.frontend.to_string();
            stack.push(pill(&name, tech_color(&name)));
            stack.push(Span::raw("  "));
        }
        if facts.backend != Backend::None {
            let name = facts.backend.to_string();
            stack.push(pill(&name, tech_color(&name)));
        }
        if stack.is_empty() {
            stack.push(Span::styled("none", Style::new().fg(GRAY)));
        }
        lines.push(row("built with", stack));
        lines.push(row(
            "features",
            vec![
                flag("https", facts.https),
                Span::raw(" "),
                flag("ci", facts.ci),
                Span::raw(" "),
                flag("hooks", facts.hooks),
            ],
        ));
        lines.push(Line::from(""));
        lines.push(heading("Environment"));
        let env = envs::current(&p.path);
        let others: Vec<String> = envs::tiers(&p.path)
            .into_iter()
            .filter(|t| env.as_deref() != Some(t.as_str()))
            .collect();
        lines.push(row(
            "active",
            vec![match &env {
                Some(t) => pill(t, tier_color(t)),
                None => Span::styled("none yet", Style::new().fg(GRAY)),
            }],
        ));
        if !others.is_empty() {
            lines.push(row(
                "also",
                vec![Span::styled(others.join(", "), Style::new().fg(GRAY))],
            ));
        }
        lines.push(row(
            "deploys to",
            vec![match &p.host {
                Some(host) => Span::styled(host.clone(), Style::new().fg(GREEN)),
                None => Span::styled("not yet", Style::new().fg(GRAY)),
            }],
        ));
    }
    Some(Card {
        name: p.name.clone(),
        border,
        lines,
    })
}

/// How tall the card needs to be at `width` columns: each line, wrapped, and the frame.
pub fn height(app: &App, width: u16) -> u16 {
    let Some(card) = card(app) else { return 3 };
    let inner = usize::from(width.saturating_sub(2)).max(1);
    let rows: usize = card
        .lines
        .iter()
        .map(|l| l.width().div_ceil(inner).max(1))
        .sum();
    u16::try_from(rows + 2).unwrap_or(u16::MAX)
}

/// What the selected project is: its state, its stack, and where it deploys.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(card) = card(app) else {
        return f.render_widget(
            Block::bordered()
                .border_style(pane_border(false))
                .title(" about "),
            area,
        );
    };
    let block = Block::bordered()
        .border_style(Style::new().fg(card.border))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(card.name, Style::new().fg(card.border).bold()),
            Span::raw(" "),
        ]));
    f.render_widget(
        Paragraph::new(card.lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
