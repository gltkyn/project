#![recursion_limit = "512"]

//! dortyuzlu — a private, local-only life-management system.
//!
//! One binary. TUI + panel on :3000. One store. Views are filters.
//! The AI runs every tick, proposes changes; user approves.
//!
//!   dortyuzlu             TUI + panel
//!   dortyuzlu --no-panel  TUI only
//!   dortyuzlu --no-tui    panel only

mod panel;

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate, NaiveTime, Timelike};
use ratatui::{
    crossterm::event::{self, Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    DefaultTerminal, Frame,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// ============================================================
// 1. HELPERS
// ============================================================

fn today() -> NaiveDate { Local::now().date_naive() }
fn today_text() -> String { today().format("%Y-%m-%d").to_string() }

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

fn minutes_of(text: &str) -> Option<u32> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M").ok()
        .map(|t| t.hour() * 60 + t.minute())
}

fn clock(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
}

fn clean_tag(word: &str) -> Option<String> {
    let tag = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    if tag.is_empty() { None } else { Some(tag) }
}

fn hashtags(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for word in text.split_whitespace() {
        if let Some(rest) = word.strip_prefix('#') {
            if let Some(tag) = clean_tag(rest) {
                if !found.contains(&tag) { found.push(tag); }
            }
        }
    }
    found
}

fn split_text(text: &str) -> (String, Vec<String>) {
    let mut words = Vec::new();
    let mut tags = Vec::new();
    for word in text.split_whitespace() {
        if let Some(rest) = word.strip_prefix('#') {
            if let Some(tag) = clean_tag(rest) {
                if !tags.contains(&tag) { tags.push(tag); }
            }
        } else {
            words.push(word);
        }
    }
    (words.join(" "), tags)
}

fn data_dir() -> PathBuf {
    let dir = std::env::var_os("DORTYUZLU_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join("dortyuzlu")))
        .unwrap_or_else(|| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    dir
}

pub(crate) fn data_path(name: &str) -> PathBuf {
    data_dir().join(name)
}

fn tail(text: &str, room: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    chars[chars.len().saturating_sub(room)..].iter().collect()
}

// ============================================================
// 2. STORAGE
// ============================================================

pub(crate) trait Entry: Serialize + DeserializeOwned + Clone {
    const FILE: &'static str;
    fn id(&self) -> u64;
    fn title(&self) -> &str;
    fn body(&self) -> &str { "" }
    fn tags(&self) -> &[String] { &[] }
}

pub(crate) struct Store<T: Entry> {
    pub items: Vec<T>,
}

impl<T: Entry> Store<T> {
    fn load() -> Self {
        let path = data_path(T::FILE);
        let items = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
                let aside = data_path(&format!("{}.corrupt", T::FILE));
                let _ = fs::rename(&path, &aside);
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        Store { items }
    }
    fn save(&self) {
        let text = serde_json::to_string_pretty(&self.items).expect("serialize");
        let temp = data_path(&format!("{}.tmp", T::FILE));
        fs::write(&temp, text).expect("write");
        fs::rename(&temp, data_path(T::FILE)).expect("rename");
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
    fn remove(&mut self, id: u64) {
        self.items.retain(|i| i.id() != id);
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum LogKind { Added, Removed, Done, Counted, Approved, Rejected }

#[derive(Clone, Serialize, Deserialize)]
struct LogEvent {
    id: u64,
    module: String,
    kind: LogKind,
    entry_id: u64,
    title: String,
}

struct Logger { next_id: u64 }

impl Logger {
    fn open() -> Self {
        let max = fs::read_to_string(data_path("history.jsonl"))
            .map(|t| t.lines()
                .filter_map(|l| serde_json::from_str::<LogEvent>(l).ok())
                .map(|e| e.id).max().unwrap_or(0))
            .unwrap_or(0);
        Logger { next_id: max + 1 }
    }
    fn log(&mut self, module: &str, kind: LogKind, entry_id: u64, title: &str) {
        let ev = LogEvent {
            id: self.next_id,
            module: module.into(),
            kind,
            entry_id,
            title: title.into(),
        };
        self.next_id += 1;
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true).append(true).open(data_path("history.jsonl"))
        {
            let _ = writeln!(f, "{}", serde_json::to_string(&ev).unwrap());
        }
    }
}

// ============================================================
// 3. ITEM
// ============================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Item {
    pub id: u64,
    pub title: String,
    #[serde(default)] pub body: String,
    #[serde(default)] pub tags: Vec<String>,

    pub kind: ItemKind,
    #[serde(default)] pub state: State,
    #[serde(default)] pub tier: Tier,
    #[serde(default)] pub throwable: Throwable,

    #[serde(default)] pub when: Option<When>,
    #[serde(default)] pub minutes: u32,
    #[serde(default)] pub floor: u32,

    #[serde(default)] pub progress: BTreeMap<NaiveDate, u32>,
    #[serde(default)] pub parent_id: Option<u64>,

    #[serde(default)] pub ai_notes: String,
    #[serde(default)] pub tier_locked: bool,
    #[serde(default)] pub check_in: bool,

    #[serde(default)] pub created: String,
}

impl Entry for Item {
    const FILE: &'static str = "items.json";
    fn id(&self) -> u64 { self.id }
    fn title(&self) -> &str { &self.title }
    fn body(&self) -> &str { &self.body }
    fn tags(&self) -> &[String] { &self.tags }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ItemKind {
    Task,
    Event,
    Daily { days: Vec<u8> },
    HabitMake { target: u32 },
    HabitQuit { target: u32, #[serde(default)] replacement: Option<u64> },
    Note,
    Project,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum State {
    #[default] Open,
    Done { on: NaiveDate },
    Failed { on: NaiveDate, why: String },
    Waiting { for_date: NaiveDate },
    Deferred { until: NaiveDate },
    Skipped { on: NaiveDate },
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Tier { Requirement, #[default] Developing, Optional, Quitting }

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Throwable { Never, UnderPressure, #[default] Anytime }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct When {
    pub date: NaiveDate,
    pub start: u32,
    pub minutes: u32,
    #[serde(default)] pub actual: Option<u32>,
}

impl Item {
    fn effective_minutes(&self) -> u32 {
        if let Some(w) = &self.when {
            if let Some(a) = w.actual { return a; }
            return self.minutes.max(w.minutes);
        }
        self.minutes
    }
    fn is_open(&self) -> bool {
        matches!(self.state, State::Open | State::Waiting { .. })
    }
    fn is_habit(&self) -> bool {
        matches!(self.kind, ItemKind::HabitMake { .. } | ItemKind::HabitQuit { .. })
    }
    fn target(&self) -> u32 {
        match self.kind {
            ItemKind::HabitMake { target } | ItemKind::HabitQuit { target, .. } => target,
            _ => 0,
        }
    }
    fn count_on(&self, day: NaiveDate) -> u32 {
        self.progress.get(&day).copied().unwrap_or(0)
    }
    fn habit_good_on(&self, day: NaiveDate) -> bool {
        match self.kind {
            ItemKind::HabitMake { target } => self.count_on(day) >= target.max(1),
            ItemKind::HabitQuit { target, .. } => self.count_on(day) <= target,
            _ => false,
        }
    }
    fn habit_streak(&self) -> u32 {
        let mut day = today();
        if matches!(self.kind, ItemKind::HabitMake { .. }) && !self.habit_good_on(day) {
            match day.pred_opt() { Some(d) => day = d, None => return 0 }
        }
        let mut run = 0;
        loop {
            if !self.habit_good_on(day) { break; }
            run += 1;
            match day.pred_opt() { Some(d) => day = d, None => break }
        }
        run
    }
}

// ============================================================
// 4. DIARY + intents
// ============================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum Intent {
    Todo { title: String },
    Event { title: String, date: String, start: String, end: String },
    Done { title: String },
    HabitCount { title: String, delta: u32 },
}

fn parse_intents(text: &str) -> Vec<Intent> {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    let mut out = Vec::new();

    if lower.starts_with("todo:") {
        if let Some(rest) = trimmed.get(5..) {
            let (title, _) = split_text(rest);
            if !title.is_empty() { out.push(Intent::Todo { title }); return out; }
        }
    }
    for prefix in ["finished ", "done ", "did "] {
        if lower.starts_with(prefix) {
            if let Some(rest) = trimmed.get(prefix.len()..) {
                let title = rest.trim().to_string();
                if !title.is_empty() { out.push(Intent::Done { title }); return out; }
            }
        }
    }
    if let Some(fields) = parse_timetable(trimmed) {
        out.push(Intent::Event {
            title: fields[0].clone(), date: fields[1].clone(),
            start: fields[2].clone(), end: fields[3].clone(),
        });
        return out;
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() == 2 {
        if let Ok(k) = words[1].parse::<u32>() {
            out.push(Intent::HabitCount { title: words[0].to_string(), delta: k });
        }
    }
    out
}

fn parse_timetable(text: &str) -> Option<[String; 4]> {
    let mut words = text.split_whitespace().peekable();
    let mut date = String::new();
    if let Some(first) = words.peek() {
        if parse_date(first).is_some() { date = first.to_string(); words.next(); }
    }
    let (start, end) = words.next()?.split_once('-')?;
    minutes_of(start)?; minutes_of(end)?;
    let title = words.collect::<Vec<&str>>().join(" ");
    if title.is_empty() { return None; }
    Some([title, date, start.into(), end.into()])
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct DiaryLine {
    pub id: u64,
    pub date: String,
    pub text: String,
    #[serde(default)] pub tags: Vec<String>,
    #[serde(default)] pub intent: Vec<Intent>,
}

impl Entry for DiaryLine {
    const FILE: &'static str = "diary.json";
    fn id(&self) -> u64 { self.id }
    fn title(&self) -> &str { &self.text }
    fn body(&self) -> &str { &self.date }
    fn tags(&self) -> &[String] { &self.tags }
}

// ============================================================
// 5. AI
// ============================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Change {
    SetState { id: u64, state: State },
    Add { item: Item },
    Delete { id: u64 },
    Count { id: u64, delta: i32 },
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Verdict { Pending, Yes, No }

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Proposal {
    pub id: u64,
    pub key: String,
    pub ask: String,
    pub why: String,
    pub change: Change,
    pub verdict: Verdict,
    pub made: String,
    #[serde(default)] pub decided: String,
}

impl Entry for Proposal {
    const FILE: &'static str = "proposals.json";
    fn id(&self) -> u64 { self.id }
    fn title(&self) -> &str { &self.ask }
    fn body(&self) -> &str { &self.why }
}

struct Curator;

impl Curator {
    fn advise(&self, items: &[Item], diary: &[DiaryLine], proposals: &[Proposal])
        -> Vec<(String, String, String, Change)>
    {
        let mut out = Vec::new();
        for line in diary {
            for (i, intent) in line.intent.iter().enumerate() {
                match intent {
                    Intent::Todo { title } => {
                        let key = format!("diary-todo:{}:{}", line.id, i);
                        if proposals.iter().any(|p| p.key == key) { continue; }
                        let (t, tags) = split_text(title);
                        out.push((
                            key,
                            format!("Add \"{}\" as a task?", title),
                            format!("You wrote: \"{}\"", line.text),
                            Change::Add { item: mk_item(t, tags, ItemKind::Task, None) },
                        ));
                    }
                    Intent::Done { title } => {
                        if let Some(item) = items.iter().find(|it|
                            it.is_open() && it.title.to_lowercase() == title.to_lowercase())
                        {
                            let key = format!("diary-done:{}:{}", line.id, item.id);
                            if proposals.iter().any(|p| p.key == key) { continue; }
                            out.push((
                                key,
                                format!("Mark \"{}\" as done?", item.title),
                                format!("You wrote: \"{}\"", line.text),
                                Change::SetState { id: item.id, state: State::Done { on: today() } },
                            ));
                        }
                    }
                    Intent::HabitCount { title, delta } => {
                        let lower = title.to_lowercase();
                        if let Some(item) = items.iter().find(|it|
                            it.is_habit() && it.title.to_lowercase().starts_with(&lower))
                        {
                            let key = format!("diary-habit:{}:{}", line.id, item.id);
                            if proposals.iter().any(|p| p.key == key) { continue; }
                            out.push((
                                key,
                                format!("Count {} on \"{}\"?", delta, item.title),
                                format!("You wrote: \"{}\"", line.text),
                                Change::Count { id: item.id, delta: *delta as i32 },
                            ));
                        }
                    }
                    Intent::Event { title, date, start, end } => {
                        let key = format!("diary-event:{}:{}", line.id, i);
                        if proposals.iter().any(|p| p.key == key) { continue; }
                        let date_p = if date.is_empty() { today() } else { parse_date(date).unwrap_or_else(today) };
                        let s = minutes_of(start).unwrap_or(0);
                        let e = minutes_of(end).unwrap_or(0);
                        let mins = if e > s { e - s } else { 1440 - s + e };
                        let (t, tags) = split_text(title);
                        out.push((
                            key,
                            format!("Add \"{}\" {}–{} as an event?", title, start, end),
                            format!("You wrote: \"{}\"", line.text),
                            Change::Add { item: mk_item(t, tags, ItemKind::Event, Some(When { date: date_p, start: s, minutes: mins, actual: None })) },
                        ));
                    }
                }
            }
        }
        out
    }
}

fn mk_item(title: String, tags: Vec<String>, kind: ItemKind, when: Option<When>) -> Item {
    let mins = when.as_ref().map(|w| w.minutes).unwrap_or(0);
    Item {
        id: 0, title, body: String::new(), tags, kind,
        state: State::Open, tier: Tier::Developing, throwable: Throwable::Anytime,
        when, minutes: mins, floor: mins,
        progress: BTreeMap::new(), parent_id: None, ai_notes: String::new(),
        tier_locked: false, check_in: false, created: today_text(),
    }
}

fn apply_change(core: &mut Core, change: Change) {
    match change {
        Change::SetState { id, state } => {
            if let Some(item) = core.items.get_mut(id) {
                item.state = state;
                core.items.save();
            }
        }
        Change::Add { mut item } => {
            item.id = core.items.next_id();
            core.items.items.push(item);
            core.items.save();
        }
        Change::Delete { id } => { core.items.remove(id); core.items.save(); }
        Change::Count { id, delta } => {
            if let Some(item) = core.items.get_mut(id) {
                let today_d = today();
                let n = item.count_on(today_d);
                let new = (n as i32 + delta).max(0) as u32;
                if new == 0 { item.progress.remove(&today_d); }
                else { item.progress.insert(today_d, new); }
                core.items.save();
            }
        }
    }
}

fn run_curator(core: &mut Core) -> usize {
    let ideas = Curator.advise(&core.items.items, &core.diary.items, &core.proposals.items);
    let mut added = 0;
    for (key, ask, why, change) in ideas {
        if core.proposals.items.iter().any(|p| p.key == key) { continue; }
        let id = core.proposals.next_id();
        core.proposals.items.push(Proposal {
            id, key, ask, why, change,
            verdict: Verdict::Pending,
            made: Local::now().format("%Y-%m-%d %H:%M").to_string(),
            decided: String::new(),
        });
        added += 1;
    }
    if added > 0 { core.proposals.save(); }
    added
}

// ============================================================
// 6. VIEWS
// ============================================================

pub(crate) struct Row { pub id: u64, pub label: String }

#[derive(Clone, Copy)]
pub(crate) struct Action {
    pub key: char,
    pub label: &'static str,
    pub needs_item: bool,
    pub prompts: &'static [&'static str],
}

pub(crate) struct Core {
    pub items: Store<Item>,
    pub diary: Store<DiaryLine>,
    pub proposals: Store<Proposal>,
    pub log: Logger,
}

pub(crate) trait View {
    fn name(&self) -> &'static str;
    fn rows(&self, core: &Core) -> Vec<Row>;
    fn detail(&self, core: &Core, id: u64) -> String;
    fn actions(&self) -> Vec<Action>;
    fn apply(&mut self, core: &mut Core, action: char, id: Option<u64>, input: &[String]) -> String;
    fn shows_day_bar(&self) -> bool { false }
    fn day_blocks(&self, _core: &Core, _date: NaiveDate) -> Vec<(u32, u32)> { Vec::new() }
    fn takes_lines(&self) -> bool { false }
    fn line_input(&mut self, _core: &mut Core, _text: &str) -> Option<String> { None }
}

// ---------- Todo ----------
struct TodoView;

impl View for TodoView {
    fn name(&self) -> &'static str { "todo" }

    fn rows(&self, core: &Core) -> Vec<Row> {
        let mut list: Vec<Row> = core.items.items.iter()
            .filter(|i| matches!(i.kind, ItemKind::Task) && i.is_open())
            .map(|i| {
                let tags = if i.tags.is_empty() { String::new() }
                    else { format!("  {}", i.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ")) };
                Row { id: i.id, label: format!("{}{}", i.title, tags) }
            })
            .collect();
        list.sort_by(|a, b| b.id.cmp(&a.id));
        list
    }

    fn detail(&self, core: &Core, id: u64) -> String {
        let Some(i) = core.items.get(id) else { return String::new() };
        let status = match &i.state {
            State::Open => "open".to_string(),
            State::Done { on } => format!("done {}", on),
            _ => "other".to_string(),
        };
        format!("{}\n\nstatus: {}\ntier: {:?}\ncreated: {}",
            i.title, status, i.tier, i.created)
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action { key: 'a', label: "add", needs_item: false, prompts: &["Task  (#tag)"] },
            Action { key: 'x', label: "done", needs_item: true, prompts: &[] },
            Action { key: 'd', label: "delete", needs_item: true, prompts: &[] },
        ]
    }

    fn apply(&mut self, core: &mut Core, action: char, id: Option<u64>, input: &[String]) -> String {
        match action {
            'a' => {
                let (title, tags) = split_text(&input[0]);
                if title.is_empty() { return "Title empty.".into(); }
                let new_id = core.items.next_id();
                core.items.items.push(mk_item(title.clone(), tags, ItemKind::Task, None));
                if let Some(i) = core.items.get_mut(new_id) { i.id = new_id; }
                core.items.save();
                core.log.log("todo", LogKind::Added, new_id, &title);
                "Added.".into()
            }
            'x' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if let Some(i) = core.items.get_mut(id) {
                    i.state = State::Done { on: today() };
                    let t = i.title.clone();
                    core.items.save();
                    core.log.log("todo", LogKind::Done, id, &t);
                    "Done.".into()
                } else { "Not found.".into() }
            }
            'd' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if let Some(i) = core.items.get(id) {
                    let t = i.title.clone();
                    core.items.remove(id);
                    core.items.save();
                    core.log.log("todo", LogKind::Removed, id, &t);
                    "Removed.".into()
                } else { "Not found.".into() }
            }
            _ => String::new(),
        }
    }
}

// ---------- Timeline ----------
struct TimelineView { date: NaiveDate }

impl View for TimelineView {
    fn name(&self) -> &'static str { "timeline" }
    fn shows_day_bar(&self) -> bool { true }

    fn day_blocks(&self, core: &Core, date: NaiveDate) -> Vec<(u32, u32)> {
        core.items.items.iter().filter_map(|i| {
            let w = i.when.as_ref()?;
            if w.date != date || !i.is_open() { return None; }
            Some((w.start, (w.start + i.effective_minutes()).min(1440)))
        }).collect()
    }

    fn rows(&self, core: &Core) -> Vec<Row> {
        let mut list: Vec<Row> = core.items.items.iter()
            .filter(|i| i.when.is_some() && i.is_open())
            .filter_map(|i| {
                let w = i.when.as_ref()?;
                if w.date != self.date { return None; }
                Some(Row { id: i.id, label: format!(
                    "{}-{}  {}", clock(w.start), clock(w.start + i.effective_minutes()), i.title) })
            })
            .collect();
        list.sort_by(|a, b| a.label.cmp(&b.label));
        list
    }

    fn detail(&self, core: &Core, id: u64) -> String {
        let Some(i) = core.items.get(id) else { return String::new() };
        let w = i.when.as_ref();
        format!("{}\n\nkind: {:?}\nwhen: {}\nplanned: {} min\nactual: {} min",
            i.title, i.kind,
            w.map(|w| format!("{} {}", w.date, clock(w.start))).unwrap_or_default(),
            w.map(|w| w.minutes).unwrap_or(0),
            i.effective_minutes())
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action { key: 'a', label: "add", needs_item: false,
                prompts: &["Title", "Date (YYYY-MM-DD)", "Start (HH:MM)", "Minutes"] },
            Action { key: 'x', label: "done", needs_item: true, prompts: &[] },
        ]
    }

    fn apply(&mut self, core: &mut Core, action: char, id: Option<u64>, input: &[String]) -> String {
        match action {
            'a' => {
                let (title, tags) = split_text(&input[0]);
                if title.is_empty() { return "Title empty.".into(); }
                let date = parse_date(&input[1]).unwrap_or_else(today);
                let start = minutes_of(&input[2]).unwrap_or(0);
                let mins: u32 = input[3].parse().unwrap_or(30);
                let new_id = core.items.next_id();
                let when = When { date, start, minutes: mins, actual: None };
                let mut it = mk_item(title.clone(), tags, ItemKind::Event, Some(when));
                it.id = new_id;
                it.floor = mins;
                core.items.items.push(it);
                core.items.save();
                core.log.log("timeline", LogKind::Added, new_id, &title);
                "Added.".into()
            }
            'x' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if let Some(i) = core.items.get_mut(id) {
                    i.state = State::Done { on: today() };
                    let t = i.title.clone();
                    core.items.save();
                    core.log.log("timeline", LogKind::Done, id, &t);
                    "Done.".into()
                } else { "Not found.".into() }
            }
            _ => String::new(),
        }
    }
}

// ---------- Diary ----------
struct DiaryView;

impl View for DiaryView {
    fn name(&self) -> &'static str { "diary" }
    fn takes_lines(&self) -> bool { true }

    fn rows(&self, core: &Core) -> Vec<Row> {
        let mut list: Vec<Row> = core.diary.items.iter()
            .map(|l| Row { id: l.id,
                label: format!("{}  {}", l.date.get(11..).unwrap_or(""), l.text) })
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    fn detail(&self, core: &Core, id: u64) -> String {
        let Some(l) = core.diary.get(id) else { return String::new() };
        let hints = if l.intent.is_empty() { String::new() }
            else { format!("\n\nintents: {:?}", l.intent) };
        format!("{}\n\n{}{}", l.date, l.text, hints)
    }

    fn actions(&self) -> Vec<Action> { vec![] }
    fn apply(&mut self, _: &mut Core, _: char, _: Option<u64>, _: &[String]) -> String { String::new() }

    fn line_input(&mut self, core: &mut Core, text: &str) -> Option<String> {
        let text = text.trim();
        if text.is_empty() { return None; }
        let id = core.diary.next_id();
        core.diary.items.push(DiaryLine {
            id,
            date: Local::now().format("%Y-%m-%d %H:%M").to_string(),
            text: text.into(),
            tags: hashtags(text),
            intent: parse_intents(text),
        });
        core.diary.save();
        core.log.log("diary", LogKind::Added, id, text);
        Some("Saved.".into())
    }
}

// ---------- Habits ----------
struct HabitsView;

impl View for HabitsView {
    fn name(&self) -> &'static str { "habits" }

    fn rows(&self, core: &Core) -> Vec<Row> {
        let t = today();
        core.items.items.iter()
            .filter(|i| i.is_habit())
            .map(|i| {
                let sym = if i.habit_good_on(t) { "x" } else { " " };
                Row { id: i.id, label: format!(
                    "[{}] {}  {}/{}  streak {}d",
                    sym, i.title, i.count_on(t), i.target(), i.habit_streak()) }
            })
            .collect()
    }

    fn detail(&self, core: &Core, id: u64) -> String {
        let Some(i) = core.items.get(id) else { return String::new() };
        format!("{}\n\nkind: {:?}\ntarget: {}\ntoday: {}\nstreak: {} days\n\n+ counts one, - takes one back",
            i.title, i.kind, i.target(), i.count_on(today()), i.habit_streak())
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action { key: 'a', label: "add", needs_item: false,
                prompts: &["Title", "b (build) or q (quit)", "Target"] },
            Action { key: '+', label: "+1", needs_item: true, prompts: &[] },
            Action { key: '-', label: "-1", needs_item: true, prompts: &[] },
            Action { key: 'd', label: "delete", needs_item: true, prompts: &[] },
        ]
    }

    fn apply(&mut self, core: &mut Core, action: char, id: Option<u64>, input: &[String]) -> String {
        match action {
            'a' => {
                let (title, tags) = split_text(&input[0]);
                if title.is_empty() { return "Title empty.".into(); }
                let is_quit = matches!(input[1].trim().to_lowercase().chars().next(), Some('q'));
                let target: u32 = input[2].trim().parse().unwrap_or(if is_quit { 0 } else { 1 });
                let kind = if is_quit {
                    ItemKind::HabitQuit { target, replacement: None }
                } else {
                    ItemKind::HabitMake { target: target.max(1) }
                };
                let new_id = core.items.next_id();
                let mut it = mk_item(title.clone(), tags, kind, None);
                it.id = new_id;
                it.tier = if is_quit { Tier::Quitting } else { Tier::Developing };
                core.items.items.push(it);
                core.items.save();
                core.log.log("habit", LogKind::Added, new_id, &title);
                "Added habit.".into()
            }
            '+' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if let Some(i) = core.items.get_mut(id) {
                    *i.progress.entry(today()).or_insert(0) += 1;
                    let t = i.title.clone();
                    core.items.save();
                    core.log.log("habit", LogKind::Counted, id, &t);
                    "Counted.".into()
                } else { "Not found.".into() }
            }
            '-' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if let Some(i) = core.items.get_mut(id) {
                    let t = today();
                    let n = i.count_on(t);
                    if n == 0 { return "Nothing to remove.".into(); }
                    if n == 1 { i.progress.remove(&t); } else { i.progress.insert(t, n - 1); }
                    core.items.save();
                    "Removed one.".into()
                } else { "Not found.".into() }
            }
            'd' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                if let Some(i) = core.items.get(id) {
                    let t = i.title.clone();
                    core.items.remove(id);
                    core.items.save();
                    core.log.log("habit", LogKind::Removed, id, &t);
                    "Removed.".into()
                } else { "Not found.".into() }
            }
            _ => String::new(),
        }
    }
}

// ---------- AI ----------
struct AiView;

impl View for AiView {
    fn name(&self) -> &'static str { "ai" }

    fn rows(&self, core: &Core) -> Vec<Row> {
        let mut pending: Vec<Row> = core.proposals.items.iter()
            .filter(|p| matches!(p.verdict, Verdict::Pending))
            .map(|p| Row { id: p.id, label: format!("[?] {}", p.ask) }).collect();
        pending.sort_by(|a, b| a.id.cmp(&b.id));
        let mut answered: Vec<Row> = core.proposals.items.iter()
            .filter(|p| !matches!(p.verdict, Verdict::Pending))
            .map(|p| Row { id: p.id, label: format!("[{}] {}", 
                if matches!(p.verdict, Verdict::Yes) { "yes" } else { "no" }, p.ask) })
            .collect();
        answered.sort_by(|a, b| b.id.cmp(&a.id));
        pending.extend(answered);
        pending
    }

    fn detail(&self, core: &Core, id: u64) -> String {
        let Some(p) = core.proposals.get(id) else { return String::new() };
        let status = match p.verdict {
            Verdict::Pending => "waiting".to_string(),
            Verdict::Yes => format!("yes ({})", p.decided),
            Verdict::No => format!("no ({})", p.decided),
        };
        format!("{}\n\nwhy: {}\n\nchange: {:?}\n\nasked: {}\nstatus: {}",
            p.ask, p.why, p.change, p.made, status)
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action { key: 'y', label: "yes", needs_item: true, prompts: &[] },
            Action { key: 'n', label: "no", needs_item: true, prompts: &[] },
            Action { key: 'r', label: "think", needs_item: false, prompts: &[] },
        ]
    }

    fn apply(&mut self, core: &mut Core, action: char, id: Option<u64>, _input: &[String]) -> String {
        match action {
            'r' => {
                let added = run_curator(core);
                if added == 0 { "Nothing new.".into() } else { format!("{} new question(s).", added) }
            }
            'y' | 'n' => {
                let Some(id) = id else { return "Nothing selected.".into() };
                let snapshot = {
                    let Some(p) = core.proposals.get(id) else { return "Not found.".into() };
                    if !matches!(p.verdict, Verdict::Pending) { return "Already answered.".into(); }
                    (p.change.clone(), p.ask.clone())
                };
                let (change, ask) = snapshot;
                if let Some(p) = core.proposals.get_mut(id) {
                    p.verdict = if action == 'y' { Verdict::Yes } else { Verdict::No };
                    p.decided = Local::now().format("%Y-%m-%d %H:%M").to_string();
                }
                core.proposals.save();
                if action == 'y' {
                    apply_change(core, change);
                    core.log.log("ai", LogKind::Approved, id, &ask);
                    format!("Yes: {}", ask)
                } else {
                    core.log.log("ai", LogKind::Rejected, id, &ask);
                    format!("No: {}", ask)
                }
            }
            _ => String::new(),
        }
    }
}

// ============================================================
// 7. APP
// ============================================================

enum Mode {
    Normal,
    Input { view: usize, action: Action, item: Option<u64>, step: usize, answers: Vec<String>, buffer: String },
}

struct App {
    core: Core,
    views: Vec<Box<dyn View>>,
    tab: usize,
    selected: Vec<usize>,
    mode: Mode,
    status: String,
    line_buffer: String,
    quit: bool,
}

impl App {
    fn new() -> Self {
        let core = Core {
            items: Store::load(),
            diary: Store::load(),
            proposals: Store::load(),
            log: Logger::open(),
        };
        let views: Vec<Box<dyn View>> = vec![
            Box::new(TodoView),
            Box::new(TimelineView { date: today() }),
            Box::new(DiaryView),
            Box::new(HabitsView),
            Box::new(AiView),
        ];
        let selected = vec![0; views.len()];
        App {
            core, views, tab: 0, selected,
            mode: Mode::Normal, status: String::new(),
            line_buffer: String::new(), quit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        self.check_inbox();
        run_curator(&mut self.core);
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
                run_curator(&mut self.core);
                last_tick = Instant::now();
            }
            if !key_waiting { continue; }
            if let TermEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') { break; }
                self.on_key(key.code);
            }
        }
        Ok(())
    }

    fn check_inbox(&mut self) {
        let inbox = data_path("inbox.jsonl");
        if !inbox.exists() { return; }
        let Ok(text) = fs::read_to_string(&inbox) else { return };
        let _ = fs::remove_file(&inbox);
        let mut added = 0;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(entry) = serde_json::from_str::<InboxLine>(line) else { continue };
            if self.take_inbox(&entry) { added += 1; }
        }
        if added > 0 { self.status = format!("{} from panel.", added); }
        self.clamp();
    }

    fn take_inbox(&mut self, entry: &InboxLine) -> bool {
        match entry.module.as_str() {
            "diary" => {
                let text = entry.text.trim();
                if text.is_empty() { return false; }
                let id = self.core.diary.next_id();
                self.core.diary.items.push(DiaryLine {
                    id,
                    date: if entry.at.is_empty() {
                        Local::now().format("%Y-%m-%d %H:%M").to_string()
                    } else { entry.at.clone() },
                    text: text.into(),
                    tags: hashtags(text),
                    intent: parse_intents(text),
                });
                self.core.diary.save();
                self.core.log.log("diary", LogKind::Added, id, text);
                true
            }
            "todo" => {
                let (title, tags) = split_text(&entry.text);
                if title.is_empty() { return false; }
                let new_id = self.core.items.next_id();
                let mut it = mk_item(title.clone(), tags, ItemKind::Task, None);
                it.id = new_id;
                self.core.items.items.push(it);
                self.core.items.save();
                self.core.log.log("todo", LogKind::Added, new_id, &title);
                true
            }
            _ => false,
        }
    }

    fn clamp(&mut self) {
        let Some(v) = self.views.get(self.tab) else { return };
        let len = v.rows(&self.core).len();
        if len == 0 { self.selected[self.tab] = 0; }
        else if self.selected[self.tab] >= len { self.selected[self.tab] = len - 1; }
    }

    fn selected_id(&self) -> Option<u64> {
        let v = self.views.get(self.tab)?;
        v.rows(&self.core).get(self.selected[self.tab]).map(|r| r.id)
    }

    fn on_key(&mut self, code: KeyCode) {
        let mode = std::mem::replace(&mut self.mode, Mode::Normal);
        match mode {
            Mode::Normal => self.key_normal(code),
            Mode::Input { view, action, item, step, answers, buffer } => {
                self.key_input(code, view, action, item, step, answers, buffer)
            }
        }
    }

    fn key_normal(&mut self, code: KeyCode) {
        if self.views[self.tab].takes_lines() { self.key_lines(code); return; }
        let count = self.views.len();
        match code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Tab | KeyCode::Right => { self.tab = (self.tab + 1) % count; self.clamp(); }
            KeyCode::BackTab | KeyCode::Left => { self.tab = (self.tab + count - 1) % count; self.clamp(); }
            KeyCode::Down | KeyCode::Char('j') => { self.selected[self.tab] += 1; self.clamp(); }
            KeyCode::Up | KeyCode::Char('k') => { self.selected[self.tab] = self.selected[self.tab].saturating_sub(1); }
            KeyCode::Char(c) => {
                if let Some(a) = self.views[self.tab].actions().into_iter().find(|a| a.key == c) {
                    self.start_action(a);
                }
            }
            _ => {}
        }
    }

    fn key_lines(&mut self, code: KeyCode) {
        let count = self.views.len();
        match code {
            KeyCode::Tab | KeyCode::Right => { self.tab = (self.tab + 1) % count; self.clamp(); }
            KeyCode::BackTab | KeyCode::Left => { self.tab = (self.tab + count - 1) % count; self.clamp(); }
            KeyCode::Esc => self.line_buffer.clear(),
            KeyCode::Enter => {
                let text = std::mem::take(&mut self.line_buffer);
                if let Some(msg) = self.views[self.tab].line_input(&mut self.core, &text) {
                    self.status = msg;
                }
            }
            KeyCode::Backspace => { self.line_buffer.pop(); }
            KeyCode::Char(c) => self.line_buffer.push(c),
            _ => {}
        }
    }

    fn start_action(&mut self, action: Action) {
        let item = self.selected_id();
        if action.needs_item && item.is_none() { self.status = "Nothing selected.".into(); return; }
        if action.prompts.is_empty() {
            self.finish_action(self.tab, action, item, &[]);
        } else {
            self.mode = Mode::Input {
                view: self.tab, action, item, step: 0, answers: Vec::new(), buffer: String::new(),
            };
        }
    }

    fn finish_action(&mut self, view: usize, action: Action, item: Option<u64>, answers: &[String]) {
        self.status = self.views[view].apply(&mut self.core, action.key, item, answers);
        self.clamp();
    }

    fn key_input(&mut self, code: KeyCode, view: usize, action: Action, item: Option<u64>,
                 step: usize, mut answers: Vec<String>, mut buffer: String) {
        match code {
            KeyCode::Esc => { self.status = "Cancelled.".into(); return; }
            KeyCode::Char(c) => buffer.push(c),
            KeyCode::Backspace => { buffer.pop(); }
            KeyCode::Enter => {
                answers.push(buffer);
                let next = step + 1;
                if next >= action.prompts.len() {
                    self.finish_action(view, action, item, &answers);
                    return;
                }
                self.mode = Mode::Input { view, action, item, step: next, answers, buffer: String::new() };
                return;
            }
            _ => {}
        }
        self.mode = Mode::Input { view, action, item, step, answers, buffer };
    }

    fn draw(&self, f: &mut Frame) {
        let show_bar = self.views.get(self.tab).map_or(false, |v| v.shows_day_bar());
        let bar_h = if show_bar { 6 } else { 0 };
        let [top, body, daybar, bottom] = Layout::vertical([
            Constraint::Length(3), Constraint::Min(0),
            Constraint::Length(bar_h), Constraint::Length(3),
        ]).areas(f.area());

        let titles: Vec<&str> = self.views.iter().map(|v| v.name()).collect();
        f.render_widget(
            Tabs::new(titles).select(self.tab)
                .block(Block::bordered().title(" life "))
                .highlight_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            top);

        if self.views[self.tab].takes_lines() {
            self.draw_lines(f, body);
        } else {
            self.draw_view(f, body);
        }
        if show_bar { self.draw_day_bar(f, daybar); }

        let room = (bottom.width as usize).saturating_sub(6);
        let (title, line) = match &self.mode {
            Mode::Input { action, step, buffer, .. } => {
                let prompt = action.prompts[*step];
                (format!(" {} ({}/{}) ", action.label, step + 1, action.prompts.len()),
                 format!("{}: {}█", prompt, tail(buffer, room.saturating_sub(prompt.len() + 2))))
            }
            Mode::Normal => {
                if self.views[self.tab].takes_lines() {
                    (format!(" {} ", self.status),
                     "type | enter save | tab switch | ctrl-c quit".to_string())
                } else {
                    let mut hints: Vec<String> = self.views[self.tab].actions().iter()
                        .map(|a| format!("{} {}", a.key, a.label)).collect();
                    hints.push("tab switch".into());
                    hints.push("q quit".into());
                    (format!(" {} ", self.status), hints.join("  |  "))
                }
            }
        };
        f.render_widget(Paragraph::new(line).block(Block::bordered().title(title)), bottom);
    }

    fn draw_view(&self, f: &mut Frame, area: Rect) {
        let [left, right] = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(area);
        let v = &self.views[self.tab];
        let rows = v.rows(&self.core);
        let sel = self.selected[self.tab].min(rows.len().saturating_sub(1));

        let items: Vec<ListItem> = rows.iter().enumerate()
            .map(|(i, r)| {
                let prefix = if i == sel { "> " } else { "  " };
                ListItem::new(format!("{}{}", prefix, r.label))
            })
            .collect();
        let list = List::new(items)
            .block(Block::bordered().title(format!(" {} ", v.name())))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        let mut state = ListState::default().with_selected(if rows.is_empty() { None } else { Some(sel) });
        f.render_stateful_widget(list, left, &mut state);

        let detail = rows.get(sel).map(|r| v.detail(&self.core, r.id))
            .unwrap_or_else(|| "Nothing here.".to_string());
        f.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: false })
                .block(Block::bordered().title(" detail ")), right);
    }

    fn draw_lines(&self, f: &mut Frame, area: Rect) {
        let rows = self.views[self.tab].rows(&self.core);
        let [history, input] = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).areas(area);
        let room = history.height.saturating_sub(2) as usize;
        let items: Vec<ListItem> = rows.iter().rev().take(room).rev()
            .map(|r| ListItem::new(r.label.clone())).collect();
        f.render_widget(List::new(items).block(Block::bordered().title(" diary ")), history);

        let w = input.width as usize;
        f.render_widget(
            Paragraph::new(format!("{}█", tail(&self.line_buffer, w.saturating_sub(4))))
                .block(Block::bordered().title(" write | enter saves ")), input);
    }

    fn draw_day_bar(&self, f: &mut Frame, area: Rect) {
        let now = Local::now();
        let date = today();
        let title = format!(" {} · now {} ", date.format("%a %d %b"), now.format("%H:%M"));
        let block = Block::bordered().title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width < 12 || inner.height < 3 { return; }
        let width = inner.width as usize;
        let now_col = ((now.hour() as usize * 60 + now.minute() as usize) * width / (24 * 60)).min(width - 1);

        let mut paint: Vec<Option<Color>> = vec![None; width];
        for (s, e) in self.views[self.tab].day_blocks(&self.core, date) {
            let from = s as usize * width / (24 * 60);
            let to = ((e as usize * width + 24 * 60 - 1) / (24 * 60)).max(from + 1);
            for cell in paint.iter_mut().take(to.min(width)).skip(from) { *cell = Some(Color::Cyan); }
        }

        let spans: Vec<ratatui::text::Span> = (0..width).map(|col| {
            use ratatui::text::Span;
            if col == now_col { Span::styled("█", Style::new().fg(Color::Yellow)) }
            else if let Some(c) = paint[col] { Span::styled("█", Style::new().fg(c)) }
            else if col < now_col { Span::styled("▒", Style::new().fg(Color::Blue)) }
            else { Span::styled("░", Style::new().fg(Color::DarkGray)) }
        }).collect();

        let mut labels = vec![' '; width];
        for h in (0..=24usize).step_by(6) {
            let col = (h * width / 24).min(width - 2);
            for (i, ch) in format!("{:02}", h).chars().enumerate() { labels[col + i] = ch; }
        }
        let labels: String = labels.into_iter().collect();
        f.render_widget(Paragraph::new(vec![Line::from(spans), Line::from(labels)]), inner);
    }
}

