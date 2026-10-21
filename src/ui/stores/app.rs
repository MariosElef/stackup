use std::{cell::Cell, path::PathBuf};

use super::run::Run;
use crate::ui::features::{
    create::{selection::Selection, wizard::Wizard},
    projects::farewell::Farewell,
};
use crate::ui::utils::fuzzy;
use crate::{
    models::Step,
    services::{
        projects::{envs::Created, git::GitInfo, urls::Urls},
        toolchain::{
            github::Github,
            machine::{InstallPlan, Machine, System},
            versions::{Choice, Picks, Question},
        },
    },
};
use crate::{
    models::{Action, Args, Facts, Param, Project},
    ui::types::Page,
};

/// What typing in the footer is for.
pub enum PromptKind {
    /// The name of a new project.
    Name,
    /// A value an action needs: `left` are the ones still to ask after this one.
    Param {
        action: Action,
        args: Box<Args>,
        param: Param,
        left: Vec<Param>,
    },
    /// A folder to look for existing projects in.
    Add,
    /// The name of a new environment (a `.env.<name>` file).
    NewTier,
    /// The project's name typed back before something that cannot be undone.
    Confirm { word: String, then: Then },
}

pub enum Then {
    Run { action: Action, args: Box<Args> },
    Forget(PathBuf),
}

pub struct Prompt {
    pub label: String,
    pub value: String,
    pub kind: PromptKind,
}

/// The question asked when something the stack needs is not installed.
pub struct InstallModal {
    pub missing: Vec<&'static str>,
    pub plan: InstallPlan,
    /// 0 is Install, 1 is Close.
    pub selected: usize,
}

impl InstallModal {
    /// Whether any of the missing tools can be installed from here.
    pub fn can_install(&self) -> bool {
        !self.plan.steps.is_empty()
    }
}

/// Asked when Create is pressed and GitHub is not set up on this computer.
pub struct GithubModal {
    pub github: Github,
    /// What Set up would run, shown before it does.
    pub steps: Vec<Step>,
    /// 0 is Set up, 1 is Skip.
    pub selected: usize,
}

/// Offered when a run fails with an error that is recognised: what went wrong, what would put it
/// right, and that the failed steps are then run again.
pub struct AutofixModal {
    pub id: &'static str,
    pub problem: String,
    /// The fix, shown before it runs.
    pub fix: Vec<Step>,
    /// What failed, run again after the fix.
    pub retry: Vec<Step>,
    /// 0 is Autofix, 1 is Close.
    pub selected: usize,
    /// Set for a Rust that is too old: the version needed, so Rust is chosen before the fix.
    pub needs_rust: Option<(u32, u32)>,
}

/// Something the command palette can do.
#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    NewProject,
    MyProjects,
    AddProjects,
    Keymaps,
    Quit,
    /// Jump to the project at this place in the list.
    OpenProject(usize),
    /// An action on the selected project.
    Run(Action),
    ForgetProject,
    /// Open the selected project's app in the browser.
    OpenApp,
    /// Make a new environment on the selected project.
    NewTier,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub label: String,
    /// A line of explanation, or the key that does the same.
    pub note: String,
    pub command: Command,
}

/// The command palette: type to filter, Enter to run.
pub struct Palette {
    pub query: String,
    pub entries: Vec<Entry>,
    pub selected: usize,
}

impl Palette {
    /// The entries that match what is typed, the closest first. Entries that match equally keep
    /// the order they were listed in.
    pub fn shown(&self) -> Vec<&Entry> {
        let mut found: Vec<(u32, &Entry)> = self
            .entries
            .iter()
            .filter_map(|e| fuzzy(&self.query, &e.label).map(|score| (score, e)))
            .collect();
        found.sort_by_key(|(score, _)| *score);
        found.into_iter().map(|(_, e)| e).collect()
    }
}

/// What is done once every version question has been answered.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AfterChoices {
    Install,
    Autofix,
    /// Which environment to switch to.
    Tier,
    /// A value for an action that asks for several: carry on with the rest.
    Param,
    /// The versions for a new project: then it is made.
    Create,
}

/// "Which version?": a list with the recommended one first. Questions come one at a time.
pub struct ChoiceModal {
    pub question: Question,
    pub options: Vec<Choice>,
    pub selected: usize,
    /// The questions still to ask after this one.
    pub next: Vec<Question>,
    pub picks: Picks,
    pub then: AfterChoices,
    /// For the environments question: which one is active, `custom` when `.env` was edited by
    /// hand, `None` when there is no `.env`.
    pub active: Option<String>,
    /// For a value an action asks for: the action, what is known so far, and what is still to ask.
    pub pending: Option<(Action, Args, Vec<Param>)>,
}

/// What making an environment did, with the choice to switch to it.
pub struct TierModal {
    pub tier: String,
    pub created: Created,
    /// 0 is "Switch to it", 1 is Close.
    pub selected: usize,
}

/// How the app ends: nothing more to do, or a project to make.
#[derive(Clone, PartialEq, Debug)]
pub enum Outcome {
    Running,
    Quit,
    Start { name: String },
}

