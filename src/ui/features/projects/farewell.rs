//! What shows when a project has been removed: its name in big letters that crumble and fall away
//! like dust, a line saying what went, and then the list again. It is quick on purpose: it is a
//! goodbye, not a wait. Any key skips it.
use std::time::Instant;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::{project_about::GRAY, stack_banner::letter};
use crate::{
    models::Facts,
    services::projects::logos::{self, Rgb},
    ui::theme::{GREEN, RED},
};

/// How long the whole thing lasts.
pub const DURATION_MS: u128 = 1800;
/// How long the name stands whole before it starts to crumble.
const HOLD: f32 = 0.30;

pub struct Farewell {
    pub name: String,
    /// What went with it, for the line under the name.
    pub detail: String,
    /// The frontend's logo and the backend's, as pixels, drawn at either side of the name.
    marks: [Option<Vec<Vec<Option<Rgb>>>>; 2],
    started: Instant,
}

impl Farewell {
    pub fn new(name: &str, detail: &str, facts: Option<&Facts>) -> Self {
        let (front, back) = facts.map_or((None, None), logos::marks);
        // As tall as the letters, and twice as wide as tall, as a cell is twice as tall as wide.
        let mark = |b: Option<&'static logos::Bitmap>| {
            b.map(|b| {
                logos::shrink(&b.pixels(), 5)
                    .into_iter()
                    .map(|row| row.into_iter().flat_map(|p| [p, p]).collect())
                    .collect()
            })
        };
        Self {
            name: name.to_string(),
            detail: detail.to_string(),
            marks: [mark(front), mark(back)],
            started: Instant::now(),
        }
    }

    /// Everything that crumbles: the frontend's logo, the name and the backend's logo in a row.
    fn sprite(&self) -> Vec<Vec<Option<Tone>>> {
        let mut rows = vec![vec![]; 5];
        let gap = 3;
        let [front, back] = &self.marks;
        let name = pixels(&self.name);
        for (y, row) in rows.iter_mut().enumerate() {
            if let Some(f) = front {
                row.extend(f[y].iter().map(|p| p.map(Tone::Logo)));
                row.extend(vec![None; gap]);
            }
            row.extend(name[y].iter().map(|on| on.then_some(Tone::Name)));
            if let Some(b) = back {
                row.extend(vec![None; gap]);
                row.extend(b[y].iter().map(|p| p.map(Tone::Logo)));
            }
        }
        rows
    }

    /// How far along it is, from 0 to 1.
    pub fn progress(&self) -> f32 {
        (self.started.elapsed().as_millis() as f32 / DURATION_MS as f32).min(1.0)
    }

    pub fn done(&self) -> bool {
        self.started.elapsed().as_millis() >= DURATION_MS
    }
}

/// What a pixel of the sprite is: part of the name, or of a logo in its own color.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Tone {
    Name,
    Logo(Rgb),
}

/// The name as a grid of pixels, `true` where a letter is.
fn pixels(name: &str) -> Vec<Vec<bool>> {
    let mut rows = vec![vec![]; 5];
    for (i, c) in name.chars().enumerate() {
        for (y, row) in rows.iter_mut().enumerate() {
            if i > 0 {
                row.push(false);
            }
            row.extend(letter(c)[y].chars().map(|p| p == '#'));
        }
    }
    rows
}

/// A number from 0 to 99 that is always the same for a pixel, so each one crumbles at its own
/// moment and the same way every time.
fn scatter(x: usize, y: usize) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h ^ (h >> 16)) % 100
}

/// The name at moment `t` (0 to 1) as `height` rows of `width` cells: solid, then each pixel
/// breaks into dust that falls and is gone.
fn frame(px: &[Vec<Option<Tone>>], t: f32, width: usize, height: usize) -> Vec<Vec<(char, Color)>> {
    let mut out = vec![vec![(' ', Color::Reset); width]; height];
    let text_w = px[0].len();
    let (left, top) = (
        width.saturating_sub(text_w) / 2,
        height.saturating_sub(5) / 2,
    );
    let q = ((t - HOLD) / (1.0 - HOLD)).clamp(0.0, 1.0) * 130.0;
    for (y, row) in px.iter().enumerate() {
        for (x, tone) in row.iter().enumerate() {
            let Some(tone) = tone else {
                continue;
            };
            let h = scatter(x, y) as f32;
            let (cx, cy) = (left + x, top + y);
            if cx >= width {
                continue;
            }
            // Whole and red until its moment; then a spark, dust that falls, and gone.
            let age = q - h;
            if age < 0.0 {
                let glow = match tone {
                    Tone::Logo((r, g, b)) => Color::Rgb(*r, *g, *b),
                    Tone::Name if t < HOLD => RED,
                    Tone::Name => Color::Indexed(167),
                };
                if cy < height {
                    out[cy][cx] = ('█', glow);
                }
                continue;
            }
            let fall = (age / 6.0) as usize;
            let (glyph, color) = match age {
                a if a < 8.0 => ('▓', Color::Indexed(173)),
                a if a < 18.0 => ('▒', Color::Indexed(138)),
                a if a < 30.0 => ('░', Color::Indexed(244)),
                _ => continue,
            };
            if let Some(cell) = out.get_mut(cy + fall).and_then(|r| r.get_mut(cx)) {
                *cell = (glyph, color);
            }
        }
    }
    out
}

