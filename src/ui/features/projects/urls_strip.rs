use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Padding, Paragraph},
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
        theme::{CYAN, GREEN, YELLOW, pane, pane_title},
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
                        .filter(|g| !fields(g).is_empty())
                        .count()
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
                (label + address + 10).max(login) as u16
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

/// How tall the row of panels is: the tallest one, its borders and the blank line above it when
/// they sit side by side, all of them one under the other when it is too narrow. Nothing when
/// there are no addresses.
pub fn height(app: &App, width: u16) -> u16 {
    let panels = panels(app, width);
    let rows = |p: &Panel| p.rows() as u16 + 3;
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

/// What a sign-in adds to its address: host and port are already in it, so only the rest.
fn fields(login: &Login) -> Vec<&(&'static str, String)> {
    login
        .lines
        .iter()
        .flatten()
        .filter(|(key, _)| !matches!(*key, "host" | "port"))
        .collect()
}

/// How wide a sign-in's line is.
fn login_width(login: &Login) -> usize {
    fields(login)
        .iter()
        .map(|(k, v)| k.len() + v.len() + 3)
        .sum()
}

/// A sign-in under its address, lined up with it: `user app  password change-me  database app`,
/// or a note such as `no password`.
fn login_line(login: &Login, indent: usize) -> Option<Line<'static>> {
    let fields = fields(login);
    if fields.is_empty() {
        return None;
    }
    let mut spans = vec![Span::raw(" ".repeat(indent))];
    for (key, value) in fields {
        if value.is_empty() {
            spans.push(Span::styled(
                format!("{key}  "),
                Style::new().fg(GRAY).italic(),
            ));
        } else {
            spans.push(Span::styled(format!("{key} "), Style::new().fg(GRAY)));
            spans.push(Span::styled(format!("{value}  "), Style::new().fg(YELLOW)));
        }
    }
    Some(Line::from(spans))
}

/// An address with its scheme quieted, so the eye lands on where it points.
fn address(url: &str, color: Style) -> Vec<Span<'static>> {
    match url.split_once("://") {
        Some((scheme, rest)) => vec![
            Span::styled(format!("{scheme}://"), Style::new().fg(GRAY)),
            Span::styled(rest.to_string(), color),
        ],
        None => vec![Span::styled(url.to_string(), color)],
    }
}

/// A line per address: a dot for whether something answers (filled and green when it does),
/// what it is and where; under one that asks for a sign-in, what to sign in with.
fn rows(links: &[Link], web: bool, logins: &[Login]) -> Vec<Line<'static>> {
    let label = links.iter().map(|l| l.label.len()).max().unwrap_or(0);
    links
        .iter()
        .flat_map(|l| {
            let dot = match l.up {
                Some(true) => Span::styled("● ", Style::new().fg(GREEN)),
                Some(false) => Span::styled("○ ", Style::new().fg(GRAY)),
                None => Span::raw("  "),
            };
            let url = if web {
                Style::new().fg(CYAN)
            } else {
                Style::new()
            };
            let mut spans = vec![
                dot,
                Span::styled(
                    format!("{:<label$}   ", l.label),
                    Style::new().fg(tech_color(&l.label)),
                ),
            ];
            spans.extend(address(&l.url, url));
            let login = logins
                .iter()
                .find(|g| g.label == l.label)
                .and_then(|g| login_line(g, label + 5));
            std::iter::once(Line::from(spans)).chain(login)
        })
        .collect()
}

/// One panel: a box with a title and its content.
fn draw(f: &mut Frame, area: Rect, panel: &Panel, tick: u64) {
    let title = match panel {
        Panel::Links(title, ..) => *title,
        Panel::Logos(_) => "built with",
    };
    let block = pane(false)
        .title(pane_title(title))
        .padding(Padding::new(1, 1, 1, 0));
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
            .map(|p| Constraint::Length(p.rows() as u16 + 3))
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
        assert_eq!(height(&app, 120), 2 + 3);
        assert_eq!(height(&app, 50), (2 + 3) + (1 + 3));
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
        // Host and port are in the address already: only the rest goes under it.
        assert_eq!(text.len(), 3, "{text:?}");
        assert!(text[0].contains("postgres"));
        assert!(text[1].contains("user app") && text[1].contains("password s3cret"));
        assert!(!text[1].contains("host"));
        assert!(text[2].contains("redis"));
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
        assert!(text(&lines[0]).starts_with('●'));
        assert!(text(&lines[1]).starts_with('○'));
    }
}
