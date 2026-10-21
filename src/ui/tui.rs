//! The full-screen wizard: choices on the left, a live preview of what they generate on the
//! right.

use std::{cell::Cell, io};

use anyhow::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::{
        event::{
            self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
            PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::supports_keyboard_enhancement,
    },
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};

use crate::{
    clients::run::have,
    models::{
        AXUM_LAYERS, Backend, FASTAPI_LAYERS, FeLib, Frontend, Plan, Service, Tool, normalize_libs,
    },
    services::compose::compose,
};

// Palette shared with the shell's `ls` (EZA_COLORS in config.fish).
const BLUE: Color = Color::Indexed(39);
const CYAN: Color = Color::Indexed(45);
const GREEN: Color = Color::Indexed(46);
const YELLOW: Color = Color::Indexed(220);
const RED: Color = Color::Indexed(203);
const PINK: Color = Color::Indexed(213);
const INK: Color = Color::Indexed(16);

#[derive(Clone)]
pub struct Selection {
    pub frontend: Frontend,
    pub backend: Backend,
    pub libs: Vec<FeLib>,
    pub services: Vec<Service>,
    pub tools: Vec<Tool>,
}

impl Selection {
    pub fn recommended() -> Self {
        Self {
            frontend: Frontend::Nuxt,
            backend: Backend::Axum,
            libs: vec![FeLib::Pinia, FeLib::Colada],
            services: vec![Service::Postgres, Service::Redis],
            tools: vec![
                Tool::Makefile,
                Tool::Gitleaks,
                Tool::Lint,
                Tool::Ci,
                Tool::Git,
            ],
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Row {
    Header(&'static str),
    Frontend(Frontend),
    Backend(Backend),
    Lib(FeLib),
    Service(Service),
    Tool(Tool),
    Gap,
    Create,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    Files,
    Compose,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Focus {
    Menu,
    Preview,
}

/// LazyVim-style modes: normal, `:` command line, `/` search, and the keymap help.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Normal,
    Command,
    Search,
    Help,
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Running,
    Create,
    Cancel,
}

const TOOLS_TO_CHECK: [&str; 5] = ["cargo", "uv", "pnpm", "git", "mkcert"];

struct App {
    name: String,
    sel: Selection,
    rows: Vec<Row>,
    cursor: usize,
    tab: Tab,
    scroll: u16,
    status: String,
    outcome: Outcome,
    installed: Vec<(&'static str, bool)>,
    focus: Focus,
    mode: Mode,
    /// Digits typed before a motion: `5j`.
    count: String,
    /// A half-typed key sequence: the first `g` of `gg`.
    seq: String,
    /// What is being typed after `:` or `/`.
    input: String,
    last_search: Option<String>,
    /// Set while drawing: the preview cannot scroll past its last line.
    max_scroll: Cell<u16>,
}

impl App {
    fn new(name: &str, sel: Selection, installed: Vec<(&'static str, bool)>) -> Self {
        let mut rows = vec![Row::Header("Frontend")];
        rows.extend(Frontend::ALL.iter().map(|f| Row::Frontend(*f)));
        rows.push(Row::Gap);
        rows.push(Row::Header("Frontend libraries"));
        rows.extend(FeLib::ALL.iter().map(|l| Row::Lib(*l)));
        rows.push(Row::Gap);
        rows.push(Row::Header("Backend"));
        rows.extend(Backend::ALL.iter().map(|b| Row::Backend(*b)));
        rows.push(Row::Gap);
        rows.push(Row::Header("Services"));
        rows.extend(Service::ALL.iter().map(|s| Row::Service(*s)));
        rows.push(Row::Gap);
        rows.push(Row::Header("Dev tools"));
        rows.extend(Tool::ALL.iter().map(|t| Row::Tool(*t)));
        rows.push(Row::Gap);
        rows.push(Row::Create);
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
            max_scroll: Cell::new(0),
        }
    }

    fn plan(&self) -> Plan {
        let mut libs = self.sel.libs.clone();
        normalize_libs(self.sel.frontend, &mut libs);
        Plan {
            name: self.name.clone(),
            frontend: self.sel.frontend,
            backend: self.sel.backend,
            libs,
            services: self.sel.services.clone(),
            tools: self.sel.tools.clone(),
            git: self.sel.tools.contains(&Tool::Git),
            remote: None,
            publish: None,
        }
    }

    fn requires(&self) -> Vec<&'static str> {
        let mut need = Vec::new();
        if self.sel.frontend != Frontend::None {
            need.push("pnpm");
        }
        match self.sel.backend {
            Backend::Axum => need.push("cargo"),
            Backend::Fastapi => need.push("uv"),
            Backend::None => {}
        }
        if self.sel.services.contains(&Service::Https) {
            need.push("mkcert");
        }
        need.push("git");
        need
    }

    fn is_installed(&self, bin: &str) -> bool {
        self.installed.iter().any(|(b, ok)| *b == bin && *ok)
    }

    fn step(&mut self, down: bool) {
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

    fn activate(&mut self) {
        self.status.clear();
        match self.rows[self.cursor] {
            Row::Frontend(f) => {
                self.sel.frontend = f;
            }
            Row::Lib(l) => self.toggle_lib(l),
            Row::Backend(b) => self.sel.backend = b,
            Row::Service(s) => toggle(&mut self.sel.services, s),
            Row::Tool(t) => toggle(&mut self.sel.tools, t),
            Row::Create => self.try_create(),
            Row::Header(_) | Row::Gap => {}
        }
        self.scroll = 0;
    }

    fn toggle_lib(&mut self, lib: FeLib) {
        if self.sel.frontend == Frontend::None {
            self.status = "choose a frontend first".into();
            return;
        }
        toggle(&mut self.sel.libs, lib);
        // Pinia Colada is built on Pinia: turning one on or the other off drags its partner.
        if lib == FeLib::Colada
            && self.sel.libs.contains(&lib)
            && !self.sel.libs.contains(&FeLib::Pinia)
        {
            self.sel.libs.push(FeLib::Pinia);
        }
        if lib == FeLib::Pinia && !self.sel.libs.contains(&lib) {
            self.sel.libs.retain(|l| *l != FeLib::Colada);
        }
    }

    fn try_create(&mut self) {
        let missing: Vec<&str> = self
            .requires()
            .into_iter()
            .filter(|b| !self.is_installed(b))
            .collect();
        if missing.is_empty() {
            self.outcome = Outcome::Create;
        } else {
            self.status = format!("install first: {}", missing.join(", "));
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.outcome = Outcome::Cancel;
            return;
        }
        match self.mode {
            Mode::Help => self.mode = Mode::Normal,
            Mode::Command => self.command_key(key),
            Mode::Search => self.search_key(key),
            Mode::Normal => self.normal_key(key, ctrl),
        }
    }

    fn normal_key(&mut self, key: KeyEvent, ctrl: bool) {
        self.status.clear();
        if key.code == KeyCode::Esc {
            self.seq.clear();
            self.count.clear();
            return;
        }
        // Mid-sequence: `g` waits for a second `g`.
        if !self.seq.is_empty() {
            match key.code {
                KeyCode::Char(c) => {
                    self.seq.push(c);
                    self.run_sequence();
                }
                _ => self.seq.clear(),
            }
            return;
        }
        if let KeyCode::Char(c) = key.code
            && !ctrl
            && (c.is_ascii_digit() && (c != '0' || !self.count.is_empty()))
        {
            self.count.push(c);
            return;
        }
        let n = self.take_count();
        match (key.code, ctrl) {
            // Window navigation, as in LazyVim: <C-h> left, <C-l> right.
            (KeyCode::Char('h'), true) | (KeyCode::Left, _) | (KeyCode::Char('h'), false) => {
                self.focus = Focus::Menu;
            }
            (KeyCode::Char('l'), true) | (KeyCode::Right, _) | (KeyCode::Char('l'), false) => {
                self.focus = Focus::Preview;
            }
            (KeyCode::Char('j'), false) | (KeyCode::Down, _) => self.down(n),
            (KeyCode::Char('k'), false) | (KeyCode::Up, _) => self.up(n),
            (KeyCode::Char('d'), true) | (KeyCode::PageDown, _) => self.half_page(true, n),
            (KeyCode::Char('u'), true) | (KeyCode::PageUp, _) => self.half_page(false, n),
            // Sections (`}` / `{` are vim's paragraph motions), and <C-j>/<C-k> where the
            // terminal can tell them apart from Enter.
            (KeyCode::Char('}'), _) | (KeyCode::Char('j'), true) => self.section(true),
            (KeyCode::Char('{'), _) | (KeyCode::Char('k'), true) => self.section(false),
            (KeyCode::Char('g'), false) => self.seq.push('g'),
            (KeyCode::Char('G'), _) => self.bottom(),
            (KeyCode::Enter, _) | (KeyCode::Char('x' | ' '), false) => {
                if self.focus == Focus::Menu {
                    self.activate();
                }
            }
            // <S-h>/<S-l> cycle buffers in LazyVim; here, the preview tabs.
            (KeyCode::Char('H'), _)
            | (KeyCode::Char('L'), _)
            | (KeyCode::Tab, _)
            | (KeyCode::BackTab, _) => self.switch_tab(),
            (KeyCode::Char('/'), false) => {
                self.mode = Mode::Search;
                self.input.clear();
            }
            (KeyCode::Char(':'), _) => {
                self.mode = Mode::Command;
                self.input.clear();
            }
            (KeyCode::Char('n'), false) => self.repeat_search(true),
            (KeyCode::Char('N'), _) => self.repeat_search(false),
            (KeyCode::Char('?'), _) => self.mode = Mode::Help,
            (KeyCode::Char('q'), false) => self.outcome = Outcome::Cancel,
            _ => {}
        }
    }

    fn take_count(&mut self) -> usize {
        let n = self.count.parse().unwrap_or(1).max(1);
        self.count.clear();
        n
    }

    /// The only multi-key sequence is `gg`.
    fn run_sequence(&mut self) {
        if self.seq == "gg" {
            self.top();
        }
        self.seq.clear();
    }

    fn down(&mut self, n: usize) {
        match self.focus {
            Focus::Menu => (0..n).for_each(|_| self.step(true)),
            Focus::Preview => self.scroll_by(n as isize),
        }
    }

    fn up(&mut self, n: usize) {
        match self.focus {
            Focus::Menu => (0..n).for_each(|_| self.step(false)),
            Focus::Preview => self.scroll_by(-(n as isize)),
        }
    }

    fn half_page(&mut self, down: bool, n: usize) {
        match self.focus {
            Focus::Menu => (0..5 * n).for_each(|_| self.step(down)),
            Focus::Preview => {
                let by = 10 * n as isize;
                self.scroll_by(if down { by } else { -by });
            }
        }
    }

    fn top(&mut self) {
        match self.focus {
            Focus::Menu => self.cursor = self.rows.iter().position(selectable).unwrap_or(0),
            Focus::Preview => self.scroll = 0,
        }
    }

    fn bottom(&mut self) {
        match self.focus {
            Focus::Menu => self.cursor = self.rows.iter().rposition(selectable).unwrap_or(0),
            Focus::Preview => self.scroll = self.max_scroll.get(),
        }
    }

    fn scroll_by(&mut self, delta: isize) {
        let max = self.max_scroll.get() as isize;
        self.scroll = (self.scroll as isize + delta).clamp(0, max) as u16;
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

    fn section(&mut self, forward: bool) {
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

    fn switch_tab(&mut self) {
        self.tab = if self.tab == Tab::Files {
            Tab::Compose
        } else {
            Tab::Files
        };
        self.scroll = 0;
    }

    fn search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                if !self.input.is_empty() {
                    self.last_search = Some(self.input.clone());
                    self.repeat_search(true);
                }
            }
            KeyCode::Backspace => {
                if self.input.pop().is_none() {
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
    }

    /// `n` / `N`: the next or previous row whose label contains the search text.
    fn repeat_search(&mut self, forward: bool) {
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

    fn command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                if self.input.pop().is_none() {
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                let cmd = std::mem::take(&mut self.input);
                match cmd.trim() {
                    "q" | "q!" | "qa" | "quit" => self.outcome = Outcome::Cancel,
                    "w" | "wq" | "wq!" | "x" | "write" | "create" => self.try_create(),
                    "h" | "help" => self.mode = Mode::Help,
                    "bn" | "bp" | "tabn" => self.switch_tab(),
                    "" => {}
                    other => self.status = format!("E492: not an editor command: {other}"),
                }
            }
            _ => {}
        }
    }
}

fn selectable(row: &Row) -> bool {
    !matches!(row, Row::Header(_) | Row::Gap)
}

fn row_label(row: &Row) -> String {
    match row {
        Row::Frontend(f) => f.to_string(),
        Row::Backend(b) => b.to_string(),
        Row::Lib(l) => l.to_string(),
        Row::Service(s) => s.to_string(),
        Row::Tool(t) => t.to_string(),
        Row::Create => "create project".into(),
        Row::Header(t) => (*t).to_string(),
        Row::Gap => String::new(),
    }
}

fn toggle<T: PartialEq>(list: &mut Vec<T>, item: T) {
    if let Some(i) = list.iter().position(|x| *x == item) {
        list.remove(i);
    } else {
        list.push(item);
    }
}

/// Runs the wizard. `None` means the user quit without creating anything.
pub fn run(name: &str, initial: Selection) -> Result<Option<Selection>> {
    let installed = TOOLS_TO_CHECK.iter().map(|b| (*b, have(b))).collect();
    let mut app = App::new(name, initial, installed);
    let mut terminal = ratatui::init();
    // Where the terminal supports it (Ghostty, kitty, WezTerm), <C-j> and <C-k> arrive distinct
    // from Enter. Elsewhere they cannot, which is why `{` and `}` exist too.
    let enhanced = supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
    }
    let result = event_loop(&mut terminal, &mut app);
    if enhanced {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result?;
    Ok((app.outcome == Outcome::Create).then_some(app.sel))
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while app.outcome == Outcome::Running {
        terminal.draw(|f| draw(f, app))?;
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
    }
    Ok(())
}

fn draw(f: &mut Frame, app: &App) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(f.area());
    let [left, right] =
        Layout::horizontal([Constraint::Length(42), Constraint::Min(30)]).areas(body);
    draw_menu(f, left, app);
    draw_preview(f, right, app);
    draw_footer(f, footer, app);
    if app.mode == Mode::Help {
        draw_help(f, f.area());
    }
}

/// The focused pane gets a bright border, the other a dim one.
fn pane_border(focused: bool) -> Style {
    if focused {
        Style::new().fg(CYAN).bold()
    } else {
        Style::new().dim()
    }
}

fn draw_menu(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Menu;
    let items: Vec<ListItem> = app
        .rows
        .iter()
        .map(|r| ListItem::new(row_line(app, r)))
        .collect();
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_style(pane_border(focused))
                .title(format!(" stackup · {} ", app.name)),
        )
        .highlight_style(if focused {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new().add_modifier(Modifier::BOLD)
        });
    let mut state = ListState::default().with_selected(Some(app.cursor));
    f.render_stateful_widget(list, area, &mut state);
}

fn row_line(app: &App, row: &Row) -> Line<'static> {
    let radio = |on: bool, label: String| {
        let (mark, style) = if on {
            ("(•) ", Style::new().fg(GREEN).bold())
        } else {
            ("( ) ", Style::new())
        };
        Line::from(vec![Span::styled(mark, style), Span::raw(label)])
    };
    let check = |on: bool, label: String| {
        let (mark, style) = if on {
            ("[x] ", Style::new().fg(GREEN).bold())
        } else {
            ("[ ] ", Style::new())
        };
        Line::from(vec![Span::styled(mark, style), Span::raw(label)])
    };
    match *row {
        Row::Header(t) => Line::from(Span::styled(t, Style::new().fg(CYAN).bold())),
        Row::Gap => Line::raw(""),
        Row::Frontend(f) => radio(app.sel.frontend == f, f.to_string()),
        Row::Backend(b) => radio(app.sel.backend == b, b.to_string()),
        Row::Lib(l) => {
            let usable = app.sel.frontend != Frontend::None;
            let line = check(usable && app.sel.libs.contains(&l), l.to_string());
            if usable { line } else { line.dim() }
        }
        Row::Service(s) => check(app.sel.services.contains(&s), s.to_string()),
        Row::Tool(t) => check(app.sel.tools.contains(&t), t.to_string()),
        Row::Create => Line::from(Span::styled(
            "▶ Create project",
            Style::new().fg(GREEN).bold(),
        )),
    }
}

fn draw_preview(f: &mut Frame, area: Rect, app: &App) {
    let plan = app.plan();
    let tab = |label: &'static str, active: bool| {
        if active {
            Span::styled(
                format!(" {label} "),
                Style::new().add_modifier(Modifier::REVERSED),
            )
        } else {
            Span::styled(format!(" {label} "), Style::new().dim())
        }
    };
    let title = Line::from(vec![
        tab("files", app.tab == Tab::Files),
        tab("docker-compose.yml", app.tab == Tab::Compose),
    ]);
    let lines = match app.tab {
        Tab::Files => {
            let mut out = vec![Line::from(Span::styled(
                format!("{}/", plan.name),
                Style::new().fg(BLUE).bold(),
            ))];
            render_tree(&tree(&plan), "", &mut out);
            out
        }
        Tab::Compose => compose(&plan)
            .lines()
            .map(|l| Line::raw(l.to_string()))
            .collect(),
    };
    // Never scroll past the last line, and let key handling know how far that is.
    let inner = area.height.saturating_sub(2) as usize;
    let max = lines.len().saturating_sub(inner) as u16;
    app.max_scroll.set(max);
    let p = Paragraph::new(lines)
        .scroll((app.scroll.min(max), 0))
        .block(
            Block::bordered()
                .border_style(pane_border(app.focus == Focus::Preview))
                .title(title),
        );
    f.render_widget(p, area);
}

/// A lualine-style status bar on top, and the command line / messages underneath.
fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let [top, bottom] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);

