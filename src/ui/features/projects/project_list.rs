use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::{
    models::Facts,
    ui::{
        stores::app::App,
        theme::{GREEN, RED, pane_border},
    },
};

/// The projects built with stackup, with what each is made of.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered()
        .border_style(pane_border(true))
        .title(" my projects ");
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
        .map(|p| {
            let facts = Facts::read(&p.path);
            let what = if facts.exists {
                Span::styled(facts.summary(), Style::new().dim())
            } else {
                Span::styled("missing", Style::new().fg(RED))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<16}", p.name), Style::new().fg(GREEN).bold()),
                what,
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(app.cursor));
    f.render_stateful_widget(list, area, &mut state);
}
