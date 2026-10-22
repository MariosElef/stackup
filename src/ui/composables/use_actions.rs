//! Running an action on a project: asking for what it needs, confirming, and running it on the in-app terminal.

use crate::clients::run::capture;
use crate::{
    models::{Action, Args, Param},
    services::projects::{
        git::Host,
        ide::{Part, parts},
        plan::{DATABASES_ONLY, plan},
        registry,
    },
    ui::{
        components::celebration::Celebration,
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

    /// `Enter` on an action: ask for what it needs, then run it. `dev` asks in its own panel
    /// which part to start, when there is more than one.
    pub fn choose_action(&mut self) {
        let Some(&action) = self.actions.get(self.action) else {
            return;
        };
        if action == Action::Dev && self.dev_parts().len() > 1 {
            self.dev_part = Some(0);
            return;
        }
        self.begin_action(action);
    }

    /// What `dev` can start, by the name it is given: both together first, and only the
    /// databases last when the project has some.
    pub fn dev_parts(&self) -> Vec<&'static str> {
        let mut out: Vec<&str> = self
            .facts
            .as_ref()
            .map(parts)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .map(Part::cli)
            .collect();
        if !self.urls.data.is_empty() {
            out.push(DATABASES_ONLY);
        }
        out
    }

    /// Enter in `dev`'s panel: start the part picked.
    pub(crate) fn start_dev_part(&mut self) {
        let Some(part) = self
            .dev_part
            .take()
            .and_then(|i| self.dev_parts().get(i).copied())
        else {
            return;
        };
        let args = Args {
            part: Some(part.into()),
            ..Args::default()
        };
        self.run_action(Action::Dev, args);
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
            Param::Host | Param::VolumeName | Param::MountPath | Param::MigrationName => {}
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
            Param::MigrationName => (
                "What the migration does (add_users, letters digits and _)",
                String::new(),
            ),
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
        } else if action == Action::MigrationRevert {
            format!(
                "REVERT the last migration of {name} (can drop tables and their data): type `{name}` to continue"
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
        self.dev_part = None;
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

    /// The celebration is over, or was skipped: leave the finished run as `q` would.
    pub fn end_celebration(&mut self) {
        if self.celebration.take().is_some() {
            self.back();
        }
    }

    /// A run on the install page finished well: say so, then leave it on its own.
    fn celebrate(&mut self) {
        let Some(run) = &self.run else { return };
        let (title, next) = if let Some(name) = &self.creating {
            (format!("{name} created"), "opening your projects")
        } else if self.github_run {
            ("GitHub is set up".to_string(), "back to the wizard")
        } else {
            let what = run.title.strip_prefix("install ").unwrap_or(&run.title);
            (format!("{what} installed"), "back to the wizard")
        };
        self.celebration = Some(Celebration::new(&title, next));
    }

    pub fn pump(&mut self) {
        if self.farewell.as_ref().is_some_and(Farewell::done) {
            return self.end_farewell();
        }
        if self.celebration.as_ref().is_some_and(Celebration::done) {
            return self.end_celebration();
        }
        let finished_install = self.page == Page::Install
            && self.celebration.is_none()
            && self.run.as_ref().is_some_and(|r| r.succeeded);
        if finished_install {
            self.celebrate();
        }
        self.tick = self.tick.wrapping_add(1);
        self.poll_usage();
        if matches!(self.page, Page::Project | Page::Run)
            && self.urls_probed.elapsed() >= std::time::Duration::from_secs(1)
        {
            self.urls_probed = std::time::Instant::now();
            crate::services::projects::urls::probe(&mut self.urls);
        }
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