    let (mode, color) = match app.mode {
        Mode::Normal => (" NORMAL ", GREEN),
        Mode::Command => (" COMMAND ", YELLOW),
        Mode::Search => (" SEARCH ", PINK),
        Mode::Help => (" HELP ", BLUE),
    };
    let focus = if app.focus == Focus::Menu {
        "menu"
    } else {
        "preview"
    };
    let mut spans = vec![
        Span::styled(mode, Style::new().fg(INK).bg(color).bold()),
        Span::raw(format!(" {focus} ")),
    ];
    if !app.count.is_empty() || !app.seq.is_empty() {
        let pending = format!("{}{}", app.count, app.seq);
        spans.push(Span::styled(
            format!(" {pending} "),
            Style::new().fg(INK).bg(CYAN),
        ));
    }
    spans.push(Span::styled("  needs: ", Style::new().dim()));
    for bin in app.requires() {
        let (mark, color) = if app.is_installed(bin) {
            ("✓", GREEN)
        } else {
            ("✗", RED)
        };
        spans.push(Span::raw(format!("{bin} ")));
        spans.push(Span::styled(format!("{mark}  "), Style::new().fg(color)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), top);

    let second = match app.mode {
        Mode::Command => Line::from(format!(":{}█", app.input)),
        Mode::Search => Line::from(format!("/{}█", app.input)),
        _ if !app.status.is_empty() => Line::from(Span::styled(
            format!(" {}", app.status),
            Style::new().fg(RED).bold(),
        )),
        _ => Line::from(Span::styled(
            " <C-h>/<C-l> panes · j/k move · <space> tick · / search · : command · ? keymaps",
            Style::new().dim(),
        )),
    };
    f.render_widget(Paragraph::new(second), bottom);
}

fn draw_help(f: &mut Frame, screen: Rect) {
    const HELP: &[(&str, &str)] = &[
        ("j k / ↑ ↓", "move (5j moves five)"),
        ("<C-h> <C-l>", "focus the menu / the preview"),
        ("h l", "same, one pane left / right"),
        (
            "<Space> <CR> x",
            "select a radio option, tick a box, or create",
        ),
        ("gg G", "top / bottom"),
        ("<C-d> <C-u>", "half a page"),
        (
            "{ }",
            "previous / next section (<C-k> <C-j> where supported)",
        ),
        ("H L  <Tab>", "switch the preview tab"),
        ("/ n N", "search the menu, next, previous"),
        (":", ":q quit · :wq create · :help"),
        ("q  <C-c>", "quit without creating anything"),
    ];
    let lines: Vec<Line> = HELP
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<14}"), Style::new().fg(CYAN).bold()),
                Span::raw((*d).to_string()),
            ])
        })
        .collect();
    let width = 74.min(screen.width);
    let height = (lines.len() as u16 + 4).min(screen.height);
    let area = Rect {
        x: screen.x + (screen.width.saturating_sub(width)) / 2,
        y: screen.y + (screen.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title(" keymaps ")
                .title_bottom(" any key closes "),
        ),
        area,
    );
}

