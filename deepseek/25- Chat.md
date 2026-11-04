# The whole code

Here it is — `Cargo.toml`, the panel, and `src/main.rs`. The file is long but every section is either kept verbatim from your original or replaced with the code from my last message. Comments mark the changed areas so you can diff against your old file.

---

## `Cargo.toml`

```toml
[package]
name = "dortyuzlu"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "dortyuzlu"
path = "src/main.rs"

[[bin]]
name = "panel"
path = "src/bin/panel.rs"

[dependencies]
chrono      = { version = "0.4", features = ["serde"] }
ratatui     = "0.29"
crossterm   = "0.28"
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
axum        = "0.7"
tokio       = { version = "1", features = ["rt-multi-thread", "net", "macros"] }
```

---

## `src/bin/panel.rs`

```rust
//! Browser panel: type, press Enter. Appends to inbox.jsonl; the TUI drains it.
//!
//!   cargo run --bin panel
//!   → http://127.0.0.1:3000

use std::io::Write;
use std::path::PathBuf;

use axum::{
    extract::Form,
    response::Html,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Entry {
    module: String,
    text: String,
}

fn data_dir() -> PathBuf {
    let dir = match std::env::var_os("DORTYUZLU_DIR") {
        Some(d) => PathBuf::from(d),
        None => match std::env::var_os("HOME") {
            Some(h) => PathBuf::from(h).join("dortyuzlu"),
            None => PathBuf::from("."),
        },
    };
    let _ = std::fs::create_dir_all(&dir);
    dir
}

async fn home() -> Html<&'static str> {
    Html(include_str!("panel.html"))
}

async fn add(Form(e): Form<Entry>) -> Html<String> {
    let at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let line = serde_json::json!({
        "module": e.module,
        "text": e.text,
        "at": at,
    });
    let path = data_dir().join("inbox.jsonl");
    let result = (|| -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(f, "{}", line)?;
        Ok(())
    })();
    let body = match result {
        Ok(()) => format!("<p>Queued: <code>{}</code></p>", escape(&e.text)),
        Err(err) => format!("<p>Could not queue: {err}</p>"),
    };
    Html(format!("{body}<p><a href=\"/\">back</a></p>"))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(home)).route("/add", post(add));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind :3000");
    println!("panel on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("serve");
}
```

---

## `src/bin/panel.html`

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>dortyuzlu</title>
    <style>
      body {
        font-family: system-ui, sans-serif;
        max-width: 42rem;
        margin: 2rem auto;
        padding: 0 1rem;
        color: #222;
      }
      h1 {
        font-weight: 600;
      }
      select,
      textarea,
      button {
        width: 100%;
        box-sizing: border-box;
        padding: 0.5rem;
        margin: 0.25rem 0;
        font: inherit;
      }
      textarea {
        min-height: 7rem;
      }
      button {
        cursor: pointer;
      }
      code {
        background: #f0f0f0;
        padding: 0.1rem 0.3rem;
        border-radius: 3px;
      }
      p {
        color: #555;
      }
    </style>
  </head>
  <body>
    <h1>dortyuzlu</h1>
    <p>
      A plain line is a diary entry. <code>todo:</code> makes a todo.
      <code>14:00-15:30 Title</code> makes an agenda event.
    </p>
    <form method="post" action="/add">
      <select name="module">
        <option value="diary">diary</option>
        <option value="todo">todo</option>
        <option value="timetable">timetable</option>
      </select>
      <textarea name="text" required autofocus></textarea>
      <button type="submit">queue</button>
    </form>
  </body>
</html>
```

---

## `src/main.rs`

```rust
//! project
//!
//! A modular, interconnected life-management system built with Rust and Ratatui.
//!
//! Two AIs, both local and both ask-first:
//!   Curator    reads todos, habits and the diary. Proposes structural fixes.
//!   Companion  reads the diary and answers questions in prose.
//!
//! Diary is the front door: lines with "todo:", "finished X", "HH:MM-HH:MM X"
//! or "title N" become *intents*; the Curator turns them into yes/no questions.
//!
//! Modules toggle from the settings tab. Diary, AI and settings are always on.
//!
//! ============================================================
//! HOW THE CODE IS BUILT (read this before adding a tab)
//! ============================================================
//!
//! Entry      what is stored: id, title, optional body and tags.
//! Store<T>   one JSON file of entries. Load, save, ids. Tabs on the same file share one.
//! Record     a kind of entry: how it is typed in, shown, and its special keys.
//! ListModule a generic tab for any Record.
//! Module     what the app sees of a tab.
//! Request    a tab asking another tab to press a key.
//! Advisor    the Curator half: proposes actions.
//! Companion  the narrative half: reads the diary, answers in prose.
//!
//! A new tab = `impl Entry` + `impl Record` for its struct, plus one line in App::new
//! (and one in ALL_MODULES so settings can toggle it).
//!
//! Quick entry, in every "(#tag)" prompt:  Buy milk #errands @quick
//!   #word  is a tag.   @group  (todos only) is main, quick, steady, resurface,
//!   secondary or serotonin.   Search with "#tag" to find by tag.
//!
//! Diary intents (the front door):
//!   todo: buy milk @quick      → proposes adding a todo
//!   finished laundry           → proposes marking a todo done
//!   smoked 2                   → proposes counting a habit
//!   14:00-15:30 Dentist #h     → proposes an agenda event
//! Everything still asks before it runs.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, Months, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use ratatui::{
    crossterm::event::{self, Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    DefaultTerminal, Frame,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// ===========================================================================
// 1. SMALL HELPERS: dates, times, tags
// ===========================================================================

const DATE_FMT: &str = "%Y-%m-%d";

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn today_text() -> String {
    today().format(DATE_FMT).to_string()
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), DATE_FMT).ok()
}

fn minutes_of(text: &str) -> Option<u32> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M")
        .ok()
        .map(|t| t.hour() * 60 + t.minute())
}

fn clock(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
}

fn day_heading(date: NaiveDate) -> String {
    let t = today();
    let relative = if date == t {
        " · today"
    } else if t.succ_opt() == Some(date) {
        " · tomorrow"
    } else if t.pred_opt() == Some(date) {
        " · yesterday"
    } else {
        ""
    };
    format!("{}{}", date.format("%a %d %b"), relative)
}

fn span_on(from: NaiveDate, start: u32, len: u32, on: NaiveDate) -> Option<(u32, u32)> {
    let a = (on - from).num_days() * 1440;
    if a < 0 {
        return None;
    }
    let lo = (start as i64).max(a);
    let hi = ((start + len) as i64).min(a + 1440);
    if lo >= hi {
        None
    } else {
        Some(((lo - a) as u32, (hi - a) as u32))
    }
}

fn desc_key(key: &str) -> String {
    key.chars()
        .map(|c| match c.to_digit(10) {
            Some(d) => char::from_digit(9 - d, 10).unwrap_or(c),
            None => c,
        })
        .collect()
}

fn clean_tag(word: &str) -> Option<String> {
    let tag = word
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if tag.is_empty() {
        None
    } else {
        Some(tag)
    }
}

fn split_text(text: &str, groups: bool) -> (String, Vec<String>, Option<String>) {
    let mut words: Vec<&str> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    let mut group: Option<String> = None;
    for word in text.split_whitespace() {
        if let Some(rest) = word.strip_prefix('#') {
            if let Some(tag) = clean_tag(rest) {
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        } else if let (true, Some(rest)) = (groups, word.strip_prefix('@')) {
            if !rest.is_empty() {
                group = Some(rest.to_lowercase());
            }
        } else {
            words.push(word);
        }
    }
    (words.join(" "), tags, group)
}

fn join_text(title: &str, tags: &[String], group: Option<&str>) -> String {
    let mut out = title.to_string();
    for tag in tags {
        out.push_str(" #");
        out.push_str(tag);
    }
    if let Some(g) = group {
        out.push_str(" @");
        out.push_str(g);
    }
    out
}

fn tags_text(tags: &[String]) -> String {
    tags.iter()
        .map(|t| format!("#{}", t))
        .collect::<Vec<String>>()
        .join(" ")
}

fn hashtags(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if let Some(rest) = word.strip_prefix('#') {
            if let Some(tag) = clean_tag(rest) {
                if !found.contains(&tag) {
                    found.push(tag);
                }
            }
        }
    }
    found
}

