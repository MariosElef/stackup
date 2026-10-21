use std::cell::Cell;

use super::selection::Selection;
use crate::{
    models::{Plan, Tool, normalize_be_libs, normalize_libs},
    ui::{
        features::create::use_menu::{menu_rows, selectable},
        types::{Focus, Mode, Outcome, Row, Tab},
    },
};

/// Everything the wizard remembers. The logic that changes it lives in `composables`.
pub struct Wizard {
    pub name: String,
    pub sel: Selection,
    pub rows: Vec<Row>,
    pub cursor: usize,
    pub tab: Tab,
    pub scroll: u16,
    pub status: String,
    pub outcome: Outcome,
    pub installed: Vec<(&'static str, bool)>,
    pub focus: Focus,
    pub mode: Mode,
    /// Digits typed before a motion: `5j`.
    pub count: String,
    /// A half-typed key sequence: the first `g` of `gg`.
    pub seq: String,
    /// What is being typed after `:` or `/`.
    pub input: String,
    pub last_search: Option<String>,
    /// Set when Create is pressed with tools missing: the app offers to install them.
    pub blocked: Option<Vec<&'static str>>,
    /// Set while drawing: the preview cannot scroll past its last line.
    pub max_scroll: Cell<u16>,
}

impl Wizard {
    pub fn new(name: &str, sel: Selection, installed: Vec<(&'static str, bool)>) -> Self {
        let rows = menu_rows();
        let cursor = rows.iter().position(selectable).unwrap_or(0);
        Self {
            name: name.to_string(),
            sel,
            rows,
            cursor,
            tab: Tab::Files,
            scroll: 0,
            status: String::new(),
            outcome: Outcome::Running,
            installed,
            focus: Focus::Menu,
            mode: Mode::Normal,
            count: String::new(),
            seq: String::new(),
            input: String::new(),
            last_search: None,
            blocked: None,
            max_scroll: Cell::new(0),
        }
    }

    pub fn plan(&self) -> Plan {
        let mut libs = self.sel.libs.clone();
        normalize_libs(self.sel.frontend, &mut libs);
        let mut be_libs = self.sel.be_libs.clone();
        normalize_be_libs(self.sel.backend, &mut be_libs);
        Plan {
            name: self.name.clone(),
            title: self.name.clone(),
            frontend: self.sel.frontend,
            backend: self.sel.backend,
            libs,
            be_libs,
            services: self.sel.services.clone(),
            tools: self.sel.tools.clone(),
            git: self.sel.tools.contains(&Tool::Git),
            remote: None,
            publish: None,
            pins: vec![],
        }
    }
}