#[derive(Clone)]
enum Node {
    Dir(String, Vec<Node>),
    File(String),
}

fn dir(name: &str, kids: Vec<Node>) -> Node {
    Node::Dir(format!("{name}/"), kids)
}

fn file(name: &str) -> Node {
    Node::File(name.to_string())
}

fn files(names: &[&str]) -> Vec<Node> {
    names.iter().map(|n| file(n)).collect()
}

/// Empty layers: shown on one line, since each is just a `.gitkeep`.
fn layers(names: &[&str]) -> Vec<Node> {
    names.iter().map(|n| dir(n, vec![])).collect()
}

/// What `stackup new` will write for this plan, as a tree.
fn tree(plan: &Plan) -> Vec<Node> {
    let lint = plan.tool(Tool::Lint);
    let mut root = Vec::new();

    match plan.frontend {
        Frontend::Nuxt => {
            let tailwind = plan.lib(FeLib::Tailwind);
            let mut app = vec![file("app.vue"), dir("pages", vec![file("index.vue")])];
            for name in plan.frontend_layers() {
                app.push(if name == "assets" && tailwind {
                    dir("assets", vec![dir("css", vec![file("main.css")])])
                } else {
                    dir(name, vec![])
                });
            }
            let mut kids = vec![dir("app", app)];
            if plan.lib(FeLib::I18n) {
                kids.push(dir("i18n", vec![dir("locales", vec![file("en.json")])]));
            }
            kids.extend(files(&["nuxt.config.ts", "package.json", "Dockerfile"]));
            if plan.lib(FeLib::Openapi) {
                kids.push(file("openapi-ts.config.ts"));
            }
            if lint {
                kids.extend(files(&[
                    ".oxlintrc.json",
                    ".oxfmtrc.jsonc",
                    "eslint.config.mjs",
                ]));
            }
            root.push(dir("frontend", kids));
        }
        // Scaffolded with Vite+: lint, format and type checks live in vite.config.ts.
        Frontend::Vue => {
            let mut src = vec![
                file("main.ts"),
                file("App.vue"),
                dir("assets", vec![file("main.css")]),
                dir("router", vec![file("index.ts")]),
            ];
            if plan.lib(FeLib::I18n) {
                src.push(file("i18n.ts"));
                src.push(dir("locales", vec![file("en.json")]));
            }
            src.extend(layers(&plan.frontend_layers()));
            let mut kids = vec![dir("src", src)];
            kids.extend(files(&[
                "vite.config.ts",
                "package.json",
                "pnpm-workspace.yaml",
                "AGENTS.md",
                "Dockerfile",
            ]));
            if plan.lib(FeLib::Openapi) {
                kids.push(file("openapi-ts.config.ts"));
            }
            root.push(dir("frontend", kids));
        }
        Frontend::None => {}
    }

    match plan.backend {
        Backend::Axum => {
            let mut kids = vec![dir(
                "src",
                [vec![file("main.rs")], layers(AXUM_LAYERS)].concat(),
            )];
            kids.extend(files(&["Cargo.toml", "rust-toolchain.toml", "Dockerfile"]));
            if lint {
                kids.extend(files(&["deny.toml", "taplo.toml"]));
            }
            root.push(dir("backend", kids));
        }
        Backend::Fastapi => {
            let pkg = |n: &str| dir(n, vec![file("__init__.py")]);
            let mut app = vec![file("__init__.py"), file("main.py")];
            app.extend(FASTAPI_LAYERS.iter().map(|n| pkg(n)));
            let mut kids = vec![dir("app", app), dir("tests", vec![file("test_health.py")])];
            kids.extend(files(&["pyproject.toml", "Dockerfile"]));
            if lint {
                kids.push(file("taplo.toml"));
            }
            root.push(dir("backend", kids));
        }
        Backend::None => {}
    }

    if plan.tool(Tool::Gitleaks) {
        root.push(dir(".githooks", vec![file("pre-commit")]));
    }
    if plan.has(Service::Keycloak) {
        root.push(dir("keycloak", vec![file("app-realm.json")]));
    }
    if plan.has(Service::Https) {
        root.push(dir("certs", vec![]));
        root.push(dir("scripts", vec![file("certs.sh")]));
        root.push(file("Caddyfile"));
    }
    root.extend(files(&[
        "docker-compose.yml",
        ".env.example",
        ".env.local",
        ".env",
        ".gitignore",
        "README.md",
    ]));
    if plan.tool(Tool::Makefile) {
        root.push(file("Makefile"));
    }
    if plan.tool(Tool::Ci) {
        root.push(file("ci.sh"));
    }
    if lint {
        root.push(file(".yamlfmt"));
    }
    root
}

