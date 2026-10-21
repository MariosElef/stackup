//! Environments: the picker, switching, and making a new one.

use crate::{
    models::{Action, Args},
    services::{
        projects::envs,
        toolchain::versions::{Choice, Picks, Question},
    },
    ui::stores::app::{AfterChoices, App, ChoiceModal, Prompt, PromptKind, TierModal},
};

impl App {
    /// The environments the open project has, the active one marked, and a way to make another.
    pub fn open_tier_picker(&mut self) {
        let Some(path) = self.project().map(|p| p.path.clone()) else {
            return;
        };
        let active = envs::current(&path);
        let mut options: Vec<Choice> = envs::tiers(&path)
            .into_iter()
            .map(|t| Choice {
                note: {
                    let n = envs::variable_count(&path, &t);
                    format!(".env.{t} · {n} variable{}", if n == 1 { "" } else { "s" })
                },
                label: t.clone(),
                value: t,
            })
            .collect();
        let selected = options
            .iter()
            .position(|o| active.as_deref() == Some(o.value.as_str()))
            .unwrap_or(0);
        options.push(Choice {
            value: "+new".into(),
            label: "a new environment".into(),
            note: "made from .env.example, with new random passwords".into(),
        });
        self.choice = Some(ChoiceModal {
            question: Question::Tier,
            options,
            selected,
            next: vec![],
            picks: Picks::default(),
            then: AfterChoices::Tier,
            active,
            pending: None,
        });
    }

    /// A tier was picked: switch to it, or ask the name of a new one.
    pub(crate) fn choose_tier(&mut self, picked: &str) {
        if picked == "+new" {
            return self.ask_new_tier();
        }
        let args = Args {
            tier: Some(picked.to_string()),
            ..Args::default()
        };
        self.run_action(Action::Env, args);
    }

    pub fn ask_new_tier(&mut self) {
        self.status.clear();
        self.prompt = Some(Prompt {
            label: "Name of the new environment (staging, prod...)".into(),
            value: String::new(),
            kind: PromptKind::NewTier,
        });
    }

    /// Makes `.env.<name>` from the template and says what it did.
    pub(crate) fn submit_new_tier(&mut self, name: &str) {
        let Some(path) = self.project().map(|p| p.path.clone()) else {
            return;
        };
        match envs::create(&path, name) {
            Ok(created) => {
                self.refresh_project_state();
                self.tier_modal = Some(TierModal {
                    tier: name.to_string(),
                    created,
                    selected: 0,
                });
            }
            Err(e) => self.status = format!("{e:#}"),
        }
    }

    /// Switch to the new environment, or leave it for later.
    pub fn answer_tier_modal(&mut self, switch: bool) {
        let Some(modal) = self.tier_modal.take() else {
            return;
        };
        if switch {
            self.choose_tier(&modal.tier);
        }
    }
}
