//! The stack as two cards side by side, the frontend and the backend, each with its logo and its
//! name in block letters, in its own colors, with a glint that sweeps across them like the
//! stackup logo's. A project without one of them shows a card that says so. Nothing is an image:
//! the logos are pixel bitmaps (see `logos`) drawn in half blocks (`▀` and `▄` give two pixels
//! to a row), so every terminal shows them.
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::project_about::GRAY;
use crate::{
    models::{Backend, Facts, Frontend},
    services::projects::logos::{self, Bitmap, Rgb, shrink},
    ui::theme::pane_border,
};

/// Height of the art, in rows: the tallest logo.
pub const ART_HEIGHT: u16 = 12;
/// The banner with its borders.
pub const HEIGHT: u16 = ART_HEIGHT + 2;

const ORANGE: Rgb = (0xF7, 0x4C, 0x00);

fn color((r, g, b): Rgb) -> Color {
    Color::Rgb(r, g, b)
}

/// `c` moved `amount` (0 to 100) of the way to white.
fn lighten((r, g, b): Rgb, amount: u16) -> Rgb {
    let mix = |v: u8| (u16::from(v) + (255 - u16::from(v)) * amount / 100) as u8;
    (mix(r), mix(g), mix(b))
}

/// A letter is 5 pixels wide and 5 tall.
pub(super) fn letter(c: char) -> [&'static str; 5] {
    match c.to_ascii_uppercase() {
        'A' => [".###.", "#...#", "#####", "#...#", "#...#"],
        'B' => ["####.", "#...#", "####.", "#...#", "####."],
        'D' => ["####.", "#...#", "#...#", "#...#", "####."],
        'G' => [".####", "#....", "#..##", "#...#", ".###."],
        'H' => ["#...#", "#...#", "#####", "#...#", "#...#"],
        'J' => ["..###", "...#.", "...#.", "#..#.", ".##.."],
        'K' => ["#..#.", "#.#..", "##...", "#.#..", "#..#."],
        'L' => ["#....", "#....", "#....", "#....", "#####"],
        'Q' => [".###.", "#...#", "#.#.#", "#..#.", ".##.#"],
        'W' => ["#...#", "#...#", "#.#.#", "##.##", "#...#"],
        'Y' => ["#...#", ".#.#.", "..#..", "..#..", "..#.."],
        'Z' => ["#####", "...#.", "..#..", ".#...", "#####"],
        '0' => [".###.", "#..##", "#.#.#", "##..#", ".###."],
        '1' => ["..#..", ".##..", "..#..", "..#..", ".###."],
        '2' => [".###.", "#...#", "..##.", ".#...", "#####"],
        '3' => ["####.", "....#", ".###.", "....#", "####."],
        '4' => ["#..#.", "#..#.", "#####", "...#.", "...#."],
        '5' => ["#####", "#....", "####.", "....#", "####."],
        '6' => [".###.", "#....", "####.", "#...#", ".###."],
        '7' => ["#####", "....#", "...#.", "..#..", "..#.."],
        '8' => [".###.", "#...#", ".###.", "#...#", ".###."],
        '9' => [".###.", "#...#", ".####", "....#", ".###."],
        '-' => [".....", ".....", "#####", ".....", "....."],
        '_' => [".....", ".....", ".....", ".....", "#####"],
        '.' => [".....", ".....", ".....", ".....", "..#.."],
        ' ' => [".....", ".....", ".....", ".....", "....."],
        'C' => [".####", "#....", "#....", "#....", ".####"],
        'E' => ["#####", "#....", "####.", "#....", "#####"],
        'F' => ["#####", "#....", "####.", "#....", "#...."],
        'I' => ["#####", "..#..", "..#..", "..#..", "#####"],
        'M' => ["#...#", "##.##", "#.#.#", "#...#", "#...#"],
        'N' => ["#...#", "##..#", "#.#.#", "#..##", "#...#"],
        'O' => [".###.", "#...#", "#...#", "#...#", ".###."],
        'P' => ["####.", "#...#", "####.", "#....", "#...."],
        'R' => ["####.", "#...#", "####.", "#..#.", "#...#"],
        'S' => [".####", "#....", ".###.", "....#", "####."],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#.."],
        'U' => ["#...#", "#...#", "#...#", "#...#", ".###."],
        'V' => ["#...#", "#...#", "#...#", ".#.#.", "..#.."],
        'X' => ["#...#", ".#.#.", "..#..", ".#.#.", "#...#"],
        _ => [".....", ".....", ".....", ".....", "....."],
    }
}

