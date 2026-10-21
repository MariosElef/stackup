//! Running an action on a project: asking for what it needs, confirming, and running it on the in-app terminal.

use crate::clients::run::capture;
use crate::{
    models::{Action, Args, Param},
    services::projects::{git::Host, plan::plan, registry},
    ui::{
        features::projects::farewell::Farewell,
        stores::{
            app::{App, Prompt, PromptKind, Then},
            run::Run,
        },
        types::Page,
    },
};

impl App {
    /// `r` on a project: delete it, once its name is typed back. A project whose folder is
    /// already gone only has its entry to remove.
    pub fn begin_remove(&mut self) {
        if !self.load_project() {
            return;
        }
        if self.facts.as_ref().is_some_and(|f| !f.exists) {
            self.status = "its folder is already gone: this stops listing it".into();
            return self.ask_forget();
        }
        self.begin_action(Action::Remove);
    }

    /// `Enter` on an action: ask for what it needs, then run it.
    pub fn choose_action(&mut self) {
        if let Some(&action) = self.actions.get(self.action) {
            self.begin_action(action);
        }
    }

    /// git: the real lazygit, run on this same in-app terminal, so nothing of it is rebuilt here.
    /// When it quits the project comes back with its branch and changes read again.
    pub(crate) fn open_lazygit(&mut self) {
        if !self.machine.has("lazygit") {
            self.status = format!(
                "lazygit is not installed: {}",
                crate::clients::run::install_hint("lazygit")
            );
            return;
        }
        self.run_action(Action::GitOpen, Args::default());
    }

    /// Starts an action: asks for what it needs, then runs it.
    pub fn begin_action(&mut self, action: Action) {
        if action == Action::GitOpen {
            return self.open_lazygit();
        }
        let params = action.params().to_vec();
        self.ask_next(action, Args::default(), params);
    }

    pub(crate) fn ask_next(&mut self, action: Action, args: Args, mut left: Vec<Param>) {
        if left.is_empty() {
            return self.confirm_or_run(action, args);
        }
        let param = left.remove(0);
        // Which environment is picked from the ones the project has (or made new), not typed.
        match param {
            Param::Tier => return self.open_tier_picker(),
            Param::Visibility => return self.open_visibility_picker(action, args, left),
            Param::RemoveFrom => return self.open_remove_picker(action, args, left),
            Param::VolumeService => return self.open_volume_service_picker(action, args, left),
            Param::VolumePick => return self.open_volume_picker(action, args, left),
            Param::BackupFile => return self.open_backup_picker(action, args, left),
            Param::Host | Param::VolumeName | Param::MountPath => {}
        }
        let project = self.project();
        let (label, value) = match param {
            Param::Host => (
                "Server (user@host)",
                project.and_then(|p| p.host.clone()).unwrap_or_default(),
            ),
            Param::VolumeName => (
                "Volume name (a ./folder shares a folder of the project instead)",
                String::new(),
            ),
            Param::MountPath => ("Path inside the container", "/data".into()),
            // Asked from a list, above.
            Param::Tier
            | Param::Visibility
            | Param::RemoveFrom
            | Param::VolumeService
            | Param::VolumePick
            | Param::BackupFile => {
                unreachable!("picked, not typed")
            }
        };
        self.prompt = Some(Prompt {
            label: label.into(),
            value,
            kind: PromptKind::Param {
                action,
                args: Box::new(args),
                param,
                left,
            },
        });
    }