// First `room` characters, ellipsis if longer. Used in chat rows.
fn truncate(s: &str, room: usize) -> String {
    if s.chars().count() <= room {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(room.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

// ===========================================================================
// 2. STORAGE: Entry, Store<T>, Logger
// ===========================================================================

fn data_dir() -> PathBuf {
    let dir = match std::env::var_os("DORTYUZLU_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join("dortyuzlu"),
            None => PathBuf::from("."),
        },
    };
    let _ = fs::create_dir_all(&dir);
    dir
}

fn data_path(name: &str) -> PathBuf {
    data_dir().join(name)
}

fn adopt_old_file(name: &str) {
    let target = data_path(name);
    if !target.exists() && Path::new(name).is_file() {
        let _ = fs::copy(name, &target);
    }
}

// Pure helper so the id rule is testable without disk.
fn next_id_from(in_file: u64, stored: u64) -> u64 {
    in_file.max(stored) + 1
}

trait Entry: Serialize + DeserializeOwned + Clone {
    const FILE: &'static str;
    fn id(&self) -> u64;
    fn title(&self) -> &str;
    fn body(&self) -> &str {
        ""
    }
    fn tags(&self) -> &[String] {
        &[]
    }
}

struct Store<T: Entry> {
    items: Vec<T>,
}

impl<T: Entry> Store<T> {
    fn load() -> Self {
        adopt_old_file(T::FILE);
        let path = data_path(T::FILE);
        let items = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
                let aside = data_path(&format!("{}.corrupt", T::FILE));
                let _ = fs::rename(&path, &aside);
                eprintln!(
                    "{} could not be read; it was kept as {}",
                    path.display(),
                    aside.display()
                );
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        Store { items }
    }

    fn save(&self) {
        let text = serde_json::to_string_pretty(&self.items).expect("serialize failed");
        let temp = data_path(&format!("{}.tmp", T::FILE));
        fs::write(&temp, text).expect("write failed");
        fs::rename(&temp, data_path(T::FILE)).expect("rename failed");
    }

    // Ids never reuse, even after deletes. A high-water mark in a sidecar file
    // keeps a deleted id from being handed to a new entry (which would confuse
    // proposals and the log).
    fn next_id(&self) -> u64 {
        let in_file = self.items.iter().map(|i| i.id()).max().unwrap_or(0);
        let hw_path = data_path(&format!("{}.high_water", T::FILE));
        let stored: u64 = fs::read_to_string(&hw_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let next = next_id_from(in_file, stored);
        let _ = fs::write(&hw_path, next.to_string());
        next
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

#[allow(type_alias_bounds)]
type Shared<T: Entry> = Rc<RefCell<Store<T>>>;

fn share<T: Entry>() -> Shared<T> {
    Rc::new(RefCell::new(Store::load()))
}

#[derive(Clone, Serialize, Deserialize)]
enum Kind {
    Added,
    Removed,
    Done,
    Reopened,
    Edited,
    Counted,
    Approved,
    Rejected,
}

fn legacy_module() -> String {
    "todo".to_string()
}

#[derive(Clone, Serialize, Deserialize)]
struct LogEvent {
    id: u64,
    #[serde(default = "legacy_module")]
    module: String,
    kind: Kind,
    #[serde(alias = "todo_id")]
    entry_id: u64,
    #[serde(alias = "header", default)]
    title: String,
}

struct Logger {
    next_id: u64,
}

impl Logger {
    const FILE: &'static str = "history.jsonl";

    fn open() -> Self {
        adopt_old_file(Self::FILE);
        let max = fs::read_to_string(data_path(Self::FILE))
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
            .open(data_path(Self::FILE))
            .expect("Failed to open history.jsonl");
        writeln!(file, "{}", line).expect("Failed to write event");
    }
}

// ===========================================================================
// 3. FUZZY FINDER
// ===========================================================================

fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    let query: Vec<char> = query.to_lowercase().chars().collect();
    let target: Vec<char> = target.to_lowercase().chars().collect();

    let mut score = 0;
    let mut qi = 0;
    let mut last_match: Option<usize> = None;

    for (ti, &c) in target.iter().enumerate() {
        if qi < query.len() && c == query[qi] {
            score += 1;
            if let Some(last) = last_match {
                if last + 1 == ti {
                    score += 5;
                }
            }
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

fn score_item<T: Entry>(item: &T, query: &str) -> Option<i32> {
    if let Some(tag) = query.strip_prefix('#') {
        let tag = tag.to_lowercase();
        let hit = !tag.is_empty() && item.tags().iter().any(|t| t.starts_with(&tag));
        return if hit { Some(100) } else { None };
    }
    let title = fuzzy_score(query, item.title()).map(|s| s * 2);
    let body = fuzzy_score(query, item.body());
    let tags = fuzzy_score(query, &item.tags().join(" "));
    title.max(body).max(tags)
}

// ===========================================================================
// 4. MODULE
// ===========================================================================

struct Row {
    id: u64,
    label: String,
    group: String,
}

impl Row {
    fn new(id: u64, label: String) -> Row {
        Row { id, label, group: String::new() }
    }
}

#[derive(Clone, Copy)]
struct Action {
    key: char,
    label: &'static str,
    needs_item: bool,
    prompts: &'static [&'static str],
}

struct Request {
    module: String,
    key: char,
    item: Option<u64>,
    input: Vec<String>,
}

trait Module {
    fn name(&self) -> &'static str;
    fn heading(&self) -> String {
        self.name().to_string()
    }
    fn shows_day_bar(&self) -> bool {
        false
    }
    fn bar_date(&self, _selected: Option<u64>) -> Option<NaiveDate> {
        None
    }
    fn day_blocks(&self, _date: NaiveDate) -> Vec<(u32, u32)> {
        Vec::new()
    }
    fn block_color(&self) -> Color {
        Color::Magenta
    }
    fn initial_row(&self) -> usize {
        0
    }
    fn rows(&self) -> Vec<Row>;
    fn detail(&self, id: u64) -> String;
    fn actions(&self) -> Vec<Action>;
    fn initial(&self, _action: char, _id: u64, _step: usize) -> String {
        String::new()
    }
    fn apply(&mut self, action: char, id: Option<u64>, input: &[String], log: &mut Logger)
        -> String;
    fn search(&self, _query: &str) -> Vec<(i32, Row)> {
        Vec::new()
    }
    fn reveal(&mut self, _id: u64) {}
    fn tick(&mut self, _log: &mut Logger) {}
    fn outbox(&mut self) -> Vec<Request> {
        Vec::new()
    }
    fn takes_lines(&self) -> bool {
        false
    }
    fn line_input(&mut self, _text: &str, _at: &str, _log: &mut Logger) -> Option<String> {
        None
    }
    fn add(&mut self, _input: &[String], _log: &mut Logger) -> Result<String, String> {
        Err("This tab doesn't take new entries.".to_string())
    }
}

// ===========================================================================
// 5. RECORD + ListModule
// ===========================================================================

type Outcome = Result<(Kind, String), String>;

trait Record: Entry {
    const NOUN: &'static str;
    const TAGGED: bool = false;
    const LINES: bool = false;
    const COLOR: Color = Color::Magenta;

    fn prompts() -> &'static [&'static str];
    fn fields(&self) -> Vec<String>;
    fn build(id: u64, input: &[String], old: Option<&Self>) -> Result<Self, String>;
    fn from_line(_id: u64, _text: &str, _at: &str) -> Option<Self> {
        None
    }

    fn label(&self, lens: u8) -> String;
    fn detail(&self, lens: u8) -> String;
    fn in_lens(&self, _lens: u8) -> bool {
        true
    }
    fn views(_lens: u8) -> &'static [&'static str] {
        &[]
    }
    fn shown(&self, _lens: u8, _view: &str) -> bool {
        true
    }
    fn group(&self, _lens: u8) -> String {
        String::new()
    }
    fn order(&self, _lens: u8) -> String {
        String::new()
    }

    fn keys(_lens: u8) -> &'static str {
        "aed"
    }
    fn extra_actions(_lens: u8) -> Vec<Action> {
        Vec::new()
    }
    fn act(&mut self, _lens: u8, _key: char, _input: &[String]) -> Option<Outcome> {
        None
    }
    fn prefill(&self, _key: char, _step: usize) -> String {
        String::new()
    }

    fn blocks(&self, _lens: u8, _date: NaiveDate) -> Vec<(u32, u32)> {
        Vec::new()
    }
    fn tick(&mut self) -> bool {
        false
    }
}

struct ListModule<T: Record> {
    name: &'static str,
    lens: u8,
    store: Shared<T>,
    view: usize,
    tag: String,
    bar: bool,
}

impl<T: Record> ListModule<T> {
    fn new(name: &'static str, lens: u8, store: &Shared<T>) -> Self {
        ListModule {
            name,
            lens,
            store: Rc::clone(store),
            view: 0,
            tag: String::new(),
            bar: false,
        }
    }

    fn with_bar(mut self) -> Self {
        self.bar = true;
        self
    }

    fn add_entry(&mut self, input: &[String], log: &mut Logger) -> Result<String, String> {
        if !T::keys(self.lens).contains('a') {
            return Err("This tab doesn't take new entries.".to_string());
        }
        let mut store = self.store.borrow_mut();
        let new_id = store.next_id();
        let item = T::build(new_id, input, None)?;
        let title = item.title().to_string();
        store.items.push(item);
        store.save();
        log.log(T::NOUN, Kind::Added, new_id, &title);
        Ok(format!("Added {}.", T::NOUN))
    }

    fn view_name(&self) -> &'static str {
        T::views(self.lens).get(self.view).copied().unwrap_or("")
    }

    fn visible(&self, item: &T) -> bool {
        item.in_lens(self.lens)
            && item.shown(self.lens, self.view_name())
            && (self.tag.is_empty() || item.tags().iter().any(|t| *t == self.tag))
    }
}

impl<T: Record> Module for ListModule<T> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn heading(&self) -> String {
        let store = self.store.borrow();
        let hidden = store
            .items
            .iter()
            .filter(|i| i.in_lens(self.lens) && !self.visible(i))
            .count();
        let mut text = self.name.to_string();
        if !self.view_name().is_empty() {
            text.push_str(&format!(" · {}", self.view_name()));
        }
        if !self.tag.is_empty() {
            text.push_str(&format!(" · #{}", self.tag));
        }
        if hidden > 0 {
            text.push_str(&format!(" · {} hidden", hidden));
        }
        text
    }

    fn shows_day_bar(&self) -> bool {
        self.bar
    }

    fn block_color(&self) -> Color {
        T::COLOR
    }

    fn day_blocks(&self, date: NaiveDate) -> Vec<(u32, u32)> {
        let store = self.store.borrow();
        let blocks: Vec<(u32, u32)> = store
            .items
            .iter()
            .filter(|i| i.in_lens(self.lens))
            .flat_map(|i| i.blocks(self.lens, date))
            .collect();
        blocks
    }

    fn rows(&self) -> Vec<Row> {
        let store = self.store.borrow();
        let mut list: Vec<(String, Row)> = store
            .items
            .iter()
            .filter(|i| self.visible(i))
            .map(|i| {
                let row = Row {
                    id: i.id(),
                    label: i.label(self.lens),
                    group: i.group(self.lens),
                };
                (i.order(self.lens), row)
            })
            .collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list.into_iter().map(|(_, row)| row).collect()
    }

    fn detail(&self, id: u64) -> String {
        let store = self.store.borrow();
        let text = match store.get(id) {
            Some(item) => item.detail(self.lens),
            None => String::new(),
        };
        text
    }

    fn actions(&self) -> Vec<Action> {
        let keys = T::keys(self.lens);
        let mut list = Vec::new();
        if keys.contains('a') {
            list.push(Action { key: 'a', label: "add", needs_item: false, prompts: T::prompts() });
        }
        list.extend(T::extra_actions(self.lens));
        if keys.contains('e') {
            list.push(Action { key: 'e', label: "edit", needs_item: true, prompts: T::prompts() });
        }
        if keys.contains('d') {
            list.push(Action { key: 'd', label: "delete", needs_item: true, prompts: &[] });
        }
        if !T::views(self.lens).is_empty() {
            list.push(Action { key: 'v', label: "view", needs_item: false, prompts: &[] });
        }
        if T::TAGGED {
            list.push(Action {
                key: 't',
                label: "tag filter",
                needs_item: false,
                prompts: &["Tag (empty = every tag)"],
            });
        }
        list
    }

    fn initial(&self, action: char, id: u64, step: usize) -> String {
        let store = self.store.borrow();
        let Some(item) = store.get(id) else { return String::new() };
        if action == 'e' {
            item.fields().get(step).cloned().unwrap_or_default()
        } else {
            item.prefill(action, step)
        }
    }

    fn apply(&mut self, key: char, id: Option<u64>, input: &[String], log: &mut Logger)
        -> String {
        let lens = self.lens;
        let keys = T::keys(lens);

        if key == 'v' && !T::views(lens).is_empty() {
            self.view = (self.view + 1) % T::views(lens).len();
            return format!("View: {}.", self.view_name());
        }
        if key == 't' && T::TAGGED {
            self.tag = clean_tag(&input[0]).unwrap_or_default();
            return if self.tag.is_empty() {
                "Showing every tag.".to_string()
            } else {
                format!("Showing #{} only.", self.tag)
            };
        }

        if key == 'a' && keys.contains('a') {
            return match self.add_entry(input, log) {
                Ok(message) | Err(message) => message,
            };
        }

        let mut store = self.store.borrow_mut();
        match key {
            'e' if keys.contains('e') => {
                let Some(id) = id else { return "Nothing selected.".to_string() };
                let Some(old) = store.get(id).cloned() else {
                    return format!("No such {}.", T::NOUN);
                };
                let new = match T::build(id, input, Some(&old)) {
                    Ok(new) => new,
                    Err(message) => return message,
                };
                let title = new.title().to_string();
                if let Some(slot) = store.get_mut(id) {
                    *slot = new;
                }
                store.save();
                log.log(T::NOUN, Kind::Edited, id, &title);
                format!("Edited {}.", T::NOUN)
            }
            'd' if keys.contains('d') => {
                let Some(id) = id else { return "Nothing selected.".to_string() };
                let Some(title) = store.get(id).map(|i| i.title().to_string()) else {
                    return format!("No such {}.", T::NOUN);
                };
                store.remove(id);
                store.save();
                log.log(T::NOUN, Kind::Removed, id, &title);
                format!("Removed {}.", T::NOUN)
            }
            _ => {
                let Some(id) = id else { return String::new() };
                match store.get_mut(id).and_then(|i| i.act(lens, key, input)) {
                    Some(Ok((kind, message))) => {
                        let title = store
                            .get(id)
                            .map(|i| i.title().to_string())
                            .unwrap_or_default();
                        store.save();
                        log.log(T::NOUN, kind, id, &title);
                        message
                    }
                    Some(Err(message)) => message,
                    None => String::new(),
                }
            }
        }
    }

    fn search(&self, query: &str) -> Vec<(i32, Row)> {
        let store = self.store.borrow();
        let hits: Vec<(i32, Row)> = store
            .items
            .iter()
            .filter(|i| i.in_lens(self.lens))
            .filter_map(|i| {
                score_item(i, query).map(|score| (score, Row::new(i.id(), i.label(self.lens))))
            })
            .collect();
        hits
    }

    fn reveal(&mut self, _id: u64) {
        self.tag.clear();
        if let Some(pos) = T::views(self.lens).iter().position(|v| *v == "all") {
            self.view = pos;
        }
    }

    fn tick(&mut self, _log: &mut Logger) {
        let mut store = self.store.borrow_mut();
        let mut changed = false;
        for item in store.items.iter_mut() {
            changed |= item.tick();
        }
        if changed {
            store.save();
        }
    }

    fn takes_lines(&self) -> bool {
        T::LINES
    }

    fn add(&mut self, input: &[String], log: &mut Logger) -> Result<String, String> {
        self.add_entry(input, log)
    }

    fn line_input(&mut self, text: &str, at: &str, log: &mut Logger) -> Option<String> {
        if !T::LINES {
            return None;
        }
        let mut store = self.store.borrow_mut();
        let id = store.next_id();
        let Some(item) = T::from_line(id, text, at) else {
            return Some("Empty line ignored.".to_string());
        };
        let title = item.title().to_string();
        store.items.push(item);
        store.save();
        log.log(T::NOUN, Kind::Added, id, &title);
        Some("Saved.".to_string())
    }
}

// ===========================================================================
// 6. TODO
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum Group {
    #[default]
    Main,
    Quick,
    Steady,
    Resurface,
    Secondary,
    Serotonin,
}

impl Group {
    const ALL: [Group; 6] = [
        Group::Main,
        Group::Quick,
        Group::Steady,
        Group::Resurface,
        Group::Secondary,
        Group::Serotonin,
    ];

    fn word(self) -> &'static str {
        match self {
            Group::Main => "main",
            Group::Quick => "quick",
            Group::Steady => "steady",
            Group::Resurface => "resurface",
            Group::Secondary => "secondary",
            Group::Serotonin => "serotonin",
        }
    }

    fn about(self) -> &'static str {
        match self {
            Group::Main => "the real list",
            Group::Quick => "easy, done in minutes",
            Group::Steady => "a little work every day",
            Group::Resurface => "comes back every day",
            Group::Secondary => "can wait",
            Group::Serotonin => "small joys",
        }
    }

    fn rank(self) -> usize {
        Group::ALL.iter().position(|g| *g == self).unwrap_or(0)
    }

    fn parse(text: &str) -> Option<Group> {
        let text = text.trim().to_lowercase();
        if text.is_empty() {
            return None;
        }
        if let Some(exact) = Group::ALL.iter().find(|g| g.word() == text) {
            return Some(*exact);
        }
        let hits: Vec<Group> = Group::ALL
            .iter()
            .copied()
            .filter(|g| g.word().starts_with(&text))
            .collect();
        if hits.len() == 1 {
            Some(hits[0])
        } else {
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Todo {
    id: u64,
    header: String,
    is_done: bool,
    #[serde(default)]
    group: Group,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    added: String,
    #[serde(default)]
    done_on: String,
    #[serde(default)]
    minutes: u32,
    #[serde(default)]
    slot: String,
}

impl Entry for Todo {
    const FILE: &'static str = "todos.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.header
    }
    fn tags(&self) -> &[String] {
        &self.tags
    }
}

impl Todo {
    fn age_days(&self) -> Option<i64> {
        parse_date(&self.added).map(|d| (today() - d).num_days())
    }

    fn placed(&self) -> Option<(NaiveDate, u32)> {
        let (date, time) = self.slot.split_once(' ')?;
        Some((parse_date(date)?, minutes_of(time)?))
    }

    fn length(&self) -> u32 {
        if self.minutes == 0 {
            30
        } else {
            self.minutes
        }
    }

    fn slot_end(&self) -> Option<NaiveDateTime> {
        let start = NaiveDateTime::parse_from_str(&self.slot, "%Y-%m-%d %H:%M").ok()?;
        Some(start + chrono::Duration::minutes(self.length() as i64))
    }
}

fn parse_when(text: &str) -> Result<String, String> {
    let text = text.trim();
    let bad = || "Use HH:MM (today) or YYYY-MM-DD HH:MM.".to_string();
    let (date, time) = match text.split_once(' ') {
        Some((d, t)) => (parse_date(d).ok_or_else(bad)?, t),
        None => (today(), text),
    };
    let minutes = minutes_of(time).ok_or_else(bad)?;
    Ok(format!("{} {}", date.format(DATE_FMT), clock(minutes)))
}

const TODO_VIEWS: &[&str] = &[
    "focus", "all", "main", "quick", "steady", "resurface", "secondary", "serotonin",
];
const DONE_VIEWS: &[&str] = &["today", "week", "all"];
const CUBE_VIEWS: &[&str] = &["all", "placed", "unplaced"];

impl Record for Todo {
    const NOUN: &'static str = "todo";
    const TAGGED: bool = true;
    const COLOR: Color = Color::Green;

    fn prompts() -> &'static [&'static str] {
        &["Todo  (#tag  @main/quick/steady/resurface/secondary/serotonin)"]
    }

    fn fields(&self) -> Vec<String> {
        let group = if self.group == Group::Main { None } else { Some(self.group.word()) };
        vec![join_text(&self.header, &self.tags, group)]
    }

    fn build(id: u64, input: &[String], old: Option<&Todo>) -> Result<Todo, String> {
        let (title, tags, group) = split_text(&input[0], true);
        if title.is_empty() {
            return Err("Title can't be empty.".to_string());
        }
        let group = match group {
            Some(word) => Group::parse(&word).ok_or_else(|| {
                format!(
                    "No group @{}. Try quick, steady, resurface, secondary, serotonin.",
                    word
                )
            })?,
            None => Group::Main,
        };
        Ok(Todo {
            id,
            header: title,
            is_done: old.map_or(false, |o| o.is_done),
            group,
            tags,
            added: old.map_or_else(today_text, |o| o.added.clone()),
            done_on: old.map(|o| o.done_on.clone()).unwrap_or_default(),
            minutes: old.map_or(0, |o| o.minutes),
            slot: old.map(|o| o.slot.clone()).unwrap_or_default(),
        })
    }

    fn label(&self, lens: u8) -> String {
        let tags = if self.tags.is_empty() {
            String::new()
        } else {
            format!("  {}", tags_text(&self.tags))
        };
        match lens {
            1 => {
                let when = if self.done_on.is_empty() {
                    String::new()
                } else {
                    format!("  ({})", self.done_on)
                };
                format!("{}{}{}", self.header, tags, when)
            }
            2 => {
                let cube = if self.minutes == 0 {
                    "□".to_string()
                } else {
                    "■".repeat(((self.minutes + 14) / 15).clamp(1, 8) as usize)
                };
                let size = if self.minutes == 0 {
                    format!("{:>4}", "?")
                } else {
                    format!("{:>3}m", self.minutes)
                };
                let at = match self.placed() {
                    Some((_, start)) => format!("  @ {}", clock(start)),
                    None => String::new(),
                };
                format!("{:<8} {}  {}{}{}", cube, size, self.header, tags, at)
            }
            _ => {
                let age = match self.age_days() {
                    Some(days) if days >= 7 => format!("  ({}d)", days),
                    _ => String::new(),
                };
                format!("{}{}{}", self.header, tags, age)
            }
        }
    }

    fn detail(&self, _lens: u8) -> String {
        let status = if self.is_done {
            format!("done {}", self.done_on)
        } else {
            "open".to_string()
        };
        let cube = if self.minutes == 0 {
            "not sized yet (m on the cubes tab)".to_string()
        } else {
            format!("{} min", self.minutes)
        };
        let slot = if self.slot.is_empty() {
            "not on the timeline (s on the cubes tab)".to_string()
        } else {
            self.slot.clone()
        };
        let mut text = format!("{}\n\nstatus: {}\n", self.header, status);
        text.push_str(&format!("group: {} ({})\n", self.group.word(), self.group.about()));
        if !self.tags.is_empty() {
            text.push_str(&format!("tags: {}\n", tags_text(&self.tags)));
        }
        if let Some(days) = self.age_days() {
            text.push_str(&format!("added: {} ({} days ago)\n", self.added, days));
        }
        text.push_str(&format!("cube: {}\ntimeline: {}\n", cube, slot));
        text
    }

    fn in_lens(&self, lens: u8) -> bool {
        if lens == 1 {
            self.is_done
        } else {
            !self.is_done
        }
    }

    fn views(lens: u8) -> &'static [&'static str] {
        match lens {
            0 => TODO_VIEWS,
            1 => DONE_VIEWS,
            _ => CUBE_VIEWS,
        }
    }

    fn shown(&self, lens: u8, view: &str) -> bool {
        match (lens, view) {
            (0, "focus") => !matches!(self.group, Group::Secondary | Group::Serotonin),
            (0, "all") | (2, "all") => true,
            (0, word) => self.group.word() == word,
            (1, "today") => self.done_on == today_text(),
            (1, "week") => parse_date(&self.done_on).map_or(false, |d| (today() - d).num_days() < 7),
            (2, "placed") => !self.slot.is_empty(),
            (2, "unplaced") => self.slot.is_empty(),
            _ => true,
        }
    }

    fn group(&self, lens: u8) -> String {
        match lens {
            1 => parse_date(&self.done_on)
                .map(day_heading)
                .unwrap_or_else(|| "earlier".to_string()),
            2 => match self.placed() {
                Some((day, _)) => day_heading(day),
                None => "pile · not on the timeline".to_string(),
            },
            _ => format!("{} · {}", self.group.word(), self.group.about()),
        }
    }

    fn order(&self, lens: u8) -> String {
        match lens {
            1 => desc_key(&format!("{}{:012}", self.done_on, self.id)),
            2 => match self.placed() {
                Some((day, start)) => {
                    format!("1{}{:04}{:012}", day.format(DATE_FMT), start, self.id)
                }
                None => format!("0{:012}", self.id),
            },
            _ => format!("{}{:012}", self.group.rank(), self.id),
        }
    }

    fn keys(lens: u8) -> &'static str {
        if lens == 1 {
            "ed"
        } else {
            "aed"
        }
    }

    fn extra_actions(lens: u8) -> Vec<Action> {
        let done = |label: &'static str| Action { key: 'x', label, needs_item: true, prompts: &[] };
        match lens {
            1 => vec![done("reopen")],
            2 => vec![
                done("done"),
                Action {
                    key: 'm',
                    label: "size",
                    needs_item: true,
                    prompts: &["Size in minutes (empty = unsized)"],
                },
                Action {
                    key: 's',
                    label: "place",
                    needs_item: true,
                    prompts: &["When (HH:MM today, or YYYY-MM-DD HH:MM; empty = take off)"],
                },
            ],
            _ => vec![
                done("done"),
                Action {
                    key: 'g',
                    label: "group",
                    needs_item: true,
                    prompts: &["Group (main quick steady resurface secondary serotonin)"],
                },
            ],
        }
    }

    fn prefill(&self, key: char, _step: usize) -> String {
        match key {
            'g' => self.group.word().to_string(),
            'm' if self.minutes > 0 => self.minutes.to_string(),
            's' => self.slot.clone(),
            _ => String::new(),
        }
    }

    fn act(&mut self, _lens: u8, key: char, input: &[String]) -> Option<Outcome> {
        match key {
            'x' => {
                self.is_done = !self.is_done;
                self.done_on = if self.is_done { today_text() } else { String::new() };
                Some(Ok(if self.is_done {
                    (Kind::Done, "Moved to done.".to_string())
                } else {
                    (Kind::Reopened, "Moved back to todo.".to_string())
                }))
            }
            'g' => Some(match Group::parse(&input[0]) {
                Some(group) => {
                    self.group = group;
                    Ok((Kind::Edited, format!("Group: {}.", group.word())))
                }
                None => Err("Groups: main quick steady resurface secondary serotonin.".to_string()),
            }),
            'm' => {
                let text = input[0].trim();
                let minutes: u32 = if text.is_empty() {
                    0
                } else {
                    match text.parse() {
                        Ok(m) => m,
                        Err(_) => return Some(Err("Size must be whole minutes, like 30.".to_string())),
                    }
                };
                if minutes > 1440 {
                    return Some(Err("A cube can't be longer than a day (1440).".to_string()));
                }
                self.minutes = minutes;
                let message = if minutes == 0 { "Size cleared." } else { "Cube resized." };
                Some(Ok((Kind::Edited, message.to_string())))
            }
            's' => {
                if input[0].trim().is_empty() {
                    self.slot.clear();
                    return Some(Ok((Kind::Edited, "Taken off the timeline.".to_string())));
                }
                match parse_when(&input[0]) {
                    Ok(slot) => {
                        self.slot = slot;
                        if self.minutes == 0 {
                            self.minutes = 30;
                        }
                        Some(Ok((Kind::Edited, format!("Placed at {}.", self.slot))))
                    }
                    Err(message) => Some(Err(message)),
                }
            }
            _ => None,
        }
    }

    fn blocks(&self, lens: u8, date: NaiveDate) -> Vec<(u32, u32)> {
        if lens != 2 || self.is_done {
            return Vec::new();
        }
        let Some((from, start)) = self.placed() else { return Vec::new() };
        span_on(from, start, self.length(), date).into_iter().collect()
    }

    fn tick(&mut self) -> bool {
        let mut changed = false;
        if self.added.is_empty() {
            self.added = today_text();
            changed = true;
        }
        if self.group == Group::Resurface && self.is_done && self.done_on != today_text() {
            self.is_done = false;
            changed = true;
        }
        changed
    }
}

