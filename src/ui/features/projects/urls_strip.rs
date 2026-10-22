use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::{
    project_about::{GRAY, tech_color},
    stack_banner,
};
use crate::{
    models::{Backend, Facts, Frontend},
    services::projects::urls::{Link, Login},
    ui::{
        stores::app::App,
        theme::{CYAN, GREEN, YELLOW, pane_border},
    },
};

/// Side by side from this width, one under the other below it.
const SIDE_BY_SIDE: u16 = 60;
/// The logos get a panel of their own from this width.
const LOGOS_FROM: u16 = 70;

fn facts(app: &App) -> Option<Facts> {
    app.project().map(|p| Facts::read(&p.path))
}

/// The logos' panel: whether it fits at `width`, and the facts to draw from.
fn logos_for(app: &App, width: u16) -> Option<Facts> {
    facts(app).filter(|f| {
        width >= LOGOS_FROM && (f.frontend != Frontend::None || f.backend != Backend::None)
    })
}

/// A panel and what it needs: its title, and how many rows of content.
enum Panel {
    /// A title, the addresses, whether they open in a browser, and how to sign in to them.
    Links(&'static str, Vec<Link>, bool, Vec<Login>),
    Logos(Facts),
}

impl Panel {
    fn rows(&self) -> usize {
        match self {
            Panel::Links(_, links, _, logins) => {
                links.len()
                    + signed_in(links, logins)
                        .map(|g| g.lines.len())
                        .sum::<usize>()
            }
            Panel::Logos(_) => usize::from(stack_banner::SMALL_HEIGHT),
        }
    }

    /// Columns the content wants, with its borders.
    fn width(&self) -> u16 {
        match self {
            Panel::Links(_, links, _, logins) => {
                let label = links.iter().map(|l| l.label.len()).max().unwrap_or(0);
                let address = links.iter().map(|l| l.url.len()).max().unwrap_or(0);
                let login = signed_in(links, logins)
                    .map(|l| login_width(l) + label + 6)
                    .max()
                    .unwrap_or(0);
                (label + address + 14).max(login) as u16
            }
            Panel::Logos(facts) => stack_banner::small_width(facts) + 2,
        }
    }
}

/// Every panel there is to show: the addresses to open, the ones to connect to, and the logos.
fn panels(app: &App, width: u16) -> Vec<Panel> {
    let urls = &app.urls;
    if urls.is_empty() {
        return vec![];
    }
    let mut out = vec![];
    if !urls.web.is_empty() {
        out.push(Panel::Links(
            "open in a browser",
            urls.web.clone(),
            true,
            urls.logins.clone(),
        ));
    }
    if !urls.data.is_empty() {
        out.push(Panel::Links(
            "connect to",
            urls.data.clone(),
            false,
            urls.logins.clone(),
        ));
    }
    if let Some(facts) = logos_for(app, width) {
        out.push(Panel::Logos(facts));
    }
    out
}

/// How tall the row of panels is: the tallest one and its borders when they sit side by side, all
/// of them one under the other when it is too narrow. Nothing when there are no addresses.
pub fn height(app: &App, width: u16) -> u16 {
    let panels = panels(app, width);
    let rows = |p: &Panel| p.rows() as u16 + 2;
    if width >= SIDE_BY_SIDE {
        panels.iter().map(rows).max().unwrap_or(0).min(14)
    } else {
        panels.iter().map(rows).sum::<u16>().min(20)
    }
}

/// The sign-ins that belong to these addresses.
fn signed_in<'a>(links: &'a [Link], logins: &'a [Login]) -> impl Iterator<Item = &'a Login> {
    logins
        .iter()
        .filter(|g| links.iter().any(|l| l.label == g.label))
}

/// The widest of a sign-in's lines.
fn login_width(login: &Login) -> usize {
    login
        .lines
        .iter()
        .map(|line| line.iter().map(|(k, v)| k.len() + v.len() + 3).sum())
        .max()
        .unwrap_or(0)
}

/// A sign-in under its address, lined up with the address: `host localhost  port 5432`, then
/// `user app  password change-me  database app`.
fn login_lines(login: &Login, indent: usize) -> Vec<Line<'static>> {
    login
        .lines
        .iter()
        .map(|fields| {
            let mut spans = vec![Span::raw(" ".repeat(indent))];
            for (key, value) in fields {
                if value.is_empty() {
                    spans.push(Span::styled(format!("{key}  "), Style::new().fg(GRAY)));
                } else {
                    spans.push(Span::styled(format!("{key} "), Style::new().fg(GRAY)));
                    spans.push(Span::styled(format!("{value}  "), Style::new().fg(YELLOW)));
                }
            }
            Line::from(spans)
        })
        .collect()
}

