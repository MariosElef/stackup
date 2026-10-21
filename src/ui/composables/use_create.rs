//! Making a project: the versions asked first, GitHub set up, and installing what is missing.
use std::path::{Path, PathBuf};

use crate::clients::run::capture;
use crate::{
    models::{Step, Tool},
    services::toolchain::{
        github::{Github, setup_steps},
        machine::install_plan,
        versions::{Picks, create_questions, questions},
    },
    ui::{
        stores::{
            app::{AfterChoices, App, GithubModal, InstallModal},
            run::Run,
        },
        types::Page,
    },
};

impl App {
    /// Create was pressed with tools missing: offer to install them, or to close.
    pub fn open_install_modal(&mut self, missing: Vec<&'static str>) {
        let plan = install_plan(
            &missing,
            Path::new("."),
            self.machine.as_ref(),
            &Picks::default(),
        );
        let selected = usize::from(plan.steps.is_empty());
        self.install = Some(InstallModal {
            missing,
            plan,
            selected,
        });
    }

    /// Create was confirmed in the wizard: make the project by running `stackup new` with these
    /// choices on the in-app terminal, in place of the wizard, so its output is shown here.
    pub fn start_create(&mut self) {
        let Some(wizard) = &self.wizard else { return };
        self.selection = wizard.sel.clone();
        // The repository goes to GitHub: if that is not set up, offer to do it first.
        if self.selection.tools.contains(&Tool::Git) && !self.github_skipped {
            let state = self.machine.github();
            if self.machine.has("gh") && !state.ready() {
                return self.open_github_modal(state);
            }
        }
        let sel = &self.selection;
        let asked = create_questions(sel.frontend, sel.backend, &sel.services);
        // Which version of each tool and service, where there is a choice.
        if asked.is_empty() {
            return self.run_create(&Picks::default());
        }
        self.open_choice(asked, Picks::default(), AfterChoices::Create);
    }

    pub(crate) fn open_github_modal(&mut self, github: Github) {
        let home = crate::clients::os::home_dir().unwrap_or_default();
        let email = capture(&home, "git", &["config", "user.email"])
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "stackup".into());
        let host = capture(&home, "hostname", &[])
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| "this computer".into());
        let steps = setup_steps(&github, &home, &email, &host);
        self.github = Some(GithubModal {
            github,
            steps,
            selected: 0,
        });
    }

    /// Skip: make the project without it, and do not ask again this session.
    pub fn skip_github(&mut self) {
        self.github = None;
        self.github_skipped = true;
        self.start_create();
    }

    /// Set up: run the steps on the in-app terminal, then back to the wizard.
    pub fn do_github_setup(&mut self) {
        let Some(modal) = self.github.take() else {
            return;
        };
        let mut run = Run::new("set up GitHub", modal.steps, self.pane.get());
        if let Err(e) = run.start_next() {
            if let Some(wizard) = &mut self.wizard {
                wizard.status = format!("{e:#}");
            }
            return;
        }
        self.run = Some(run);
        self.github_run = true;
        self.diagnosed = false;
        self.page = Page::Install;
    }

    /// Makes the project, with the versions that were picked.
    pub(crate) fn run_create(&mut self, picks: &Picks) {
        let Some(wizard) = &self.wizard else { return };
        let name = wizard.name.clone();
        let mut args = vec!["new".to_string(), name.clone()];
        args.extend(self.selection.cli_args());
        for pin in picks.pins() {
            args.push("--pin".to_string());
            args.push(pin);
        }
        let step = Step {
            label: format!("create {name}"),
            prog: self.exe.to_string_lossy().into_owned(),
            args,
            // Tells it it is inside the menu: it ends by saying how to get back.
            env: vec![("STACKUP_MENU".to_string(), "1".to_string())],
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };
        let mut run = Run::new(&format!("create {name}"), vec![step], self.pane.get());
        if let Err(e) = run.start_next() {
            if let Some(wizard) = &mut self.wizard {
                wizard.status = format!("{e:#}");
            }
            return;
        }
        self.run = Some(run);
        self.creating = Some(name);
        // A half-made project is not something to retry or fix from here.
        self.diagnosed = true;
        self.page = Page::Install;
    }

    /// Install was pressed: ask which versions, where there is a choice, then run it.
    pub fn do_install(&mut self) {
        let Some(modal) = &self.install else { return };
        let questions = questions(&modal.missing, self.machine.as_ref());
        if questions.is_empty() {
            return self.run_install();
        }
        self.open_choice(questions, Picks::default(), AfterChoices::Install);
    }

    /// Runs what the modal offered, on the in-app terminal, in place of the wizard.
    pub(crate) fn run_install(&mut self) {
        let Some(modal) = self.install.take() else {
            return;
        };
        let names: Vec<&str> = modal.missing.to_vec();
        let mut run = Run::new(
            &format!("install {}", names.join(", ")),
            modal.plan.steps,
            self.pane.get(),
        );
        if let Err(e) = run.start_next() {
            if let Some(wizard) = &mut self.wizard {
                wizard.status = format!("{e:#}");
            }
            return;
        }
        self.run = Some(run);
        self.diagnosed = false;
        self.page = Page::Install;
    }
}
