use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::{
    models::Plan,
    ui::{
        features::create::use_file_tree::{Node, file_tree},
        theme::BLUE,
    },
};

/// The project folder and everything generated inside it, one line per entry.
pub fn lines(plan: &Plan) -> Vec<Line<'static>> {
    let mut out = vec![Line::from(Span::styled(
        format!("{}/", plan.name),
        Style::new().fg(BLUE).bold(),
    ))];
    render_tree(&file_tree(plan), "", &mut out);
    out
}

fn render_tree(nodes: &[Node], prefix: &str, out: &mut Vec<Line<'static>>) {
    for (i, node) in nodes.iter().enumerate() {
        let last = i + 1 == nodes.len();
        let branch = format!("{prefix}{}", if last { "└─ " } else { "├─ " });
        match node {
            Node::Dir(name, kids) => {
                let mut spans = vec![
                    Span::raw(branch),
                    Span::styled(name.clone(), Style::new().fg(BLUE).bold()),
                ];
                if kids.is_empty() {
                    spans.push(Span::styled("  .gitkeep", Style::new().dim()));
                }
                out.push(Line::from(spans));
                render_tree(
                    kids,
                    &format!("{prefix}{}", if last { "   " } else { "│  " }),
                    out,
                );
            }
            Node::File(name) => {
                let style = if name == ".gitkeep" {
                    Style::new().dim()
                } else {
                    Style::new()
                };
                out.push(Line::from(vec![
                    Span::raw(branch),
                    Span::styled(name.clone(), style),
                ]));
            }
        }
    }
}
