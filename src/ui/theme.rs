use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType},
};

// Palette shared with the shell's `ls` (EZA_COLORS in config.fish).
pub const BLUE: Color = Color::Indexed(39);
pub const CYAN: Color = Color::Indexed(45);
pub const GREEN: Color = Color::Indexed(46);
pub const YELLOW: Color = Color::Indexed(220);
pub const RED: Color = Color::Indexed(203);
pub const PINK: Color = Color::Indexed(213);
pub const INK: Color = Color::Indexed(16);
/// The row the cursor is on in a list: lifted off the background, not inverted.
pub const SELECTED_BG: Color = Color::Indexed(237);
/// Dividers inside a pane: there, but quieter than the borders.
pub const RULE: Color = Color::Indexed(239);

/// Secondary text: pane titles, labels, notes.
pub const GRAY: Color = Color::Indexed(244);

/// The focused pane gets a bright border, the other a dim one.
pub fn pane_border(focused: bool) -> Style {
    if focused {
        Style::new().fg(CYAN)
    } else {
        Style::new().fg(RULE)
    }
}

/// Every box in the app: rounded corners, its border lit when it has the keys.
pub fn pane(focused: bool) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(pane_border(focused))
}

/// A pane's title, the same quiet gray everywhere.
pub fn pane_title(text: &str) -> Line<'static> {
    Line::from(Span::styled(format!(" {text} "), Style::new().fg(GRAY)))
}

/// The row the cursor is on: a lifted background, never inverted colors.
pub fn selected() -> Style {
    Style::new().bg(SELECTED_BG).add_modifier(Modifier::BOLD)
}

/// A button in a dialog: filled when it is the one Enter presses.
pub fn button(label: &str, selected: bool) -> Span<'static> {
    let text = format!(" {label} ");
    if selected {
        Span::styled(text, Style::new().fg(INK).bg(CYAN).bold())
    } else {
        Span::styled(text, Style::new().bg(SELECTED_BG).fg(GRAY))
    }
}

/// An environment's color everywhere it shows, so prod is never mistaken for the others.
pub fn tier_color(tier: &str) -> Color {
    match tier {
        "local" => GREEN,
        "dev" => YELLOW,
        "prod" => RED,
        "custom" => GRAY,
        _ => CYAN,
    }
}