// ===========================================================================
// 7. HABITS
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum HabitKind {
    #[default]
    Make,
    Quit,
}

#[derive(Clone, Serialize, Deserialize)]
struct Habit {
    id: u64,
    title: String,
    kind: HabitKind,
    target: u32,
    #[serde(default)]
    unit: String,
    started: String,
    #[serde(default)]
    counts: BTreeMap<String, u32>,
    #[serde(default)]
    tags: Vec<String>,
}

impl Entry for Habit {
    const FILE: &'static str = "habits.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn tags(&self) -> &[String] {
        &self.tags
    }
}

impl Habit {
    fn count(&self, day: NaiveDate) -> u32 {
        self.counts
            .get(&day.format(DATE_FMT).to_string())
            .copied()
            .unwrap_or(0)
    }

    fn good(&self, day: NaiveDate) -> bool {
        let n = self.count(day);
        match self.kind {
            HabitKind::Make => n >= self.target.max(1),
            HabitKind::Quit => n <= self.target,
        }
    }

    fn start_day(&self) -> NaiveDate {
        parse_date(&self.started).unwrap_or_else(today)
    }

    fn streak(&self) -> u32 {
        let start = self.start_day();
        let mut day = today();
        if self.kind == HabitKind::Make && !self.good(day) {
            match day.pred_opt() {
                Some(d) => day = d,
                None => return 0,
            }
        }
        let mut run = 0;
        while day >= start && self.good(day) {
            run += 1;
            match day.pred_opt() {
                Some(d) => day = d,
                None => break,
            }
        }
        run
    }

    fn best_streak(&self) -> u32 {
        let mut day = self.start_day();
        let (mut run, mut best) = (0, 0);
        while day <= today() {
            if self.good(day) {
                run += 1;
                best = best.max(run);
            } else {
                run = 0;
            }
            match day.succ_opt() {
                Some(d) => day = d,
                None => break,
            }
        }
        best
    }

    fn consistency(&self) -> (u32, u32) {
        let start = self.start_day();
        let mut day = today();
        let (mut good, mut total) = (0, 0);
        for _ in 0..30 {
            if day < start {
                break;
            }
            let open = day == today() && self.kind == HabitKind::Make && !self.good(day);
            if !open {
                total += 1;
                if self.good(day) {
                    good += 1;
                }
            }
            match day.pred_opt() {
                Some(d) => day = d,
                None => break,
            }
        }
        (good, total)
    }

    fn recent(&self, n: i64) -> String {
        let start = self.start_day();
        (0..n)
            .rev()
            .map(|back| {
                let day = today() - chrono::Duration::days(back);
                if day < start {
                    '·'
                } else if self.good(day) {
                    '✓'
                } else if day == today() && self.kind == HabitKind::Make {
                    '○'
                } else {
                    '✗'
                }
            })
            .collect()
    }

    fn unit_text(&self) -> String {
        if self.unit.is_empty() {
            String::new()
        } else {
            format!(" {}", self.unit)
        }
    }
}

impl Record for Habit {
    const NOUN: &'static str = "habit";
    const TAGGED: bool = true;

    fn prompts() -> &'static [&'static str] {
        &[
            "Habit  (#tag)",
            "Build or quit? (b / q)",
            "Daily target (build: at least, quit: at most; empty = 1 / 0)",
            "Unit (glasses, cigarettes... optional)",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            join_text(&self.title, &self.tags, None),
            match self.kind {
                HabitKind::Make => "b".to_string(),
                HabitKind::Quit => "q".to_string(),
            },
            self.target.to_string(),
            self.unit.clone(),
        ]
    }

    fn build(id: u64, input: &[String], old: Option<&Habit>) -> Result<Habit, String> {
        let (title, tags, _) = split_text(&input[0], false);
        if title.is_empty() {
            return Err("Title can't be empty.".to_string());
        }
        let kind = match input[1].trim().to_lowercase().chars().next() {
            Some('b') | Some('m') | None => HabitKind::Make,
            Some('q') => HabitKind::Quit,
            Some(_) => return Err("Type b to build a habit or q to quit one.".to_string()),
        };
        let target: u32 = if input[2].trim().is_empty() {
            if kind == HabitKind::Make { 1 } else { 0 }
        } else {
            input[2]
                .trim()
                .parse()
                .map_err(|_| "Target must be a whole number.".to_string())?
        };
        if kind == HabitKind::Make && target == 0 {
            return Err("A habit to build needs a target of at least 1.".to_string());
        }
        Ok(Habit {
            id,
            title,
            kind,
            target,
            unit: input[3].trim().to_string(),
            started: old.map_or_else(today_text, |o| o.started.clone()),
            counts: old.map(|o| o.counts.clone()).unwrap_or_default(),
            tags,
        })
    }

    fn label(&self, _lens: u8) -> String {
        let n = self.count(today());
        let ok = self.good(today());
        let unit = self.unit_text();
        match self.kind {
            HabitKind::Make => format!(
                "[{}] {}  {}/{}{}  streak {}d",
                if ok { "x" } else { " " },
                self.title,
                n,
                self.target,
                unit,
                self.streak()
            ),
            HabitKind::Quit => format!(
                "[{}] {}  {} (max {}){}  {} {}d",
                if ok { "x" } else { "!" },
                self.title,
                n,
                self.target,
                unit,
                if self.target == 0 { "clean" } else { "streak" },
                self.streak()
            ),
        }
    }

    fn detail(&self, _lens: u8) -> String {
        let (good, total) = self.consistency();
        let goal = match self.kind {
            HabitKind::Make => format!("build: at least {}{} a day", self.target, self.unit_text()),
            HabitKind::Quit if self.target == 0 => format!("quit: none at all{}", self.unit_text()),
            HabitKind::Quit => format!("quit: at most {}{} a day", self.target, self.unit_text()),
        };
        let mut text = format!("{}\n\n{}\n", self.title, goal);
        text.push_str(&format!("today: {}{}\n", self.count(today()), self.unit_text()));
        text.push_str(&format!("streak: {} days (best {})\n", self.streak(), self.best_streak()));
        text.push_str(&format!("last 30 days: {} of {} good\n", good, total));
        text.push_str(&format!("last 14 days: {}\n", self.recent(14)));
        text.push_str(&format!("started: {}\n", self.started));
        if !self.tags.is_empty() {
            text.push_str(&format!("tags: {}\n", tags_text(&self.tags)));
        }
        text.push_str("\n+ counts one today, - takes one back");
        text
    }

    fn group(&self, _lens: u8) -> String {
        match self.kind {
            HabitKind::Make => "habits to build".to_string(),
            HabitKind::Quit => "habits to quit".to_string(),
        }
    }

    fn order(&self, _lens: u8) -> String {
        let rank = if self.kind == HabitKind::Make { 0 } else { 1 };
        format!("{}{:012}", rank, self.id)
    }

    fn extra_actions(_lens: u8) -> Vec<Action> {
        vec![
            Action { key: '+', label: "+1", needs_item: true, prompts: &[] },
            Action { key: '-', label: "-1", needs_item: true, prompts: &[] },
        ]
    }

    fn act(&mut self, _lens: u8, key: char, _input: &[String]) -> Option<Outcome> {
        match key {
            '+' => {
                *self.counts.entry(today_text()).or_insert(0) += 1;
                let n = self.count(today());
                let message = match self.kind {
                    HabitKind::Make if n >= self.target => {
                        format!("{}/{} today. Done!", n, self.target)
                    }
                    HabitKind::Make => format!("{}/{} today.", n, self.target),
                    HabitKind::Quit if n > self.target => format!(
                        "{} today, over the limit of {}. Streak reset.",
                        n, self.target
                    ),
                    HabitKind::Quit => format!("{} of {} allowed today.", n, self.target),
                };
                Some(Ok((Kind::Counted, message)))
            }
            '-' => {
                let day = today_text();
                let current = self.counts.get(&day).copied().unwrap_or(0);
                if current == 0 {
                    return Some(Err("Nothing counted today.".to_string()));
                }
                if current == 1 {
                    self.counts.remove(&day);
                } else {
                    self.counts.insert(day, current - 1);
                }
                Some(Ok((Kind::Counted, "Took one off.".to_string())))
            }
            _ => None,
        }
    }
}

