//! What is printed when the project starts for development: which parts run and where, with each
//! technology's logo, so the output that follows (the compilers, the watchers) has a header that
//! says what it belongs to. The logos are drawn in half blocks with 24-bit colors.
use std::path::Path;

use serde_json::Value;

use super::{
    logos::{self, Bitmap, Rgb, shrink},
    urls::{self, Link},
};
use crate::{
    clients::docker,
    models::{Backend, Facts, Frontend},
};

/// Which parts of the project start.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Which {
    Backend,
    Frontend,
    Both,
}

/// How tall the logos are, in pixels: two to a row of the terminal.
const PIXELS: usize = 8;
/// The room the logo gets, so the text beside it lines up.
const LOGO_COLUMNS: usize = 18;

const DIM: &str = "\x1b[38;5;244m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[38;5;45m";
const UNDERLINE: &str = "\x1b[4m";
const RESET: &str = "\x1b[0m";

fn fg((r, g, b): Rgb) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

fn bg((r, g, b): Rgb) -> String {
    format!("\x1b[48;2;{r};{g};{b}m")
}

/// What one part is: its name, its logo, its color, and how it runs.
struct Part {
    role: &'static str,
    name: &'static str,
    icon: &'static Bitmap,
    ink: Rgb,
    /// The command it runs and the folder it runs in.
    runs: String,
    folder: &'static str,
    /// The address it answers on when it runs here, and the same behind the HTTPS proxy.
    here: String,
    secure: Option<String>,
    /// Whether the proxy answers on the secure address right now.
    proxy_up: bool,
    /// Whether something answers on the plain address right now.
    serving: bool,
    /// Whether this start runs it; when not, it is listed with how to start it.
    started: bool,
}

/// Whether something answers on the port of this address.
fn answering(link: &Link) -> bool {
    urls::port_of(&link.url).is_some_and(urls::listening)
}

fn find<'a>(links: &'a [Link], label: &str) -> Option<&'a Link> {
    links.iter().find(|l| l.label == label)
}

fn backend(facts: &Facts, links: &[Link]) -> Option<Part> {
    // The path the link opens: the api has no page at `/`, so a bare address is a blank 404.
    let (name, icon, ink, runs, page) = match facts.backend {
        Backend::Axum => (
            "axum",
            &logos::FERRIS,
            (0xF7, 0x4C, 0x00),
            "cargo watch -x run",
            "/health",
        ),
        Backend::Fastapi => (
            "FastAPI",
            &logos::FASTAPI,
            (0x00, 0xB8, 0xA9),
            "uv run uvicorn app.main:app --reload",
            "/docs",
        ),
        Backend::None => return None,
    };
    Some(Part {
        role: "backend",
        name,
        icon,
        ink,
        runs: runs.into(),
        folder: "backend",
        here: find(links, "api").map_or("http://localhost:8000".into(), |l| l.url.clone()) + page,
        secure: find(links, "api (https)").map(|l| format!("{}{page}", l.url)),
        proxy_up: find(links, "api (https)").is_some_and(answering),
        serving: find(links, "api").is_some_and(answering),
        started: true,
    })
}

fn frontend(facts: &Facts, links: &[Link]) -> Option<Part> {
    let (name, icon, ink, runs) = match facts.frontend {
        Frontend::Nuxt => ("Nuxt", &logos::NUXT, (0x00, 0xDC, 0x82), "pnpm dev"),
        Frontend::Vue => ("Vue", &logos::VUE, (0x41, 0xB8, 0x83), "pnpm dev"),
        Frontend::Topcoat => (
            "Topcoat",
            &logos::TOPCOAT_ICON,
            (0xEE, 0xEE, 0xF2),
            "topcoat dev",
        ),
        Frontend::None => return None,
    };
    Some(Part {
        role: "frontend",
        name,
        icon,
        ink,
        runs: runs.into(),
        folder: "frontend",
        here: find(links, "app").map_or("http://localhost:3000".into(), |l| l.url.clone()),
        secure: find(links, "app (https)").map(|l| l.url.clone()),
        proxy_up: find(links, "app (https)").is_some_and(answering),
        serving: find(links, "app").is_some_and(answering)
            || (facts.frontend == Frontend::Vue && urls::listening(super::https_dev::VITE_PORT)),
        started: true,
    })
}

