//! Moving between the app's pages: home, the list, a project, and back.

use std::path::PathBuf;

use crate::ui::features::create::use_requirements::TOOLS_TO_CHECK;
use crate::{
    models::{Action, Facts},
    services::projects::{
        envs,
        git::{self},
        urls::{self, Urls},
    },
    ui::{
        stores::app::{App, Outcome},
        types::Page,
    },
};

impl App {
    pub fn go_home(&mut self) {
        self.page = Page::Home;
        self.run = None;
        self.status.clear();
    }

    pub fn open_projects(&mut self) {
        self.reload();
        self.page = Page::Projects;
        self.status.clear();
        // What a project looks like may have changed while it was open.
        self.overview = None;
        self.usage.clear();
        self.load_overview();
    }

    /// Reads the quick look at the selected project, once: drawing uses what is kept.
    pub(crate) fn load_overview(&mut self) {
        let Some(path) = self.project().map(|p| p.path.clone()) else {
            self.overview = None;
            return;
        };
        if self.overview.as_ref().is_some_and(|(p, _)| *p == path) {
            return;
        }
        let overview = crate::services::projects::overview::read(&path);
        self.overview = Some((path.clone(), overview));
        self.measure(path);
    }

    /// Starts measuring a project's room, once: the answer comes back through `poll_usage`.
    fn measure(&mut self, path: PathBuf) {
        if self.usage.contains_key(&path) || !self.measuring.insert(path.clone()) {
            return;
        }
        let tx = self.usage_tx.clone();
        std::thread::spawn(move || {
            let usage = crate::services::projects::disk::usage(&path);
            let _ = tx.send((path, usage));
        });
    }

    /// Takes in the measurements that have finished. Called on every beat.
    pub(crate) fn poll_usage(&mut self) {
        let Some(rx) = &self.usage_rx else { return };
        while let Ok((path, usage)) = rx.try_recv() {
            self.measuring.remove(&path);
            self.usage.insert(path, usage);
        }
    }

    /// Reads the project's folder again, so what is offered is what is there now.
    pub fn open_project(&mut self) {
        if self.load_project() {
            self.page = Page::Project;
        }
    }

    /// Reads the selected project's folder: what it holds, and the actions that apply.
    pub(crate) fn load_project(&mut self) -> bool {
        let Some(project) = self.project() else {
            self.status = "no project here yet: press n to start one".into();
            return false;
        };
        let path = project.path.clone();
        let facts = Facts::read(&path);
        self.actions = Action::ALL
            .iter()
            .copied()
            .filter(|a| a.applies(&facts))
            .collect();
        self.status = if facts.exists {
            String::new()
        } else {
            format!(
                "{} is missing: press h, then d to forget it",
                path.display()
            )
        };
        self.urls = if facts.exists {
            urls::read(&path)
        } else {
            Urls::default()
        };
        self.env = envs::current(&path);
        self.git = git::read(&path);
        self.facts = Some(facts);
        self.action = 0;
        true
    }

    /// Reads again what changes while a project is used: where it is reachable (it may have just
    /// been started or stopped) and which environment is active.
    pub fn refresh_project_state(&mut self) {
        if let Some(path) = self.project().map(|p| p.path.clone()) {
            self.urls = urls::read(&path);
            self.env = envs::current(&path);
            self.git = git::read(&path);
        }
    }

    /// `o`: open the selected project's app in the browser.
    pub fn open_app(&mut self) {
        if self.page != Page::Project && !self.load_project() {
            return;
        }
        match self.urls.primary().map(|l| l.url.clone()) {
            None => self.status = "nothing to open: this project publishes no web port".into(),
            Some(url) => match (self.opener)(&url) {
                Ok(()) => self.status = format!("opened {url}"),
                Err(e) => self.status = format!("{e:#}"),
            },
        }
    }

    /// One step back: out of the output, then out of the project, then the list.
    pub fn back(&mut self) {
        self.status.clear();
        self.autofix = None;
        self.fixed.clear();
        match self.page {
            Page::Run => {
                self.run = None;
                if std::mem::take(&mut self.removed) {
                    self.open_projects();
                } else {
                    // It may have started or stopped something: the addresses show which.
                    self.refresh_project_state();
                    self.page = Page::Project;
                }
            }
            Page::Project => self.open_projects(),
            Page::Projects => self.go_home(),
            Page::New => {
                self.wizard = None;
                if self.wizard_only {
                    self.outcome = Outcome::Quit;
                } else {
                    self.go_home();
                }
            }
            Page::Install if self.creating.is_some() => self.finish_create(),
            Page::Install => {
                let done = self.run.as_ref().is_some_and(|r| r.succeeded);
                self.run = None;
                // What is installed now decides what the wizard's footer shows.
                self.installed = Self::installed_tools();
                if let Some(wizard) = &mut self.wizard {
                    wizard.installed = self.installed.clone();
                    wizard.status = if std::mem::take(&mut self.github_run) {
                        if done {
                            "GitHub is set up: press Create project again".into()
                        } else {
                            "GitHub setup did not finish: nothing more was changed".into()
                        }
                    } else if done {
                        "installed: press Create project again".into()
                    } else {
                        "the install did not finish: nothing was changed".into()
                    };
                }
                self.page = Page::New;
            }
            Page::Home => self.outcome = Outcome::Quit,
        }
    }

    /// Leaving the output of a creation: a made project is on the list now, selected; one that
    /// did not finish sends you back to the wizard with your choices as they were.
    pub(crate) fn finish_create(&mut self) {
        let name = self.creating.take().unwrap_or_default();
        let done = self.run.as_ref().is_some_and(|r| r.succeeded);
        self.run = None;
        if !done {
            self.page = Page::New;
            if let Some(wizard) = &mut self.wizard {
                wizard.status =
                    format!("{name} was not made: a half-made folder may be left behind");
            }
            return;
        }
        self.wizard = None;
        self.open_projects();
        let made = std::env::current_dir()
            .ok()
            .and_then(|cwd| crate::clients::os::canonical(&cwd.join(&name)).ok());
        if let Some(i) = made.and_then(|p| self.projects.iter().position(|x| x.path == p)) {
            self.cursor = i;
        }
        self.status = format!("made {name}: Enter opens it");
    }

    pub fn installed_tools() -> Vec<(&'static str, bool)> {
        TOOLS_TO_CHECK
            .iter()
            .map(|b| (*b, crate::clients::run::have(b)))
            .collect()
    }
}
