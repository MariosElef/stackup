use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
};

use crate::{
    models::Group,
    ui::{
        stores::app::App,
        theme::{CYAN, GREEN, pane_border, tier_color},
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

/// What can be done to the open project, in sections: Run, Build & ship, Environment, Git,
/// Project. It stays where it is while an action's output takes the place of the description.
/// The headers are not rows you can stop on.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.page == Page::Project;
    let mut items: Vec<ListItem> = vec![];
    let mut selected_row = 0;
    let mut group: Option<Group> = None;
    for (i, a) in app.actions.iter().enumerate() {
        if group != Some(a.group()) {
            group = Some(a.group());
            items.push(ListItem::new(Line::from(Span::styled(
                a.group().label().to_string(),
                Style::new().fg(CYAN).add_modifier(Modifier::BOLD),
            ))));
        }
        if i == app.action {
            selected_row = items.len();
        }
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  ▶ ", Style::new().fg(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(
                a.label().to_string(),
                Style::new().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
        ])));
    }
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_style(pane_border(focused))
                .title(title(app)),
        )
        .highlight_style(if focused {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new().add_modifier(Modifier::BOLD)
        });
    let mut state = ListState::default().with_selected(Some(selected_row));
    f.render_stateful_widget(list, area, &mut state);
}