/// The panel: the crumbling name, and under it what went.
pub fn render(f: &mut Frame, area: Rect, farewell: &Farewell) {
    let block = Block::bordered()
        .border_style(Style::new().fg(RED))
        .title(Line::from(Span::styled(
            " removed ",
            Style::new().fg(RED).bold(),
        )));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let t = farewell.progress();
    let (width, height) = (usize::from(inner.width), usize::from(inner.height));
    let sprite = farewell.sprite();
    let needs = sprite[0].len();
    // Room for the name and two lines under it; a long name is written plain instead.
    let big = needs <= width && height >= 9;
    let mut lines: Vec<Line> = vec![];
    if big {
        let grid = frame(&sprite, t, width, height.saturating_sub(4).min(14));
        for row in grid {
            lines.push(Line::from(
                row.into_iter()
                    .map(|(c, color)| Span::styled(c.to_string(), Style::new().fg(color)))
                    .collect::<Vec<_>>(),
            ));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(
            Line::from(Span::styled(
                farewell.name.clone(),
                Style::new().fg(if t < HOLD { RED } else { GRAY }).bold(),
            ))
            .centered(),
        );
    }
    lines.push(Line::from(""));
    let said = if t < HOLD {
        Span::styled("deleting...", Style::new().fg(GRAY))
    } else {
        Span::styled(
            format!("✓ deleted · {}", farewell.detail),
            Style::new().fg(GREEN).bold(),
        )
    };
    lines.push(Line::from(said).centered());
    let top = height.saturating_sub(lines.len()) / 2;
    let mut padded = vec![Line::from(""); if big { 0 } else { top }];
    padded.extend(lines);
    f.render_widget(Paragraph::new(padded), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(grid: &[Vec<(char, Color)>]) -> usize {
        grid.iter().flatten().filter(|(c, _)| *c != ' ').count()
    }

    #[test]
    fn the_name_stands_whole_then_crumbles_to_nothing() {
        let sprite = Farewell::new("Portfolio", "x", None).sprite();
        let at = |t| solid(&frame(&sprite, t, 70, 14));
        let whole = at(0.0);
        assert!(whole > 100, "the letters are drawn: {whole}");
        assert_eq!(at(HOLD - 0.01), whole, "nothing moves while it is held");
        assert!(at(0.6) != whole, "then it breaks up");
        assert_eq!(at(1.0), 0, "and nothing is left");
    }

    #[test]
    fn every_pixel_crumbles_the_same_way_each_time() {
        let sprite = Farewell::new("shop", "x", None).sprite();
        assert_eq!(frame(&sprite, 0.6, 40, 14), frame(&sprite, 0.6, 40, 14));
        assert!((0..40).all(|x| scatter(x, 3) < 100));
        assert_ne!(scatter(1, 1), scatter(2, 1));
    }

    #[test]
    fn it_is_over_after_its_time_and_starts_at_the_beginning() {
        let f = Farewell::new("shop", "x", None);
        assert!(!f.done());
        assert!(f.progress() < 0.5);
        assert!(f.progress() >= 0.0);
    }

    #[test]
    fn the_logos_of_what_it_was_built_with_stand_beside_the_name_and_crumble_with_it() {
        let facts = Facts {
            exists: true,
            frontend: crate::models::Frontend::Nuxt,
            backend: crate::models::Backend::Fastapi,
            https: false,
            ci: false,
            hooks: false,
            git: false,
            remote: false,
            rust_build: false,
            has_volumes: false,
            migrations: false,
        };
        let bare = Farewell::new("shop", "x", None).sprite();
        let dressed = Farewell::new("shop", "x", Some(&facts)).sprite();
        assert!(dressed[0].len() > bare[0].len() + 10, "a logo at each side");
        assert!(
            dressed[2]
                .iter()
                .flatten()
                .any(|t| matches!(t, Tone::Logo(_)))
        );
        assert!(matches!(dressed[2][0], Some(Tone::Logo(_)) | None));
        let left = |t| solid(&frame(&dressed, t, 90, 14));
        assert_eq!(left(1.0), 0, "the logos are gone too");
    }

    #[test]
    fn a_name_with_digits_dashes_and_capitals_has_every_letter() {
        let blank = pixels(" ");
        for c in "Portfolio-2_my.app".chars() {
            assert_ne!(pixels(&c.to_string()), blank, "{c}");
        }
    }
}
