//! The lists a person picks from (a version, private or public, where to remove from) and what happens with the answer.
use std::path::Path;

use crate::services::projects::volumes;
use crate::{
    models::{Action, Args, Param},
    services::{
        projects::git::{self, Host},
        toolchain::{
            autofix::with_toolchain,
            machine::{install_plan, rust_steps},
            versions::{Choice, Picks, Question, options as version_options},
        },
    },
    ui::stores::app::{AfterChoices, App, ChoiceModal},
};

impl App {
    /// The next version question, with its options.
    pub(crate) fn open_choice(
        &mut self,
        mut questions: Vec<Question>,
        picks: Picks,
        then: AfterChoices,
    ) {
        let question = questions.remove(0);
        let options = version_options(question, self.machine.as_ref());
        self.choice = Some(ChoiceModal {
            question,
            options,
            selected: 0,
            next: questions,
            picks,
            then,
            active: None,
            pending: None,
        });
    }

    /// Enter on a version: remember it, ask the next question, or go on with what was asked for.
    pub fn answer_choice(&mut self) {
        let Some(mut choice) = self.choice.take() else {
            return;
        };
        let Some(picked) = choice.options.get(choice.selected).map(|c| c.value.clone()) else {
            return;
        };
        choice.picks.set(choice.question, &picked);
        if !choice.next.is_empty() {
            return self.open_choice(choice.next, choice.picks, choice.then);
        }
        match choice.then {
            AfterChoices::Install => {
                // The plan shown before is the default one; this is the one asked for.
                let missing = self.install.as_ref().map(|m| m.missing.clone());
                if let Some(missing) = missing {
                    let plan = install_plan(
                        &missing,
                        Path::new("."),
                        self.machine.as_ref(),
                        &choice.picks,
                    );
                    if let Some(modal) = &mut self.install {
                        modal.plan = plan;
                    }
                }
                self.run_install();
            }
            AfterChoices::Create => self.run_create(&choice.picks),
            AfterChoices::Autofix => self.use_rust(&picked),
            AfterChoices::Tier => self.choose_tier(&picked),
            AfterChoices::Param => {
                if let Some(pending) = choice.pending {
                    self.answer_param(choice.question, &picked, pending);
                }
            }
        }
    }

    /// The Rust that was chosen for an autofix: get it, and run what failed with it.
    pub(crate) fn use_rust(&mut self, toolchain: &str) {
        let machine = self.machine.as_ref();
        if let Some(modal) = &mut self.autofix {
            let dir = modal
                .fix
                .first()
                .map_or_else(|| Path::new(".").to_path_buf(), |s| s.cwd.clone());
            modal.fix = rust_steps(machine, &dir, toolchain, "get Rust");
            modal.retry = modal
                .retry
                .iter()
                .map(|s| with_toolchain(s, toolchain))
                .collect();
        }
        self.run_autofix();
    }

    /// Private or public, before a GitHub repository is made.
    pub(crate) fn open_visibility_picker(&mut self, action: Action, args: Args, left: Vec<Param>) {
        let options = version_options(Question::Visibility, self.machine.as_ref());
        self.choice = Some(ChoiceModal {
            question: Question::Visibility,
            options,
            selected: 0,
            next: vec![],
            picks: Picks::default(),
            then: AfterChoices::Param,
            active: None,
            pending: Some((action, args, left)),
        });
    }

    /// Before removing: only this computer, or its repository on GitHub or GitLab too. Asked only
    /// when there is one to delete; otherwise there is nothing to choose.
    pub(crate) fn open_remove_picker(&mut self, action: Action, args: Args, left: Vec<Param>) {
        let Some((host, slug)) = self.deletable_remote() else {
            return self.ask_next(action, args, left);
        };
        let options = vec![
            Choice {
                value: "local".into(),
                label: "this computer only".into(),
                note: format!("the {} repository {slug} stays", host.label()),
            },
            Choice {
                value: host.key().into(),
                label: format!("this computer and {} ({slug})", host.label()),
                note: format!("also deletes the repository on {}: no undo", host.label()),
            },
        ];
        self.choice = Some(ChoiceModal {
            question: Question::RemoveFrom,
            options,
            selected: 0,
            next: vec![],
            picks: Picks::default(),
            then: AfterChoices::Param,
            active: None,
            pending: Some((action, args, left)),
        });
    }