/// Everything the menu remembers. The logic that changes it lives in `composables`.
pub struct App {
    /// Where stackup keeps its list of projects.
    pub dir: PathBuf,
    pub page: Page,
    /// Start, My projects, Quit.
    pub home: usize,
    pub projects: Vec<Project>,
    pub cursor: usize,
    /// The actions that apply to the open project, and which one the cursor is on.
    pub actions: Vec<Action>,
    pub action: usize,
    /// What the open project's folder holds.
    pub facts: Option<Facts>,
    pub prompt: Option<Prompt>,
    pub status: String,
    pub run: Option<Run>,
    /// The program that makes a project: stackup itself, run on the in-app terminal.
    pub exe: PathBuf,
    /// The name of the project being made, while it is.
    pub creating: Option<String>,
    /// A removal finished: the project is gone, so going back returns to the list.
    pub removed: bool,
    /// The goodbye shown after a removal, until it ends or a key is pressed.
    pub farewell: Option<Farewell>,
    /// What the project being removed was built with, read before its folder goes.
    pub removing: Option<Facts>,
    /// What the running action was, for what to remember when it succeeds.
    pub after: Option<(Action, Args)>,
    /// The inside of the output pane, as last drawn: a terminal is sized to it.
    pub pane: Cell<(u16, u16)>,
    pub tick: u64,
    pub help: bool,
    /// Open when Create was pressed with tools missing: install them, or close.
    pub install: Option<InstallModal>,
    /// Open when Create was pressed and GitHub is not set up yet.
    pub github: Option<GithubModal>,
    /// Set up was left out for this session: Create goes on without asking again.
    pub github_skipped: bool,
    /// The run on the install page is the GitHub setup, not an install.
    pub github_run: bool,
    /// Open when a run failed in a way that can be fixed.
    pub autofix: Option<AutofixModal>,
    /// Open while asking which version to install.
    pub choice: Option<ChoiceModal>,
    /// The command palette, while it is open.
    pub palette: Option<Palette>,
    /// An environment was just made: switch to it, or close.
    pub tier_modal: Option<TierModal>,
    /// The open project's active environment, read when it was opened.
    pub env: Option<String>,
    /// Where git is for the open project: branch, changes, ahead and behind.
    pub git: Option<GitInfo>,
    /// Where the open project is reachable, read when it was opened.
    pub urls: Urls,
    /// A quick look at the project selected in the list: read when it is selected, not every frame.
    pub overview: Option<(PathBuf, crate::services::projects::overview::Overview)>,
    /// How much room each project takes, measured away from the screen (a big `target/` takes a
    /// moment to walk). A project not in here yet shows "measuring".
    pub usage: std::collections::HashMap<PathBuf, crate::services::projects::disk::Usage>,
    /// Where a measurement arrives, and for which projects one is already on its way.
    pub usage_rx:
        Option<std::sync::mpsc::Receiver<(PathBuf, crate::services::projects::disk::Usage)>>,
    pub usage_tx: std::sync::mpsc::Sender<(PathBuf, crate::services::projects::disk::Usage)>,
    pub measuring: std::collections::HashSet<PathBuf>,
    /// How an address is opened in the browser. Replaceable, so tests open nothing.
    pub opener: fn(&str) -> anyhow::Result<()>,
    /// The fixes already tried since this was started, so one is never offered twice.
    pub fixed: Vec<&'static str>,
    /// Whether the current run's failure was already looked at.
    pub diagnosed: bool,
    /// What this machine has: asked by the install plan and the fixes. Replaceable in tests.
    pub machine: Box<dyn Machine>,
    pub wizard: Option<Wizard>,
    /// Opened straight on the wizard (`stackup new`), so leaving it leaves the app.
    pub wizard_only: bool,
    pub installed: Vec<(&'static str, bool)>,
    pub selection: Selection,
    pub outcome: Outcome,
    /// Keys typed toward a motion: `g` waits for a second `g`.
    pub seq: String,
}

impl App {
    pub fn new(dir: PathBuf, installed: Vec<(&'static str, bool)>) -> Self {
        let (usage_tx, usage_rx) = std::sync::mpsc::channel();
        let mut app = Self {
            dir,
            page: Page::Home,
            home: 0,
            projects: vec![],
            cursor: 0,
            actions: vec![],
            action: 0,
            facts: None,
            prompt: None,
            status: String::new(),
            run: None,
            after: None,
            removed: false,
            farewell: None,
            removing: None,
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("stackup")),
            creating: None,
            pane: Cell::new((24, 80)),
            tick: 0,
            help: false,
            install: None,
            autofix: None,
            github: None,
            github_skipped: false,
            github_run: false,
            choice: None,
            palette: None,
            tier_modal: None,
            env: None,
            git: None,
            urls: Urls::default(),
            overview: None,
            usage: std::collections::HashMap::new(),
            usage_rx: Some(usage_rx),
            usage_tx,
            measuring: std::collections::HashSet::new(),
            opener: crate::clients::run::open_url,
            fixed: vec![],
            diagnosed: false,
            machine: Box::new(System),
            wizard: None,
            wizard_only: false,
            installed,
            selection: Selection::recommended(),
            outcome: Outcome::Running,
            seq: String::new(),
        };
        app.reload();
        app
    }

    /// The project list as it is on disk now.
    pub fn reload(&mut self) {
        match crate::services::projects::registry::load(&self.dir) {
            Ok(list) => self.projects = list,
            Err(e) => self.status = format!("{e:#}"),
        }
        self.cursor = self.cursor.min(self.projects.len().saturating_sub(1));
        // The selection may be another project now.
        self.load_overview();
    }

    pub fn project(&self) -> Option<&Project> {
        self.projects.get(self.cursor)
    }
}
