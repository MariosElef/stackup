use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Padding, Paragraph},
};

use crate::{
    models::Facts,
    ui::{
        stores::app::App,
        theme::{CYAN, GRAY, RED, pane, pane_title, selected},
    },
};

/// The projects built with stackup, with what each is made of.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = pane(true)
        .title(pane_title("my projects"))
        .padding(Padding::new(1, 1, 1, 0));
    if app.projects.is_empty() {
        let text = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(" Nothing here yet.", Style::new().dim())),
            Line::from(Span::styled(
                " Press n to start a project, or a to add ones you already have.",
                Style::new().dim(),
            )),
        ])
        .block(block);
        return f.render_widget(text, area);
    }
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let facts = Facts::read(&p.path);
            let what = if facts.exists {
                Span::styled(facts.summary(), Style::new().fg(GRAY))
            } else {
                Span::styled("missing", Style::new().fg(RED))
            };
            let icon = if i == app.cursor { CYAN } else { GRAY };
            ListItem::new(Line::from(vec![
                Span::styled(" \u{f07b}  ", Style::new().fg(icon)),
                Span::styled(format!("{:<18}", p.name), Style::new().bold()),
                what,
            ]))
        })
        .collect();
    let list = List::new(items).block(block).highlight_style(selected());
    let mut state = ListState::default().with_selected(Some(app.cursor));
    f.render_stateful_widget(list, area, &mut state);
}