    /// Where the open project's repository is, if it is somewhere stackup can delete it and the
    /// program that does it is here.
    pub(crate) fn deletable_remote(&self) -> Option<(Host, String)> {
        let path = self.project()?.path.clone();
        let (host, slug) = git::read(&path)?.remote.as_deref().and_then(git::hosted)?;
        self.machine.has(host.tool()).then_some((host, slug))
    }

    /// A value was picked for an action that asks for several: keep it, and ask the rest.
    pub(crate) fn answer_param(
        &mut self,
        question: Question,
        picked: &str,
        pending: (Action, Args, Vec<Param>),
    ) {
        let (action, mut args, left) = pending;
        match question {
            Question::Visibility => args.visibility = Some(picked.to_string()),
            // "local" is the answer that deletes nothing elsewhere.
            Question::RemoveFrom if picked != "local" => args.remove_remote = Some(picked.into()),
            Question::VolumeService => args.service = Some(picked.to_string()),
            Question::Volume => args.volume = Some(picked.to_string()),
            Question::Backup => args.backup = Some(picked.to_string()),
            _ => {}
        }
        self.ask_next(action, args, left);
    }

    /// The compose file's text for the open project.
    fn compose_text(&self) -> Option<String> {
        let path = self.project()?.path.join("docker-compose.yml");
        std::fs::read_to_string(path).ok()
    }

    /// A list to pick from, with the answer going back to the action that asked.
    fn open_param_picker(
        &mut self,
        question: Question,
        options: Vec<Choice>,
        pending: (Action, Args, Vec<Param>),
    ) {
        if options.is_empty() {
            self.status = format!("nothing to pick: {}", question.title());
            return;
        }
        self.choice = Some(ChoiceModal {
            question,
            options,
            selected: 0,
            next: vec![],
            picks: Picks::default(),
            then: AfterChoices::Param,
            active: None,
            pending: Some(pending),
        });
    }

    /// Which service the new volume goes in: every service of the compose file.
    pub(crate) fn open_volume_service_picker(
        &mut self,
        action: Action,
        args: Args,
        left: Vec<Param>,
    ) {
        let text = self.compose_text().unwrap_or_default();
        let options = volumes::services(&text)
            .into_iter()
            .map(|s| {
                let held: Vec<String> = volumes::mounts(&text)
                    .into_iter()
                    .filter(|m| m.service == s)
                    .map(|m| m.target)
                    .collect();
                Choice {
                    note: if held.is_empty() {
                        "stores nothing yet".into()
                    } else {
                        format!("already keeps {}", held.join(", "))
                    },
                    label: s.clone(),
                    value: s,
                }
            })
            .collect();
        self.open_param_picker(Question::VolumeService, options, (action, args, left));
    }

    /// Which of the declared volumes, with where each is used.
    pub(crate) fn open_volume_picker(&mut self, action: Action, args: Args, left: Vec<Param>) {
        let text = self.compose_text().unwrap_or_default();
        let options = volumes::named(&text)
            .into_iter()
            .map(|v| {
                let used: Vec<String> = volumes::mounts(&text)
                    .into_iter()
                    .filter(|m| m.source == v)
                    .map(|m| format!("{} {}", m.service, m.target))
                    .collect();
                Choice {
                    note: if used.is_empty() {
                        "not mounted by any service".into()
                    } else {
                        used.join(", ")
                    },
                    label: v.clone(),
                    value: v,
                }
            })
            .collect();
        self.open_param_picker(Question::Volume, options, (action, args, left));
    }

    /// Which saved backup of the chosen volume, the newest first.
    pub(crate) fn open_backup_picker(&mut self, action: Action, args: Args, left: Vec<Param>) {
        let Some(root) = self.project().map(|p| p.path.clone()) else {
            return;
        };
        let volume = args.volume.clone().unwrap_or_default();
        let options = volumes::backups(&root)
            .into_iter()
            .filter(|f| f.starts_with(&format!("{volume}-")))
            .map(|f| Choice {
                note: "in backups/".into(),
                label: f.clone(),
                value: f,
            })
            .collect();
        self.open_param_picker(Question::Backup, options, (action, args, left));
    }
}
