use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Padding},
};

use crate::ui::{
    features::create::wizard::Wizard,
    theme::{CYAN, GRAY, GREEN, pane, pane_title, selected},
    types::{Focus, Row},
};

pub fn render(f: &mut Frame, area: Rect, w: &Wizard) {
    let focused = w.focus == Focus::Menu;
    let items: Vec<ListItem> = w
        .rows
        .iter()
        .map(|r| ListItem::new(row_line(w, r)))
        .collect();
    let list = List::new(items)
        .block(
            pane(focused)
                .title(pane_title(&format!("stackup · {}", w.name)))
                .padding(Padding::new(1, 1, 1, 0)),
        )
        .highlight_style(if focused {
            selected()
        } else {
            Style::new().add_modifier(Modifier::BOLD)
        });
    let mut state = ListState::default().with_selected(Some(w.cursor));
    f.render_stateful_widget(list, area, &mut state);
}

fn row_line(w: &Wizard, row: &Row) -> Line<'static> {
    let radio = |on: bool, label: String| {
        let (mark, style) = if on {
            (" \u{f192}  ", Style::new().fg(CYAN))
        } else {
            (" \u{f10c}  ", Style::new().fg(GRAY))
        };
        Line::from(vec![Span::styled(mark, style), Span::raw(label)])
    };
    let check = |on: bool, label: String| {
        let (mark, style) = if on {
            (" \u{f14a}  ", Style::new().fg(CYAN))
        } else {
            (" \u{f096}  ", Style::new().fg(GRAY))
        };
        Line::from(vec![Span::styled(mark, style), Span::raw(label)])
    };
    match *row {
        Row::Header(t) => Line::from(Span::styled(t, Style::new().fg(CYAN).dim().bold())),
        Row::Gap => Line::raw(""),
        Row::Frontend(f) => radio(w.sel.frontend == f, f.to_string()),
        Row::Backend(b) => radio(w.sel.backend == b, b.to_string()),
        Row::Lib(l) => {
            let usable = l.works_with(w.sel.frontend);
            let line = check(usable && w.sel.libs.contains(&l), l.to_string());
            if usable { line } else { line.dim() }
        }
        Row::BeLib(l) => {
            let postgres = w.sel.services.contains(&crate::models::Service::Postgres);
            let usable = l.why_not(w.sel.backend, postgres).is_none();
            let line = check(usable && w.sel.be_libs.contains(&l), l.to_string());
            if usable { line } else { line.dim() }
        }
        Row::Service(s) => {
            let mut line = check(w.sel.services.contains(&s), s.to_string());
            if let Some(driver) = s.driver(w.sel.backend) {
                line.push_span(Span::styled(format!("  · {driver}"), Style::new().dim()));
            }
            line
        }
        Row::Tool(t) => check(w.sel.tools.contains(&t), t.to_string()),
        Row::Create => Line::from(Span::styled(
            " \u{f135}  Create project",
            Style::new().fg(GREEN).bold(),
        )),
    }
}
