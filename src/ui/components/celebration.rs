//! What shows when an install or a creation finishes well: a check mark that draws itself, sparks
//! that burst out of it and fade, and what was done under it. Then it is gone and the screen
//! goes back on its own. Quick on purpose, and any key skips it.
use std::time::Instant;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use crate::ui::theme::{CYAN, GREEN, YELLOW};

/// How long the whole thing lasts.
pub const DURATION_MS: u128 = 1600;
/// The share of the time the check takes to draw itself.
const DRAW: f32 = 0.35;

const GRAY: Color = Color::Indexed(244);

pub struct Celebration {
    /// What was done: "sqlx installed".
    pub title: String,
    /// Where it goes next: "back to the wizard".
    pub next: String,
    started: Instant,
}

impl Celebration {
    pub fn new(title: &str, next: &str) -> Self {
        Self {
            title: title.to_string(),
            next: next.to_string(),
            started: Instant::now(),
        }
    }

    /// From 0 at the start to 1 at the end.
    pub fn progress(&self) -> f32 {
        (self.started.elapsed().as_millis() as f32 / DURATION_MS as f32).min(1.0)
    }

    pub fn done(&self) -> bool {
        self.started.elapsed().as_millis() >= DURATION_MS
    }
}

/// The check, in the order it is drawn: down the short stroke, then up the long one. Cells are
/// twice as tall as wide, so it is twice as wide as tall.
const CHECK: &[(i16, i16)] = &[
    (-6, 0),
    (-5, 0),
    (-4, 1),
    (-3, 1),
    (-2, 2),
    (-1, 2),
    (0, 1),
    (1, 1),
    (2, 0),
    (3, 0),
    (4, -1),
    (5, -1),
    (6, -2),
    (7, -2),
];

/// The sparks: their direction (x, y) and how they look.
const SPARKS: &[(f32, f32, char)] = &[
    (1.0, 0.0, '✦'),
    (0.7, -0.7, '·'),
    (0.0, -1.0, '✧'),
    (-0.7, -0.7, '·'),
    (-1.0, 0.0, '✦'),
    (-0.7, 0.7, '·'),
    (0.0, 1.0, '✧'),
    (0.7, 0.7, '·'),
    (0.92, -0.38, '•'),
    (-0.92, -0.38, '•'),
    (-0.92, 0.38, '•'),
    (0.92, 0.38, '•'),
];

/// What is in each cell of a `width` by `height` grid at `t`: the check, drawn up to where it has
/// got, and the sparks, flying out once it is whole and fading as they go.
fn grid(t: f32, width: usize, height: usize) -> Vec<Vec<(char, Color)>> {
    let mut cells = vec![vec![(' ', GREEN); width]; height];
    let (cx, cy) = (width as i16 / 2, height as i16 / 2);
    let mut put = |x: i16, y: i16, c: char, color: Color| {
        if let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y))
            && y < height
            && x < width
        {
            cells[y][x] = (c, color);
        }
    };
    let drawn = ((t / DRAW).min(1.0) * CHECK.len() as f32).ceil() as usize;
    for &(x, y) in &CHECK[..drawn] {
        put(cx + x, cy + y, '█', GREEN);
    }
    if t > DRAW {
        // 0 when the check is whole, 1 at the end: out fast, then slowing, then gone.
        let s = (t - DRAW) / (1.0 - DRAW);
        let reach = 1.0 - (1.0 - s) * (1.0 - s);
        let color = match s {
            s if s < 0.35 => YELLOW,
            s if s < 0.7 => CYAN,
            _ => GRAY,
        };
        let (rx, ry) = (f32::from(cx).min(22.0), f32::from(cy).min(6.0));
        for &(dx, dy, c) in SPARKS {
            // Starting just outside the check, so it is never covered.
            let x = cx + (dx * (9.0 + (rx - 9.0).max(0.0) * reach)).round() as i16;
            let y = cy + (dy * (3.0 + (ry - 3.0).max(0.0) * reach)).round() as i16;
            put(x, y, c, color);
        }
    }
    cells
}

/// Over whatever was on the screen: a box in the middle, cleared, with the animation in it.
pub fn render(f: &mut Frame, area: Rect, c: &Celebration) {
    let width = area.width.min(56);
    let height = area.height.min(15);
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    f.render_widget(Clear, rect);
    let block = Block::bordered().border_style(Style::new().fg(GREEN));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let t = c.progress();
    let art = usize::from(inner.height.saturating_sub(3));
    let mut lines: Vec<Line> = grid(t, usize::from(inner.width), art)
        .into_iter()
        .map(|row| {
            Line::from(
                row.into_iter()
                    .map(|(ch, color)| Span::styled(ch.to_string(), Style::new().fg(color)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    lines.push(Line::from(""));
    // The words come in with the sparks, once the check is whole.
    if t > DRAW {
        lines.push(
            Line::from(Span::styled(
                format!("✓ {}", c.title),
                Style::new().fg(GREEN).bold(),
            ))
            .centered(),
        );
        lines.push(Line::from(Span::styled(c.next.clone(), Style::new().fg(GRAY))).centered());
    }
    f.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(grid: &[Vec<(char, Color)>], c: char) -> usize {
        grid.iter().flatten().filter(|(ch, _)| *ch == c).count()
    }

    #[test]
    fn the_check_draws_itself_then_the_sparks_fly_out() {
        let start = grid(0.0, 50, 10);
        assert_eq!(count(&start, '█'), 0, "nothing yet");
        let half = grid(DRAW / 2.0, 50, 10);
        let part = count(&half, '█');
        assert!(part > 0 && part < CHECK.len(), "{part}");
        assert_eq!(count(&half, '✦'), 0, "no sparks before the check is whole");
        let after = grid(0.7, 50, 10);
        assert_eq!(count(&after, '█'), CHECK.len());
        assert!(count(&after, '✦') > 0);
    }

    #[test]
    fn it_never_draws_outside_a_small_screen() {
        for t in [0.0, 0.5, 1.0] {
            let g = grid(t, 6, 2);
            assert_eq!((g.len(), g[0].len()), (2, 6));
        }
    }
}