/// What a card shows for one technology.
struct Mark {
    /// The name in the card's title.
    title: &'static str,
    logo: &'static Bitmap,
    /// The mark alone, for the small strip, where a wordmark would not be readable.
    icon: &'static Bitmap,
    /// The name in block letters beside the logo, when the logo is not a wordmark already.
    letters: Option<(&'static str, Rgb)>,
    /// The color of the title.
    ink: Rgb,
}

fn frontend(f: Frontend) -> Option<Mark> {
    Some(match f {
        Frontend::Nuxt => Mark {
            title: "Nuxt",
            logo: &logos::NUXT,
            icon: &logos::NUXT,
            letters: Some(("NUXT", (0x00, 0xDC, 0x82))),
            ink: (0x00, 0xDC, 0x82),
        },
        Frontend::Vue => Mark {
            title: "Vue",
            logo: &logos::VUE,
            icon: &logos::VUE,
            letters: Some(("VUE", (0x41, 0xB8, 0x83))),
            ink: (0x41, 0xB8, 0x83),
        },
        // Topcoat's own logo is its name.
        Frontend::Topcoat => Mark {
            title: "Topcoat",
            logo: &logos::TOPCOAT,
            icon: &logos::TOPCOAT_ICON,
            letters: None,
            ink: (0xEE, 0xEE, 0xF2),
        },
        Frontend::None => return None,
    })
}

fn backend(b: Backend) -> Option<Mark> {
    Some(match b {
        Backend::Axum => Mark {
            title: "axum",
            logo: &logos::FERRIS,
            icon: &logos::FERRIS,
            letters: Some(("AXUM", ORANGE)),
            ink: ORANGE,
        },
        Backend::Fastapi => Mark {
            title: "FastAPI",
            logo: &logos::FASTAPI,
            icon: &logos::FASTAPI,
            letters: Some(("FASTAPI", (0x00, 0xB8, 0xA9))),
            ink: (0x00, 0xB8, 0xA9),
        },
        Backend::None => return None,
    })
}

/// One cell of the art: what to draw and in which colors, before the glint.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Cell {
    Empty,
    /// A glyph in a color.
    Glyph(char, Rgb),
    /// A half block with a color for the top pixel and one for the bottom, either may be absent.
    Pixels(Option<Rgb>, Option<Rgb>),
}

/// Rows of pixels as rows of cells, two pixel rows to a cell row.
fn cells_of(px: &[Vec<Option<Rgb>>]) -> Vec<Vec<Cell>> {
    px.chunks(2)
        .map(|pair| {
            (0..pair[0].len())
                .map(|x| {
                    let bottom = pair.get(1).and_then(|r| r.get(x)).copied().flatten();
                    match (pair[0][x], bottom) {
                        (None, None) => Cell::Empty,
                        (top, bottom) => Cell::Pixels(top, bottom),
                    }
                })
                .collect()
        })
        .collect()
}

/// The logo as rows of cells.
fn logo_cells(logo: &Bitmap) -> Vec<Vec<Cell>> {
    cells_of(&logo.pixels())
}

/// The name in block letters, 5 pixels tall.
fn name_cells(name: &str, ink: Rgb) -> Vec<Vec<Cell>> {
    (0..5)
        .map(|r| {
            let mut row = vec![];
            for (i, ch) in name.chars().enumerate() {
                if i > 0 {
                    row.push(Cell::Empty);
                }
                row.extend(letter(ch)[r].chars().map(|c| {
                    if c == '#' {
                        Cell::Glyph('█', ink)
                    } else {
                        Cell::Empty
                    }
                }));
            }
            row
        })
        .collect()
}

