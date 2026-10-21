use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Clear, Paragraph, Wrap},
};

/// How tall `lines` are once they wrap at `width`.
fn wrapped_height(lines: &[Line], width: u16) -> u16 {
    let width = usize::from(width.max(1));
    lines
        .iter()
        .map(|l| l.width().div_ceil(width).max(1))
        .sum::<usize>() as u16
}

/// A box in the middle of the screen, over whatever is there. It sizes itself to its lines
/// (wrapping the long ones), and says in its bottom border what the keys are.
pub fn render(
    f: &mut Frame,
    screen: Rect,
    title: &str,
    lines: Vec<Line<'static>>,
    keys: &str,
    border: Style,
) {
    let width = 84.min(screen.width);
    let inner = width.saturating_sub(2);
    let height = (wrapped_height(&lines, inner) + 2).min(screen.height);
    let area = Rect {
        x: screen.x + screen.width.saturating_sub(width) / 2,
        y: screen.y + screen.height.saturating_sub(height) / 2,
        width,
        height,
    };
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::bordered()
                .border_style(border)
                .title(format!(" {title} "))
                .title_bottom(format!(" {keys} ")),
        ),
        area,
    );
}