/// The logo as terminal rows of half blocks, each `LOGO_COLUMNS` wide.
fn logo_rows(icon: &Bitmap) -> Vec<String> {
    let px = shrink(&icon.pixels(), PIXELS);
    px.chunks(2)
        .map(|pair| {
            let mut row = String::new();
            let mut used = 0;
            for x in 0..pair[0].len() {
                let top = pair[0][x];
                let bottom = pair.get(1).and_then(|r| r[x]);
                match (top, bottom) {
                    (None, None) => row.push(' '),
                    (Some(t), Some(b)) => row.push_str(&format!("{}{}▀{RESET}", fg(t), bg(b))),
                    (Some(t), None) => row.push_str(&format!("{}▀{RESET}", fg(t))),
                    (None, Some(b)) => row.push_str(&format!("{}▄{RESET}", fg(b))),
                }
                used += 1;
            }
            row.push_str(&" ".repeat(LOGO_COLUMNS.saturating_sub(used)));
            row
        })
        .collect()
}

/// A part as its logo with lines of text beside it.
fn block(part: &Part) -> Vec<String> {
    let state = if part.started {
        String::new()
    } else {
        format!("  {DIM}not started{RESET}")
    };
    let mut text = vec![
        format!(
            "{DIM}{}{RESET} {}{BOLD}{}{RESET}{state}",
            part.role,
            fg(part.ink),
            part.name
        ),
        format!("{DIM}here   {RESET}{CYAN}{UNDERLINE}{}{RESET}", part.here),
    ];
    if let Some(secure) = &part.secure {
        // A link only looks like one when it will answer: the proxy is up, and the server behind
        // it is either started by this run (the banner comes just before it starts) or running.
        let live = part.proxy_up && (part.started || part.serving);
        text.push(if live {
            format!("{DIM}https  {RESET}{CYAN}{UNDERLINE}{secure}{RESET}")
        } else {
            format!(
                "{DIM}https  {secure}  not running: needs the proxy and this {} started{RESET}",
                part.role
            )
        });
    }
    text.push(if part.started {
        format!("{DIM}runs   {}  in {}/{RESET}", part.runs, part.folder)
    } else {
        format!("{DIM}start  stackup start {}{RESET}", part.role)
    });
    let logo = logo_rows(part.icon);
    let rows = logo.len().max(text.len());
    (0..rows)
        .map(|i| {
            let l = logo
                .get(i)
                .cloned()
                .unwrap_or_else(|| " ".repeat(LOGO_COLUMNS));
            format!("  {l}{}", text.get(i).cloned().unwrap_or_default())
        })
        .collect()
}