/// A line per address: a dot for whether something answers, what it is, where, and the word;
/// under one that asks for a sign-in, what to sign in with.
fn rows(links: &[Link], web: bool, logins: &[Login]) -> Vec<Line<'static>> {
    let label = links.iter().map(|l| l.label.len()).max().unwrap_or(0);
    let address = links.iter().map(|l| l.url.len()).max().unwrap_or(0);
    links
        .iter()
        .flat_map(|l| {
            let (dot, word, color) = match l.up {
                Some(true) => ("●", "up", GREEN),
                Some(false) => ("○", "down", GRAY),
                None => (" ", "", GRAY),
            };
            let url = if web {
                Style::new().fg(CYAN).add_modifier(Modifier::UNDERLINED)
            } else {
                Style::new()
            };
            let line = Line::from(vec![
                Span::styled(format!("  {dot} "), Style::new().fg(color)),
                Span::styled(
                    format!("{:<label$}  ", l.label),
                    Style::new().fg(tech_color(&l.label)).bold(),
                ),
                Span::styled(format!("{:<address$}", l.url), url),
                Span::styled(format!("  {word}"), Style::new().fg(color)),
            ]);
            let login = logins
                .iter()
                .find(|g| g.label == l.label)
                .map(|g| login_lines(g, label + 6))
                .unwrap_or_default();
            std::iter::once(line).chain(login)
        })
        .collect()
}

/// One panel: a box with a title and its content.
fn draw(f: &mut Frame, area: Rect, panel: &Panel, tick: u64) {
    let title = match panel {
        Panel::Links(title, ..) => *title,
        Panel::Logos(_) => "built with",
    };
    let block = Block::bordered()
        .border_style(pane_border(false))
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Style::new().fg(GRAY),
        )));
    let inner = block.inner(area);
    f.render_widget(block, area);
    match panel {
        Panel::Links(_, links, web, logins) => {
            f.render_widget(Paragraph::new(rows(links, *web, logins)), inner)
        }
        Panel::Logos(facts) => stack_banner::render_small(f, inner, facts, tick),
    }
}

/// Where the open project is reachable and what it is built with, each in a panel of its own:
/// the web addresses, where to connect to its data, and the logos, with a dot for whether
/// something answers. `o` opens the app.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let panels = panels(app, area.width);
    if panels.is_empty() {
        return;
    }
    if area.width >= SIDE_BY_SIDE {
        // The logos keep what they need; the lists share the rest by what they hold.
        let widths: Vec<Constraint> = panels
            .iter()
            .map(|p| match p {
                Panel::Logos(_) => Constraint::Length(p.width()),
                _ => Constraint::Min(p.width()),
            })
            .collect();
        for (p, a) in panels
            .iter()
            .zip(Layout::horizontal(widths).split(area).iter())
        {
            draw(f, *a, p, app.tick);
        }
    } else {
        let heights: Vec<Constraint> = panels
            .iter()
            .map(|p| Constraint::Length(p.rows() as u16 + 2))
            .collect();
        for (p, a) in panels
            .iter()
            .zip(Layout::vertical(heights).split(area).iter())
        {
            draw(f, *a, p, app.tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::projects::urls::Urls;

    fn link(label: &str, up: Option<bool>) -> Link {
        Link {
            label: label.into(),
            url: format!("https://{label}.localhost"),
            up,
        }
    }

    #[test]
    fn it_takes_no_room_without_addresses_and_fits_its_lists() {
        let mut app = App::new(std::env::temp_dir(), vec![]);
        assert_eq!(height(&app, 120), 0);
        app.urls = Urls {
            web: vec![link("app", Some(true)), link("api", Some(false))],
            data: vec![link("postgres", None)],
            logins: vec![],
        };
        // No project open, so no logos. Side by side: the taller list and the borders; too narrow
        // for that: each panel under the other.
        assert_eq!(height(&app, 120), 2 + 2);
        assert_eq!(height(&app, 50), (2 + 2) + (1 + 2));
    }

    #[test]
    fn a_database_shows_what_to_sign_in_with_under_its_address() {
        let login = Login {
            label: "postgres".into(),
            lines: vec![
                vec![("host", "localhost".into()), ("port", "5432".into())],
                vec![("user", "app".into()), ("password", "s3cret".into())],
            ],
        };
        let lines = rows(
            &[link("postgres", None), link("redis", None)],
            false,
            &[login],
        );
        let text: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        assert_eq!(text.len(), 4, "{text:?}");
        assert!(text[0].contains("postgres"));
        assert!(text[1].contains("host localhost") && text[1].contains("port 5432"));
        assert!(text[2].contains("user app") && text[2].contains("password s3cret"));
        assert!(text[3].contains("redis"));
    }

    #[test]
    fn a_row_says_whether_something_answers() {
        let text = |l: &Line| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        };
        let lines = rows(
            &[link("app", Some(true)), link("api", Some(false))],
            true,
            &[],
        );
        assert!(text(&lines[0]).contains('●') && text(&lines[0]).ends_with("up"));
        assert!(text(&lines[1]).contains('○') && text(&lines[1]).ends_with("down"));
    }
}