/// A whole mark as `ART_HEIGHT` rows, the logo and the name centered on its middle.
fn mark_cells(mark: &Mark, columns: usize) -> Vec<Vec<Cell>> {
    let logo = logo_cells(mark.logo);
    let logo_width = logo.first().map_or(0, Vec::len);
    let name = mark
        .letters
        .map(|(n, ink)| name_cells(n, ink))
        .filter(|n| logo_width + 3 + n.first().map_or(0, Vec::len) <= columns);
    let rows = usize::from(ART_HEIGHT);
    let logo_top = rows.saturating_sub(logo.len()) / 2;
    let name_top = rows.saturating_sub(5) / 2;
    let mut grid: Vec<Vec<Cell>> = (0..rows)
        .map(|r| {
            let mut row = r
                .checked_sub(logo_top)
                .and_then(|i| logo.get(i))
                .cloned()
                .unwrap_or_else(|| vec![Cell::Empty; logo_width]);
            if let Some(name) = &name {
                row.extend([Cell::Empty; 3]);
                row.extend(
                    r.checked_sub(name_top)
                        .and_then(|i| name.get(i))
                        .cloned()
                        .unwrap_or_else(|| vec![Cell::Empty; name[0].len()]),
                );
            }
            row
        })
        .collect();
    // Centered across the card.
    let width = grid.first().map_or(0, Vec::len);
    let pad = columns.saturating_sub(width) / 2;
    for row in &mut grid {
        row.splice(0..0, std::iter::repeat_n(Cell::Empty, pad));
    }
    grid
}

/// How much the glint lightens the pixel at column `x`, row `y`: white on the band itself, less
/// beside it, nothing away from it. `tick` moves the band one column.
fn glint(x: usize, y: usize, tick: u64, total: usize) -> u16 {
    let span = total as i64 + i64::from(ART_HEIGHT) * 2 + 24;
    let band = (tick as i64 % span) - 12;
    match ((x + y * 2) as i64 - band).abs() {
        0 => 85,
        1..=2 => 55,
        3..=5 => 25,
        _ => 0,
    }
}

/// A cell as a span, with the glint at its column.
fn span(cell: Cell, x: usize, y: usize, tick: u64, total: usize) -> Span<'static> {
    let g = glint(x, y, tick, total);
    let lit = |c: Rgb| color(lighten(c, g));
    match cell {
        Cell::Empty | Cell::Pixels(None, None) => Span::raw(" "),
        Cell::Glyph(c, rgb) => Span::styled(c.to_string(), Style::new().fg(lit(rgb))),
        Cell::Pixels(Some(top), Some(bottom)) => {
            Span::styled("▀", Style::new().fg(lit(top)).bg(lit(bottom)))
        }
        Cell::Pixels(Some(top), None) => Span::styled("▀", Style::new().fg(lit(top))),
        Cell::Pixels(None, Some(bottom)) => Span::styled("▄", Style::new().fg(lit(bottom))),
    }
}

