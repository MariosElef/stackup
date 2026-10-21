use crate::{
    models::{Backend, BeLib, FeLib, Frontend, Service, Tool},
    ui::{
        features::create::wizard::Wizard,
        types::{Focus, Row},
        utils::toggle,
    },
};

/// The menu top to bottom: headers, options, gaps, and the create button.
pub fn menu_rows() -> Vec<Row> {
    let mut rows = vec![Row::Header("Frontend")];
    rows.extend(Frontend::ALL.iter().map(|f| Row::Frontend(*f)));
    rows.push(Row::Gap);
    rows.push(Row::Header("Frontend libraries"));
    rows.extend(FeLib::ALL.iter().map(|l| Row::Lib(*l)));
    rows.push(Row::Gap);
    rows.push(Row::Header("Backend"));
    rows.extend(Backend::ALL.iter().map(|b| Row::Backend(*b)));
    rows.push(Row::Gap);
    rows.push(Row::Header("Backend libraries"));
    rows.extend(BeLib::ALL.iter().map(|l| Row::BeLib(*l)));
    rows.push(Row::Gap);
    rows.push(Row::Header("Services"));
    rows.extend(Service::ALL.iter().map(|s| Row::Service(*s)));
    rows.push(Row::Gap);
    rows.push(Row::Header("Dev tools"));
    rows.extend(Tool::ALL.iter().map(|t| Row::Tool(*t)));
    rows.push(Row::Gap);
    rows.push(Row::Create);
    rows
}

pub fn selectable(row: &Row) -> bool {
    !matches!(row, Row::Header(_) | Row::Gap)
}

pub fn row_label(row: &Row) -> String {
    match row {
        Row::Frontend(f) => f.to_string(),
        Row::Backend(b) => b.to_string(),
        Row::BeLib(l) => l.to_string(),
        Row::Lib(l) => l.to_string(),
        Row::Service(s) => s.to_string(),
        Row::Tool(t) => t.to_string(),
        Row::Create => "create project".into(),
        Row::Header(t) => (*t).to_string(),
        Row::Gap => String::new(),
    }
}

impl Wizard {
    pub fn step(&mut self, down: bool) {
        let mut i = self.cursor;
        loop {
            let next = if down { i + 1 } else { i.wrapping_sub(1) };
            if next >= self.rows.len() {
                return;
            }
            i = next;
            if selectable(&self.rows[i]) {
                self.cursor = i;
                return;
            }
        }
    }

    /// Picks a radio option, ticks a checkbox, or presses the create button.
    pub fn activate(&mut self) {
        self.status.clear();
        match self.rows[self.cursor] {
            Row::Frontend(f) => {
                self.sel.frontend = f;
            }
            Row::Lib(l) => {
                if let Err(why) = self.sel.toggle_lib(l) {
                    self.status = why.into();
                }
            }
            Row::Backend(b) => self.sel.backend = b,
            Row::BeLib(l) => {
                if let Err(why) = self.sel.toggle_be_lib(l) {
                    self.status = why.into();
                }
            }
            Row::Service(s) => toggle(&mut self.sel.services, s),
            Row::Tool(t) => toggle(&mut self.sel.tools, t),
            Row::Create => self.try_create(),
            Row::Header(_) | Row::Gap => {}
        }
        self.scroll = 0;
    }

    /// The first selectable row of each section, for `{` and `}`.
    fn section_starts(&self) -> Vec<usize> {
        (0..self.rows.len())
            .filter(|&i| {
                selectable(&self.rows[i])
                    && (self.rows[i] == Row::Create
                        || (i > 0 && matches!(self.rows[i - 1], Row::Header(_))))
            })
            .collect()
    }

    pub fn section(&mut self, forward: bool) {
        if self.focus != Focus::Menu {
            return;
        }
        let starts = self.section_starts();
        let target = if forward {
            starts.iter().find(|&&i| i > self.cursor)
        } else {
            starts.iter().rev().find(|&&i| i < self.cursor)
        };
        if let Some(&i) = target {
            self.cursor = i;
        }
    }

    /// `n` / `N`: the next or previous row whose label contains the search text.
    pub fn repeat_search(&mut self, forward: bool) {
        let Some(needle) = self.last_search.clone() else {
            self.status = "no previous search".into();
            return;
        };
        let needle = needle.to_lowercase();
        let len = self.rows.len();
        for step in 1..=len {
            let i = if forward {
                (self.cursor + step) % len
            } else {
                (self.cursor + len - step % len) % len
            };
            if selectable(&self.rows[i])
                && row_label(&self.rows[i]).to_lowercase().contains(&needle)
            {
                self.cursor = i;
                self.focus = Focus::Menu;
                return;
            }
        }
        self.status = format!("pattern not found: {needle}");
    }
}