fn render_tree(nodes: &[Node], prefix: &str, out: &mut Vec<Line<'static>>) {
    for (i, node) in nodes.iter().enumerate() {
        let last = i + 1 == nodes.len();
        let branch = format!("{prefix}{}", if last { "└─ " } else { "├─ " });
        match node {
            Node::Dir(name, kids) => {
                let mut spans = vec![
                    Span::raw(branch),
                    Span::styled(name.clone(), Style::new().fg(BLUE).bold()),
                ];
                if kids.is_empty() {
                    spans.push(Span::styled("  .gitkeep", Style::new().dim()));
                }
                out.push(Line::from(spans));
                render_tree(
                    kids,
                    &format!("{prefix}{}", if last { "   " } else { "│  " }),
                    out,
                );
            }
            Node::File(name) => {
                let style = if name == ".gitkeep" {
                    Style::new().dim()
                } else {
                    Style::new()
                };
                out.push(Line::from(vec![
                    Span::raw(branch),
                    Span::styled(name.clone(), style),
                ]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn all_installed() -> Vec<(&'static str, bool)> {
        TOOLS_TO_CHECK.iter().map(|b| (*b, true)).collect()
    }

    fn app() -> App {
        App::new("demo", Selection::recommended(), all_installed())
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// Types a string as keys; `\n` is Enter, `\x04` is Ctrl-d, and so on.
    fn keys(app: &mut App, text: &str) {
        for ch in text.chars() {
            let key = match ch {
                '\n' => KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                '\x04' => KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
                '\x08' => KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
                '\x0c' => KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
                c => KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            };
            app.handle_key(key);
        }
    }

    fn screen(app: &App) -> String {
        let mut t = Terminal::new(TestBackend::new(120, 50)).unwrap();
        t.draw(|f| draw(f, app)).unwrap();
        t.backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn cursor_skips_headers_and_gaps() {
        let mut a = app();
        assert!(matches!(a.rows[a.cursor], Row::Frontend(Frontend::Nuxt)));
        for _ in 0..30 {
            press(&mut a, KeyCode::Down);
            assert!(selectable(&a.rows[a.cursor]));
        }
        assert!(matches!(a.rows[a.cursor], Row::Create));
    }

    #[test]
    fn space_selects_radios_and_toggles_checkboxes() {
        let mut a = app();
        press(&mut a, KeyCode::Down); // Vue
        press(&mut a, KeyCode::Enter);
        assert_eq!(a.sel.frontend, Frontend::Vue);
        let svc = a
            .rows
            .iter()
            .position(|r| *r == Row::Service(Service::Mongodb))
            .unwrap();
        a.cursor = svc;
        press(&mut a, KeyCode::Enter);
        assert!(a.sel.services.contains(&Service::Mongodb));
        press(&mut a, KeyCode::Enter);
        assert!(!a.sel.services.contains(&Service::Mongodb));
    }

    #[test]
    fn preview_follows_the_selection() {
        let mut a = app();
        assert!(screen(&a).contains("pages"), "Nuxt shows pages/");
        a.sel.frontend = Frontend::Vue;
        assert!(screen(&a).contains("views"), "Vue shows views/");
        a.sel.backend = Backend::Fastapi;
        let s = screen(&a);
        assert!(s.contains("repositories") && s.contains("pyproject.toml"));
        a.tab = Tab::Compose;
        assert!(screen(&a).contains("postgres:alpine"));
        a.sel.services.retain(|s| *s != Service::Postgres);
        assert!(!screen(&a).contains("postgres:alpine"));
    }

    #[test]
    fn create_is_blocked_while_a_tool_is_missing() {
        let mut a = App::new("demo", Selection::recommended(), vec![("cargo", false)]);
        keys(&mut a, ":wq\n");
        assert_eq!(a.outcome, Outcome::Running);
        assert!(a.status.contains("cargo"));
        let mut b = app();
        keys(&mut b, ":wq\n");
        assert_eq!(b.outcome, Outcome::Create);
    }

    #[test]
    fn q_and_ctrl_c_cancel() {
        let mut a = app();
        press(&mut a, KeyCode::Char('q'));
        assert_eq!(a.outcome, Outcome::Cancel);
        let mut b = app();
        b.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(b.outcome, Outcome::Cancel);
    }

    fn row_of(a: &App, lib: FeLib) -> usize {
        a.rows.iter().position(|r| *r == Row::Lib(lib)).unwrap()
    }

    #[test]
    fn colada_and_pinia_pull_each_other() {
        let mut a = app();
        a.sel.libs.clear();
        a.cursor = row_of(&a, FeLib::Colada);
        press(&mut a, KeyCode::Enter);
        assert!(a.sel.libs.contains(&FeLib::Colada) && a.sel.libs.contains(&FeLib::Pinia));
        a.cursor = row_of(&a, FeLib::Pinia);
        press(&mut a, KeyCode::Enter);
        assert!(a.sel.libs.is_empty(), "turning Pinia off turns Colada off");
    }

    #[test]
    fn libraries_bring_their_folders() {
        let mut a = app();
        a.sel.libs = vec![];
        let s = screen(&a);
        assert!(!s.contains("stores/") && !s.contains("queries/") && !s.contains("contracts/"));
        a.sel.libs = vec![FeLib::Pinia, FeLib::Colada, FeLib::Zod, FeLib::I18n];
        let s = screen(&a);
        assert!(s.contains("stores/") && s.contains("queries/") && s.contains("contracts/"));
        assert!(s.contains("locales"), "i18n adds i18n/locales");
    }

    #[test]
    fn libs_need_a_frontend() {
        let mut a = app();
        a.sel.frontend = Frontend::Vue;
        a.cursor = row_of(&a, FeLib::I18n);
        press(&mut a, KeyCode::Enter);
        assert!(a.sel.libs.contains(&FeLib::I18n));
        a.sel.frontend = Frontend::None;
        a.cursor = row_of(&a, FeLib::Zod);
        press(&mut a, KeyCode::Enter);
        assert!(a.status.contains("frontend"));
    }

    #[test]
    fn ctrl_h_and_ctrl_l_switch_panes() {
        let mut a = app();
        assert_eq!(a.focus, Focus::Menu);
        keys(&mut a, "\x0c");
        assert_eq!(a.focus, Focus::Preview);
        keys(&mut a, "\x08");
        assert_eq!(a.focus, Focus::Menu);
        keys(&mut a, "l");
        assert_eq!(a.focus, Focus::Preview);
        keys(&mut a, "h");
        assert_eq!(a.focus, Focus::Menu);
    }

    #[test]
    fn j_and_k_scroll_the_preview_when_it_has_focus() {
        let mut a = app();
        screen(&a); // a draw tells the app how far the preview can scroll
        let before = a.cursor;
        keys(&mut a, "\x0c");
        keys(&mut a, "5j");
        assert_eq!(a.cursor, before, "the menu cursor stays put");
        assert_eq!(a.scroll, 5.min(a.max_scroll.get()));
        keys(&mut a, "G");
        assert_eq!(a.scroll, a.max_scroll.get());
        keys(&mut a, "gg");
        assert_eq!(a.scroll, 0);
        keys(&mut a, "\x04");
        assert!(a.scroll > 0, "ctrl-d scrolls half a page");
    }

    #[test]
    fn counts_gg_and_big_g_move_the_menu() {
        let mut a = app();
        keys(&mut a, "3j");
        let after_three = a.cursor;
        keys(&mut a, "gg");
        keys(&mut a, "j");
        keys(&mut a, "j");
        keys(&mut a, "j");
        assert_eq!(a.cursor, after_three);
        keys(&mut a, "G");
        assert!(matches!(a.rows[a.cursor], Row::Create));
        keys(&mut a, "gg");
        assert!(matches!(a.rows[a.cursor], Row::Frontend(Frontend::Nuxt)));
    }

    #[test]
    fn braces_jump_between_sections() {
        let mut a = app();
        keys(&mut a, "}");
        assert!(
            matches!(a.rows[a.cursor], Row::Lib(_)),
            "next section is the libraries"
        );
        keys(&mut a, "}");
        assert!(matches!(a.rows[a.cursor], Row::Backend(_)));
        keys(&mut a, "{");
        assert!(matches!(a.rows[a.cursor], Row::Lib(_)));
    }

    #[test]
    fn space_ticks_like_enter() {
        let mut a = app();
        let mongo = a
            .rows
            .iter()
            .position(|r| *r == Row::Service(Service::Mongodb))
            .unwrap();
        a.cursor = mongo;
        keys(&mut a, " ");
        assert!(a.sel.services.contains(&Service::Mongodb));
        assert!(a.seq.is_empty(), "space starts no sequence");
        keys(&mut a, " ");
        assert!(!a.sel.services.contains(&Service::Mongodb));
        keys(&mut a, "x");
        assert!(a.sel.services.contains(&Service::Mongodb), "x ticks too");
    }

    #[test]
    fn command_line_works_like_vim() {
        let mut a = app();
        keys(&mut a, ":q\n");
        assert_eq!(a.outcome, Outcome::Cancel);
        let mut b = app();
        keys(&mut b, ":wq\n");
        assert_eq!(b.outcome, Outcome::Create);
        let mut c = app();
        keys(&mut c, ":bogus\n");
        assert!(c.status.contains("E492"));
        assert_eq!(c.mode, Mode::Normal);
        let mut d = app();
        keys(&mut d, ":q");
        assert!(
            screen(&d).contains(":q"),
            "the command line shows what is typed"
        );
        d.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(d.mode, Mode::Normal);
        assert_eq!(d.outcome, Outcome::Running);
    }

    #[test]
    fn slash_searches_the_menu_and_n_repeats() {
        let mut a = app();
        keys(&mut a, "/mongo\n");
        assert!(matches!(a.rows[a.cursor], Row::Service(Service::Mongodb)));
        keys(&mut a, "gg/pin\n");
        assert!(matches!(a.rows[a.cursor], Row::Lib(FeLib::Pinia)));
        keys(&mut a, "n");
        assert!(matches!(a.rows[a.cursor], Row::Lib(FeLib::Colada)));
        keys(&mut a, "N");
        assert!(matches!(a.rows[a.cursor], Row::Lib(FeLib::Pinia)));
        keys(&mut a, "/nothing-like-this\n");
        assert!(a.status.contains("not found"));
    }

    #[test]
    fn question_mark_opens_help_and_any_key_closes_it() {
        let mut a = app();
        keys(&mut a, "?");
        assert_eq!(a.mode, Mode::Help);
        assert!(screen(&a).contains("keymaps"));
        keys(&mut a, "x");
        assert_eq!(a.mode, Mode::Normal);
        assert_eq!(
            a.outcome,
            Outcome::Running,
            "the closing key is not also acted on"
        );
    }

    #[test]
    fn shift_h_and_l_switch_the_preview_tab() {
        let mut a = app();
        keys(&mut a, "L");
        assert_eq!(a.tab, Tab::Compose);
        keys(&mut a, "H");
        assert_eq!(a.tab, Tab::Files);
    }

    #[test]
    fn the_statusline_shows_the_mode_and_pending_keys() {
        let mut a = app();
        assert!(screen(&a).contains("NORMAL"));
        keys(&mut a, "3");
        assert!(screen(&a).contains('3'));
        a.count.clear();
        keys(&mut a, ":");
        assert!(screen(&a).contains("COMMAND"));
    }

    #[test]
    fn git_is_a_tool_toggle_that_feeds_the_plan() {
        let mut a = app();
        assert!(a.plan().git);
        a.cursor = a
            .rows
            .iter()
            .position(|r| *r == Row::Tool(Tool::Git))
            .unwrap();
        press(&mut a, KeyCode::Enter);
        assert!(!a.plan().git);
    }
}