/// The lines of a card's inside, `columns` wide, at moment `tick`: the mark, or what says there is
/// none.
pub fn lines(role: &str, facts: &Facts, columns: usize, tick: u64) -> Vec<Line<'static>> {
    let found = match role {
        "frontend" => frontend(facts.frontend),
        _ => backend(facts.backend),
    };
    let Some(found) = found else {
        let said = format!("no {role} in this project");
        let top = usize::from(ART_HEIGHT) / 2 - 1;
        return (0..usize::from(ART_HEIGHT))
            .map(|r| {
                if r == top {
                    Line::from(Span::styled(said.clone(), Style::new().fg(GRAY).italic()))
                        .centered()
                } else {
                    Line::from("")
                }
            })
            .collect();
    };
    let grid = mark_cells(&found, columns);
    let total = grid.first().map_or(0, Vec::len);
    grid.iter()
        .enumerate()
        .map(|(y, row)| {
            Line::from(
                row.iter()
                    .enumerate()
                    .map(|(x, &cell)| span(cell, x, y, tick, total))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn card(f: &mut Frame, area: Rect, role: &str, facts: &Facts, tick: u64) {
    let found = match role {
        "frontend" => frontend(facts.frontend),
        _ => backend(facts.backend),
    };
    let mut title = vec![Span::styled(format!(" {role}"), Style::new().fg(GRAY))];
    if let Some(m) = &found {
        title.push(Span::styled(" · ", Style::new().fg(GRAY)));
        title.push(Span::styled(
            format!("{} ", m.title),
            Style::new().fg(color(m.ink)).bold(),
        ));
    } else {
        title.push(Span::raw(" "));
    }
    let block = Block::bordered()
        .border_style(pane_border(false))
        .title(Line::from(title));
    let columns = usize::from(area.width.saturating_sub(2));
    f.render_widget(
        Paragraph::new(lines(role, facts, columns, tick)).block(block),
        area,
    );
}

/// The two cards: the frontend on the left and the backend on the right.
pub fn render(f: &mut Frame, area: Rect, facts: &Facts, tick: u64) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);
    card(f, left, "frontend", facts, tick);
    card(f, right, "backend", facts, tick);
}

/// How tall the small logos are, in pixels: two to a row of the terminal.
const SMALL_PIXELS: usize = 8;
/// How many rows the small logos take.
pub const SMALL_HEIGHT: u16 = (SMALL_PIXELS / 2) as u16;

/// The project's logos, small, side by side: the frontend's and the backend's, when it has them.
pub fn small_lines(facts: &Facts, columns: usize, tick: u64) -> Vec<Line<'static>> {
    let marks = [frontend(facts.frontend), backend(facts.backend)];
    let grids: Vec<Vec<Vec<Cell>>> = marks
        .iter()
        .flatten()
        .map(|m| cells_of(&shrink(&m.icon.pixels(), SMALL_PIXELS)))
        .collect();
    let total: usize = grids.iter().map(|g| g[0].len() + 4).sum();
    (0..SMALL_PIXELS / 2)
        .map(|y| {
            let mut spans = vec![Span::raw(" ")];
            let mut x = 1;
            for g in &grids {
                for &cell in &g[y] {
                    if x < columns {
                        spans.push(span(cell, x, y, tick, total + 2));
                    }
                    x += 1;
                }
                spans.push(Span::raw("    "));
                x += 4;
            }
            Line::from(spans)
        })
        .collect()
}

/// How wide the small logos are, with a space on each side.
pub fn small_width(facts: &Facts) -> u16 {
    let widest = small_lines(facts, usize::MAX, 0)
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or(0);
    u16::try_from(widest.saturating_sub(3)).unwrap_or(u16::MAX)
}

