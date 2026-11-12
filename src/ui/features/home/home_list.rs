use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Padding},
};

use crate::ui::{
    stores::app::App,
    theme::{CYAN, GRAY, pane, pane_title, selected},
};

/// The home menu: where to go. Each row is an icon, the name and what it is.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let mine = app.projects.len();
    let rows = [
        (
            "\u{f135}",
            "Start",
            "Make a new project from the official tools".to_string(),
        ),
        (
            "\u{f07c}",
            "My projects",
            match mine {
                0 => "None yet: the ones you build with stackup show up here".to_string(),
                1 => "1 project: run it, build it, deploy it".to_string(),
                n => format!("{n} projects: run them, build them, deploy them"),
            },
        ),
        ("\u{f08b}", "Quit", String::new()),
    ];
    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, (icon, name, about))| {
            let picked = i == app.home;
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {icon}  "),
                    if picked {
                        Style::new().fg(CYAN)
                    } else {
                        Style::new().fg(GRAY)
                    },
                ),
                Span::styled(format!("{name:<14}"), Style::new().bold()),
                Span::styled(about.clone(), Style::new().fg(GRAY)),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(
            pane(true)
                .title(pane_title("stackup"))
                .padding(Padding::new(1, 1, 1, 1)),
        )
        .highlight_style(selected());
    let mut state = ListState::default().with_selected(Some(app.home));
    f.render_stateful_widget(list, area, &mut state);
}
