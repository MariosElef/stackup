//! The command palette: every command in one box, filtered as you type.
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    models::{Action, Facts},
    ui::{
        stores::app::{App, Command, Entry, Outcome, Palette},
        types::Page,
    },
};

fn entry(label: &str, note: &str, command: Command) -> Entry {
    Entry {
        label: label.to_string(),
        note: note.to_string(),
        command,
    }
}

impl App {
    /// The pages the palette opens on; on the others keys belong to what is running or to the
    /// wizard's own command line.
    pub fn palette_available(&self) -> bool {
        matches!(self.page, Page::Home | Page::Projects | Page::Project)
    }

    pub fn open_palette(&mut self) {
        let entries = self.palette_entries();
        self.palette = Some(Palette {
            query: String::new(),
            entries,
            selected: 0,
        });
    }

    /// What can be done from here: always the same few, a jump to each project, and the actions
    /// that apply to the project that is selected or open.
    fn palette_entries(&self) -> Vec<Entry> {
        let mut out = vec![
            entry("Start a new project", "s", Command::NewProject),
            entry("My projects", "p", Command::MyProjects),
            entry("Add projects that already exist", "a", Command::AddProjects),
        ];
        if let Some(project) = self.project()
            && matches!(self.page, Page::Projects | Page::Project)
        {
            let facts = Facts::read(&project.path);
            for action in Action::ALL.iter().filter(|a| a.applies(&facts)) {
                out.push(entry(
                    &format!("{} · {}", project.name, action.name()),
                    action.about(),
                    Command::Run(*action),
                ));
            }
            out.push(entry(
                &format!("{} · new environment", project.name),
                "a .env file with new random passwords",
                Command::NewTier,
            ));
            out.push(entry(
                &format!("{} · open in the browser", project.name),
                "o: the app's address",
                Command::OpenApp,
            ));
            out.push(entry(
                &format!("{} · stop listing", project.name),
                "d: its files stay",
                Command::ForgetProject,
            ));
        }
        for (i, project) in self.projects.iter().enumerate() {
            out.push(entry(
                &format!("Open {}", project.name),
                &Facts::read(&project.path).summary(),
                Command::OpenProject(i),
            ));
        }
        out.push(entry("Keymaps", "?", Command::Keymaps));
        out.push(entry("Quit", "q", Command::Quit));
        out
    }

    /// Typing filters; the arrows or Ctrl-N/Ctrl-P move; Enter runs; Esc closes.
    pub fn palette_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let Some(palette) = &mut self.palette else {
            return;
        };
        let shown = palette.shown().len();
        match key.code {
            KeyCode::Esc => self.palette = None,
            KeyCode::Char('c') if ctrl => self.palette = None,
            KeyCode::Down => palette.selected = (palette.selected + 1).min(shown.saturating_sub(1)),
            KeyCode::Char('n') if ctrl => {
                palette.selected = (palette.selected + 1).min(shown.saturating_sub(1));
            }
            KeyCode::Up => palette.selected = palette.selected.saturating_sub(1),
            KeyCode::Char('p') if ctrl => palette.selected = palette.selected.saturating_sub(1),
            KeyCode::Backspace => {
                if palette.query.pop().is_none() {
                    self.palette = None;
                } else {
                    palette.selected = 0;
                }
            }
            KeyCode::Char(c) if !ctrl => {
                palette.query.push(c);
                palette.selected = 0;
            }
            KeyCode::Enter => {
                let command = palette
                    .shown()
                    .get(palette.selected)
                    .map(|e| e.command.clone());
                self.palette = None;
                if let Some(command) = command {
                    self.run_command(command);
                }
            }
            _ => {}
        }
    }

    /// Does what the key for it would do.
    fn run_command(&mut self, command: Command) {
        match command {
            Command::NewProject => self.ask_name(),
            Command::MyProjects => self.open_projects(),
            Command::AddProjects => {
                self.open_projects();
                self.ask_add();
            }
            Command::Keymaps => self.help = true,
            Command::Quit => self.outcome = Outcome::Quit,
            Command::OpenProject(i) => {
                self.cursor = i;
                self.open_project();
            }
            Command::Run(action) => self.run_from_palette(action),
            Command::ForgetProject => self.ask_forget(),
            Command::OpenApp => self.open_app(),
            Command::NewTier => {
                if self.page != Page::Project {
                    self.open_project();
                }
                self.ask_new_tier();
            }
        }
    }

    /// An action on the selected project, from wherever the palette was opened. It need not be in
    /// the project's list: the status, log, commit and branch actions are only here.
    pub fn run_from_palette(&mut self, action: Action) {
        if self.page != Page::Project {
            self.open_project();
        }
        self.begin_action(action);
    }
}