/// Just the logos, no box: they sit in a corner of the page.
pub fn render_small(f: &mut Frame, area: Rect, facts: &Facts, tick: u64) {
    let columns = usize::from(area.width);
    f.render_widget(Paragraph::new(small_lines(facts, columns, tick)), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(frontend: Frontend, backend: Backend) -> Facts {
        Facts {
            exists: true,
            frontend,
            backend,
            https: false,
            ci: false,
            hooks: false,
            git: false,
            remote: false,
            rust_build: false,
            has_volumes: false,
        }
    }

    fn text(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn every_name_is_drawn_from_letters_that_exist() {
        for name in ["NUXT", "VUE", "AXUM", "FASTAPI"] {
            for c in name.chars() {
                assert_ne!(letter(c), letter('?'), "{c} in {name} has no letters");
            }
        }
    }

    #[test]
    fn every_logo_is_whole_even_and_only_uses_its_own_colors() {
        for (name, logo) in [
            ("vue", &logos::VUE),
            ("nuxt", &logos::NUXT),
            ("fastapi", &logos::FASTAPI),
            ("ferris", &logos::FERRIS),
            ("topcoat", &logos::TOPCOAT),
        ] {
            assert_eq!(logo.rows.len() % 2, 0, "{name} has an odd number of rows");
            let width = logo.rows[0].len();
            assert!(
                logo.rows.iter().all(|r| r.len() == width),
                "{name} is ragged"
            );
            assert!(
                logo.rows.len() <= usize::from(ART_HEIGHT) * 2,
                "{name} is too tall"
            );
            for b in logo
                .rows
                .iter()
                .flat_map(|r| r.bytes())
                .filter(|b| *b != b' ')
            {
                assert!(
                    usize::from(b - b'a') < logo.colors.len(),
                    "{name}: color {b}"
                );
            }
        }
    }

    #[test]
    fn each_side_shows_its_own_technology_or_says_there_is_none() {
        let f = facts(Frontend::Topcoat, Backend::None);
        let backend = text(&lines("backend", &f, 60, 0)).join("\n");
        assert!(backend.contains("no backend in this project"), "{backend}");
        let frontend = text(&lines("frontend", &f, 60, 0));
        assert!(frontend.iter().any(|r| r.contains('▀') || r.contains('▄')));
        let f = facts(Frontend::None, Backend::Fastapi);
        let frontend = text(&lines("frontend", &f, 60, 0)).join("\n");
        assert!(
            frontend.contains("no frontend in this project"),
            "{frontend}"
        );
        assert!(
            text(&lines("backend", &f, 100, 0))
                .iter()
                .any(|r| r.contains('█')),
            "FASTAPI name"
        );
    }

    #[test]
    fn a_card_is_as_tall_as_asked_and_never_wider_than_it_is_given() {
        for columns in [20, 40, 70, 100] {
            for (fe, be) in [
                (Frontend::Topcoat, Backend::Axum),
                (Frontend::Nuxt, Backend::Fastapi),
                (Frontend::Vue, Backend::None),
            ] {
                for role in ["frontend", "backend"] {
                    let lines = lines(role, &facts(fe, be), columns, 0);
                    assert_eq!(lines.len(), usize::from(ART_HEIGHT));
                    let widest = lines.iter().map(Line::width).max().unwrap_or(0);
                    // A logo wider than the card is cut by the card, never wider than the logos.
                    assert!(widest <= columns.max(60), "{columns} {role}: {widest}");
                }
            }
        }
    }

    #[test]
    fn the_name_is_left_out_when_the_card_is_too_narrow_for_it() {
        let f = facts(Frontend::Nuxt, Backend::None);
        let wide = text(&lines("frontend", &f, 80, 0)).join("");
        let narrow = text(&lines("frontend", &f, 40, 0)).join("");
        assert!(wide.contains('█'));
        assert!(!narrow.contains('█'));
    }

    #[test]
    fn the_glint_sweeps_across_the_marks_and_reaches_almost_white() {
        let f = facts(Frontend::Nuxt, Backend::Fastapi);
        let colors = |tick| -> Vec<Option<Color>> {
            lines("frontend", &f, 100, tick)
                .iter()
                .flat_map(|l| l.spans.iter().map(|s| s.style.fg))
                .collect()
        };
        assert_ne!(colors(0), colors(45));
        let brightest = (0..200)
            .flat_map(|t| colors(t).into_iter().flatten())
            .filter_map(|c| match c {
                Color::Rgb(r, g, b) => Some(u16::from(r) + u16::from(g) + u16::from(b)),
                _ => None,
            })
            .max()
            .unwrap();
        assert!(
            brightest > 3 * 230,
            "the band lights pixels almost to white: {brightest}"
        );
        assert_eq!(lighten((0, 0, 0), 100), (255, 255, 255));
        assert_eq!(lighten((10, 20, 30), 0), (10, 20, 30));
    }

    #[test]
    fn small_logos_are_five_rows_and_each_present_technology_shows() {
        let both = small_lines(&facts(Frontend::Nuxt, Backend::Fastapi), 80, 0);
        assert_eq!(both.len(), usize::from(SMALL_HEIGHT));
        let drawn = |lines: &[Line]| text(lines).join("").matches(['▀', '▄']).count();
        let one = small_lines(&facts(Frontend::Nuxt, Backend::None), 80, 0);
        assert!(drawn(&one) > 10);
        assert!(drawn(&both) > drawn(&one));
        assert_eq!(
            drawn(&small_lines(&facts(Frontend::None, Backend::None), 80, 0)),
            0
        );
        for l in &both {
            assert!(l.width() <= 80);
        }
    }

    #[test]
    fn shrinking_keeps_the_shape() {
        let small = shrink(&logos::FASTAPI.pixels(), 10);
        assert_eq!(small.len(), 10);
        assert!(small.iter().flatten().flatten().count() > 40);
        assert!(small[5][5].is_some(), "the middle of the disc is filled");
        assert!(small[0][0].is_none(), "its corner is empty");
    }
}
