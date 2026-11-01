use std::fs;
use std::io::Write;

use ratatui::{
    crossterm::event::{self, Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    DefaultTerminal, Frame,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Layer 1: Entry, what any stored thing must provide
// ---------------------------------------------------------------------------

trait Entry: Serialize + DeserializeOwned + Clone {
    const FILE: &'static str; // "todos.json", "pages.json"
    fn id(&self) -> u64;
    fn title(&self) -> &str;
    fn body(&self) -> &str {
        "" // default: nothing extra to search
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Todo {
    id: u64,
    header: String,
    is_done: bool,
}

impl Entry for Todo {
    const FILE: &'static str = "todos.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.header
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Page {
    id: u64,
    title: String,
    text: String,
}

impl Entry for Page {
    const FILE: &'static str = "pages.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn body(&self) -> &str {
        &self.text
    }
}

// ---------------------------------------------------------------------------
// Layer 2: Store<T>, written once, used by every part
// ---------------------------------------------------------------------------

struct Store<T: Entry> {
    items: Vec<T>,
}

impl<T: Entry> Store<T> {
    fn load() -> Self {
        let items = match fs::read_to_string(T::FILE) {
            Ok(text) => serde_json::from_str(&text)
                .unwrap_or_else(|_| panic!("{} is corrupted", T::FILE)),
            Err(_) => Vec::new(),
        };
        Store { items }
    }
    fn save(&self) {
        let text = serde_json::to_string_pretty(&self.items).expect("serialize failed");
        fs::write(T::FILE, text).expect("write failed");
    }
    fn next_id(&self) -> u64 {
        self.items.iter().map(|i| i.id()).max().unwrap_or(0) + 1
    }
    fn get(&self, id: u64) -> Option<&T> {
        self.items.iter().find(|i| i.id() == id)
    }
    fn get_mut(&mut self, id: u64) -> Option<&mut T> {
        self.items.iter_mut().find(|i| i.id() == id)
    }
    fn remove(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.id() != id);
        self.items.len() != before
    }
}

// ---------------------------------------------------------------------------
// Shared event log: every part writes into the same history.jsonl
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
enum Kind {
    Added,
    Removed,
    Done,
    Reopened,
    Edited,
}

fn legacy_module() -> String {
    "todo".to_string() // old log lines had no module field; they were all todos
}

#[derive(Clone, Serialize, Deserialize)]
struct LogEvent {
    id: u64, // this event's own id, unique forever
    #[serde(default = "legacy_module")]
    module: String, // which part wrote it: "todo", "dict", ...
    kind: Kind,
    #[serde(alias = "todo_id")]
    entry_id: u64, // which entry of that module it happened to
    #[serde(alias = "header", default)]
    title: String, // the title at that moment
}

struct Logger {
    next_id: u64,
}

impl Logger {
    const FILE: &'static str = "history.jsonl";

    fn open() -> Self {
        let max = fs::read_to_string(Self::FILE)
            .map(|text| {
                text.lines()
                    .filter_map(|l| serde_json::from_str::<LogEvent>(l).ok())
                    .map(|e| e.id)
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        Logger { next_id: max + 1 }
    }

    fn log(&mut self, module: &str, kind: Kind, entry_id: u64, title: &str) {
        let event = LogEvent {
            id: self.next_id,
            module: module.to_string(),
            kind,
            entry_id,
            title: title.to_string(),
        };
        self.next_id += 1;
        let line = serde_json::to_string(&event).expect("Failed to turn event into text");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(Self::FILE)
            .expect("Failed to open history.jsonl");
        writeln!(file, "{}", line).expect("Failed to write event");
    }
}

// ---------------------------------------------------------------------------
// Fuzzy finder
// ---------------------------------------------------------------------------

// Returns None if the query's letters don't appear in order in the target.
// Higher score = better match.
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    let query: Vec<char> = query.to_lowercase().chars().collect();
    let target: Vec<char> = target.to_lowercase().chars().collect();

    let mut score = 0;
    let mut qi = 0;
    let mut last_match: Option<usize> = None;

    for (ti, &c) in target.iter().enumerate() {
        if qi < query.len() && c == query[qi] {
            score += 1;
            // bonus: matched right after the previous matched letter
            if let Some(last) = last_match {
                if last + 1 == ti {
                    score += 5;
                }
            }
            // bonus: matched at the start of a word
            if ti == 0 || target[ti - 1] == ' ' {
                score += 3;
            }
            last_match = Some(ti);
            qi += 1;
        }
    }

    if qi == query.len() {
        Some(score)
    } else {
        None
    }
}

// One search for every Store<T>: a title match counts double a body match.
fn search_store<T: Entry>(store: &Store<T>, query: &str) -> Vec<(i32, Row)> {
    store
        .items
        .iter()
        .filter_map(|item| {
            let title_score = fuzzy_score(query, item.title()).map(|s| s * 2);
            let body_score = fuzzy_score(query, item.body());
            title_score.max(body_score).map(|score| {
                (
                    score,
                    Row {
                        id: item.id(),
                        label: item.title().to_string(),
                    },
                )
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Layer 3: Module, what a "part" is
// ---------------------------------------------------------------------------

struct Row {
    id: u64,
    label: String,
}

// A command the UI offers for a module. The UI asks each prompt in order,
// then hands the answers to Module::apply. No prompts = runs immediately.
#[derive(Clone, Copy)]
struct Action {
    key: char,
    label: &'static str,
    needs_item: bool, // acts on the selected row
    prompts: &'static [&'static str],
}

trait Module {
    fn name(&self) -> &'static str;
    fn rows(&self) -> Vec<Row>;
    fn detail(&self, id: u64) -> String;
    fn actions(&self) -> Vec<Action>;
    // Text to pre-fill in the input box (used by edit). Default: empty.
    fn initial(&self, _action: char, _id: u64, _step: usize) -> String {
        String::new()
    }
    // Does the work and returns a status message for the UI.
    fn apply(&mut self, action: char, id: Option<u64>, input: &[String], log: &mut Logger)
        -> String;
    // The fuzzy finder asks every part.
    fn search(&self, query: &str) -> Vec<(i32, Row)>;
}

// ----- the todo part -----

struct TodoModule {
    store: Store<Todo>,
}

impl TodoModule {
    fn new() -> Self {
        TodoModule { store: Store::load() }
    }
}

impl Module for TodoModule {
    fn name(&self) -> &'static str {
        "todo"
    }

    fn rows(&self) -> Vec<Row> {
        self.store
            .items
            .iter()
            .map(|t| Row {
                id: t.id,
                label: format!("[{}] {}", if t.is_done { "x" } else { " " }, t.header),
            })
            .collect()
    }

    fn detail(&self, id: u64) -> String {
        match self.store.get(id) {
            Some(t) => format!(
                "{}\n\nid: {}\nstatus: {}",
                t.header,
                t.id,
                if t.is_done { "done" } else { "open" }
            ),
            None => String::new(),
        }
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action { key: 'a', label: "add", needs_item: false, prompts: &["Title"] },
            Action { key: 'x', label: "done/undo", needs_item: true, prompts: &[] },
            Action { key: 'e', label: "edit", needs_item: true, prompts: &["Title"] },
            Action { key: 'd', label: "delete", needs_item: true, prompts: &[] },
        ]
    }

    fn initial(&self, action: char, id: u64, _step: usize) -> String {
        match (action, self.store.get(id)) {
            ('e', Some(t)) => t.header.clone(),
            _ => String::new(),
        }
    }

    fn apply(&mut self, action: char, id: Option<u64>, input: &[String], log: &mut Logger)
        -> String {
        match action {
            'a' => {
                let title = input[0].trim().to_string();
                if title.is_empty() {
                    return "Title can't be empty.".into();
                }
                let id = self.store.next_id();
                self.store.items.push(Todo { id, header: title.clone(), is_done: false });
                self.store.save();
                log.log(self.name(), Kind::Added, id, &title);
                "Todo added.".into()
            }
            'x' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                let Some(todo) = self.store.get_mut(id) else { return "No such todo.".into() };
                todo.is_done = !todo.is_done;
                let kind = if todo.is_done { Kind::Done } else { Kind::Reopened };
                let title = todo.header.clone();
                self.store.save();
                log.log(self.name(), kind, id, &title);
                "Todo updated.".into()
            }
            'e' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                let title = input[0].trim().to_string();
                if title.is_empty() {
                    return "Title can't be empty.".into();
                }
                let Some(todo) = self.store.get_mut(id) else { return "No such todo.".into() };
                todo.header = title.clone();
                self.store.save();
                log.log(self.name(), Kind::Edited, id, &title);
                "Todo edited.".into()
            }
            'd' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if self.store.remove(id) {
                    self.store.save();
                    log.log(self.name(), Kind::Removed, id, "");
                    "Todo removed.".into()
                } else {
                    "No such todo.".into()
                }
            }
            _ => String::new(),
        }
    }

    fn search(&self, query: &str) -> Vec<(i32, Row)> {
        search_store(&self.store, query)
    }
}

// ----- the dictionary part -----

struct DictModule {
    store: Store<Page>,
}

impl DictModule {
    fn new() -> Self {
        DictModule { store: Store::load() }
    }
}

impl Module for DictModule {
    fn name(&self) -> &'static str {
        "dict"
    }

    fn rows(&self) -> Vec<Row> {
        self.store
            .items
            .iter()
            .map(|p| Row { id: p.id, label: p.title.clone() })
            .collect()
    }

    fn detail(&self, id: u64) -> String {
        match self.store.get(id) {
            Some(p) => format!("# {}\n\n{}", p.title, p.text),
            None => String::new(),
        }
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action { key: 'a', label: "add", needs_item: false, prompts: &["Title", "Text"] },
            Action { key: 'e', label: "edit", needs_item: true, prompts: &["Title", "Text"] },
            Action { key: 'd', label: "delete", needs_item: true, prompts: &[] },
        ]
    }

    fn initial(&self, action: char, id: u64, step: usize) -> String {
        match (action, self.store.get(id)) {
            ('e', Some(p)) if step == 0 => p.title.clone(),
            ('e', Some(p)) => p.text.clone(),
            _ => String::new(),
        }
    }

    fn apply(&mut self, action: char, id: Option<u64>, input: &[String], log: &mut Logger)
        -> String {
        match action {
            'a' => {
                let title = input[0].trim().to_string();
                if title.is_empty() {
                    return "Title can't be empty.".into();
                }
                let id = self.store.next_id();
                self.store.items.push(Page { id, title: title.clone(), text: input[1].clone() });
                self.store.save();
                log.log(self.name(), Kind::Added, id, &title);
                "Page added.".into()
            }
            'e' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                let title = input[0].trim().to_string();
                if title.is_empty() {
                    return "Title can't be empty.".into();
                }
                let Some(page) = self.store.get_mut(id) else { return "No such page.".into() };
                page.title = title.clone();
                page.text = input[1].clone();
                self.store.save();
                log.log(self.name(), Kind::Edited, id, &title);
                "Page updated.".into()
            }
            'd' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if self.store.remove(id) {
                    self.store.save();
                    log.log(self.name(), Kind::Removed, id, "");
                    "Page removed.".into()
                } else {
                    "No such page.".into()
                }
            }
            _ => String::new(),
        }
    }

    fn search(&self, query: &str) -> Vec<(i32, Row)> {
        search_store(&self.store, query)
    }
}

