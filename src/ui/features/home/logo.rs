//! The stackup logo, drawn like mc2's: block letters with a soft `░` edge, and a bright band
//! that sweeps across them one column per tick.
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::ui::theme::{BLUE, CYAN};

/// Each letter is 5 pixels wide and 8 tall: rows 2 to 6 are the x-height, `t` and `k` reach up,
/// `p` reaches down.
const LETTERS: [[&str; 8]; 7] = [
    // s
    [
        ".....", ".....", ".####", "#....", ".###.", "....#", "####.", ".....",
    ],
    // t
    [
        ".....", "..#..", "#####", "..#..", "..#..", "..#..", "...##", ".....",
    ],
    // a
    [
        ".....", ".....", ".###.", "....#", ".####", "#...#", ".####", ".....",
    ],
    // c
    [
        ".....", ".....", ".####", "#....", "#....", "#....", ".####", ".....",
    ],
    // k
    [
        "#....", "#....", "#..#.", "#.#..", "##...", "#.#..", "#..#.", ".....",
    ],
    // u
    [
        ".....", ".....", "#...#", "#...#", "#...#", "#..##", ".##.#", ".....",
    ],
    // p
    [
        ".....", ".....", "####.", "#...#", "#...#", "####.", "#....", "#....",
    ],
];

pub const HEIGHT: u16 = 8;
const LETTER_WIDTH: usize = 5;
/// Two columns to a pixel, and one pixel between letters.
pub const WIDTH: u16 = (LETTERS.len() * LETTER_WIDTH * 2 + (LETTERS.len() - 1) * 2) as u16;

const WHITE: Color = Color::Indexed(231);
const SKY: Color = Color::Indexed(51);

fn on(letter: &[&str; 8], row: usize, col: usize) -> bool {
    letter[row].as_bytes().get(col) == Some(&b'#')
}

/// The logo as text: `█` for a stroke and `░` for the soft edge before it.
fn rows() -> Vec<String> {
    (0..HEIGHT as usize)
        .map(|row| {
            LETTERS
                .iter()
                .map(|letter| {
                    (0..LETTER_WIDTH)
                        .map(|col| {
                            if on(letter, row, col) {
                                "██"
                            } else if on(letter, row, col + 1) {
                                " ░"
                            } else {
                                "  "
                            }
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect()
}

/// A bright band crosses the letters diagonally; `tick` moves it one column.
fn shade(x: usize, y: usize, tick: u64) -> Color {
    let span = WIDTH as i64 + HEIGHT as i64 + 24;
    let band = (tick as i64 % span) - 12;
    match ((x + y) as i64 - band).abs() {
        0 => WHITE,
        1..=2 => SKY,
        3..=5 => CYAN,
        _ => BLUE,
    }
}

pub fn lines(tick: u64) -> Vec<Line<'static>> {
    rows()
        .iter()
        .enumerate()
        .map(|(y, row)| {
            Line::from(
                row.chars()
                    .enumerate()
                    .map(|(x, c)| match c {
                        '█' => Span::styled("█", Style::new().fg(shade(x, y, tick))),
                        '░' => Span::styled("░", Style::new().fg(BLUE).dim()),
                        _ => Span::raw(" "),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_as_wide_as_the_logo() {
        for row in rows() {
            assert_eq!(row.chars().count(), WIDTH as usize, "{row}");
        }
        assert_eq!(rows().len(), HEIGHT as usize);
    }

    #[test]
    fn the_band_moves_across_the_whole_logo_with_the_tick() {
        let colors = |tick| -> Vec<Option<Color>> {
            lines(tick)
                .iter()
                .flat_map(|l| l.spans.iter().map(|s| s.style.fg))
                .collect()
        };
        assert_ne!(colors(0), colors(40));
        let bright = (0..160).any(|t| colors(t).contains(&Some(WHITE)));
        assert!(bright, "the band reaches the letters at some tick");
    }
}