// ===========================================================================
// 8. DAILY
// ===========================================================================

const DAY_NAMES: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
const WEEKDAYS: [u8; 5] = [0, 1, 2, 3, 4];
const WEEKEND: [u8; 2] = [5, 6];

fn all_days() -> Vec<u8> {
    (0..7).collect()
}

fn day_index(word: &str) -> Option<u8> {
    let word = word.to_lowercase();
    DAY_NAMES
        .iter()
        .position(|name| word.starts_with(*name))
        .map(|i| i as u8)
}

fn parse_days(text: &str) -> Result<Vec<u8>, String> {
    let text = text.trim().to_lowercase();
    match text.as_str() {
        "" | "daily" | "every" | "everyday" | "all" => return Ok(all_days()),
        "weekday" | "weekdays" => return Ok(WEEKDAYS.to_vec()),
        "weekend" | "weekends" => return Ok(WEEKEND.to_vec()),
        _ => {}
    }
    let bad = || format!("Don't know \"{}\". Try daily, weekdays, mon-fri or mon,wed,fri.", text);
    let mut days: Vec<u8> = Vec::new();
    for part in text.split(|c: char| c == ',' || c.is_whitespace()).filter(|p| !p.is_empty()) {
        if let Some((from, to)) = part.split_once('-') {
            let (Some(from), Some(to)) = (day_index(from), day_index(to)) else {
                return Err(bad());
            };
            let mut day = from;
            loop {
                if !days.contains(&day) {
                    days.push(day);
                }
                if day == to {
                    break;
                }
                day = (day + 1) % 7;
            }
        } else {
            let Some(day) = day_index(part) else { return Err(bad()) };
            if !days.contains(&day) {
                days.push(day);
            }
        }
    }
    if days.is_empty() {
        return Err(bad());
    }
    days.sort();
    Ok(days)
}

fn days_text(days: &[u8]) -> String {
    if days.len() == 7 {
        "daily".to_string()
    } else if days == WEEKDAYS.as_slice() {
        "weekdays".to_string()
    } else if days == WEEKEND.as_slice() {
        "weekend".to_string()
    } else {
        days.iter()
            .map(|d| DAY_NAMES[(*d as usize) % 7])
            .collect::<Vec<&str>>()
            .join(",")
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Daily {
    id: u64,
    title: String,
    duration: u32,
    specs: String,
    #[serde(default)]
    start: String,
    #[serde(default)]
    done_dates: Vec<String>,
    #[serde(default = "all_days")]
    days: Vec<u8>,
    #[serde(default)]
    tags: Vec<String>,
}

impl Entry for Daily {
    const FILE: &'static str = "dailies.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn body(&self) -> &str {
        &self.specs
    }
    fn tags(&self) -> &[String] {
        &self.tags
    }
}

impl Daily {
    fn active_on(&self, day: NaiveDate) -> bool {
        self.days.contains(&(day.weekday().num_days_from_monday() as u8))
    }
}

impl Record for Daily {
    const NOUN: &'static str = "daily";
    const TAGGED: bool = true;
    const COLOR: Color = Color::Cyan;

    fn prompts() -> &'static [&'static str] {
        &[
            "Daily  (#tag)",
            "Duration (minutes)",
            "Specifications",
            "Days (daily, weekdays, weekend, mon-fri, mon,wed...)",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            join_text(&self.title, &self.tags, None),
            self.duration.to_string(),
            self.specs.clone(),
            days_text(&self.days),
        ]
    }

    fn build(id: u64, input: &[String], old: Option<&Daily>) -> Result<Daily, String> {
        let (title, tags, _) = split_text(&input[0], false);
        if title.is_empty() {
            return Err("Title can't be empty.".to_string());
        }
        let duration: u32 = input[1]
            .trim()
            .parse()
            .map_err(|_| "Duration must be whole minutes, like 60.".to_string())?;
        if duration == 0 || duration > 24 * 60 {
            return Err("Duration must be between 1 and 1440 minutes.".to_string());
        }
        Ok(Daily {
            id,
            title,
            duration,
            specs: input[2].trim().to_string(),
            start: old.map(|o| o.start.clone()).unwrap_or_default(),
            done_dates: old.map(|o| o.done_dates.clone()).unwrap_or_default(),
            days: parse_days(&input[3])?,
            tags,
        })
    }

    fn label(&self, _lens: u8) -> String {
        let mark = if !self.active_on(today()) {
            "-"
        } else if self.done_dates.contains(&today_text()) {
            "x"
        } else {
            " "
        };
        let at = if self.start.is_empty() {
            String::new()
        } else {
            format!("  @ {}", self.start)
        };
        format!(
            "[{}] {}  {} min{}  {}",
            mark,
            self.title,
            self.duration,
            at,
            days_text(&self.days)
        )
    }

    fn detail(&self, _lens: u8) -> String {
        let on_bar = if self.start.is_empty() {
            "no (s puts it on the day bar)".to_string()
        } else {
            format!("at {}", self.start)
        };
        let specs = if self.specs.is_empty() { "(none)" } else { self.specs.as_str() };
        let mut text = format!(
            "{}\n\nduration: {} min\ndays: {}\nday bar: {}\nspecifications: {}\n",
            self.title,
            self.duration,
            days_text(&self.days),
            on_bar,
            specs
        );
        if !self.tags.is_empty() {
            text.push_str(&format!("tags: {}\n", tags_text(&self.tags)));
        }
        text.push_str(&format!(
            "\ndone today: {}\ndays done in total: {}",
            if self.done_dates.contains(&today_text()) { "yes" } else { "no" },
            self.done_dates.len()
        ));
        text
    }

    fn group(&self, _lens: u8) -> String {
        if self.active_on(today()) {
            "today".to_string()
        } else {
            "other days".to_string()
        }
    }

    fn order(&self, _lens: u8) -> String {
        let rank = if self.active_on(today()) { 0 } else { 1 };
        let start = if self.start.is_empty() { "99:99" } else { self.start.as_str() };
        format!("{}{}{:012}", rank, start, self.id)
    }

    fn extra_actions(_lens: u8) -> Vec<Action> {
        vec![
            Action { key: 'x', label: "done today", needs_item: true, prompts: &[] },
            Action {
                key: 's',
                label: "day bar",
                needs_item: true,
                prompts: &["Start time (HH:MM, empty = take off the bar)"],
            },
        ]
    }

    fn prefill(&self, key: char, _step: usize) -> String {
        if key == 's' {
            self.start.clone()
        } else {
            String::new()
        }
    }

    fn act(&mut self, _lens: u8, key: char, input: &[String]) -> Option<Outcome> {
        match key {
            'x' => {
                if !self.active_on(today()) {
                    return Some(Err(format!("Not planned for today ({}).", days_text(&self.days))));
                }
                let day = today_text();
                let done = match self.done_dates.iter().position(|d| *d == day) {
                    Some(pos) => {
                        self.done_dates.remove(pos);
                        false
                    }
                    None => {
                        self.done_dates.push(day);
                        true
                    }
                };
                Some(Ok(if done {
                    (Kind::Done, "Done for today.".to_string())
                } else {
                    (Kind::Reopened, "Marked as not done today.".to_string())
                }))
            }
            's' => {
                let text = input[0].trim();
                if text.is_empty() {
                    self.start.clear();
                    return Some(Ok((Kind::Edited, "Taken off the day bar.".to_string())));
                }
                match minutes_of(text) {
                    Some(minutes) => {
                        self.start = clock(minutes);
                        Some(Ok((Kind::Edited, format!("On the day bar at {}.", self.start))))
                    }
                    None => Some(Err("Start must look like 19:00.".to_string())),
                }
            }
            _ => None,
        }
    }

    fn blocks(&self, _lens: u8, date: NaiveDate) -> Vec<(u32, u32)> {
        let Some(start) = minutes_of(&self.start) else { return Vec::new() };
        [date.pred_opt(), Some(date)]
            .into_iter()
            .flatten()
            .filter(|from| self.active_on(*from))
            .filter_map(|from| span_on(from, start, self.duration, date))
            .collect()
    }
}

// ===========================================================================
// 9. DICTIONARY pages
// ===========================================================================

#[derive(Clone, Serialize, Deserialize)]
struct Page {
    id: u64,
    title: String,
    text: String,
    #[serde(default)]
    tags: Vec<String>,
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
    fn tags(&self) -> &[String] {
        &self.tags
    }
}

impl Record for Page {
    const NOUN: &'static str = "page";
    const TAGGED: bool = true;

    fn prompts() -> &'static [&'static str] {
        &["Title  (#tag)", "Text"]
    }

    fn fields(&self) -> Vec<String> {
        vec![join_text(&self.title, &self.tags, None), self.text.clone()]
    }

    fn build(id: u64, input: &[String], _old: Option<&Page>) -> Result<Page, String> {
        let (title, tags, _) = split_text(&input[0], false);
        if title.is_empty() {
            return Err("Title can't be empty.".to_string());
        }
        Ok(Page { id, title, text: input[1].clone(), tags })
    }

    fn label(&self, _lens: u8) -> String {
        if self.tags.is_empty() {
            self.title.clone()
        } else {
            format!("{}  {}", self.title, tags_text(&self.tags))
        }
    }

    fn detail(&self, _lens: u8) -> String {
        let mut text = format!("# {}\n\n{}", self.title, self.text);
        if !self.tags.is_empty() {
            text.push_str(&format!("\n\n{}", tags_text(&self.tags)));
        }
        text
    }

    fn order(&self, _lens: u8) -> String {
        self.title.to_lowercase()
    }
}

// ===========================================================================
// 10. CALENDAR EVENTS
// ===========================================================================

#[derive(Clone, Serialize, Deserialize)]
struct CalEvent {
    id: u64,
    title: String,
    date: String,
    start: String,
    end: String,
    #[serde(default)]
    tags: Vec<String>,
}

impl Entry for CalEvent {
    const FILE: &'static str = "events.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn tags(&self) -> &[String] {
        &self.tags
    }
}

impl CalEvent {
    fn start_min(&self) -> u32 {
        minutes_of(&self.start).unwrap_or(0)
    }

    fn end_min(&self) -> u32 {
        minutes_of(&self.end).unwrap_or(0)
    }

    fn overnight(&self) -> bool {
        self.end_min() < self.start_min()
    }

    fn length(&self) -> u32 {
        if self.overnight() {
            1440 - self.start_min() + self.end_min()
        } else {
            self.end_min() - self.start_min()
        }
    }

    fn first_day(&self) -> NaiveDate {
        parse_date(&self.date).unwrap_or_else(today)
    }

    fn last_day(&self) -> NaiveDate {
        let day = self.first_day();
        if self.overnight() {
            day.succ_opt().unwrap_or(day)
        } else {
            day
        }
    }
}

const EVENT_VIEWS: &[&str] = &["upcoming", "past", "all"];

impl Record for CalEvent {
    const NOUN: &'static str = "event";
    const TAGGED: bool = true;

