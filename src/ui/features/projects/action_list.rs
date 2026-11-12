use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Padding},
};

use crate::{
    models::{Action, Group},
    ui::{
        stores::app::App,
        theme::{CYAN, GREEN, RULE, SELECTED_BG, pane, tier_color},
        types::Page,
    },
};

/// The title: the project's name, which environment is active in that environment's color, and
/// where git is: the branch, what changed, and how far ahead or behind.
fn title(app: &App) -> Line<'static> {
    let mut spans = vec![Span::raw(format!(
        " {}",
        app.project()
            .map_or("actions".to_string(), |p| p.name.clone())
    ))];
    if let Some(env) = &app.env {
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(
            env.clone(),
            Style::new()
                .fg(tier_color(env))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(git) = &app.git {
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(git.branch.clone(), Style::new().fg(CYAN)));
        if git.dirty > 0 {
            spans.push(Span::styled(
                format!(" ✱{}", git.dirty),
                Style::new().fg(ratatui::style::Color::Indexed(220)),
            ));
        }
        if git.ahead > 0 {
            spans.push(Span::styled(
                format!(" ↑{}", git.ahead),
                Style::new().fg(GREEN),
            ));
        }
        if git.behind > 0 {
            spans.push(Span::styled(
                format!(" ↓{}", git.behind),
                Style::new().fg(ratatui::style::Color::Indexed(203)),
            ));
        }
    }
    spans.push(Span::raw(" "));
    Line::from(spans)
}

/// A Nerd Font glyph for each action, so the list can be read at a glance.
fn icon(a: Action) -> &'static str {
    match a {
        Action::Up => "\u{f04b}",
        Action::Down => "\u{f04d}",
        Action::Logs => "\u{f03a}",
        Action::Dev => "\u{f121}",
        Action::Migrate => "\u{f1c0}",
        Action::MigrationNew => "\u{f067}",
        Action::MigrationRevert => "\u{f0e2}",
        Action::Query => "\u{f120}",
        Action::Ps => "\u{f0ca}",
        Action::Stop => "\u{f04c}",
        Action::Images => "\u{f1b2}",
        Action::Clean => "\u{f1f8}",
        Action::Volumes => "\u{f0a0}",
        Action::VolumeAdd => "\u{f0fe}",
        Action::VolumeBackup => "\u{f0c7}",
        Action::VolumeRestore => "\u{f1da}",
        Action::Build => "\u{f0ad}",
        Action::Deploy => "\u{f135}",
        Action::Ci => "\u{f058}",
        Action::Fmt => "\u{f036}",
        Action::CargoClean => "\u{f12d}",
        Action::Env => "\u{f084}",
        Action::Certs => "\u{f023}",
        Action::GitOpen => "\u{f1d3}",
        Action::GitInit => "\u{f126}",
        Action::GitRepo => "\u{f09b}",
        Action::Hooks => "\u{f132}",
        Action::Doctor => "\u{f0f1}",
        Action::Remove => "\u{f00d}",
    }
}

/// A section's divider: its name, then a thin rule to the edge.
fn divider(label: &str, width: usize) -> Line<'static> {
    let rule = width.saturating_sub(label.chars().count() + 1);
    Line::from(vec![
        Span::styled(
            label.to_string(),
            Style::new().fg(CYAN).add_modifier(Modifier::DIM),
        ),
        Span::styled(format!(" {}", "─".repeat(rule)), Style::new().fg(RULE)),
    ])
}

/// What can be done to the open project, in sections: Run, Build & ship, Environment, Git,
/// Project. It stays where it is while an action's output takes the place of the description.
/// The dividers are not rows you can stop on. Each row is an icon and a name; the selected one
/// sits on a lifted background with its icon lit.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.page == Page::Project;
    let block = pane(focused)
        .title(title(app))
        .padding(Padding::new(1, 1, 1, 0));
    let width = block.inner(area).width as usize;
    let mut items: Vec<ListItem> = vec![];
    let mut selected_row = 0;
    let mut group: Option<Group> = None;
    for (i, a) in app.actions.iter().enumerate() {
        if group != Some(a.group()) {
            if group.is_some() {
                items.push(ListItem::new(Line::raw("")));
            }
            group = Some(a.group());
            items.push(ListItem::new(divider(a.group().label(), width)));
        }
        let picked = i == app.action;
        if picked {
            selected_row = items.len();
        }
        let (glyph, label) = match (picked, focused) {
            (true, true) => (
                Style::new().fg(CYAN),
                Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            (true, false) => (
                Style::new().fg(CYAN),
                Style::new().add_modifier(Modifier::BOLD),
            ),
            (false, true) => (Style::new().add_modifier(Modifier::DIM), Style::new()),
            (false, false) => (
                Style::new().add_modifier(Modifier::DIM),
                Style::new().add_modifier(Modifier::DIM),
            ),
        };
        let mut row = vec![
            Span::styled(format!(" {} ", icon(*a)), glyph),
            Span::styled(format!(" {}", a.label()), label),
        ];
        // Left running in the background with `d`.
        if app.is_detached(*a) {
            row.push(Span::styled("  ●", Style::new().fg(GREEN)));
        }
        items.push(ListItem::new(Line::from(row)));
    }
    let list = List::new(items).block(block).highlight_style(if focused {
        Style::new().bg(SELECTED_BG)
    } else {
        Style::new()
    });
    let mut state = ListState::default().with_selected(Some(selected_row));
    f.render_stateful_widget(list, area, &mut state);
}
