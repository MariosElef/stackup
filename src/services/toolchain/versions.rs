//! Which versions of a tool can be installed: what is asked, and the options for each question.
//! The first option is the one that is recommended.
use super::machine::{Machine, TOPCOAT_CLI_REV, rust_version};
use crate::models::{Backend, Frontend, Service};

/// What is asked before something is installed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Question {
    /// Which Rust, when the one in use is older than `needed` (major, minor).
    Rust {
        needed: (u32, u32),
    },
    /// Which Rust the new project is pinned to (its rust-toolchain.toml).
    RustPin {
        needed: (u32, u32),
    },
    /// Which Topcoat CLI.
    Topcoat,
    /// Which version of a service the new project runs in Docker.
    Postgres,
    Redis,
    Mongodb,
    Keycloak,
    /// Which pnpm.
    Pnpm,
    /// Which environment to make active, or whether to make a new one.
    Tier,
    /// Whether a new GitHub repository is private or public.
    Visibility,
    /// Whether removing a project also deletes its repository on GitHub or GitLab.
    RemoveFrom,
    /// Which service gets a new volume.
    VolumeService,
    /// Which of the project's volumes.
    Volume,
    /// Which saved backup.
    Backup,
}

impl Question {
    pub fn title(self) -> &'static str {
        match self {
            Question::Rust { .. } => "Which Rust do you want?",
            Question::RustPin { .. } => "Which Rust for the project?",
            Question::Topcoat => "Which Topcoat CLI do you want?",
            Question::Postgres => "Which Postgres?",
            Question::Redis => "Which Redis?",
            Question::Mongodb => "Which MongoDB?",
            Question::Keycloak => "Which Keycloak?",
            Question::Pnpm => "Which pnpm do you want?",
            Question::Tier => "Which environment?",
            Question::Visibility => "Who can see the new repository?",
            Question::RemoveFrom => "Remove it from where?",
            Question::VolumeService => "Which service stores it?",
            Question::Volume => "Which volume?",
            Question::Backup => "Which backup?",
        }
    }

    /// One line about why it is asked.
    pub fn why(self) -> String {
        match self {
            Question::Rust { needed } => format!(
                "The Topcoat CLI needs Rust {}.{} or newer, and the one in use is older.",
                needed.0, needed.1
            ),
            Question::RustPin { needed } => format!(
                "Written to rust-toolchain.toml, so everyone builds with it. {}",
                if needed >= super::machine::TOPCOAT_RUST {
                    format!("Topcoat needs {}.{} or newer.", needed.0, needed.1)
                } else {
                    format!("The project needs {}.{} or newer.", needed.0, needed.1)
                }
            ),
            Question::Postgres => "The image in docker-compose.yml. Data made by one major version needs a migration to open in another.".into(),
            Question::Redis => "The image in docker-compose.yml.".into(),
            Question::Mongodb => "The image in docker-compose.yml. Data made by one major version needs a migration to open in another.".into(),
            Question::Keycloak => "The image in docker-compose.yml; the realm is imported again on every start.".into(),
            Question::Topcoat => "Topcoat is new and changes often, so the version matters.".into(),
            Question::Pnpm => "Nuxt and Vue projects are installed with pnpm.".into(),
            Question::Tier => {
                "`.env` becomes a copy of the one you pick; the active one is marked.".into()
            }
            Question::VolumeService => {
                "The service whose container gets the storage: what it writes at the path you give next is kept.".into()
            }
            Question::Volume => "The named volumes the compose file declares.".into(),
            Question::Backup => "From the project's backups/ folder, the newest first. Restoring replaces what is in the volume.".into(),
            Question::RemoveFrom => {
                "Deleting the repository on GitHub or GitLab cannot be undone: its history and issues go too."
                    .into()
            }
            Question::Visibility => {
                "It is made on your GitHub account, and the project is pushed to it.".into()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// What is passed on: a toolchain name, a revision, a major version.
    pub value: String,
    pub label: String,
    pub note: String,
}

fn choice(value: &str, label: &str, note: &str) -> Choice {
    Choice {
        value: value.to_string(),
        label: label.to_string(),
        note: note.to_string(),
    }
}

/// What the person has picked so far; anything not picked takes the recommended option.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picks {
    pub rust: Option<String>,
    pub topcoat: Option<String>,
    pub pnpm: Option<String>,
    /// The services' versions: (what is pinned, the version).
    pub services: Vec<(&'static str, String)>,
}

impl Picks {
    fn service(&mut self, key: &'static str, value: Option<String>) {
        self.services.retain(|(k, _)| *k != key);
        self.services.extend(value.map(|v| (key, v)));
    }

    /// What `stackup new --pin` is given for these picks.
    pub fn pins(&self) -> Vec<String> {
        let mut out = vec![];
        if let Some(rust) = &self.rust {
            out.push(format!("rust={}", crate::models::rust_channel(rust)));
        }
        if let Some(topcoat) = &self.topcoat {
            out.push(format!("topcoat={topcoat}"));
        }
        out.extend(self.services.iter().map(|(k, v)| format!("{k}={v}")));
        out
    }

    pub fn set(&mut self, question: Question, value: &str) {
        let value = Some(value.to_string());
        match question {
            Question::Rust { .. } | Question::RustPin { .. } => self.rust = value,
            Question::Postgres => self.service("postgres", value),
            Question::Redis => self.service("redis", value),
            Question::Mongodb => self.service("mongodb", value),
            Question::Keycloak => self.service("keycloak", value),
            Question::Topcoat => self.topcoat = value,
            Question::Pnpm => self.pnpm = value,
            // The environments are the project's own files: they are not a version.
            Question::Tier
            | Question::Visibility
            | Question::RemoveFrom
            | Question::VolumeService
            | Question::Volume
            | Question::Backup => {}
        }
    }
}

/// The questions to ask before installing `missing`, in order. Only what has a real choice.
pub fn questions(missing: &[&'static str], m: &dyn Machine) -> Vec<Question> {
    let mut asked = vec![];
    for &bin in missing {
        match bin {
            "topcoat" if m.has("cargo") => {
                let needed = super::machine::TOPCOAT_RUST;
                // Only when the Rust in use is too old is there anything to choose about Rust.
                if m.has("rustup") && !m.active_rust().is_some_and(|v| v >= needed) {
                    asked.push(Question::Rust { needed });
                }
                asked.push(Question::Topcoat);
            }
            "pnpm" if m.has("npm") => asked.push(Question::Pnpm),
            _ => {}
        }
    }
    asked
}

/// What is asked when a project is created: the versions of what the stack uses.
pub fn create_questions(
    frontend: Frontend,
    backend: Backend,
    services: &[Service],
) -> Vec<Question> {
    let mut asked = vec![];
    if frontend == Frontend::Topcoat {
        asked.push(Question::RustPin {
            needed: super::machine::TOPCOAT_RUST,
        });
    } else if backend == Backend::Axum {
        asked.push(Question::RustPin { needed: (1, 85) });
    }
    if frontend == Frontend::Topcoat {
        asked.push(Question::Topcoat);
    }
    for (service, question) in [
        (Service::Postgres, Question::Postgres),
        (Service::Redis, Question::Redis),
        (Service::Mongodb, Question::Mongodb),
        (Service::Keycloak, Question::Keycloak),
    ] {
        if services.contains(&service) {
            asked.push(question);
        }
    }
    asked
}

/// The options for `question`, the recommended one first.
pub fn options(question: Question, m: &dyn Machine) -> Vec<Choice> {
    match question {
        Question::Rust { needed } | Question::RustPin { needed } => {
            let mut out: Vec<Choice> = vec![];
            // One already installed is the quickest: nothing to download.
            for t in m.toolchains() {
                if rust_version(&t).is_some_and(|v| v >= needed) {
                    out.push(choice(&t, &t, "already installed: nothing to download"));
                }
            }
            out.push(choice(
                "stable",
                "latest stable",
                "updates Rust with `rustup update stable`",
            ));
            let pin = format!("{}.{}.0", needed.0, needed.1);
            if !out.iter().any(|c| c.value.starts_with(&pin)) {
                out.push(choice(
                    &pin,
                    &pin,
                    "the oldest Rust the Topcoat CLI works with",
                ));
            }
            out.push(choice(
                "beta",
                "beta",
                "the next stable, for trying what is coming",
            ));
            out.push(choice("nightly", "nightly", "the newest and least stable"));
            out
        }
        Question::Topcoat => vec![
            choice(
                TOPCOAT_CLI_REV,
                &format!("{TOPCOAT_CLI_REV} (tested with stackup)"),
                "the revision stackup's projects and Dockerfiles are made with",
            ),
            choice(
                "main",
                "latest (main)",
                "the newest Topcoat: it may be ahead of what stackup was tested with",
            ),
        ],
        // Built from the project's files by the menu, which knows where the project is.
        // Built by the menu, which knows where the project's repository is.
        Question::Tier
        | Question::RemoveFrom
        | Question::VolumeService
        | Question::Volume
        | Question::Backup => vec![],
        Question::Visibility => vec![
            choice(
                "private",
                "private",
                "only you, and people you invite, can see the code",
            ),
            choice(
                "public",
                "public",
                "anyone on the internet can see the code",
            ),
        ],
        Question::Postgres => vec![
            choice("latest", "latest (18)", "the newest major version"),
            choice("17", "17", "the previous major version, widely used"),
            choice("16", "16", "still supported"),
            choice("15", "15", "still supported"),
            choice("14", "14", "older, supported until late 2026"),
        ],
        Question::Redis => vec![
            choice("latest", "latest (8)", "the newest major version"),
            choice("7", "7", "the previous major version, widely used"),
            choice("6", "6", "older"),
        ],
        Question::Mongodb => vec![
            choice("8", "8", "the current major version"),
            choice("7", "7", "the previous major version"),
            choice("6", "6", "older"),
            choice("latest", "latest", "whatever MongoDB publishes newest"),
        ],
        Question::Keycloak => vec![
            choice("latest", "latest", "the newest release (26.8 today)"),
            choice("26.0", "26.0", "the 26 line's first release"),
            choice("25.0", "25.0", "the previous major version"),
            choice("24.0", "24.0", "older"),
        ],
        Question::Pnpm => vec![
            choice("latest", "latest", "the newest pnpm"),
            choice("10", "10", "the current major version"),
            choice("9", "9", "the previous major version"),
            choice("8", "8", "older, still used by many projects"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::toolchain::machine::{Fake, fake};

    fn old_rust(toolchains: &'static [&'static str]) -> Fake {
        Fake {
            rust: Some((1, 97)),
            toolchains,
            ..fake(&["cargo", "rustup", "npm"])
        }
    }

    #[test]
    fn rust_is_only_asked_about_when_the_one_in_use_is_too_old() {
        let new = Fake {
            rust: Some((1, 99)),
            ..fake(&["cargo", "rustup"])
        };
        assert_eq!(questions(&["topcoat"], &new), [Question::Topcoat]);
        assert_eq!(
            questions(&["topcoat"], &old_rust(&[])),
            [Question::Rust { needed: (1, 99) }, Question::Topcoat]
        );
        // Nothing here can install Rust without rustup, so there is nothing to ask.
        let no_rustup = Fake {
            rust: Some((1, 97)),
            ..fake(&["cargo"])
        };
        assert_eq!(questions(&["topcoat"], &no_rustup), [Question::Topcoat]);
    }

    #[test]
    fn only_tools_with_a_real_choice_are_asked_about() {
        assert_eq!(
            questions(&["mkcert", "uv", "docker", "git"], &old_rust(&[])),
            []
        );
        assert_eq!(questions(&["pnpm"], &old_rust(&[])), [Question::Pnpm]);
        assert_eq!(
            questions(&["pnpm"], &fake(&["brew"])),
            [],
            "without npm there is only latest"
        );
    }

    #[test]
    fn rust_offers_what_is_installed_first_then_latest_and_the_common_ones() {
        let m = old_rust(&[
            "stable-aarch64-apple-darwin",
            "1.85-aarch64-apple-darwin",
            "1.99.0-aarch64-apple-darwin",
        ]);
        let opts = options(Question::Rust { needed: (1, 99) }, &m);
        let values: Vec<&str> = opts.iter().map(|c| c.value.as_str()).collect();
        assert_eq!(
            values,
            ["1.99.0-aarch64-apple-darwin", "stable", "beta", "nightly"]
        );
        assert!(opts[0].note.contains("already installed"));
        assert!(
            !values.contains(&"1.85-aarch64-apple-darwin"),
            "too old to offer"
        );
    }

    #[test]
    fn rust_offers_the_oldest_workable_version_when_none_is_installed() {
        let opts = options(
            Question::Rust { needed: (1, 99) },
            &old_rust(&["stable-aarch64-apple-darwin"]),
        );
        let values: Vec<&str> = opts.iter().map(|c| c.value.as_str()).collect();
        assert_eq!(values, ["stable", "1.99.0", "beta", "nightly"]);
    }

    #[test]
    fn topcoat_recommends_the_tested_revision_and_offers_latest() {
        let opts = options(Question::Topcoat, &old_rust(&[]));
        assert_eq!(opts[0].value, TOPCOAT_CLI_REV);
        assert!(opts[0].label.contains("tested with stackup"));
        assert_eq!(opts[1].value, "main");
    }

    #[test]
    fn picks_remember_each_answer() {
        let mut picks = Picks::default();
        picks.set(Question::Topcoat, "main");
        picks.set(Question::Rust { needed: (1, 99) }, "stable");
        picks.set(Question::Pnpm, "10");
        assert_eq!(picks.topcoat.as_deref(), Some("main"));
        assert_eq!(picks.rust.as_deref(), Some("stable"));
        assert_eq!(picks.pnpm.as_deref(), Some("10"));
    }
}