    fn prompts() -> &'static [&'static str] {
        &[
            "Event  (#tag)",
            "Date (YYYY-MM-DD, empty = today)",
            "Start (HH:MM)",
            "End (HH:MM; earlier than start = next day)",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            join_text(&self.title, &self.tags, None),
            self.date.clone(),
            self.start.clone(),
            self.end.clone(),
        ]
    }

    fn build(id: u64, input: &[String], _old: Option<&CalEvent>) -> Result<CalEvent, String> {
        let (title, tags, _) = split_text(&input[0], false);
        if title.is_empty() {
            return Err("Title can't be empty.".to_string());
        }
        let date = if input[1].trim().is_empty() {
            today()
        } else {
            parse_date(&input[1]).ok_or("Date must look like 2026-10-24.")?
        };
        let start = minutes_of(&input[2]).ok_or("Start must look like 09:30.")?;
        let end = minutes_of(&input[3]).ok_or("End must look like 10:30.")?;
        if start == end {
            return Err("Start and end can't be the same.".to_string());
        }
        Ok(CalEvent {
            id,
            title,
            date: date.format(DATE_FMT).to_string(),
            start: clock(start),
            end: clock(end),
            tags,
        })
    }

    fn label(&self, _lens: u8) -> String {
        let tags = if self.tags.is_empty() {
            String::new()
        } else {
            format!("  {}", tags_text(&self.tags))
        };
        format!(
            "{}-{}{}  {}{}",
            self.start,
            self.end,
            if self.overnight() { " (+1)" } else { "" },
            self.title,
            tags
        )
    }

    fn detail(&self, _lens: u8) -> String {
        let mut text = format!(
            "{}\n\ndate: {}\ntime: {} - {}{}\nlength: {} min\n",
            self.title,
            self.date,
            self.start,
            self.end,
            if self.overnight() { " (next day)" } else { "" },
            self.length()
        );
        if !self.tags.is_empty() {
            text.push_str(&format!("tags: {}\n", tags_text(&self.tags)));
        }
        text
    }

    fn views(_lens: u8) -> &'static [&'static str] {
        EVENT_VIEWS
    }

    fn shown(&self, _lens: u8, view: &str) -> bool {
        match view {
            "upcoming" => self.last_day() >= today(),
            "past" => self.last_day() < today(),
            _ => true,
        }
    }

    fn group(&self, _lens: u8) -> String {
        day_heading(self.first_day())
    }

    fn order(&self, _lens: u8) -> String {
        format!("{}{}{:012}", self.date, self.start, self.id)
    }

    fn blocks(&self, _lens: u8, date: NaiveDate) -> Vec<(u32, u32)> {
        span_on(self.first_day(), self.start_min(), self.length(), date)
            .into_iter()
            .collect()
    }
}

// ===========================================================================
// 11. DIARY: with intents, the front door
// ===========================================================================

// What the parser recognised in a diary line. The Curator turns these into
// yes/no proposals; nothing here acts on its own.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
enum Intent {
    Todo { title: String, group: Group },
    Event { title: String, date: String, start: String, end: String },
    HabitCount { title: String, delta: u32 },
    Done { title: String },
}

// Turn one diary line into zero or more intents. Hand-rolled; the keyword is
// case-insensitive but the title keeps its case.
fn parse_intents(text: &str) -> Vec<Intent> {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    let mut out = Vec::new();

    if lower.starts_with("todo:") {
        if let Some(rest) = trimmed.get(5..) {
            let (title, _tags, group) = split_text(rest, true);
            if !title.is_empty() {
                let g = group.as_deref().and_then(Group::parse).unwrap_or(Group::Main);
                out.push(Intent::Todo { title, group: g });
                return out;
            }
        }
    }

    for prefix in ["finished ", "done ", "did "] {
        if lower.starts_with(prefix) {
            if let Some(rest) = trimmed.get(prefix.len()..) {
                let title = rest.trim().to_string();
                if !title.is_empty() {
                    out.push(Intent::Done { title });
                    return out;
                }
            }
        }
    }

    if let Some([title, date, start, end]) = parse_timetable(trimmed) {
        out.push(Intent::Event { title, date, start, end });
        return out;
    }

    // "smoked 2" — exactly two words, second is a number. Narrow on purpose:
    // "I read 5 pages" is not a count.
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() == 2 {
        if let Ok(k) = words[1].parse::<u32>() {
            out.push(Intent::HabitCount { title: words[0].to_string(), delta: k });
            return out;
        }
    }

    out
}

#[derive(Clone, Serialize, Deserialize)]
struct DiaryLine {
    id: u64,
    date: String,
    text: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    intent: Vec<Intent>,
}

impl Entry for DiaryLine {
    const FILE: &'static str = "diary.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.text
    }
    fn body(&self) -> &str {
        &self.date
    }
    fn tags(&self) -> &[String] {
        &self.tags
    }
}

impl Record for DiaryLine {
    const NOUN: &'static str = "diary";
    const LINES: bool = true;

    fn prompts() -> &'static [&'static str] {
        &[]
    }

    fn fields(&self) -> Vec<String> {
        Vec::new()
    }

    fn build(_id: u64, _input: &[String], _old: Option<&DiaryLine>) -> Result<DiaryLine, String> {
        Err("Write in the box at the bottom.".to_string())
    }

    fn from_line(id: u64, text: &str, at: &str) -> Option<DiaryLine> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let date = if at.trim().is_empty() {
            Local::now().format("%Y-%m-%d %H:%M").to_string()
        } else {
            at.trim().to_string()
        };
        Some(DiaryLine {
            id,
            date,
            text: text.to_string(),
            tags: hashtags(text),
            intent: parse_intents(text),
        })
    }

    fn label(&self, _lens: u8) -> String {
        format!("{}  {}", self.date.get(11..).unwrap_or(""), self.text)
    }

    fn detail(&self, _lens: u8) -> String {
        let hints = if self.intent.is_empty() {
            String::new()
        } else {
            format!("\n\nintents the AI can act on:\n{:?}", self.intent)
        };
        format!("{}\n\n{}{}", self.date, self.text, hints)
    }

    fn group(&self, _lens: u8) -> String {
        self.date.get(..10).and_then(parse_date).map(day_heading).unwrap_or_default()
    }

    fn order(&self, _lens: u8) -> String {
        format!("{}{:012}", self.date, self.id)
    }

    fn keys(_lens: u8) -> &'static str {
        ""
    }
}

// ===========================================================================
// 12. CALENDAR
// ===========================================================================

struct CalendarModule {
    events: Shared<CalEvent>,
    month_offset: i32,
}

impl CalendarModule {
    fn new(events: &Shared<CalEvent>) -> Self {
        CalendarModule { events: Rc::clone(events), month_offset: 0 }
    }

    fn first_of_month(&self) -> NaiveDate {
        let first = today().with_day(1).expect("every month has a day 1");
        let shifted = if self.month_offset >= 0 {
            first.checked_add_months(Months::new(self.month_offset as u32))
        } else {
            first.checked_sub_months(Months::new(self.month_offset.unsigned_abs()))
        };
        shifted.unwrap_or(first)
    }

    fn days_in_month(first: NaiveDate) -> u32 {
        match first.checked_add_months(Months::new(1)) {
            Some(next) => (next - first).num_days() as u32,
            None => 31,
        }
    }

    fn events_on(&self, date: NaiveDate) -> Vec<CalEvent> {
        let store = self.events.borrow();
        let mut list: Vec<CalEvent> = store
            .items
            .iter()
            .filter(|e| e.first_day() == date || (e.overnight() && e.last_day() == date))
            .cloned()
            .collect();
        list.sort_by(|a, b| a.start.cmp(&b.start));
        list
    }
}

impl Module for CalendarModule {
    fn name(&self) -> &'static str {
        "calendar"
    }

    fn heading(&self) -> String {
        format!("calendar · {}", self.first_of_month().format("%B %Y"))
    }

    fn shows_day_bar(&self) -> bool {
        true
    }

    fn bar_date(&self, selected: Option<u64>) -> Option<NaiveDate> {
        selected.and_then(|day| self.first_of_month().with_day(day as u32))
    }

    fn day_blocks(&self, date: NaiveDate) -> Vec<(u32, u32)> {
        let store = self.events.borrow();
        let blocks: Vec<(u32, u32)> = store
            .items
            .iter()
            .flat_map(|e| e.blocks(0, date))
            .collect();
        blocks
    }

    fn initial_row(&self) -> usize {
        today().day0() as usize
    }

    fn rows(&self) -> Vec<Row> {
        let first = self.first_of_month();
        let now = today();
        (1..=Self::days_in_month(first))
            .map(|d| {
                let date = first.with_day(d).unwrap_or(first);
                let count = self.events_on(date).len();
                let mut label = format!("{} {:>2}", date.format("%a"), d);
                if date == now {
                    label.push_str("  (today)");
                }
                if count > 0 {
                    label.push_str(&format!("  {}", "*".repeat(count.min(5))));
                }
                Row::new(d as u64, label)
            })
            .collect()
    }

    fn detail(&self, id: u64) -> String {
        let first = self.first_of_month();
        let Some(date) = first.with_day(id as u32) else { return String::new() };
        let now = today();

        let mut out = format!("{}\n\n", first.format("%B %Y"));
        out.push_str(" Mo  Tu  We  Th  Fr  Sa  Su \n");
        let offset = first.weekday().num_days_from_monday();
        for _ in 0..offset {
            out.push_str("    ");
        }
        let mut col = offset;
        for d in 1..=Self::days_in_month(first) {
            let day_date = first.with_day(d).unwrap_or(first);
            let (left, right) = if d == id as u32 {
                ('[', ']')
            } else if day_date == now {
                ('(', ')')
            } else if !self.events_on(day_date).is_empty() {
                (' ', '*')
            } else {
                (' ', ' ')
            };
            out.push_str(&format!("{}{:>2}{}", left, d, right));
            col += 1;
            if col % 7 == 0 {
                out.push('\n');
            }
        }
        out.push_str("\n\n");

        out.push_str(&format!("{}\n", date.format("%A %d %B")));
        let events = self.events_on(date);
        if events.is_empty() {
            out.push_str("(nothing planned)\n");
        }
        for e in events {
            let note = if e.first_day() != date { "  (from the evening before)" } else { "" };
            out.push_str(&format!("{}-{}  {}{}\n", e.start, e.end, e.title, note));
        }
        out
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action {
                key: 'a',
                label: "add event",
                needs_item: true,
                prompts: &["Event  (#tag)", "Start (HH:MM)", "End (HH:MM; earlier = next day)"],
            },
            Action { key: 'n', label: "next month", needs_item: false, prompts: &[] },
            Action { key: 'p', label: "prev month", needs_item: false, prompts: &[] },
            Action { key: 't', label: "this month", needs_item: false, prompts: &[] },
        ]
    }

    fn apply(&mut self, action: char, id: Option<u64>, input: &[String], log: &mut Logger)
        -> String {
        match action {
            'n' | 'p' | 't' => {
                match action {
                    'n' => self.month_offset += 1,
                    'p' => self.month_offset -= 1,
                    _ => self.month_offset = 0,
                }
                format!("Showing {}.", self.first_of_month().format("%B %Y"))
            }
            'a' => {
                let Some(day) = id else { return "Nothing selected.".to_string() };
                let Some(date) = self.first_of_month().with_day(day as u32) else {
                    return "No such day.".to_string();
                };
                let typed = [
                    input[0].clone(),
                    date.format(DATE_FMT).to_string(),
                    input[1].clone(),
                    input[2].clone(),
                ];
                let mut store = self.events.borrow_mut();
                let new_id = store.next_id();
                let event = match CalEvent::build(new_id, &typed, None) {
                    Ok(event) => event,
                    Err(message) => return message,
                };
                let title = event.title.clone();
                store.items.push(event);
                store.save();
                log.log(CalEvent::NOUN, Kind::Added, new_id, &title);
                "Event added.".to_string()
            }
            _ => String::new(),
        }
    }
}

// ===========================================================================
// 13. THE AI: two roles, both local, both ask-first
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Verdict {
    Pending,
    Yes,
    No,
}

#[derive(Clone, Serialize, Deserialize)]
struct Proposal {
    id: u64,
    key: String,
    ask: String,
    why: String,
    module: String,
    action: char,
    item: Option<u64>,
    input: Vec<String>,
    status: Verdict,
    made: String,
    #[serde(default)]
    decided: String,
}

impl Entry for Proposal {
    const FILE: &'static str = "proposals.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.ask
    }
    fn body(&self) -> &str {
        &self.why
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct ChatMsg {
    id: u64,
    you: String,
    companion: String,
    when: String,
}

impl Entry for ChatMsg {
    const FILE: &'static str = "chat.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.you
    }
    fn body(&self) -> &str {
        &self.companion
    }
}

struct Idea {
    key: String,
    ask: String,
    why: String,
    module: &'static str,
    action: char,
    item: Option<u64>,
    input: Vec<String>,
}

trait Advisor {
    fn advise(
        &self,
        todos: &[Todo],
        habits: &[Habit],
        diary: &[DiaryLine],
        now: NaiveDateTime,
    ) -> Vec<Idea>;
}

// The narrative half. Read-only over the diary.
trait Companion {
    fn reflect(&self, diary: &[DiaryLine], question: &str) -> String;
    fn summarize_day(&self, diary: &[DiaryLine], date: NaiveDate) -> Option<String>;
}

// ---- Curator: rules today, a local model tomorrow -------------------------

struct Curator;

