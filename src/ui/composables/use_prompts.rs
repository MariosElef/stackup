//! What is typed in the footer: a name, a folder to add, the name to forget a project.
use std::path::Path;

use crate::services::projects::volumes;
use crate::{
    models::{Param, slug},
    services::projects::registry,
    ui::{
        features::create::wizard::Wizard,
        stores::app::{App, Prompt, PromptKind, Then},
        types::Page,
    },
};

impl App {
    /// `a`: ask which folder to add projects from, starting at the one stackup was run in.
    pub fn ask_add(&mut self) {
        self.status.clear();
        let here =
            std::env::current_dir().map_or_else(|_| String::new(), |d| d.display().to_string());
        self.prompt = Some(Prompt {
            label: "Add projects from (a project, or a folder of them)".into(),
            value: here,
            kind: PromptKind::Add,
        });
    }

    pub(crate) fn submit_add(&mut self, typed: &str) {
        // `~` is the home folder, as in a shell.
        let path = crate::clients::os::expand_home(typed);
        let Ok(path) = crate::clients::os::canonical(&path) else {
            self.status = format!("{typed} is not a folder");
            return;
        };
        match registry::add_found(&self.dir, &path) {
            Ok(found) if found.added.is_empty() && found.known.is_empty() => {
                self.status = format!(
                    "no project in or under {}: a project has a docker-compose.yml",
                    path.display()
                );
            }
            Ok(found) => {
                let known = match found.known.len() {
                    0 => String::new(),
                    n => format!(" ({n} already listed)"),
                };
                self.status = if found.added.is_empty() {
                    format!("nothing new{known}")
                } else {
                    format!(
                        "added {}: {}{known}",
                        found.added.len(),
                        found.added.join(", ")
                    )
                };
                self.reload();
            }
            Err(e) => self.status = format!("{e:#}"),
        }
    }

    pub fn ask_name(&mut self) {
        self.status.clear();
        self.prompt = Some(Prompt {
            label: "Project name".into(),
            value: String::new(),
            kind: PromptKind::Name,
        });
    }

    /// Opens the wizard for a new project of this name.
    pub fn open_wizard(&mut self, name: &str) {
        let wizard = Wizard::new(name, self.selection.clone(), self.installed.clone());
        self.wizard = Some(wizard);
        self.page = Page::New;
    }

    pub(crate) fn submit_name(&mut self, name: &str) {
        let project = slug(name);
        if project.is_empty() {
            self.status = "a name needs letters or digits".into();
        } else if Path::new(name).exists() {
            self.status = format!("`{name}` already exists here");
        } else {
            self.open_wizard(name);
        }
    }

    pub fn ask_forget(&mut self) {
        let Some(project) = self.project() else {
            return;
        };
        let (name, path) = (project.name.clone(), project.path.clone());
        self.prompt = Some(Prompt {
            label: format!("stop listing {name}? its files stay: type `{name}` to continue"),
            value: String::new(),
            kind: PromptKind::Confirm {
                word: name,
                then: Then::Forget(path),
            },
        });
    }

    pub(crate) fn forget(&mut self, path: &Path) {
        match registry::forget(&self.dir, path) {
            Ok(()) => self.status = String::new(),
            Err(e) => self.status = format!("{e:#}"),
        }
        self.reload();
    }

    /// Enter on a prompt: use what was typed.
    pub fn submit_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        let value = prompt.value.trim().to_string();
        match prompt.kind {
            PromptKind::Name => self.submit_name(&value),
            PromptKind::Add => self.submit_add(&value),
            PromptKind::NewTier => self.submit_new_tier(&value),
            PromptKind::Param {
                action,
                args,
                param,
                left,
            } => {
                let mut args = *args;
                if value.is_empty() {
                    self.status = format!("{} needs a value", action.name());
                    return;
                }
                match param {
                    Param::Host => args.host = Some(value),
                    Param::Tier => args.tier = Some(value),
                    Param::VolumeName => {
                        if !volumes::valid_name(&value) && !volumes::valid_folder(&value) {
                            self.status = format!(
                                "`{value}` is not a volume name (lowercase letters, digits . _ -) or a ./folder"
                            );
                            return;
                        }
                        args.volume = Some(value);
                    }
                    Param::MountPath => {
                        if !volumes::valid_mount(&value) {
                            self.status = format!("`{value}` is not a path: write it like /data");
                            return;
                        }
                        args.mount = Some(value);
                    }
                    Param::Visibility
                    | Param::RemoveFrom
                    | Param::VolumeService
                    | Param::VolumePick
                    | Param::BackupFile => {}
                }
                self.ask_next(action, args, left);
            }
            PromptKind::Confirm { word, then } => {
                if value != word {
                    self.status = "not confirmed".into();
                    return;
                }
                match then {
                    Then::Run { action, args } => self.run_action(action, *args),
                    Then::Forget(path) => self.forget(&path),
                }
            }
        }
    }
}
