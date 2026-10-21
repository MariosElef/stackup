//! When a run fails with an error that has a known fix: offering it, and running it.
use std::path::Path;

use crate::{
    services::toolchain::{
        autofix::diagnose,
        versions::{Picks, Question},
    },
    ui::{
        stores::{
            app::{AfterChoices, App, AutofixModal},
            run::Run,
        },
        types::Page,
    },
};

impl App {
    /// Looks at what a failed run printed; if it is a failure with a known fix, asks.
    pub(crate) fn offer_autofix(&mut self) {
        let Some(run) = &self.run else { return };
        let root = match self.page {
            Page::Run => self.project().map(|p| p.path.clone()),
            _ => None,
        }
        .unwrap_or_else(|| Path::new(".").to_path_buf());
        let Some(fix) = diagnose(
            &run.log_text(),
            &root,
            std::env::consts::OS,
            self.machine.as_ref(),
        ) else {
            return;
        };
        if self.fixed.contains(&fix.id) {
            return;
        }
        self.autofix = Some(AutofixModal {
            id: fix.id,
            problem: fix.problem,
            fix: fix.steps,
            retry: run.steps().to_vec(),
            selected: 0,
            needs_rust: fix.needs_rust,
        });
    }

    /// Autofix was pressed. For an old Rust, which one to get is asked first.
    pub fn do_autofix(&mut self) {
        let needed = self.autofix.as_ref().and_then(|m| m.needs_rust);
        if let Some(needed) = needed
            && self.machine.has("rustup")
        {
            return self.open_choice(
                vec![Question::Rust { needed }],
                Picks::default(),
                AfterChoices::Autofix,
            );
        }
        self.run_autofix();
    }

    /// Runs the fix, then what failed, again, in the same place.
    pub(crate) fn run_autofix(&mut self) {
        let Some(modal) = self.autofix.take() else {
            return;
        };
        self.fixed.push(modal.id);
        let title = self
            .run
            .as_ref()
            .map_or("autofix".to_string(), |r| format!("autofix · {}", r.title));
        let steps: Vec<_> = modal.fix.into_iter().chain(modal.retry).collect();
        let mut run = Run::new(&title, steps, self.pane.get());
        if let Err(e) = run.start_next() {
            self.status = format!("{e:#}");
            return;
        }
        self.run = Some(run);
        self.diagnosed = false;
    }
}