/// The banner for starting `which` parts of the project in `root`.
pub fn banner(root: &Path, facts: &Facts, which: Which) -> String {
    let config = docker::config(root).unwrap_or(Value::Null);
    let caddyfile = std::fs::read_to_string(root.join("Caddyfile")).ok();
    let mut urls = urls::links(&config, caddyfile.as_deref());
    // The databases' addresses with what to sign in with, ready to paste into a client.
    urls::add_logins(&mut urls, &super::envs::dotenv(root));
    // Every part the project has: the ones this start runs, then the others as not started, so
    // each address is on the screen.
    let mut parts = vec![];
    parts.extend(backend(facts, &urls.web).map(|mut p| {
        p.started = which != Which::Frontend;
        p
    }));
    parts.extend(frontend(facts, &urls.web).map(|mut p| {
        p.started = which != Which::Backend;
        p
    }));
    let mut out = String::from("\n");
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for line in block(part) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    if which != Which::Frontend && !urls.data.is_empty() {
        let data: Vec<String> = urls
            .data
            .iter()
            .map(|l| format!("{}{}{RESET} {DIM}{}{RESET}", CYAN, l.label, l.url))
            .collect();
        out.push_str(&format!(
            "\n  {DIM}databases in Docker{RESET}  {}\n",
            data.join("  ")
        ));
    }
    out.push_str(&format!(
        "\n  {DIM}Ctrl-C stops {}{RESET}\n\n",
        if parts.iter().filter(|p| p.started).count() > 1 {
            "both"
        } else {
            "it"
        }
    ));
    out
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
            migrations: false,
        }
    }

    /// The text of a banner without its colors.
    fn plain(s: &str) -> String {
        let mut out = String::new();
        let mut skipping = false;
        for c in s.chars() {
            match (skipping, c) {
                (_, '\x1b') => skipping = true,
                (true, 'm') => skipping = false,
                (false, c) => out.push(c),
                _ => {}
            }
        }
        out
    }

    fn links() -> Vec<Link> {
        let link = |label: &str, url: &str| Link {
            label: label.into(),
            url: url.into(),
            up: None,
        };
        vec![
            link("app", "http://localhost:3000"),
            link("api", "http://localhost:8000"),
            link("app (https)", "https://app.localhost:58443"),
            link("api (https)", "https://api.localhost:58443"),
        ]
    }

    #[test]
    fn a_secure_link_is_live_for_a_server_this_run_starts_once_the_proxy_answers() {
        // The proxy answers; the servers behind it do not yet, as when the banner is printed.
        let proxy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = proxy.local_addr().unwrap().port();
        let closed = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let link = |label: &str, url: String| Link {
            label: label.into(),
            url,
            up: None,
        };
        let links = [
            link("api", format!("http://localhost:{closed}")),
            link("api (https)", format!("https://api.localhost:{port}")),
        ];
        let f = facts(Frontend::None, Backend::Axum);
        let mut back = backend(&f, &links).unwrap();
        let text = plain(&block(&back).join("\n"));
        assert!(!text.contains("not running"), "this run starts it: {text}");
        back.started = false;
        let text = plain(&block(&back).join("\n"));
        assert!(text.contains("not running"), "nothing will answer: {text}");
    }

    #[test]
    fn each_part_says_where_it_runs_and_the_https_address_when_there_is_one() {
        let f = facts(Frontend::Topcoat, Backend::Axum);
        let back = plain(&block(&backend(&f, &links()).unwrap()).join("\n"));
        assert!(back.contains("backend axum"), "{back}");
        assert!(back.contains("http://localhost:8000/health"), "{back}");
        assert!(
            back.contains("https://api.localhost:58443/health"),
            "{back}"
        );
        assert!(
            back.contains("not running: needs the proxy and this backend started"),
            "nothing answers on 58443: {back}"
        );
        assert!(back.contains("cargo watch -x run  in backend/"), "{back}");
        let front = plain(&block(&frontend(&f, &links()).unwrap()).join("\n"));
        assert!(front.contains("topcoat dev  in frontend/"), "{front}");
        assert!(front.contains("https://app.localhost:58443"), "{front}");
        // Without the proxy there is only the plain address, and the default port.
        let bare = plain(&block(&frontend(&f, &[]).unwrap()).join("\n"));
        assert!(
            bare.contains("http://localhost:3000") && !bare.contains("https"),
            "{bare}"
        );
    }

    #[test]
    fn the_logo_is_drawn_beside_the_text_and_every_line_starts_in_the_same_column() {
        let f = facts(Frontend::Nuxt, Backend::Fastapi);
        let lines = block(&frontend(&f, &[]).unwrap());
        assert!(lines.len() >= 4);
        assert!(lines[0].contains('▀') || lines[0].contains('▄'));
        for l in &lines {
            let p = plain(l);
            let text = p.trim_start_matches(' ');
            let before = p.chars().count() - text.chars().count();
            assert!(before <= 2 + LOGO_COLUMNS);
        }
        let widths: Vec<usize> = lines
            .iter()
            .map(|l| plain(l).chars().take(2 + LOGO_COLUMNS).count())
            .collect();
        assert!(widths.iter().all(|w| *w == 2 + LOGO_COLUMNS), "{widths:?}");
    }

    #[test]
    fn a_project_without_a_part_prints_only_the_other() {
        let f = facts(Frontend::None, Backend::Fastapi);
        assert!(frontend(&f, &[]).is_none());
        assert!(backend(&f, &[]).is_some());
        let none = facts(Frontend::None, Backend::None);
        assert!(backend(&none, &[]).is_none() && frontend(&none, &[]).is_none());
    }

    #[test]
    fn the_part_that_does_not_start_is_listed_with_its_address_and_how_to_start_it() {
        let f = facts(Frontend::Topcoat, Backend::Axum);
        let mut front = frontend(&f, &links()).unwrap();
        front.started = false;
        let text = plain(&block(&front).join("\n"));
        assert!(text.contains("not started"), "{text}");
        assert!(text.contains("http://localhost:3000"), "{text}");
        assert!(text.contains("stackup start frontend"), "{text}");
        assert!(!text.contains("topcoat dev"), "{text}");
    }
}