// ============================================================
// 8. INBOX SHAPE (shared with panel)
// ============================================================

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct InboxLine {
    pub module: String,
    #[serde(default)] pub text: String,
    #[serde(default)] pub at: String,
}

// ============================================================
// 9. CLI + main
// ============================================================

fn main() -> ExitCode {
    match parse_cli() {
        Cli::Help => { print_help(); ExitCode::SUCCESS }
        Cli::TuiAndPanel => {
            if let Err(e) = panel::spawn_background(panel_addr()) {
                eprintln!("panel: {} (TUI only)", e);
            }
            run_tui()
        }
        Cli::TuiOnly => run_tui(),
        Cli::PanelOnly => panel::run_foreground(panel_addr()),
    }
}

fn panel_addr() -> String {
    std::env::var("PANEL_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string())
}

fn run_tui() -> ExitCode {
    let mut app = App::new();
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("error: {}", e); ExitCode::FAILURE }
    }
}

enum Cli { TuiAndPanel, TuiOnly, PanelOnly, Help }

fn parse_cli() -> Cli {
    let mut mode = Cli::TuiAndPanel;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--no-panel" => mode = Cli::TuiOnly,
            "--no-tui" | "--panel-only" => mode = Cli::PanelOnly,
            "-h" | "--help" => return Cli::Help,
            other => { eprintln!("unknown arg: {}", other); return Cli::Help; }
        }
    }
    mode
}