// ---------------------------------------------------------------------------
// The ratatui interface
// ---------------------------------------------------------------------------

struct SearchHit {
    score: i32,
    module: usize, // index into App::modules
    id: u64,
    label: String,
}

enum Mode {
    Normal,
    Input {
        module: usize,
        action: Action,
        item: Option<u64>,
        step: usize,
        answers: Vec<String>,
        buffer: String,
    },
    Search {
        query: String,
        hits: Vec<SearchHit>,
        selected: usize,
    },
}

struct App {
    modules: Vec<Box<dyn Module>>,
    log: Logger,
    tab: usize,
    selected: Vec<usize>, // selected row per module
    mode: Mode,
    status: String,
    quit: bool,
}

impl App {
    fn new() -> Self {
        // Adding a new part = write a struct that implements Module + one line here.
        let modules: Vec<Box<dyn Module>> =
            vec![Box::new(TodoModule::new()), Box::new(DictModule::new())];
        let selected = vec![0; modules.len()];
        App {
            modules,
            log: Logger::open(),
            tab: 0,
            selected,
            mode: Mode::Normal,
            status: String::new(),
            quit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.quit {
            terminal.draw(|f| self.draw(f))?;
            if let TermEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
                {
                    break;
                }
                self.on_key(key.code);
            }
        }
        Ok(())
    }

