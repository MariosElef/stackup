use ratatui::style::Color;

use crate::models::{Backend, BeLib, FeLib, Frontend, Service, Tool};

#[derive(Clone, Copy, PartialEq)]
pub enum Row {
    Header(&'static str),
    Frontend(Frontend),
    Backend(Backend),
    BeLib(BeLib),
    Lib(FeLib),
    Service(Service),
    Tool(Tool),
    Gap,
    Create,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    Files,
    Compose,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Focus {
    Menu,
    Preview,
}

/// LazyVim-style modes: normal, `:` command line, `/` search, and the keymap help.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Normal,
    Command,
    Search,
    Help,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Running,
    Create,
    Cancel,
}

/// The screens of the app, as in mc2: a home menu, then the projects, then one project's actions
/// (and, when one runs, its output beside them).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Page {
    Home,
    Projects,
    Project,
    Run,
    /// The wizard that makes a new project.
    New,
    /// A missing tool being installed, from the wizard, on the in-app terminal.
    Install,
}

/// What the bottom of the screen says: a status bar, then a line for typing or messages.
pub struct Footer {
    pub mode: &'static str,
    pub mode_color: Color,
    pub focus: String,
    /// Keys typed so far toward a motion: `5` or `g`.
    pub pending: String,
    /// Programs the choices need, and whether each is installed.
    pub needs: Vec<(&'static str, bool)>,
    pub line: FooterLine,
}

pub enum FooterLine {
    /// What is being typed: `:q█`.
    Input(String),
    /// A message, shown in red.
    Status(String),
    Hint(&'static str),
}