fn print_help() {
    println!("dortyuzlu\n");
    println!("  dortyuzlu             TUI + panel on :3000");
    println!("  dortyuzlu --no-panel  TUI only");
    println!("  dortyuzlu --no-tui    panel only");
}

// ============================================================
// 10. TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_round_trip() {
        let it = mk_item("test".into(), vec!["x".into()], ItemKind::Task, None);
        let s = serde_json::to_string(&it).unwrap();
        let back: Item = serde_json::from_str(&s).unwrap();
        assert_eq!(back.title, "test");
        assert!(matches!(back.kind, ItemKind::Task));
    }

    #[test]
    fn habit_streak_counts_today() {
        let mut h = mk_item("read".into(), vec![], ItemKind::HabitMake { target: 5 }, None);
        h.progress.insert(today(), 5);
        assert!(h.habit_good_on(today()));
        assert_eq!(h.habit_streak(), 1);
    }

    #[test]
    fn intents_parse() {
        use Intent::*;
        assert!(matches!(parse_intents("todo: buy milk").as_slice(),
            [Todo { title }] if title == "buy milk"));
        assert!(matches!(parse_intents("finished laundry").as_slice(),
            [Done { title }] if title == "laundry"));
    }

    #[test]
    fn timetable_parses() {
        let got = parse_timetable("14:00-15:30 Dentist").unwrap();
        assert_eq!(got[0], "Dentist");
        assert_eq!(got[2], "14:00");
    }
}