impl Advisor for Curator {
    fn advise(
        &self,
        todos: &[Todo],
        habits: &[Habit],
        diary: &[DiaryLine],
        now: NaiveDateTime,
    ) -> Vec<Idea> {
        let mut ideas = Vec::new();

        for todo in todos.iter().filter(|t| !t.is_done) {
            if let Some(end) = todo.slot_end() {
                if end <= now {
                    ideas.push(Idea {
                        key: format!("cube-ended:{}:{}", todo.id, todo.slot),
                        ask: format!("Mark \"{}\" as done?", todo.header),
                        why: format!("Its timeline slot ended at {}.", end.format("%Y-%m-%d %H:%M")),
                        module: "todo",
                        action: 'x',
                        item: Some(todo.id),
                        input: Vec::new(),
                    });
                }
            }
            if let Some(days) = todo.age_days() {
                if todo.group == Group::Main && days >= 30 {
                    ideas.push(Idea {
                        key: format!("stale:{}", todo.id),
                        ask: format!("Move \"{}\" to secondary?", todo.header),
                        why: format!("It has been on the main list for {} days.", days),
                        module: "todo",
                        action: 'g',
                        item: Some(todo.id),
                        input: vec!["secondary".to_string()],
                    });
                }
            }
        }

        for line in diary {
            for (i, intent) in line.intent.iter().enumerate() {
                match intent {
                    Intent::Todo { title, group } => {
                        let suffix = if *group == Group::Main {
                            String::new()
                        } else {
                            format!(" @{}", group.word())
                        };
                        ideas.push(Idea {
                            key: format!("diary-todo:{}:{}", line.id, i),
                            ask: format!("Add \"{}\" to your {} todos?", title, group.word()),
                            why: format!("You wrote: \"{}\"", line.text),
                            module: "todo",
                            action: 'a',
                            item: None,
                            input: vec![format!("{}{}", title, suffix)],
                        });
                    }
                    Intent::Event { title, date, start, end } => {
                        ideas.push(Idea {
                            key: format!("diary-event:{}:{}", line.id, i),
                            ask: format!("Put \"{}\" {}–{} in the agenda?", title, start, end),
                            why: format!("You wrote: \"{}\"", line.text),
                            module: "agenda",
                            action: 'a',
                            item: None,
                            input: vec![title.clone(), date.clone(), start.clone(), end.clone()],
                        });
                    }
                    Intent::HabitCount { title, delta } => {
                        let lower = title.to_lowercase();
                        if let Some(habit) = habits
                            .iter()
                            .find(|h| h.title.to_lowercase().starts_with(&lower))
                        {
                            for k in 1..=*delta {
                                ideas.push(Idea {
                                    key: format!("diary-habit:{}:{}:{}", line.id, habit.id, k),
                                    ask: format!("Count {} on \"{}\"?", k, habit.title),
                                    why: format!("You wrote: \"{}\"", line.text),
                                    module: "habit",
                                    action: '+',
                                    item: Some(habit.id),
                                    input: Vec::new(),
                                });
                            }
                        }
                    }
                    Intent::Done { title } => {
                        let lower = title.to_lowercase();
                        if let Some(todo) = todos
                            .iter()
                            .find(|t| !t.is_done && t.header.to_lowercase() == lower)
                        {
                            ideas.push(Idea {
                                key: format!("diary-done:{}:{}", line.id, todo.id),
                                ask: format!("Mark \"{}\" as done?", todo.header),
                                why: format!("You wrote: \"{}\"", line.text),
                                module: "todo",
                                action: 'x',
                                item: Some(todo.id),
                                input: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
        ideas
    }
}

// ---- Companion: a stub you replace with a real local model ---------------

struct LocalModelCompanion;

impl Companion for LocalModelCompanion {
    fn reflect(&self, diary: &[DiaryLine], question: &str) -> String {
        let recent: Vec<&str> = diary.iter().rev().take(5).map(|d| d.text.as_str()).collect();
        format!(
            "(no local model connected yet)\n\
             you asked: {question}\n\
             last {} diary lines:\n  - {}\n\n\
             plug a llama.cpp / ollama handle into Companion::reflect and this becomes a real answer.",
            recent.len(),
            recent.join("\n  - ")
        )
    }

    fn summarize_day(&self, _diary: &[DiaryLine], _date: NaiveDate) -> Option<String> {
        None
    }
}

// ---- The AI tab ----------------------------------------------------------

struct AiModule {
    store: Store<Proposal>,
    chat: Store<ChatMsg>,
    todos: Shared<Todo>,
    habits: Shared<Habit>,
    diary: Shared<DiaryLine>,
    curator: Vec<Box<dyn Advisor>>,
    companion: Box<dyn Companion>,
    chat_view: bool,
    last_summary: String,
    out: Vec<Request>,
}

impl AiModule {
    fn new(todos: &Shared<Todo>, diary: &Shared<DiaryLine>, habits: &Shared<Habit>) -> Self {
        let last_summary = fs::read_to_string(data_path("last_summary.txt"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        AiModule {
            store: Store::load(),
            chat: Store::load(),
            todos: Rc::clone(todos),
            habits: Rc::clone(habits),
            diary: Rc::clone(diary),
            curator: vec![Box::new(Curator)],
            companion: Box::new(LocalModelCompanion),
            chat_view: false,
            last_summary,
            out: Vec::new(),
        }
    }

    fn think(&mut self) -> usize {
        let now = Local::now().naive_local();
        let mut ideas: Vec<Idea> = Vec::new();
        {
            let todos = self.todos.borrow();
            let habits = self.habits.borrow();
            let diary = self.diary.borrow();
            for advisor in &self.curator {
                ideas.extend(advisor.advise(&todos.items, &habits.items, &diary.items, now));
            }
        }
        let mut added = 0;
        for idea in ideas {
            if self.store.items.iter().any(|p| p.key == idea.key) {
                continue;
            }
            let id = self.store.next_id();
            self.store.items.push(Proposal {
                id,
                key: idea.key,
                ask: idea.ask,
                why: idea.why,
                module: idea.module.to_string(),
                action: idea.action,
                item: idea.item,
                input: idea.input,
                status: Verdict::Pending,
                made: Local::now().format("%Y-%m-%d %H:%M").to_string(),
                decided: String::new(),
            });
            added += 1;
        }
        if added > 0 {
            self.store.save();
        }
        added
    }

    fn summarize_if_needed(&mut self) {
        let today_str = today_text();
        if self.last_summary == today_str {
            return;
        }
        let summary = {
            let diary = self.diary.borrow();
            self.companion.summarize_day(&diary.items, today())
        };
        if let Some(summary) = summary {
            let mut store = self.diary.borrow_mut();
            let id = store.next_id();
            if let Some(line) = DiaryLine::from_line(id, &format!("#companion {summary}"), "") {
                store.items.push(line);
                store.save();
            }
            drop(store);
            self.last_summary = today_str.clone();
            let _ = fs::write(data_path("last_summary.txt"), &today_str);
        }
    }
}

impl Module for AiModule {
    fn name(&self) -> &'static str {
        "ai"
    }

    fn heading(&self) -> String {
        if self.chat_view {
            return format!("ai · companion chat · {} messages", self.chat.items.len());
        }
        let waiting = self.store.items.iter().filter(|p| p.status == Verdict::Pending).count();
        format!("ai · {} waiting for you", waiting)
    }

    fn rows(&self) -> Vec<Row> {
        if self.chat_view {
            return self
                .chat
                .items
                .iter()
                .map(|c| Row::new(c.id, format!("you: {}", truncate(&c.you, 60))))
                .collect();
        }
        let mut list: Vec<(String, Row)> = self
            .store
            .items
            .iter()
            .map(|p| {
                let (group, mark, order) = match p.status {
                    Verdict::Pending => ("waiting for your answer", "?", format!("0{:012}", p.id)),
                    Verdict::Yes => (
                        "answered",
                        "yes",
                        format!("1{}", desc_key(&format!("{}{:012}", p.decided, p.id))),
                    ),
                    Verdict::No => (
                        "answered",
                        "no",
                        format!("1{}", desc_key(&format!("{}{:012}", p.decided, p.id))),
                    ),
                };
                (
                    order,
                    Row {
                        id: p.id,
                        label: format!("[{mark}] {}", p.ask),
                        group: group.to_string(),
                    },
                )
            })
            .collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list.into_iter().map(|(_, r)| r).collect()
    }

    fn detail(&self, id: u64) -> String {
        if self.chat_view {
            let Some(msg) = self.chat.items.iter().find(|c| c.id == id) else {
                return String::new();
            };
            return format!("{}\n\nyou: {}\n\ncompanion: {}\n", msg.when, msg.you, msg.companion);
        }
        let Some(p) = self.store.get(id) else { return String::new() };
        let status = match p.status {
            Verdict::Pending => "waiting for your answer".to_string(),
            Verdict::Yes => format!("you said yes ({})", p.decided),
            Verdict::No => format!("you said no ({})", p.decided),
        };
        let typed = if p.input.is_empty() {
            String::new()
        } else {
            format!(" with \"{}\"", p.input.join("\", \""))
        };
        format!(
            "{}\n\nwhy: {}\n\nif you say yes it presses '{}' on the {} tab for item {}{}\n\nasked: {}\nstatus: {}",
            p.ask,
            p.why,
            p.action,
            p.module,
            p.item.map_or("-".to_string(), |i| i.to_string()),
            typed,
            p.made,
            status
        )
    }

    fn actions(&self) -> Vec<Action> {
        if self.chat_view {
            vec![
                Action {
                    key: 'g',
                    label: "send",
                    needs_item: false,
                    prompts: &["Message to the companion"],
                },
                Action { key: 'c', label: "questions", needs_item: false, prompts: &[] },
            ]
        } else {
            vec![
                Action { key: 'y', label: "yes", needs_item: true, prompts: &[] },
                Action { key: 'n', label: "no", needs_item: true, prompts: &[] },
                Action { key: 'r', label: "think now", needs_item: false, prompts: &[] },
                Action { key: 'c', label: "companion chat", needs_item: false, prompts: &[] },
            ]
        }
    }

    fn apply(&mut self, key: char, id: Option<u64>, input: &[String], log: &mut Logger) -> String {
        if key == 'c' {
            self.chat_view = !self.chat_view;
            return if self.chat_view { "companion chat".to_string() } else { "questions".to_string() };
        }

        if key == 'g' && self.chat_view {
            let question = input.first().map(|s| s.trim().to_string()).unwrap_or_default();
            if question.is_empty() {
                return "Nothing to send.".to_string();
            }
            let reply = {
                let diary = self.diary.borrow();
                self.companion.reflect(&diary.items, &question)
            };
            let id = self.chat.next_id();
            self.chat.items.push(ChatMsg {
                id,
                you: question,
                companion: reply,
                when: Local::now().format("%Y-%m-%d %H:%M").to_string(),
            });
            self.chat.save();
            return "Sent.".to_string();
        }

        if key == 'r' && !self.chat_view {
            let added = self.think();
            return if added == 0 {
                "Nothing new to suggest.".to_string()
            } else {
                format!("{added} new question(s).")
            };
        }

        if (key == 'y' || key == 'n') && !self.chat_view {
            let Some(id) = id else { return "Nothing selected.".to_string() };
            let (module, action, item, input_, title) = {
                let Some(p) = self.store.get_mut(id) else {
                    return "No such question.".to_string();
                };
                if p.status != Verdict::Pending {
                    return "Already answered.".to_string();
                }
                p.status = if key == 'y' { Verdict::Yes } else { Verdict::No };
                p.decided = Local::now().format("%Y-%m-%d %H:%M").to_string();
                (p.module.clone(), p.action, p.item, p.input.clone(), p.ask.clone())
            };
            self.store.save();
            if key == 'y' {
                self.out.push(Request { module, key: action, item, input: input_ });
                log.log("ai", Kind::Approved, id, &title);
                format!("Yes: {title}")
            } else {
                log.log("ai", Kind::Rejected, id, &title);
                format!("No: {title} (won't be asked again)")
            }
        } else {
            String::new()
        }
    }

    fn tick(&mut self, _log: &mut Logger) {
        self.think();
        self.summarize_if_needed();
    }

    fn outbox(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.out)
    }
}

// ===========================================================================
// 13b. SETTINGS: toggle which modules are visible on the next start
// ===========================================================================

const ALL_MODULES: &[(&str, &str)] = &[
    ("todo",     "the main list, grouped and calm"),
    ("done",     "finished todos, newest first"),
    ("cubes",    "open todos as sized timeline blocks"),
    ("daily",    "recurring things on chosen weekdays"),
    ("habit",    "habits to build and to quit"),
    ("agenda",   "events with times, past and upcoming"),
    ("calendar", "a month grid; day bar for the selected day"),
    ("dict",     "your own dictionary of pages"),
];

const ALWAYS_ON: &[&str] = &["diary", "ai", "settings"];

fn default_enabled() -> Vec<String> {
    ALL_MODULES.iter().map(|(n, _)| n.to_string()).collect()
}

#[derive(Clone, Serialize, Deserialize)]
struct Settings {
    #[serde(default = "default_enabled")]
    enabled: Vec<String>,
}

impl Settings {
    fn load() -> Self {
        fs::read_to_string(data_path("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| Settings { enabled: default_enabled() })
    }
    fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(data_path("settings.json"), text);
        }
    }
}

struct SettingsModule {
    settings: Settings,
}

impl SettingsModule {
    fn new(settings: Settings) -> Self {
        SettingsModule { settings }
    }
}

impl Module for SettingsModule {
    fn name(&self) -> &'static str {
        "settings"
    }

    fn heading(&self) -> String {
        let on = ALL_MODULES
            .iter()
            .filter(|(n, _)| self.settings.enabled.iter().any(|e| e == n))
            .count();
        format!("settings · {on}/{} on · changes apply on restart", ALL_MODULES.len())
    }

    fn rows(&self) -> Vec<Row> {
        ALL_MODULES
            .iter()
            .enumerate()
            .map(|(i, (name, about))| {
                let on = self.settings.enabled.iter().any(|e| e == name);
                let mark = if ALWAYS_ON.contains(name) {
                    "·"
                } else if on {
                    "x"
                } else {
                    " "
                };
                Row::new(i as u64, format!("[{mark}] {name:<10} {about}"))
            })
            .collect()
    }

    fn detail(&self, id: u64) -> String {
        let Some((name, about)) = ALL_MODULES.get(id as usize) else {
            return String::new();
        };
        let on = self.settings.enabled.iter().any(|e| e == name);
        let always = ALWAYS_ON.contains(name);
        let state = if always { "always on" } else if on { "on" } else { "off" };
        let action = if always {
            "this tab can't be turned off — the diary is the input and the AI reads it"
        } else {
            "press 't' to toggle · restart the app to apply"
        };
        format!("{name}\n\n{about}\n\nstate: {state}\n\n{action}")
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action { key: 't', label: "toggle", needs_item: true, prompts: &[] },
            Action { key: 'o', label: "all on", needs_item: false, prompts: &[] },
            Action { key: 'f', label: "all off", needs_item: false, prompts: &[] },
        ]
    }

    fn apply(&mut self, key: char, id: Option<u64>, _input: &[String], _log: &mut Logger) -> String {
        match key {
            't' => {
                let Some(id) = id else { return "Nothing selected.".to_string() };
                let Some((name, _)) = ALL_MODULES.get(id as usize) else {
                    return "No such module.".to_string();
                };
                if ALWAYS_ON.contains(name) {
                    return format!("{name} is always on.");
                }
                let name = name.to_string();
                match self.settings.enabled.iter().position(|e| e == &name) {
                    Some(p) => {
                        self.settings.enabled.remove(p);
                    }
                    None => {
                        self.settings.enabled.push(name.clone());
                    }
                }
                self.settings.save();
                format!("{name} toggled (restart to apply).")
            }
            'o' => {
                self.settings.enabled = default_enabled();
                self.settings.save();
                "All modules on (restart to apply).".to_string()
            }
            'f' => {
                self.settings.enabled.clear();
                self.settings.save();
                "All optional modules off (restart to apply).".to_string()
            }
            _ => String::new(),
        }
    }
}

// ===========================================================================
// 14. THE RATATUI INTERFACE
// ===========================================================================

struct SearchHit {
    score: i32,
    module: usize,
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
}

struct App {
    modules: Vec<Box<dyn Module>>,
    log: Logger,
    tab: usize,
    selected: Vec<usize>,
    search_query: String,
    search_hits: Vec<SearchHit>,
    search_sel: usize,
    mode: Mode,
    status: String,
    line_buffer: String,
    quit: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct InboxLine {
    module: String,
    text: String,
    #[serde(default)]
    at: String,
    #[serde(default)]
    why: String,
}

fn keep_rejected(entry: &InboxLine) {
    let Ok(line) = serde_json::to_string(entry) else { return };
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_path("inbox.rejected.jsonl"))
    {
        let _ = writeln!(file, "{}", line);
    }
}

fn parse_timetable(text: &str) -> Option<[String; 4]> {
    let mut words = text.split_whitespace().peekable();
    let mut date = String::new();
    if let Some(first) = words.peek() {
        if parse_date(first).is_some() {
            date = first.to_string();
            words.next();
        }
    }
    let (start, end) = words.next()?.split_once('-')?;
    minutes_of(start)?;
    minutes_of(end)?;
    let title = words.collect::<Vec<&str>>().join(" ");
    if title.is_empty() {
        return None;
    }
    Some([title, date, start.to_string(), end.to_string()])
}

fn tail(text: &str, room: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    chars[chars.len().saturating_sub(room)..].iter().collect()
}

impl App {
    fn new() -> Self {
        let todos: Shared<Todo> = share();
        let dailies: Shared<Daily> = share();
        let habits: Shared<Habit> = share();
        let events: Shared<CalEvent> = share();
        let pages: Shared<Page> = share();
        let diary: Shared<DiaryLine> = share();

        let settings = Settings::load();
        let enabled = settings.enabled.clone();

        let all: Vec<(&'static str, Box<dyn Module>)> = vec![
            ("todo", Box::new(ListModule::new("todo", 0, &todos).with_bar())),
            ("done", Box::new(ListModule::new("done", 1, &todos).with_bar())),
            ("cubes", Box::new(ListModule::new("cubes", 2, &todos).with_bar())),
            ("daily", Box::new(ListModule::new("daily", 0, &dailies).with_bar())),
            ("habit", Box::new(ListModule::new("habit", 0, &habits))),
            ("agenda", Box::new(ListModule::new("agenda", 0, &events).with_bar())),
            ("calendar", Box::new(CalendarModule::new(&events))),
            ("diary", Box::new(ListModule::new("diary", 0, &diary))),
            ("dict", Box::new(ListModule::new("dict", 0, &pages))),
            ("ai", Box::new(AiModule::new(&todos, &diary, &habits))),
        ];

        let mut modules: Vec<Box<dyn Module>> = Vec::new();
        for (name, module) in all {
            if ALWAYS_ON.contains(&name) || enabled.iter().any(|e| e.as_str() == name) {
                modules.push(module);
            }
        }
        // Settings is always last, so enabling a new module doesn't shift tabs
        // that already exist in the log.
        modules.push(Box::new(SettingsModule::new(settings)));

        let selected: Vec<usize> = modules.iter().map(|m| m.initial_row()).collect();
        App {
            modules,
            log: Logger::open(),
            tab: 0,
            selected,
            search_query: String::new(),
            search_hits: Vec::new(),
            search_sel: 0,
            mode: Mode::Normal,
            status: String::new(),
            line_buffer: String::new(),
            quit: false,
        }
    }

    fn catch_up(&mut self) {
        let last = fs::read_to_string(data_path("last_seen.txt"))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if let Ok(when) = NaiveDateTime::parse_from_str(&last, "%Y-%m-%d %H:%M") {
            let gap = Local::now().naive_local() - when;
            if gap.num_hours() >= 1 {
                self.status = format!(
                    "Welcome back. Away {}h {}m.",
                    gap.num_hours(),
                    gap.num_minutes() % 60
                );
            }
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        self.catch_up();
        self.tick_all();
        self.check_inbox();
        let mut last_tick = Instant::now();
        let mut last_inbox = Instant::now();
        while !self.quit {
            terminal.draw(|f| self.draw(f))?;
            let key_waiting = event::poll(Duration::from_secs(2))?;
            if last_inbox.elapsed() >= Duration::from_secs(2) {
                self.check_inbox();
                last_inbox = Instant::now();
            }
            if last_tick.elapsed() >= Duration::from_secs(30) {
                self.tick_all();
                last_tick = Instant::now();
            }
            if !key_waiting {
                continue;
            }
            if let TermEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                self.on_key(key.code);
            }
        }
        let _ = fs::write(
            data_path("last_seen.txt"),
            Local::now().format("%Y-%m-%d %H:%M").to_string(),
        );
        Ok(())
    }

    fn check_inbox(&mut self) {
        let inbox = data_path("inbox.jsonl");
        let working = data_path("inbox.processing");
        if !working.exists() {
            if !inbox.exists() || fs::rename(&inbox, &working).is_err() {
                return;
            }
        }
        let Ok(text) = fs::read_to_string(&working) else { return };

        let (mut added, mut failed) = (0, 0);
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let result = match serde_json::from_str::<InboxLine>(line) {
                Ok(entry) => self.take_inbox_entry(&entry).map_err(|why| (entry, why)),
                Err(_) => Err((
                    InboxLine {
                        module: String::new(),
                        text: line.to_string(),
                        at: String::new(),
                        why: String::new(),
                    },
                    "not a valid inbox line".to_string(),
                )),
            };
            match result {
                Ok(()) => added += 1,
                Err((mut entry, why)) => {
                    failed += 1;
                    entry.why = why;
                    keep_rejected(&entry);
                }
            }
        }
        let _ = fs::remove_file(&working);

        self.status = if failed == 0 {
            format!("Inbox: {} new from the browser.", added)
        } else {
            format!(
                "Inbox: {} new, {} refused (kept in inbox.rejected.jsonl).",
                added, failed
            )
        };
        self.clamp_selection();
    }

    fn take_inbox_entry(&mut self, entry: &InboxLine) -> Result<(), String> {
        let text = entry.text.trim();
        if text.is_empty() {
            return Err("empty text".to_string());
        }
        match entry.module.as_str() {
            "diary" => self.add_diary_line(text, &entry.at),
            "todo" => self.add_to("todo", vec![text.to_string()]),
            "timetable" => match parse_timetable(text) {
                Some(fields) => self.add_to("agenda", fields.to_vec()),
                None => self.add_diary_line(&format!("timetable: {} #timetable", text), &entry.at),
            },
            other => Err(format!("unknown module \"{}\"", other)),
        }
    }

    fn add_diary_line(&mut self, text: &str, at: &str) -> Result<(), String> {
        let index = self
            .modules
            .iter()
            .position(|m| m.takes_lines())
            .ok_or("there is no diary tab")?;
        self.modules[index]
            .line_input(text, at, &mut self.log)
            .map(|_| ())
            .ok_or_else(|| "the diary refused the line".to_string())
    }

    fn add_to(&mut self, module: &str, input: Vec<String>) -> Result<(), String> {
        let index = self
            .modules
            .iter()
            .position(|m| m.name() == module)
            .ok_or_else(|| format!("there is no {} tab", module))?;
        self.modules[index].add(&input, &mut self.log).map(|_| ())
    }

    fn tick_all(&mut self) {
        for module in self.modules.iter_mut() {
            module.tick(&mut self.log);
        }
        self.flush();
        self.clamp_selection();
    }

    fn flush(&mut self) {
        for _ in 0..4 {
            let mut requests: Vec<Request> = Vec::new();
            for module in self.modules.iter_mut() {
                requests.extend(module.outbox());
            }
            if requests.is_empty() {
                break;
            }
            for request in requests {
                let target = self
                    .modules
                    .iter()
                    .position(|m| m.name() == request.module.as_str());
                let Some(index) = target else { continue };
                let result = self.modules[index].apply(
                    request.key,
                    request.item,
                    &request.input,
                    &mut self.log,
                );
                if !result.is_empty() {
                    self.status = format!("{}  →  {}", self.status, result);
                }
            }
        }
    }

    fn is_search_tab(&self) -> bool {
        self.tab == self.modules.len()
    }

    fn clamp_selection(&mut self) {
        let Some(module) = self.modules.get(self.tab) else { return };
        let len = module.rows().len();
        let sel = &mut self.selected[self.tab];
        if len == 0 {
            *sel = 0;
        } else if *sel >= len {
            *sel = len - 1;
        }
    }

    fn selected_id(&self) -> Option<u64> {
        let module = self.modules.get(self.tab)?;
        let rows = module.rows();
        let id = rows.get(self.selected[self.tab]).map(|r| r.id);
        id
    }

    fn run_search(&self, query: &str) -> Vec<SearchHit> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for (mi, module) in self.modules.iter().enumerate() {
            for (score, row) in module.search(query) {
                hits.push(SearchHit { score, module: mi, id: row.id, label: row.label });
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score));
        hits
    }

    fn go_to_tab(&mut self, tab: usize) {
        self.tab = tab;
        self.status.clear();
        if self.is_search_tab() {
            self.search_hits = self.run_search(&self.search_query);
            self.search_sel = 0;
        } else {
            self.clamp_selection();
        }
    }

    fn on_key(&mut self, code: KeyCode) {
        let mode = std::mem::replace(&mut self.mode, Mode::Normal);
        match mode {
            Mode::Normal => self.key_normal(code),
            Mode::Input { module, action, item, step, answers, buffer } => {
                self.key_input(code, module, action, item, step, answers, buffer)
            }
        }
    }

    fn key_normal(&mut self, code: KeyCode) {
        if self.is_search_tab() {
            self.key_search_tab(code);
            return;
        }
        if self.modules[self.tab].takes_lines() {
            self.key_lines(code);
            return;
        }
        let count = self.modules.len() + 1;
        match code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Tab | KeyCode::Right => self.go_to_tab((self.tab + 1) % count),
            KeyCode::BackTab | KeyCode::Left => self.go_to_tab((self.tab + count - 1) % count),
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected[self.tab] += 1;
                self.clamp_selection();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected[self.tab] = self.selected[self.tab].saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.selected[self.tab] += 10;
                self.clamp_selection();
            }
            KeyCode::PageUp => {
                self.selected[self.tab] = self.selected[self.tab].saturating_sub(10);
            }
            KeyCode::Home => self.selected[self.tab] = 0,
            KeyCode::End => {
                self.selected[self.tab] = usize::MAX;
                self.clamp_selection();
            }
            KeyCode::Char('/') => self.go_to_tab(self.modules.len()),
            KeyCode::Char(c) => {
                let action = self.modules[self.tab].actions().into_iter().find(|a| a.key == c);
                if let Some(action) = action {
                    self.start_action(action);
                }
            }
            _ => {}
        }
    }

    fn key_lines(&mut self, code: KeyCode) {
        let count = self.modules.len() + 1;
        match code {
            KeyCode::Tab | KeyCode::Right => self.go_to_tab((self.tab + 1) % count),
            KeyCode::BackTab | KeyCode::Left => self.go_to_tab((self.tab + count - 1) % count),
            KeyCode::Esc => self.line_buffer.clear(),
            KeyCode::Enter => {
                let text = std::mem::take(&mut self.line_buffer);
                if let Some(message) = self.modules[self.tab].line_input(&text, "", &mut self.log) {
                    self.status = message;
                }
            }
            KeyCode::Backspace => {
                self.line_buffer.pop();
            }
            KeyCode::Char(c) => self.line_buffer.push(c),
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
        self.flush();
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
                return;
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

    fn key_search_tab(&mut self, code: KeyCode) {
        let count = self.modules.len() + 1;
        match code {
            KeyCode::Tab | KeyCode::Right => self.go_to_tab((self.tab + 1) % count),
            KeyCode::BackTab | KeyCode::Left => self.go_to_tab((self.tab + count - 1) % count),
            KeyCode::Esc => self.go_to_tab(0),
            KeyCode::Enter => {
                if let Some(hit) = self.search_hits.get(self.search_sel) {
                    let (module, id) = (hit.module, hit.id);
                    self.go_to_tab(module);
                    self.modules[module].reveal(id);
                    let rows = self.modules[module].rows();
                    if let Some(pos) = rows.iter().position(|r| r.id == id) {
                        self.selected[module] = pos;
                    }
                }
            }
            KeyCode::Down => {
                if self.search_sel + 1 < self.search_hits.len() {
                    self.search_sel += 1;
                }
            }
            KeyCode::Up => self.search_sel = self.search_sel.saturating_sub(1),
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.search_hits = self.run_search(&self.search_query);
                self.search_sel = 0;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.search_hits = self.run_search(&self.search_query);
                self.search_sel = 0;
            }
            _ => {}
        }
    }

    fn draw(&self, f: &mut Frame) {
        let show_bar = self.modules.get(self.tab).map_or(false, |m| m.shows_day_bar());
        let bar_height = if show_bar { 7 } else { 0 };
        let [top, body, daybar, bottom] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(bar_height),
            Constraint::Length(3),
        ])
        .areas(f.area());

        let mut titles: Vec<&str> = self.modules.iter().map(|m| m.name()).collect();
        titles.push("search");
        let tabs = Tabs::new(titles)
            .select(self.tab)
            .block(Block::bordered().title(" life "))
            .highlight_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(tabs, top);

        match self.modules.get(self.tab) {
            None => self.draw_search(f, body),
            Some(m) if m.takes_lines() => self.draw_lines(f, body),
            Some(_) => self.draw_module(f, body),
        }
        if show_bar {
            self.draw_day_bar(f, daybar);
        }

        let room = (bottom.width as usize).saturating_sub(6);
        let (title, line) = match &self.mode {
            Mode::Input { action, step, buffer, .. } => {
                let prompt = action.prompts[*step];
                let space = room.saturating_sub(prompt.chars().count() + 2);
                (
                    format!(" {} ({}/{}) ", action.label, step + 1, action.prompts.len()),
                    format!("{}: {}█", prompt, tail(buffer, space)),
                )
            }
            Mode::Normal => match self.modules.get(self.tab) {
                None => (
                    format!(" search, {} hits ", self.search_hits.len()),
                    format!(
                        "/ {}█   (enter open | tab switch | esc back | ctrl-c quit)",
                        tail(&self.search_query, room.saturating_sub(60))
                    ),
                ),
                Some(m) if m.takes_lines() => (
                    format!(" {} ", self.status),
                    "type a line | enter save | esc clear | tab switch | ctrl-c quit".to_string(),
                ),
                Some(m) => {
                    let mut hints: Vec<String> = m
                        .actions()
                        .iter()
                        .map(|a| format!("{} {}", a.key, a.label))
                        .collect();
                    hints.push("/ search".into());
                    hints.push("tab switch".into());
                    hints.push("q quit".into());
                    (format!(" {} ", self.status), hints.join("  |  "))
                }
            },
        };
        f.render_widget(Paragraph::new(line).block(Block::bordered().title(title)), bottom);
    }