    pub(crate) fn confirm_or_run(&mut self, action: Action, args: Args) {
        let Some(project) = self.project() else {
            return;
        };
        if !action.risky() {
            return self.run_action(action, args);
        }
        let name = project.name.clone();
        let label = if action == Action::Remove {
            // A folder cannot be got back: say what goes, and if there is unsaved work in it. The
            // path itself is in the About panel; the prompt must fit on one line.
            let dirty = capture(&project.path, "git", &["status", "--porcelain"])
                .is_some_and(|out| !out.is_empty());
            let warning = if dirty { ", uncommitted changes!" } else { "" };
            let remote = args
                .remove_remote
                .as_deref()
                .and_then(Host::parse)
                .map_or(String::new(), |h| {
                    format!(" AND its {} repository", h.label())
                });
            format!(
                "REMOVE {name} (folder, containers, data{remote}{warning}): type `{name}` to continue"
            )
        } else if action == Action::VolumeRestore {
            let (volume, backup) = (
                args.volume.clone().unwrap_or_default(),
                args.backup.clone().unwrap_or_default(),
            );
            format!(
                "RESTORE {volume} from {backup} (replaces its data, stops what uses it): type `{name}` to continue"
            )
        } else {
            let host = args
                .host
                .clone()
                .or_else(|| project.host.clone())
                .unwrap_or_default();
            let tier = args.tier.clone().unwrap_or_else(|| "prod".into());
            format!(
                "{} {name} to {host} [{tier}]: type `{name}` to continue",
                action.name()
            )
        };
        self.prompt = Some(Prompt {
            label,
            value: String::new(),
            kind: PromptKind::Confirm {
                word: name,
                then: Then::Run {
                    action,
                    args: Box::new(args),
                },
            },
        });
    }

    /// Works out the steps and starts them on the in-app terminal.
    pub fn run_action(&mut self, action: Action, args: Args) {
        let (Some(project), Some(facts)) = (self.project().cloned(), self.facts.clone()) else {
            return;
        };
        let steps = match plan(action, &project, &facts, &args) {
            Ok(steps) => steps,
            Err(e) => {
                self.status = format!("{e:#}");
                return;
            }
        };
        let mut run = Run::new(
            &format!("{} · {}", action.name(), project.name),
            steps,
            self.pane.get(),
        );
        if let Err(e) = run.start_next() {
            self.status = format!("{e:#}");
            return;
        }
        self.removing = (action == Action::Remove).then(|| facts.clone());
        self.after = Some((action, args));
        self.run = Some(run);
        self.diagnosed = false;
        self.page = Page::Run;
        self.status.clear();
    }

    /// Called on every beat of the event loop: the logo's band, and the running action.
    /// The goodbye is over: back to the list.
    pub fn end_farewell(&mut self) {
        if self.farewell.take().is_some() {
            self.back();
        }
    }

    pub fn pump(&mut self) {
        if self.farewell.as_ref().is_some_and(Farewell::done) {
            return self.end_farewell();
        }
        self.tick = self.tick.wrapping_add(1);
        self.poll_usage();
        let size = self.pane.get();
        let Some(run) = &mut self.run else { return };
        run.pump(size);
        let failed = run
            .ended
            .as_ref()
            .is_some_and(|how| how.starts_with("failed"));
        if failed && !self.diagnosed {
            self.diagnosed = true;
            self.offer_autofix();
            return;
        }
        let Some(run) = &self.run else { return };
        if !run.succeeded {
            return;
        }
        match self.after.take() {
            Some((Action::Deploy, args)) => {
                if let (Some(host), Some(project)) = (args.host, self.projects.get(self.cursor)) {
                    // Where a deploy went is remembered, so the menu offers it next time.
                    let _ = registry::set_host(&self.dir, &project.path, &host);
                    self.reload();
                }
            }
            Some((Action::Remove, args)) => {
                // The folder is gone: it leaves the list too, and a goodbye shows before the list.
                let name = self.project().map(|p| p.name.clone());
                if let Some(path) = self.project().map(|p| p.path.clone()) {
                    let _ = registry::forget(&self.dir, &path);
                }
                self.removed = true;
                self.reload();
                let mut detail = String::from("the folder, the containers and their data");
                if let Some(host) = args.remove_remote.as_deref().and_then(Host::parse) {
                    detail.push_str(&format!(" and the {} repository", host.label()));
                }
                let facts = self.removing.take();
                self.farewell = Some(Farewell::new(
                    &name.unwrap_or_default(),
                    &detail,
                    facts.as_ref(),
                ));
            }
            // lazygit was quit: straight back to the project, with git read again.
            Some((Action::GitOpen, _)) => self.back(),
            _ => {}
        }
    }
}
