use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
};

use crate::ui::{
    stores::app::App,
    theme::{GREEN, pane_border},
};

/// The home menu: where to go. Each row is `▶ name  what it is`, as in mc2.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let mine = app.projects.len();
    let rows = [
        (
            "Start",
            "Make a new project from the official tools".to_string(),
        ),
        (
            "My projects",
            match mine {
                0 => "None yet: the ones you build with stackup show up here".to_string(),
                1 => "1 project: run it, build it, deploy it".to_string(),
                n => format!("{n} projects: run them, build them, deploy them"),
            },
        ),
        ("Quit", String::new()),
    ];
    let items: Vec<ListItem> = rows
        .iter()
        .map(|(name, about)| {
            ListItem::new(Line::from(vec![
                Span::styled("▶ ", Style::new().fg(GREEN).bold()),
                Span::styled(format!("{name:<13}"), Style::new().fg(GREEN).bold()),
                Span::styled(about.clone(), Style::new().dim()),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_style(pane_border(true))
                .title(" stackup "),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(app.home));
    f.render_stateful_widget(list, area, &mut state);
}