    fn draw_lines(&self, f: &mut Frame, area: Rect) {
        let module = &self.modules[self.tab];
        let rows = module.rows();
        let [history, input_area] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).areas(area);

        let room = history.height.saturating_sub(2) as usize;
        let mut items: Vec<ListItem> = Vec::new();
        let mut last = String::new();
        for row in rows.iter().skip(rows.len().saturating_sub(room)) {
            if !row.group.is_empty() && row.group != last {
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("── {} ──", row.group),
                    Style::new().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                ))));
                last = row.group.clone();
            }
            items.push(ListItem::new(row.label.clone()));
        }
        let skip = items.len().saturating_sub(room);
        let items: Vec<ListItem> = items.into_iter().skip(skip).collect();
        f.render_widget(List::new(items).block(Block::bordered().title(" diary ")), history);

        let space = (input_area.width as usize).saturating_sub(4);
        let input = Paragraph::new(format!("{}█", tail(&self.line_buffer, space))).block(
            Block::bordered()
                .title(" write your story · Enter saves this line · #word makes a tag "),
        );
        f.render_widget(input, input_area);
    }

    fn draw_module(&self, f: &mut Frame, area: Rect) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(area);

        let module = &self.modules[self.tab];
        let rows = module.rows();
        let sel = self.selected[self.tab].min(rows.len().saturating_sub(1));

        let mut items: Vec<ListItem> = Vec::new();
        let mut list_sel: Option<usize> = None;
        let mut last = String::new();
        for (i, row) in rows.iter().enumerate() {
            if !row.group.is_empty() && row.group != last {
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("── {} ──", row.group),
                    Style::new().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                ))));
                last = row.group.clone();
            }
            if i == sel {
                list_sel = Some(items.len());
            }
            items.push(ListItem::new(row.label.clone()));
        }
        let list = List::new(items)
            .block(Block::bordered().title(format!(" {} ", module.heading())))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        let mut state = ListState::default().with_selected(list_sel);
        f.render_stateful_widget(list, left, &mut state);

        let detail = match rows.get(sel) {
            Some(row) => module.detail(row.id),
            None => "Nothing here.".to_string(),
        };
        f.render_widget(
            Paragraph::new(detail)
                .wrap(Wrap { trim: false })
                .block(Block::bordered().title(" detail ")),
            right,
        );
    }

    fn draw_day_bar(&self, f: &mut Frame, area: Rect) {
        let module = &self.modules[self.tab];
        let now = Local::now();
        let date = module.bar_date(self.selected_id()).unwrap_or_else(today);
        let is_today = date == today();
        let past = date < today();

        let title = if is_today {
            format!(" {} · now {} ", date.format("%a %d %b"), now.format("%H:%M"))
        } else {
            format!(" {} ", date.format("%A %d %B"))
        };
        let block = Block::bordered().title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width < 12 || inner.height < 5 {
            return;
        }

        let width = inner.width as usize;
        let minutes = (now.hour() * 60 + now.minute()) as usize;
        let now_col = (minutes * width / (24 * 60)).min(width - 1);

        let mut paint: Vec<Option<Color>> = vec![None; width];
        for m in &self.modules {
            let color = m.block_color();
            for (start, end) in m.day_blocks(date) {
                let from = start as usize * width / (24 * 60);
                let to = ((end as usize * width + 24 * 60 - 1) / (24 * 60)).max(from + 1);
                for cell in paint.iter_mut().take(to.min(width)).skip(from) {
                    *cell = Some(color);
                }
            }
        }

        let marker = if is_today {
            Line::from(vec![
                Span::raw(" ".repeat(now_col)),
                Span::styled("▼", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ])
        } else {
            Line::from("")
        };

        let spans: Vec<Span> = (0..width)
            .map(|col| {
                if is_today && col == now_col {
                    Span::styled("█", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else if let Some(color) = paint[col] {
                    Span::styled("█", Style::new().fg(color))
                } else if past || (is_today && col < now_col) {
                    Span::styled("▒", Style::new().fg(Color::Blue))
                } else {
                    Span::styled("░", Style::new().fg(Color::DarkGray))
                }
            })
            .collect();
        let bar = Line::from(spans);

        let step = if width >= 32 { 3 } else { 6 };
        let mut labels = vec![' '; width];
        for h in (0..=24usize).step_by(step) {
            let col = (h * width / 24).min(width - 2);
            for (i, ch) in format!("{:02}", h).chars().enumerate() {
                labels[col + i] = ch;
            }
        }
        let labels: String = labels.into_iter().collect();

        let key = |color: Color, text: &'static str| {
            vec![Span::styled("■ ", Style::new().fg(color)), Span::raw(text)]
        };
        let mut legend: Vec<Span> = Vec::new();
        legend.extend(key(Color::Magenta, "event   "));
        legend.extend(key(Color::Cyan, "daily   "));
        legend.extend(key(Color::Green, "cube   "));
        legend.push(Span::styled("▒ ", Style::new().fg(Color::Blue)));
        legend.push(Span::raw("passed"));

        let lines = vec![marker, bar.clone(), bar, Line::from(labels), Line::from(legend)];
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn draw_search(&self, f: &mut Frame, area: Rect) {
        let hits = &self.search_hits;
        let items: Vec<ListItem> = hits
            .iter()
            .map(|h| {
                ListItem::new(format!(
                    "[{}] {}  {}",
                    h.score,
                    self.modules[h.module].name(),
                    h.label
                ))
            })
            .collect();
        let list = List::new(items)
            .block(Block::bordered().title(" results · try #tag to search tags "))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        let mut state = ListState::default()
            .with_selected(if hits.is_empty() { None } else { Some(self.search_sel) });
        f.render_stateful_widget(list, area, &mut state);
    }
}

fn main() {
    let mut app = App::new();
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("error: {e}");
    }
}

// ===========================================================================
// TESTS
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> NaiveDate {
        parse_date(text).expect("test date")
    }

    #[test]
    fn spans_cross_midnight() {
        let from = d("2026-10-24");
        assert_eq!(span_on(from, 23 * 60, 420, from), Some((1380, 1440)));
        assert_eq!(span_on(from, 23 * 60, 420, d("2026-10-25")), Some((0, 360)));
        assert_eq!(span_on(from, 23 * 60, 420, d("2026-10-26")), None);
        assert_eq!(span_on(from, 23 * 60, 420, d("2026-10-23")), None);
    }

    #[test]
    fn overnight_event_is_painted_on_both_days() {
        let input: Vec<String> = ["Sleep #rest", "2026-10-24", "21:00", "05:00"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let event = CalEvent::build(1, &input, None).expect("valid event");
        assert_eq!(event.length(), 480);
        assert_eq!(event.tags, vec!["rest".to_string()]);
        assert_eq!(event.blocks(0, d("2026-10-24")), vec![(1260, 1440)]);
        assert_eq!(event.blocks(0, d("2026-10-25")), vec![(0, 300)]);
    }

    #[test]
    fn day_lists() {
        assert_eq!(parse_days("mon-fri"), Ok(vec![0, 1, 2, 3, 4]));
        assert_eq!(parse_days("fri-mon"), Ok(vec![0, 4, 5, 6]));
        assert_eq!(parse_days("mon, wed fri"), Ok(vec![0, 2, 4]));
        assert_eq!(parse_days("weekend"), Ok(vec![5, 6]));
        assert_eq!(parse_days(""), Ok((0..7).collect::<Vec<u8>>()));
        assert!(parse_days("someday").is_err());
        assert_eq!(days_text(&[0, 1, 2, 3, 4]), "weekdays");
    }

    #[test]
    fn typed_tags_and_groups() {
        let (title, tags, group) = split_text("Buy milk #Errands @quick", true);
        assert_eq!(title, "Buy milk");
        assert_eq!(tags, vec!["errands".to_string()]);
        assert_eq!(group, Some("quick".to_string()));
        let (title, _, group) = split_text("mail me@x.com @home", false);
        assert_eq!(title, "mail me@x.com @home");
        assert_eq!(group, None);
        assert_eq!(join_text("Buy milk", &tags, Some("quick")), "Buy milk #errands @quick");
    }

    #[test]
    fn group_names() {
        assert!(Group::parse("sero") == Some(Group::Serotonin));
        assert!(Group::parse("q") == Some(Group::Quick));
        assert!(Group::parse("main") == Some(Group::Main));
        assert!(Group::parse("s").is_none());
        assert!(Group::parse("nope").is_none());
    }

    #[test]
    fn newest_first_keys() {
        assert!(desc_key("2026-10-25") < desc_key("2026-10-24"));
        assert!(desc_key("000000000012") < desc_key("000000000011"));
    }

    #[test]
    fn fuzzy_finds_letters_in_order() {
        assert!(fuzzy_score("rst", "Rust").is_some());
        assert!(fuzzy_score("tsr", "Rust").is_none());
    }

    #[test]
    fn timetable_lines_become_events() {
        let got = parse_timetable("14:00-15:30 Dentist #health").expect("an event");
        assert_eq!(got, ["Dentist #health", "", "14:00", "15:30"].map(String::from));
        let got = parse_timetable("2026-10-30 9:00-10:00 Standup").expect("an event");
        assert_eq!(got, ["Standup", "2026-10-30", "9:00", "10:00"].map(String::from));
        assert!(parse_timetable("lunch with Ali").is_none());
        assert!(parse_timetable("14:00-15:30").is_none());
        assert!(parse_timetable("25:00-26:00 Nope").is_none());
    }

    #[test]
    fn quit_habit_streak_starts_clean() {
        let input: Vec<String> = ["Smoking", "q", "", "cigarettes"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut habit = Habit::build(1, &input, None).expect("valid habit");
        assert_eq!(habit.streak(), 1);
        assert!(habit.act(0, '+', &[]).is_some());
        assert_eq!(habit.streak(), 0);
        assert!(habit.act(0, '-', &[]).is_some());
        assert_eq!(habit.streak(), 1);
    }

    #[test]
    fn intents_from_diary_lines() {
        use Intent::*;
        assert_eq!(
            parse_intents("todo: buy milk @quick"),
            vec![Todo { title: "buy milk".into(), group: Group::Quick }]
        );
        assert_eq!(
            parse_intents("TODO: write report"),
            vec![Todo { title: "write report".into(), group: Group::Main }]
        );
        assert_eq!(
            parse_intents("finished laundry"),
            vec![Done { title: "laundry".into() }]
        );
        assert_eq!(
            parse_intents("smoked 2"),
            vec![HabitCount { title: "smoked".into(), delta: 2 }]
        );
        let ev = parse_intents("14:00-15:30 Dentist #health");
        assert!(matches!(
            ev.as_slice(),
            [Event { title, date, start, end }]
            if title == "Dentist #health" && date.is_empty() && start == "14:00" && end == "15:30"
        ));
        assert!(parse_intents("I read 5 pages").is_empty());
        assert!(parse_intents("just a thought").is_empty());
    }

    #[test]
    fn next_id_rule_never_reuses() {
        // Pure rule: id = max(entries, high-water) + 1.
        assert_eq!(next_id_from(0, 0), 1);
        assert_eq!(next_id_from(2, 0), 3);
        // Deleted newest: file has 2, high-water says 3.
        assert_eq!(next_id_from(2, 3), 4);
        // Handed-out id from another writer: high-water ahead of file.
        assert_eq!(next_id_from(5, 9), 10);
    }

    #[test]
    fn chat_msg_round_trips() {
        let original = ChatMsg {
            id: 7,
            you: "how was today?".into(),
            companion: "quiet, and that's fine.".into(),
            when: "2026-10-24 22:00".into(),
        };
        let text = serde_json::to_string(&original).unwrap();
        let back: ChatMsg = serde_json::from_str(&text).unwrap();
        assert_eq!(back.you, original.you);
        assert_eq!(back.companion, original.companion);
    }
}
```

---

## What you get when you `cargo run`

1. **Diary is the front door.** Type `todo: buy milk @quick` in the diary tab. Two seconds later, the AI tab shows: _"Add 'buy milk' to your quick todos?"_ Press `y`, verify the todo exists. Same for `finished laundry`, `smoked 2`, `14:00-15:30 Dentist`.
2. **Two AIs** — `Curator` proposes structural fixes and turns intents into proposals. `Companion` (stub today) answers chat. Both are plug points for a real local model. `proposals.json` and `chat.json` are your training data.
3. **Modules toggle** from the settings tab. Disable `dict`, restart, the tab is gone. Re-enable, restart, the tab and its data are back. Diary, AI, and settings can't be turned off.
4. **No lost ids.** Delete a todo, add a new one — the new one gets a _higher_ id than any before it. Proposals and log entries always point at the right thing.
5. **Panel actually works** — `cargo run --bin panel` with the TUI open, type into the browser, and the entry appears in the TUI within 2 s.
6. **Catch-up** on start — you see how long you were away.

If something doesn't compile on your toolchain (edition/version mismatch on `axum`, `ratatui`, etc.), tell me the exact `cargo check` output and I'll fix that single error. Otherwise this is ready to `cargo run`.
