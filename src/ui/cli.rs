//! How the command line looks: the same palette as the menu, as plain ANSI codes. Colors switch
//! off when output is not a terminal, or when `NO_COLOR` is set.
use std::{
    env,
    io::{self, IsTerminal, Write, stderr, stdout},
};

use anyhow::{Result, bail};

use crate::{
    models::{Facts, Project, Step},
    services::projects::{doctor::Check, urls::Urls},
};

const CYAN: u8 = 45;
const GREEN: u8 = 46;
const YELLOW: u8 = 220;
const RED: u8 = 203;

fn paint_if(on: bool, code: u8, text: &str) -> String {
    if on {
        format!("\x1b[38;5;{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn colors(terminal: bool) -> bool {
    terminal && env::var_os("NO_COLOR").is_none()
}

fn paint(code: u8, text: &str) -> String {
    paint_if(colors(stdout().is_terminal()), code, text)
}

fn dim(text: &str) -> String {
    if colors(stdout().is_terminal()) {
        format!("\x1b[2m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// For messages that go to stderr, which may be a terminal when stdout is not.
pub fn error(text: &str) -> String {
    paint_if(colors(stderr().is_terminal()), RED, text)
}

fn tier_code(tier: &str) -> Option<u8> {
    match tier {
        "local" => Some(GREEN),
        "dev" => Some(YELLOW),
        // Prod in red everywhere it shows, so it is never mistaken for the others.
        "prod" => Some(RED),
        _ => None,
    }
}

pub fn tier(tier: &str) -> String {
    tier_code(tier).map_or_else(|| tier.to_string(), |c| paint(c, tier))
}

/// A step about to run: what it is for, then the command.
pub fn show_step(step: &Step) {
    println!("{}", paint(CYAN, &format!("# {}", step.label)));
    println!("{}", dim(&format!("$ {}", step.command_line())));
}

/// One line per check, a fix for each failure. Returns how many failed.
pub fn show_checks(checks: &[Check]) -> usize {
    let mut failed = 0;
    for c in checks {
        match &c.fix {
            None => println!("  {} {}", paint(GREEN, "✓"), c.name),
            Some(fix) => {
                failed += 1;
                println!("  {} {}: {fix}", paint(RED, "✗"), paint(RED, &c.name));
            }
        }
    }
    if failed == 0 {
        println!(
            "\n{}",
            paint(GREEN, &format!("all {} checks passed", checks.len()))
        );
    }
    failed
}

pub fn show_projects(projects: &[Project]) {
    if projects.is_empty() {
        println!("no projects yet: `stackup new NAME` makes one, or run `stackup` for the menu");
    }
    for p in projects {
        let facts = Facts::read(&p.path);
        let what = if facts.exists {
            facts.summary()
        } else {
            paint(RED, "missing")
        };
        println!(
            "  {:<20} {:<28} {}",
            paint(GREEN, &p.name),
            what,
            dim(&p.path.display().to_string())
        );
    }
}

/// Where a project is reachable: a dot for whether something is answering, then the address.
pub fn show_urls(urls: &Urls) {
    let row = |l: &crate::services::projects::urls::Link| {
        let dot = match l.up {
            Some(true) => paint(GREEN, "●"),
            Some(false) => dim("○"),
            None => " ".to_string(),
        };
        println!("  {dot} {:<13} {}", paint(CYAN, &l.label), l.url);
    };
    urls.web.iter().for_each(row);
    if !urls.data.is_empty() {
        println!("{}", dim("  connect to its data:"));
        urls.data.iter().for_each(row);
    }
}

/// The environments a project has, the active one marked.
pub fn show_envs(tiers: &[String], current: Option<&str>) {
    if tiers.is_empty() && current == Some("local") {
        // Only a `.env`: that is the one environment, and it is the local one.
        println!(
            "  {} {}{}",
            paint(GREEN, "●"),
            tier("local"),
            dim("  active (.env)")
        );
        println!(
            "{}",
            dim("  another: `stackup env NAME --create` (saves .env as .env.local first)")
        );
    } else if tiers.is_empty() {
        println!("no .env yet: copy .env.example to .env and fill it in");
    }
    for t in tiers {
        let mark = if current == Some(t.as_str()) {
            paint(GREEN, "●")
        } else {
            dim("○")
        };
        let note = if current == Some(t.as_str()) {
            dim("  active")
        } else {
            String::new()
        };
        println!("  {mark} {}{note}", tier(t));
    }
    if current == Some("custom") {
        println!(
            "{}",
            dim("  .env was edited by hand: it matches none of them")
        );
    }
}

/// What making an environment did, and what is left for the person.
pub fn show_created(tier_name: &str, made: &crate::services::projects::envs::Created) {
    println!("made .env.{} (readable only by you)", tier(tier_name));
    if !made.generated.is_empty() {
        println!("  new random values: {}", made.generated.join(", "));
    }
    if !made.review.is_empty() {
        println!(
            "  {} {}",
            paint(YELLOW, "still the template's, look at:"),
            made.review.join(", ")
        );
    }
    println!("  switch to it with: stackup env {tier_name}");
}

/// Makes the name be typed back before something that leaves this machine, as mc2 does.
pub fn confirm(what: &str, word: &str) -> Result<()> {
    if !io::stdin().is_terminal() {
        bail!("{what}\nthis needs your confirmation: pass --yes when there is no terminal");
    }
    print!("{what}\nType `{word}` to continue: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    if line.trim() != word {
        bail!("not confirmed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_is_plain_text_unless_switched_on() {
        assert_eq!(paint_if(false, GREEN, "ok"), "ok");
        assert_eq!(paint_if(true, GREEN, "ok"), "\x1b[38;5;46mok\x1b[0m");
    }

    #[test]
    fn prod_is_red_dev_yellow_local_green() {
        assert_eq!(tier_code("prod"), Some(RED));
        assert_eq!(tier_code("dev"), Some(YELLOW));
        assert_eq!(tier_code("local"), Some(GREEN));
        assert_eq!(tier_code("staging"), None);
    }
}
