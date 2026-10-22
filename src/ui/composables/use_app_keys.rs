//! Keys on the app's pages. The motions are the wizard's: `j`/`k`, `gg`, `G`, `h` back, `l` in.
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    models::Action,
    services::toolchain::versions::Question,
    ui::{
        stores::app::{App, Outcome},
        types::{Outcome as WizardOutcome, Page},
        utils::key_bytes,
    },
};

/// The home menu: Start, My projects, Quit.
pub const HOME_ITEMS: usize = 3;

enum Move {
    Down,
    Up,
    Top,
    Bottom,
}

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        // Any key ends the goodbye early, and the celebration.
        if self.farewell.is_some() {
            return self.end_farewell();
        }
        if self.celebration.is_some() {
            return self.end_celebration();
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if self.tier_modal.is_some() {
            return self.tier_key(key);
        }
        if self.choice.is_some() {
            return self.choice_key(key);
        }
        if self.autofix.is_some() {
            return self.autofix_key(key);
        }
        if self.github.is_some() {
            return self.github_key(key);
        }
        if self.palette.is_some() {
            return self.palette_key(key);
        }
        if self.help {
            self.help = false;
            return;
        }
        if self.prompt.is_some() {
            return self.prompt_key(key);
        }
        let opens_palette =
            key.code == KeyCode::Char(':') || (ctrl && key.code == KeyCode::Char('p'));
        if opens_palette && self.prompt.is_none() && self.palette_available() {
            return self.open_palette();
        }
        match self.page {
            Page::New => self.new_key(key),
            Page::Run | Page::Install => self.run_key(key),
            _ if ctrl && key.code == KeyCode::Char('c') => self.outcome = Outcome::Quit,
            Page::Home => self.home_key(key),
            Page::Projects => self.projects_key(key),
            Page::Project => self.project_key(key),
        }
    }

    /// `j`, `k`, `gg` and `G` on a list of `len` items.
    fn motion(&mut self, key: KeyEvent) -> Option<Move> {
        let code = key.code;
        if !self.seq.is_empty() {
            self.seq.clear();
            return (code == KeyCode::Char('g')).then_some(Move::Top);
        }
        match code {
            KeyCode::Char('j') | KeyCode::Down => Some(Move::Down),
            KeyCode::Char('k') | KeyCode::Up => Some(Move::Up),
            KeyCode::Char('G') => Some(Move::Bottom),
            KeyCode::Char('g') => {
                self.seq.push('g');
                None
            }
            _ => None,
        }
    }

    fn moved(cursor: usize, len: usize, m: Move) -> usize {
        match m {
            Move::Down => (cursor + 1).min(len.saturating_sub(1)),
            Move::Up => cursor.saturating_sub(1),
            Move::Top => 0,
            Move::Bottom => len.saturating_sub(1),
        }
    }

    fn home_key(&mut self, key: KeyEvent) {
        self.status.clear();
        if let Some(m) = self.motion(key) {
            self.home = Self::moved(self.home, HOME_ITEMS, m);
            return;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Char('l' | ' ') => match self.home {
                0 => self.ask_name(),
                1 => self.open_projects(),
                _ => self.outcome = Outcome::Quit,
            },
            KeyCode::Char('s') => self.ask_name(),
            KeyCode::Char('p') => self.open_projects(),
            KeyCode::Char('q') | KeyCode::Esc => self.outcome = Outcome::Quit,
            KeyCode::Char('?') => self.help = true,
            _ => {}
        }
    }

    fn projects_key(&mut self, key: KeyEvent) {
        self.status.clear();
        if let Some(m) = self.motion(key) {
            self.cursor = Self::moved(self.cursor, self.projects.len(), m);
            self.load_overview();
            return;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Char('l' | ' ') => self.open_project(),
            KeyCode::Char('n' | 's') => self.ask_name(),
            KeyCode::Char('a') => self.ask_add(),
            KeyCode::Char('d') => self.ask_forget(),
            KeyCode::Char('r') => self.begin_remove(),
            KeyCode::Char('o') => self.open_app(),
            KeyCode::Char('e') => self.run_from_palette(Action::Env),
            KeyCode::Char('h' | 'q') | KeyCode::Esc => self.back(),
            KeyCode::Char('?') => self.help = true,
            _ => {}
        }
    }

    /// `}` and `{`: to the first action of the next or the previous section.
    fn jump_section(&mut self, forward: bool) {
        let starts: Vec<usize> = (0..self.actions.len())
            .filter(|&i| i == 0 || self.actions[i].group() != self.actions[i - 1].group())
            .collect();
        let target = if forward {
            starts.iter().find(|&&i| i > self.action)
        } else {
            starts.iter().rev().find(|&&i| i < self.action)
        };
        if let Some(&i) = target {
            self.action = i;
        }
    }

    fn project_key(&mut self, key: KeyEvent) {
        self.status.clear();
        if self.dev_part.is_some() {
            return self.dev_part_key(key);
        }
        if let Some(m) = self.motion(key) {
            self.action = Self::moved(self.action, self.actions.len(), m);
            return;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Char('l' | ' ') => self.choose_action(),
            KeyCode::Char('}') => self.jump_section(true),
            KeyCode::Char('{') => self.jump_section(false),
            KeyCode::Char('r') => self.begin_remove(),
            KeyCode::Char('o') => self.open_app(),
            KeyCode::Char('e') => self.run_from_palette(Action::Env),
            KeyCode::Char('h' | 'q') | KeyCode::Esc => self.back(),
            KeyCode::Char('?') => self.help = true,
            _ => {}
        }
    }

    /// In `dev`'s panel: `j`/`k` pick the part, Enter starts it, Esc goes back to the actions.
    fn dev_part_key(&mut self, key: KeyEvent) {
        let last = self.dev_parts().len().saturating_sub(1);
        let Some(row) = &mut self.dev_part else {
            return;
        };
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => *row = (*row + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => *row = row.saturating_sub(1),
            KeyCode::Enter | KeyCode::Char('l' | ' ') => self.start_dev_part(),
            KeyCode::Esc | KeyCode::Char('h' | 'q') => self.dev_part = None,
            _ => {}
        }
    }

    /// While something runs, every key goes to it, except Ctrl-] which comes back to the menu.
    fn run_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let live = self.run.as_ref().is_some_and(|r| r.live());
        if live {
            if ctrl && key.code == KeyCode::Char(']') {
                self.back();
            } else if let Some(run) = &mut self.run {
                run.type_bytes(&key_bytes(key));
            }
            return;
        }
        match key.code {
            KeyCode::Char('q' | 'h') | KeyCode::Esc | KeyCode::Enter => self.back(),
            KeyCode::Char('c' | ']') if ctrl => self.back(),
            _ => {}
        }
    }

    /// The wizard has its own keys and its own modes; the app only hears how it ended, and when
    /// Create was blocked by a missing tool.
    fn new_key(&mut self, key: KeyEvent) {
        if self.install.is_some() {
            return self.install_key(key);
        }
        let Some(wizard) = &mut self.wizard else {
            return self.back();
        };
        wizard.handle_key(key);
        if let Some(missing) = wizard.blocked.take() {
            return self.open_install_modal(missing);
        }
        match wizard.outcome {
            WizardOutcome::Running => {}
            WizardOutcome::Cancel => self.back(),
            WizardOutcome::Create if !self.wizard_only => self.start_create(),
            WizardOutcome::Create => {
                let name = wizard.name.clone();
                self.selection = wizard.sel.clone();
                self.outcome = Outcome::Start { name };
            }
        }
    }

    /// Install or Close: arrows or `h`/`l` choose, Enter confirms, `i` and `c` are shortcuts.
    fn install_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let Some(modal) = &mut self.install else {
            return;
        };
        let can = modal.can_install();
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::Char('h' | 'l') if can => {
                modal.selected = 1 - modal.selected;
            }
            KeyCode::Char('i') if can => self.do_install(),
            KeyCode::Enter if can && modal.selected == 0 => self.do_install(),
            KeyCode::Enter | KeyCode::Char('c' | 'q') | KeyCode::Esc => self.install = None,
            KeyCode::Char('x') if ctrl => self.install = None,
            _ if ctrl && key.code == KeyCode::Char('c') => self.install = None,
            _ => {}
        }
    }

    /// Switch to the environment that was just made, or leave it.
    fn tier_key(&mut self, key: KeyEvent) {
        let Some(modal) = &mut self.tier_modal else {
            return;
        };
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::Char('h' | 'l') => {
                modal.selected = 1 - modal.selected;
            }
            KeyCode::Char('s') => self.answer_tier_modal(true),
            KeyCode::Enter => {
                let switch = modal.selected == 0;
                self.answer_tier_modal(switch);
            }
            KeyCode::Esc | KeyCode::Char('c' | 'q') => self.tier_modal = None,
            _ => {}
        }
    }

    /// A version question: `j`/`k` or the arrows move, a number picks, Enter confirms, Esc goes
    /// back to what was asked before.
    fn choice_key(&mut self, key: KeyEvent) {
        let Some(choice) = &mut self.choice else {
            return;
        };
        let last = choice.options.len().saturating_sub(1);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => choice.selected = (choice.selected + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => choice.selected = choice.selected.saturating_sub(1),
            KeyCode::Char(c @ '1'..='9') => {
                let n = c as usize - '1' as usize;
                if n <= last {
                    choice.selected = n;
                }
            }
            KeyCode::Enter => self.answer_choice(),
            // In the environments list, `n` goes straight to making a new one.
            KeyCode::Char('n') if choice.question == Question::Tier => {
                choice.selected = last;
                self.answer_choice();
            }
            KeyCode::Esc | KeyCode::Char('q' | 'c') => self.choice = None,
            _ => {}
        }
    }

    /// Set up or Skip: arrows or `h`/`l` choose, Enter confirms, `s` and `k` are shortcuts.
    fn github_key(&mut self, key: KeyEvent) {
        let Some(modal) = &mut self.github else {
            return;
        };
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::Char('h' | 'l') => {
                modal.selected = 1 - modal.selected;
            }
            KeyCode::Char('s') => self.do_github_setup(),
            KeyCode::Enter if modal.selected == 0 => self.do_github_setup(),
            KeyCode::Enter | KeyCode::Char('k' | 'c' | 'q') | KeyCode::Esc => self.skip_github(),
            _ => {}
        }
    }

    /// Autofix or Close: arrows or `h`/`l` choose, Enter confirms, `a` and `c` are shortcuts.
    fn autofix_key(&mut self, key: KeyEvent) {
        let Some(modal) = &mut self.autofix else {
            return;
        };
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::Char('h' | 'l') => {
                modal.selected = 1 - modal.selected;
            }
            KeyCode::Char('a') => self.do_autofix(),
            KeyCode::Enter if modal.selected == 0 => self.do_autofix(),
            KeyCode::Enter | KeyCode::Char('c' | 'q') | KeyCode::Esc => self.autofix = None,
            _ => {}
        }
    }

    /// Typing in the footer: Enter uses it, Esc drops it.
    fn prompt_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => self.prompt = None,
            KeyCode::Char('c') if ctrl => self.prompt = None,
            KeyCode::Enter => self.submit_prompt(),
            KeyCode::Backspace => {
                if let Some(p) = &mut self.prompt {
                    p.value.pop();
                }
            }
            KeyCode::Char(c) if !ctrl => {
                if let Some(p) = &mut self.prompt {
                    p.value.push(c);
                }
            }
            _ => {}
        }
    }
}