    // ----- selection helpers -----

    fn clamp_selection(&mut self) {
        let len = self.modules[self.tab].rows().len();
        let sel = &mut self.selected[self.tab];
        if len == 0 {
            *sel = 0;
        } else if *sel >= len {
            *sel = len - 1;
        }
    }

    fn selected_id(&self) -> Option<u64> {
        self.modules[self.tab]
            .rows()
            .get(self.selected[self.tab])
            .map(|r| r.id)
    }

    fn run_search(&self, query: &str) -> Vec<SearchHit> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for (mi, module) in self.modules.iter().enumerate() {
            for (score, row) in module.search(query) {
                hits.push(SearchHit {
                    score,
                    module: mi,
                    id: row.id,
                    label: row.label,
                });
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score));
        hits
    }

    // ----- input handling -----

    fn on_key(&mut self, code: KeyCode) {
        // take the mode out so we can change self freely, put it back after
        let mode = std::mem::replace(&mut self.mode, Mode::Normal);
        match mode {
            Mode::Normal => self.key_normal(code),
            Mode::Input { module, action, item, step, answers, buffer } => {
                self.key_input(code, module, action, item, step, answers, buffer)
            }
            Mode::Search { query, hits, selected } => self.key_search(code, query, hits, selected),
        }
    }

    fn key_normal(&mut self, code: KeyCode) {
        let count = self.modules.len();
        match code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Tab | KeyCode::Right => {
                self.tab = (self.tab + 1) % count;
                self.status.clear();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = (self.tab + count - 1) % count;
                self.status.clear();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected[self.tab] += 1;
                self.clamp_selection();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected[self.tab] = self.selected[self.tab].saturating_sub(1);
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search { query: String::new(), hits: Vec::new(), selected: 0 };
            }
            KeyCode::Char(c) => {
                let action = self.modules[self.tab].actions().into_iter().find(|a| a.key == c);
                if let Some(action) = action {
                    self.start_action(action);
                }
            }
            _ => {}
        }
    }

    fn start_action(&mut self, action: Action) {
        let item = self.selected_id();
        if action.needs_item && item.is_none() {
            self.status = "Nothing selected.".into();
            return;
        }
        if action.prompts.is_empty() {
            self.finish_action(self.tab, action, item, &[]);
        } else {
            let buffer = match item {
                Some(id) => self.modules[self.tab].initial(action.key, id, 0),
                None => String::new(),
            };
            self.mode = Mode::Input {
                module: self.tab,
                action,
                item,
                step: 0,
                answers: Vec::new(),
                buffer,
            };
        }
    }

    fn finish_action(&mut self, module: usize, action: Action, item: Option<u64>, answers: &[String]) {
        self.status = self.modules[module].apply(action.key, item, answers, &mut self.log);
        self.clamp_selection();
    }

    #[allow(clippy::too_many_arguments)]
    fn key_input(
        &mut self,
        code: KeyCode,
        module: usize,
        action: Action,
        item: Option<u64>,
        step: usize,
        mut answers: Vec<String>,
        mut buffer: String,
    ) {
        match code {
            KeyCode::Esc => {
                self.status = "Cancelled.".into();
                return; // mode stays Normal
            }
            KeyCode::Char(c) => buffer.push(c),
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Enter => {
                answers.push(buffer);
                let step = step + 1;
                if step >= action.prompts.len() {
                    self.finish_action(module, action, item, &answers);
                    return;
                }
                let buffer = match item {
                    Some(id) => self.modules[module].initial(action.key, id, step),
                    None => String::new(),
                };
                self.mode = Mode::Input { module, action, item, step, answers, buffer };
                return;
            }
            _ => {}
        }
        self.mode = Mode::Input { module, action, item, step, answers, buffer };
    }

    fn key_search(&mut self, code: KeyCode, mut query: String, mut hits: Vec<SearchHit>, mut selected: usize) {
        match code {
            KeyCode::Esc => return,
            KeyCode::Enter => {
                if let Some(hit) = hits.get(selected) {
                    self.tab = hit.module;
                    let rows = self.modules[hit.module].rows();
                    if let Some(pos) = rows.iter().position(|r| r.id == hit.id) {
                        self.selected[hit.module] = pos;
                    }
                }
                return;
            }
            KeyCode::Down => {
                if selected + 1 < hits.len() {
                    selected += 1;
                }
            }
            KeyCode::Up => selected = selected.saturating_sub(1),
            KeyCode::Char(c) => {
                query.push(c);
                hits = self.run_search(&query);
                selected = 0;
            }
            KeyCode::Backspace => {
                query.pop();
                hits = self.run_search(&query);
                selected = 0;
            }
            _ => {}
        }
        self.mode = Mode::Search { query, hits, selected };
    }

    // ----- drawing -----

    fn draw(&self, f: &mut Frame) {
        let [top, body, bottom] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .areas(f.area());

        // tabs
        let titles: Vec<&str> = self.modules.iter().map(|m| m.name()).collect();
        let tabs = Tabs::new(titles)
            .select(self.tab)
            .block(Block::bordered().title(" life "))
            .highlight_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(tabs, top);

        // body
        match &self.mode {
            Mode::Search { hits, selected, .. } => self.draw_search(f, body, hits, *selected),
            _ => self.draw_module(f, body),
        }

        // bottom line: prompt, search box or key hints
        let (title, line) = match &self.mode {
            Mode::Input { action, step, buffer, .. } => (
                format!(" {} ({}/{}) ", action.label, step + 1, action.prompts.len()),
                format!("{}: {}█", action.prompts[*step], buffer),
            ),
            Mode::Search { query, hits, .. } => {
                (format!(" search, {} hits ", hits.len()), format!("/ {}█", query))
            }
            Mode::Normal => {
                let mut hints: Vec<String> = self.modules[self.tab]
                    .actions()
                    .iter()
                    .map(|a| format!("{} {}", a.key, a.label))
                    .collect();
                hints.push("/ search".into());
                hints.push("tab switch".into());
                hints.push("q quit".into());
                (format!(" {} ", self.status), hints.join("  |  "))
            }
        };
        f.render_widget(Paragraph::new(line).block(Block::bordered().title(title)), bottom);
    }

    fn draw_module(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(area);

        let module = &self.modules[self.tab];
        let rows = module.rows();
        let sel = self.selected[self.tab];

        let items: Vec<ListItem> = rows.iter().map(|r| ListItem::new(r.label.clone())).collect();
        let list = List::new(items)
            .block(Block::bordered().title(format!(" {} ", module.name())))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        let mut state = ListState::default().with_selected(if rows.is_empty() { None } else { Some(sel) });
        f.render_stateful_widget(list, left, &mut state);

        let detail = rows.get(sel).map(|r| module.detail(r.id)).unwrap_or_default();
        f.render_widget(
            Paragraph::new(detail)
                .wrap(Wrap { trim: false })
                .block(Block::bordered().title(" detail ")),
            right,
        );
    }

    fn draw_search(&self, f: &mut Frame, area: ratatui::layout::Rect, hits: &[SearchHit], selected: usize) {
        let items: Vec<ListItem> = hits
            .iter()
            .map(|h| {
                ListItem::new(format!(
                    "[{}] {} {}. {}",
                    h.score,
                    self.modules[h.module].name(),
                    h.id,
                    h.label
                ))
            })
            .collect();
        let list = List::new(items)
            .block(Block::bordered().title(" results (enter opens, esc closes) "))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        let mut state = ListState::default().with_selected(if hits.is_empty() { None } else { Some(selected) });
        f.render_stateful_widget(list, area, &mut state);
    }
}

fn main() {
    let mut app = App::new();
    let mut terminal = ratatui::init(); // also installs a panic hook that restores the terminal
    let result = app.run(&mut terminal);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("error: {e}");
    }
}
