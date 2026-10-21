//! What the bottom of the screen says, for the wizard and for the app's pages.
use crate::{
    services::toolchain::versions::Question,
    ui::{
        features::create::wizard::Wizard,
        stores::app::{App, PromptKind},
        theme::{BLUE, GREEN, PINK, RED, YELLOW},
        types::{Focus, Footer, FooterLine, Mode, Page},
    },
};

impl Wizard {
    pub fn footer(&self) -> Footer {
        let (mode, mode_color) = match self.mode {
            Mode::Normal => (" NORMAL ", GREEN),
            Mode::Command => (" COMMAND ", YELLOW),
            Mode::Search => (" SEARCH ", PINK),
            Mode::Help => (" HELP ", BLUE),
        };
        let focus = if self.focus == Focus::Menu {
            "menu"
        } else {
            "preview"
        };
        let line = match self.mode {
            Mode::Command => FooterLine::Input(format!(":{}█", self.input)),
            Mode::Search => FooterLine::Input(format!("/{}█", self.input)),
            _ if !self.status.is_empty() => FooterLine::Status(self.status.clone()),
            _ => FooterLine::Hint(
                " <C-h>/<C-l> panes · j/k move · <space> tick · / search · : command · ? keymaps",
            ),
        };
        Footer {
            mode,
            mode_color,
            focus: focus.to_string(),
            pending: format!("{}{}", self.count, self.seq),
            needs: self
                .requires()
                .into_iter()
                .map(|bin| (bin, self.is_installed(bin)))
                .collect(),
            line,
        }
    }
}

impl App {
    pub fn footer(&self) -> Footer {
        if self.tier_modal.is_some() {
            return Footer {
                mode: " NEW ENV ",
                mode_color: GREEN,
                focus: "environment".into(),
                pending: String::new(),
                needs: vec![],
                line: FooterLine::Hint(" ←/→ choose · Enter confirm · s switch to it · c close"),
            };
        }
        if self.palette.is_some() {
            return Footer {
                mode: " COMMAND ",
                mode_color: YELLOW,
                focus: "command palette".into(),
                pending: String::new(),
                needs: vec![],
                line: FooterLine::Hint(" type to filter · ↑/↓ choose · Enter run · Esc close"),
            };
        }
        if let Some(choice) = &self.choice {
            // Environments are switched, versions are chosen: the bar says which this is.
            let environments = choice.question == Question::Tier;
            return Footer {
                mode: if environments {
                    " ENVIRONMENTS "
                } else {
                    " CHOOSE "
                },
                mode_color: YELLOW,
                focus: if environments {
                    "which one is active"
                } else {
                    "version"
                }
                .into(),
                pending: String::new(),
                needs: vec![],
                line: FooterLine::Hint(if environments {
                    " ↑/↓ choose · Enter switch · n new environment · Esc close"
                } else {
                    " ↑/↓ choose · 1-9 pick · Enter confirm · Esc back"
                }),
            };
        }
        if self.autofix.is_some() {
            return Footer {
                mode: " AUTOFIX ",
                mode_color: YELLOW,
                focus: "something failed".into(),
                pending: String::new(),
                needs: vec![],
                line: FooterLine::Hint(" ←/→ choose · Enter confirm · a autofix · c close"),
            };
        }
        if self.install.is_some() {
            return Footer {
                mode: " INSTALL ",
                mode_color: YELLOW,
                focus: "missing tools".into(),
                pending: String::new(),
                needs: vec![],
                line: FooterLine::Hint(if self.install.as_ref().is_some_and(|m| m.can_install()) {
                    " ←/→ choose · Enter confirm · i install · c close"
                } else {
                    " Enter close"
                }),
            };
        }
        if self.page == Page::New
            && self.prompt.is_none()
            && let Some(wizard) = &self.wizard
        {
            return wizard.footer();
        }
        let (mode, mode_color) = match (&self.prompt, self.help) {
            (_, true) => (" HELP ", BLUE),
            (Some(p), _) if matches!(p.kind, PromptKind::Confirm { .. }) => (" CONFIRM ", RED),
            (Some(_), _) => (" ASK ", YELLOW),
            _ => (" NORMAL ", GREEN),
        };
        let focus = match self.page {
            Page::Home => "home".to_string(),
            Page::Projects => format!("projects · {}", self.projects.len()),
            Page::Project => self.project().map_or("project".into(), |p| p.name.clone()),
            Page::Run | Page::Install => self
                .run
                .as_ref()
                .map_or("running".into(), |r| r.title.clone()),
            Page::New => "new project".to_string(),
        };
        let line = if let Some(p) = &self.prompt {
            FooterLine::Input(format!("{}: {}█", p.label, p.value))
        } else if !self.status.is_empty() {
            FooterLine::Status(self.status.clone())
        } else {
            FooterLine::Hint(match self.page {
                Page::Home => {
                    " j/k move · <CR> open · s start · p projects · : commands · q quit · ? keymaps"
                }
                Page::Projects => {
                    " j/k move · <CR> open · n new · a add · d forget · r remove · : commands · h back · ? keymaps"
                }
                Page::Project => {
                    " j/k move · <CR> run · e env · o open · r remove · : commands · h back · ? keymaps"
                }
                Page::Run | Page::Install if self.run.as_ref().is_some_and(|r| r.live()) => {
                    " Ctrl-] back to the menu (stops it) · every other key goes to the program"
                }
                Page::Run | Page::Install => " q back",
                Page::New => "",
            })
        };
        Footer {
            mode,
            mode_color,
            focus,
            pending: self.seq.clone(),
            needs: vec![],
            line,
        }
    }
}
