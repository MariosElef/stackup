use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::file_tree;
use crate::{
    services::generate::compose::compose,
    ui::{
        features::create::wizard::Wizard,
        theme::pane_border,
        types::{Focus, Tab},
    },
};

pub fn render(f: &mut Frame, area: Rect, w: &Wizard) {
    let plan = w.plan();
    let tab = |label: &'static str, active: bool| {
        if active {
            Span::styled(
                format!(" {label} "),
                Style::new().add_modifier(Modifier::REVERSED),
            )
        } else {
            Span::styled(format!(" {label} "), Style::new().dim())
        }
    };
    let title = Line::from(vec![
        tab("files", w.tab == Tab::Files),
        tab("docker-compose.yml", w.tab == Tab::Compose),
    ]);
    let lines = match w.tab {
        Tab::Files => file_tree::lines(&plan),
        Tab::Compose => compose(&plan)
            .lines()
            .map(|l| Line::raw(l.to_string()))
            .collect(),
    };
    // Never scroll past the last line, and let key handling know how far that is.
    let inner = area.height.saturating_sub(2) as usize;
    let max = lines.len().saturating_sub(inner) as u16;
    w.max_scroll.set(max);
    let p = Paragraph::new(lines).scroll((w.scroll.min(max), 0)).block(
        Block::bordered()
            .border_style(pane_border(w.focus == Focus::Preview))
            .title(title),
    );
    f.render_widget(p, area);
}
