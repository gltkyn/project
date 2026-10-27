//! project
//!
//! A modular, interconnected life-management system built with Rust and Ratatui.
//! Activities share underlying data and connect tasks, habits, diary entries,
//! scheduling, and completion history around a central timeline.
//!
//! Dependencies (Cargo.toml):
//!   serde = { version = "1", features = ["derive"] }
//!   serde_json = "1"
//!   chrono = "0.4"
//!   ratatui = "0.29"      (0.28 or newer)
//! Run the tests with `cargo test`. Data files are written next to where you run it.
//!
//! ============================================================
//! HOW THE CODE IS BUILT (read this before adding a tab)
//! ============================================================
//!
//! Entry      what is stored: id, title, optional body and tags.
//! Store<T>   one JSON file of entries. Load, save, ids. Tabs on the same file share one.
//! Record     a kind of entry: how it is typed in, shown, and its special keys.
//! ListModule a generic tab for any Record: add / edit / delete, views, tag filter,
//!            search and day-bar blocks. The "lens" number lets several tabs show
//!            one file differently (todo / done / cubes all show todos.json).
//! Module     what the app sees of a tab. ListModule, CalendarModule and AiModule
//!            implement it.
//! Request    a tab asking another tab to press a key. How approved AI suggestions run.
//! Advisor    anything that proposes actions to the AI tab (built-in Rules, later a
//!            local model).
//!
//! A new tab = `impl Entry` + `impl Record` for its struct, plus one line in App::new.
//!
//! Quick entry, in every "(#tag)" prompt:  Buy milk #errands @quick
//!   #word  is a tag.   @group  (todos only) is main, quick, steady, resurface,
//!   secondary or serotonin.   Search with "#tag" to find by tag.
//!
//! ============================================================
//! DONE: — IMPLEMENTED IN THE LAST CODE (NOT YET VERIFIED)
//! ============================================================
//!
//! NOTE: nothing below has been compiled or run yet. Run `cargo check`, then
//! `cargo test`, and fix what the compiler finds before trusting it.
//!
//! CORE APPLICATION
//! [x] Ratatui terminal interface.
//! [x] Modular architecture using a shared Module interface.
//! [x] Tab-based navigation between modules.
//! [x] Shared logging system (history.jsonl; every tab writes to it).
//! [x] JSON-based persistent storage (written to a temp file and renamed, and a
//!     file that can't be read is kept as <name>.corrupt instead of crashing).
//! [x] Fuzzy search across all tabs, with #tag search.
//! [x] Shared interface for listing items, viewing details, and actions.
//! [x] Generic Record / ListModule: add, edit, delete, views and tag filter are
//!     written once for every tab instead of once per tab.
//! [x] Lists can be grouped under headings.
//!
//! TODO TAB, GROUPS AND FOCUS
//! [x] Todos belong to a group: main, quick (easily done), steady (needs a little
//!     work every day), resurface (comes back every day), secondary, serotonin.
//! [x] The todo tab opens on "focus" (hides secondary and serotonin) and says how
//!     many are hidden. `v` cycles focus / all / one group at a time.
//! [x] Todos older than a week show their age.
//! [x] Resurface todos reopen themselves the day after you finish them.
//! [x] Done tab: newest first, with views today / week / all.
//!
//! TAGS
//! [x] #tags on todos, habits, dailies, dictionary pages, events and diary lines.
//! [x] `t` filters a tab by one tag. Search "#tag" finds a tag across all tabs.
//!
//! CUBES TAB
//! [x] Open todos shown as blocks with a size (m) and a time on the timeline (s).
//! [x] Placed cubes are painted on the day bar for their day (green).
//!
//! DAILY TAB
//! [x] Dailies repeat on chosen weekdays: daily, weekdays, weekend, mon-fri, mon,wed...
//! [x] Duration and specifications; can be put on the day bar at a start time (s).
//! [x] Crossing midnight works (sleep 23:00 for 7 hours shows on both days).
//! [x] Today's dailies are listed first; ticking one off counts for today only.
//!
//! HABIT TAB (BUILD AND QUIT)
//! [x] Habits to build ("at least N a day") and habits to quit ("at most N a day").
//! [x] + counts one today, - takes one back; missed days count as zero by themselves.
//! [x] Streak, best streak, last 30 days, last 14 days at a glance.
//!
//! AGENDA, CALENDAR AND DAY BAR
//! [x] Agenda: upcoming events by day, views upcoming / past / all.
//! [x] Events can run past midnight (end earlier than start = ends the next day).
//! [x] Calendar: a month grid; the day bar shows the SELECTED day.
//! [x] Day bar: 24 hours, passed time shaded, planned time coloured by kind
//!     (event, daily, cube), "now" marked, legend underneath.
//!
//! DIARY
//! [x] One line per Enter press, timestamped, grouped by day, #words become tags.
//!
//! AI TAB (ASKS BEFORE EVERY STEP)
//! [x] The assistant only proposes; every proposal waits for your yes or no.
//! [x] Yes runs the action like a key press on the target tab and logs it; No is
//!     remembered and never asked again. proposals.json keeps every answer.
//! [x] Built-in rules: ask to mark a cube done once its time is over; ask to move a
//!     main todo to secondary after 30 days.
//! [x] Advisor trait: the one place a local model plugs in.
//!
//! ============================================================
//! TODO: — PLANNED FEATURES
//! ============================================================
//! 
//! NOTE: make them use,
//! [ ] for todo
//! [x] for done 
//! [c] for compiled
//! [w] for written
//! [...] for etc
//!
//! 0. CHECK THE LAST CODE FIRST
//! [ ] Run cargo check and fix compilation errors (nothing here has been compiled).
//! [ ] Run cargo test; add tests for the day-bar painting and the AI flow.
//! [ ] Try a real day: sleep across midnight, a habit streak, a resurfacing todo.
//!
//! 1. TIMELINE AND ACTIVITY CUBES
//! [ ] Build the main timeline as the central application view.
//! [ ] Display activity names and relevant information inside blocks.
//! [ ] Support overlapping activities and scheduling conflicts.
//! [ ] Preserve historical timelines for past days (the bar shows past days'
//!     plans, but not what really happened).
//! [ ] Cubes that cross midnight listed on both days in the cubes tab.
//!
//! 2. DRAG-AND-DROP INTERACTION
//! [ ] Support mouse interaction with timeline blocks.
//! [ ] Drag activities to different times.
//! [ ] Resize blocks to change activity duration.
//! [ ] Move activities between dates.
//! [ ] Update the underlying schedule when a block moves.
//! [ ] Display clear feedback when activities conflict.
//!
//! 3. AUTOMATIC COMPLETION AND TIME TRACKING
//! [ ] Check the current time when the application starts (ticks run now; they
//!     only reopen resurface todos and ask the AI questions).
//! [ ] Detect activities whose scheduled end times have passed (done for cubes:
//!     the AI asks; not yet for events and dailies).
//! [ ] Apply each activity's completion rules; automatically move eligible
//!     activities from Todo to Done without asking, when the user allows it.
//! [ ] Process missed transitions after the application was closed.
//! [ ] Avoid completing the same activity more than once.
//! [ ] Distinguish time-based completion from activities requiring manual
//!     confirmation.
//! [ ] Handle activities crossing midnight.
//!
//! Example:
//! Sleep is scheduled from 23:00 on June 3 until 06:00 on June 4.
//! When the app opens after 06:00 on June 4, it recognizes the end
//! of the sleep period and moves that occurrence to Done, preserving
//! its original scheduled date and times.
//!
//! 4. INTERCONNECTED MODULES
//! [ ] Establish a shared underlying activity model (todos, cubes, dailies and
//!     events are still four separate kinds of entry).
//! [ ] Ensure modules reference shared activities instead of duplicating them.
//! [ ] Define how changes in one module affect other modules.
//! [ ] Connect Todo and Done to the timeline (done through cubes).
//! [ ] Connect Agenda and Calendar to shared scheduling data (they share
//!     events.json; dailies and cubes are not in the agenda yet).
//! [ ] Connect Habit tracking to scheduled and completed activities (a daily
//!     that is ticked off should count a habit).
//! [ ] Allow diary entries to reference relevant activities when useful.
//! [ ] Keep module views synchronized when an activity changes.
//! [ ] Maintain consistent IDs, dates, and history across modules.
//!
//! 5. HABIT SYSTEM
//! [ ] Weekly targets and schedules (three times a week, not every day).
//! [ ] Time-of-day targets, such as waking at 06:00 (record the actual time).
//! [ ] Distinguish skipped habits from missed ones.
//! [ ] Connect habits to relevant timeline activities.
//! [ ] Weekly consistency view.
//!
//! 6. TODO LIST CALM
//! [ ] Snooze a todo until a date, so it leaves the list and comes back.
//! [ ] A tag / group overview screen with counts.
//! [ ] Tags should make the todos grouped and bearable.
//! [ ] Bulk actions: move all quick todos, tag several at once.
//! [ ] Show only what fits today by comparing cube sizes with free time.
//!
//! 7. LOCAL AI THAT KNOWS YOUR WRITING
//! [ ] Run a local, non-commercial model on the machine; nothing leaves it.
//! [ ] Train or prompt it on the diary, todos and history.jsonl (your own writing).
//! [ ] Implement Advisor for it, so its ideas arrive as yes/no questions.
//! [ ] Learn from proposals.json (every yes and no is a labelled example).
//! [ ] More rules and ideas: suggest groups and tags for new todos, suggest a time
//!     slot for a cube, notice a habit slipping, draft diary lines from the day.
//! [ ] A "why" the AI can explain, and a way to tell it "never ask about this".
//!
//! 8. DIARY INTEGRATION
//! [ ] Associate diary entries with dates and relevant activities.
//! [ ] Review diary entries alongside the timeline.
//! [ ] Optionally link diary entries to habits or tasks.
//! [ ] Edit or delete a diary line.
//!
//! 9. PERSISTENCE AND RELIABILITY
//! [ ] Encrypt data at rest ("blackbox").
//! [ ] Recover correctly after closing and restarting the app.
//! [ ] Prevent duplicate completion records.
//! [ ] Test persistence and automatic state transitions.
//! [ ] Split the single file into modules (core, records, tabs, ui) once it settles.
//!
//! 10. FINAL INTEGRATION AND TESTING
//! [ ] Define the responsibilities and data relationships of all modules.
//! [ ] Implement central timeline and cube rendering.
//! [ ] Implement drag-and-drop scheduling.
//! [ ] Implement automatic completion rules.
//! [ ] Test interactions across connected modules.
//! [ ] Test realistic daily usage, including sleep, habits, and missed schedules.
//!
//! 11. USER'S SCATTERED LISTS ON SOCIAL MEDIA.
//! [ ] Get a list of users online saves 
//! [ ] Find a way to get them melted down.
//! [ ] like pinterest pins, youtube watchlists, movie watchlists.
//!
//! 12. ADD MORE ENTRY METHODS AND TYPES
//! [ ] Mobile entry taking.
//! [ ] web browser locale entry taking.
//! [ ] Tui entry taking.
//! [ ] Add a sync for all of them.
//! [ ] 
//!
//! 13. MAKE THE APP ALIVE AND AFFECTIVE
//! [ ] 
//! [ ] 
//!
//! 14. ADD WEEKLY REVIEW
//! [ ] 
//!
//! 15. ADD FLOATING MASCOT
//! [ ] 
//!
//! 16. ADD VERSIONED DATA SAVING ON JSON
//! [ ] 
//!
//! 17. WRITE BETTER TESTS
//! [ ] 
//!
//! ============================================================
//! ARCHITECTURAL PRIORITY
//! ============================================================
//!
//! Design the shared activity data model before the main timeline view.
//! Define what an activity is, when it starts and ends, how it repeats,
//! and what causes it to move from Todo to Done. Today a todo (with a cube
//! size and slot), a daily and an event each store their own time; the next
//! step is one shared "activity" they all point to.
//!
//! The timeline and all modules should operate on the same underlying
//! data rather than becoming separate systems that must be synchronized.

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

// "09:30" or "9:30" -> minutes since midnight
fn minutes_of(text: &str) -> Option<u32> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M")
        .ok()
        .map(|t| t.hour() * 60 + t.minute())
}

fn clock(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
}

// "Sat 24 Oct · today"
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

// Where does something that starts on `from` at minute `start` and lasts `len`
// minutes fall on the day `on`? Handles crossing midnight (and several days).
// Returns (start, end) in minutes since midnight of `on`.
fn span_on(from: NaiveDate, start: u32, len: u32, on: NaiveDate) -> Option<(u32, u32)> {
    let a = (on - from).num_days() * 1440; // minutes from `from` 00:00 to `on` 00:00
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

// Turns a sort key upside down so the newest sorts first: digits are mirrored.
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

// "Buy milk #errands @quick" -> ("Buy milk", ["errands"], Some("quick"))
// Words starting with # are tags. With `groups` on, a word starting with @ is a group.
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

// The reverse of split_text, used to pre-fill the edit box.
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

// All #words of a free-form line (the diary uses this).
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

// ===========================================================================
// 2. STORAGE: Entry (what is stored), Store<T> (the file), Logger (history)
// ===========================================================================

// All data lives in one folder shared by every part of the system: this app,
// the browser panel (src/bin/panel.rs) and whatever comes next.
// The default is ~/dortyuzlu; DORTYUZLU_DIR overrides it.
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

// Before the shared folder existed, files were kept wherever the app was
// started. The first time, copy such a file over (the old copy stays untouched).
fn adopt_old_file(name: &str) {
    let target = data_path(name);
    if !target.exists() && Path::new(name).is_file() {
        let _ = fs::copy(name, &target);
    }
}

trait Entry: Serialize + DeserializeOwned + Clone {
    const FILE: &'static str; // "todos.json", "pages.json"
    fn id(&self) -> u64;
    fn title(&self) -> &str;
    fn body(&self) -> &str {
        "" // default: nothing extra to search
    }
    fn tags(&self) -> &[String] {
        &[] // default: this kind of entry has no tags
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
                // never crash on, or overwrite, a file we can't read: keep it aside
                let aside = data_path(&format!("{}.corrupt", T::FILE));
                let _ = fs::rename(&path, &aside);
                eprintln!("{} could not be read; it was kept as {}", path.display(), aside.display());
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        Store { items }
    }
    fn save(&self) {
        let text = serde_json::to_string_pretty(&self.items).expect("serialize failed");
        // write beside, then rename: a crash mid-write can't leave half a file
        let temp = data_path(&format!("{}.tmp", T::FILE));
        fs::write(&temp, text).expect("write failed");
        fs::rename(&temp, data_path(T::FILE)).expect("rename failed");
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

// Several tabs can be views of one file; they share one Store.
#[allow(type_alias_bounds)]
type Shared<T: Entry> = Rc<RefCell<Store<T>>>;

fn share<T: Entry>() -> Shared<T> {
    Rc::new(RefCell::new(Store::load()))
}

// One history file for every part: history.jsonl

#[derive(Clone, Serialize, Deserialize)]
enum Kind {
    Added,
    Removed,
    Done,
    Reopened,
    Edited,
    Counted,  // a habit was counted
    Approved, // the user said yes to a proposal
    Rejected, // the user said no
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

// One scoring rule for every kind of entry. "#tag" searches tags only;
// otherwise a title match counts double a body or tag match.
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
// 4. MODULE: what a "part" (a tab) is
// ===========================================================================

struct Row {
    id: u64,
    label: String,
    group: String, // rows with the same non-empty group sit under one heading
}

impl Row {
    fn new(id: u64, label: String) -> Row {
        Row { id, label, group: String::new() }
    }
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

// A module asking another module to do something. It runs exactly like a key
// press on that tab, so a module can never do more than the user could.
struct Request {
    module: String,
    key: char,
    item: Option<u64>,
    input: Vec<String>,
}

trait Module {
    fn name(&self) -> &'static str;
    // Title of the list box (name plus the current view and filters).
    fn heading(&self) -> String {
        self.name().to_string()
    }
    // Should the day bar (24h strip with a "now" marker) show on this tab?
    fn shows_day_bar(&self) -> bool {
        false
    }
    // Which day the bar shows. None = today. `selected` is the selected row.
    fn bar_date(&self, _selected: Option<u64>) -> Option<NaiveDate> {
        None
    }
    // Stretches of the given day this part wants painted on the day bar,
    // as (start, end) in minutes since midnight.
    fn day_blocks(&self, _date: NaiveDate) -> Vec<(u32, u32)> {
        Vec::new()
    }
    fn block_color(&self) -> Color {
        Color::Magenta
    }
    // Which row is selected when the tab is first opened.
    fn initial_row(&self) -> usize {
        0
    }
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
    fn search(&self, _query: &str) -> Vec<(i32, Row)> {
        Vec::new()
    }
    // The search tab opened one of our rows: make sure no filter hides it.
    fn reveal(&mut self, _id: u64) {}
    // Called at start-up and every half minute: time-based housekeeping.
    fn tick(&mut self, _log: &mut Logger) {}
    // Requests this module wants other modules to run (see Request).
    fn outbox(&mut self) -> Vec<Request> {
        Vec::new()
    }
    // A tab like the diary takes whole lines typed at the bottom instead of commands.
    fn takes_lines(&self) -> bool {
        false
    }
    // `at` is when it was written ("YYYY-MM-DD HH:MM"), or empty for now. The
    // browser panel can hand in a line long after it was typed.
    fn line_input(&mut self, _text: &str, _at: &str, _log: &mut Logger) -> Option<String> {
        None
    }
    // Adds a new entry from typed answers, like the `a` key. The browser
    // panel's todos and timetable entries come in this way.
    fn add(&mut self, _input: &[String], _log: &mut Logger) -> Result<String, String> {
        Err("This tab doesn't take new entries.".to_string())
    }
}

// ===========================================================================
// 5. RECORD + ListModule: the part that used to be copy-pasted per tab
//
// A kind of entry (todo, habit, page, event...) implements Record: how it is
// typed in, how it is shown, and its special keys. ListModule<T> then gives
// it a tab with add / edit / delete, views, tag filter, search and day-bar
// blocks, so a new tab is one `impl Record` plus one line in App::new.
//
// `lens` lets several tabs look at one file differently (todo / done / cubes).
// ===========================================================================

type Outcome = Result<(Kind, String), String>; // Ok = (what to log, message), Err = message

trait Record: Entry {
    const NOUN: &'static str; // "todo": used in messages and as the log name
    const TAGGED: bool = false; // offer the tag filter on this tab?
    const LINES: bool = false; // is this tab a typed-line log (diary)?
    const COLOR: Color = Color::Magenta; // colour of its blocks on the day bar

    // ----- typing things in -----
    fn prompts() -> &'static [&'static str];
    // The current values as text, one per prompt (fills the edit box).
    fn fields(&self) -> Vec<String>;
    // Builds an entry from the typed answers. `old` is Some when editing,
    // so values that aren't typed (counts, dates...) can be kept.
    fn build(id: u64, input: &[String], old: Option<&Self>) -> Result<Self, String>;
    // `at` = "YYYY-MM-DD HH:MM" of when it was written, empty = now
    fn from_line(_id: u64, _text: &str, _at: &str) -> Option<Self> {
        None
    }

    // ----- showing things -----
    fn label(&self, lens: u8) -> String;
    fn detail(&self, lens: u8) -> String;
    fn in_lens(&self, _lens: u8) -> bool {
        true // does this tab show this entry at all?
    }
    fn views(_lens: u8) -> &'static [&'static str] {
        &[] // names the `v` key cycles through
    }
    fn shown(&self, _lens: u8, _view: &str) -> bool {
        true // is it visible in the current view?
    }
    fn group(&self, _lens: u8) -> String {
        String::new() // heading it sits under
    }
    fn order(&self, _lens: u8) -> String {
        String::new() // sort key, ascending; must start with the group's rank
    }

    // ----- keys -----
    fn keys(_lens: u8) -> &'static str {
        "aed" // which of add / edit / delete this tab offers
    }
    fn extra_actions(_lens: u8) -> Vec<Action> {
        Vec::new()
    }
    // Runs a special key on one entry. None = not one of its keys.
    fn act(&mut self, _lens: u8, _key: char, _input: &[String]) -> Option<Outcome> {
        None
    }
    fn prefill(&self, _key: char, _step: usize) -> String {
        String::new()
    }

    // ----- time -----
    fn blocks(&self, _lens: u8, _date: NaiveDate) -> Vec<(u32, u32)> {
        Vec::new()
    }
    // Housekeeping. Returns true if something changed (so it gets saved).
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

    // Adding is its own method (the `a` key and the browser panel both use it).
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
        list.sort_by(|a, b| a.0.cmp(&b.0)); // stable: ties keep file order
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

        // the two keys that change what the tab shows, not the data
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
// 6. TODO: three tabs on one file
//      lens 0 = "todo"   open todos, grouped, with a calm "focus" view
//      lens 1 = "done"   finished todos, newest first
//      lens 2 = "cubes"  open todos as sized blocks that can be put on the timeline
// ===========================================================================

// Every todo sits in one group. The groups keep a long list calm: the todo tab
// opens on "focus", which hides the groups that don't need attention today.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum Group {
    #[default]
    Main, // the real list
    Quick,     // easily done
    Steady,    // needs a little work every day
    Resurface, // comes back every day, reopens itself after you finish it
    Secondary, // can wait
    Serotonin, // small joys
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

    // Exact word, or a prefix that only one group starts with ("sero", "q", "res").
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
    added: String, // "YYYY-MM-DD"; todos from before this field get today's date once
    #[serde(default)]
    done_on: String, // "YYYY-MM-DD" it was finished
    #[serde(default)]
    minutes: u32, // cube size in minutes, 0 = not sized yet
    #[serde(default)]
    slot: String, // "YYYY-MM-DD HH:MM" when placed on the timeline, empty = not placed
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

    // (day, start minute) when the cube is on the timeline
    fn placed(&self) -> Option<(NaiveDate, u32)> {
        let (date, time) = self.slot.split_once(' ')?;
        Some((parse_date(date)?, minutes_of(time)?))
    }

    // an unsized cube counts as half an hour
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

// "18:30" means today at 18:30; "2026-10-30 18:30" names the day.
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
        // no @group typed means main; the edit box shows @group, so removing it resets it
        let group = match group {
            Some(word) => Group::parse(&word)
                .ok_or_else(|| format!("No group @{}. Try quick, steady, resurface, secondary, serotonin.", word))?,
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
                Action { key: 'm', label: "size", needs_item: true, prompts: &["Size in minutes (empty = unsized)"] },
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
                            self.minutes = 30; // a cube on the timeline needs a size
                        }
                        Some(Ok((Kind::Edited, format!("Placed at {}.", self.slot))))
                    }
                    Err(message) => Some(Err(message)),
                }
            }
            _ => None,
        }
    }

    // a cube is painted on the day it is placed on (and the next, if it runs past midnight)
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
            self.added = today_text(); // age counting starts the first time it is seen
            changed = true;
        }
        // a resurfacing todo opens again on the day after you finished it
        if self.group == Group::Resurface && self.is_done && self.done_on != today_text() {
            self.is_done = false;
            changed = true;
        }
        changed
    }
}

// ===========================================================================
// 7. HABITS: ones to build and ones to quit, counted per day
//    Build:  "read 20 pages"      target = at least this many a day
//    Quit:   "smoking, max 5"     target = at most this many a day (0 = none)
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
    unit: String, // "glasses", "cigarettes": only for display
    started: String, // "YYYY-MM-DD": streaks never count days before this
    #[serde(default)]
    counts: BTreeMap<String, u32>, // "YYYY-MM-DD" -> how many times that day
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

    // Did that day go well? Build: reached the target. Quit: stayed within the limit.
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

    // Good days in a row, counting back from today. A build-habit not yet done
    // today is still open, so it neither counts nor breaks the streak.
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

    // (good days, counted days) over the last 30 days; an open day isn't counted
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

    // the last `n` days, oldest first: ✓ good, ✗ missed or slipped, ○ still open, · before it started
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
                    HabitKind::Make if n >= self.target => format!("{}/{} today. Done!", n, self.target),
                    HabitKind::Make => format!("{}/{} today.", n, self.target),
                    HabitKind::Quit if n > self.target => {
                        format!("{} today, over the limit of {}. Streak reset.", n, self.target)
                    }
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
// 8. DAILY: things you do on set weekdays (sleep, school...), with a duration,
//    that can sit on the day bar
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

// "daily", "weekdays", "weekend", "mon-fri", "mon,wed,fri" -> sorted day numbers (0 = Monday)
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
                day = (day + 1) % 7; // mon-sun and fri-mon both work
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
    duration: u32, // minutes
    specs: String, // free-form details: what exactly, where, with what
    #[serde(default)]
    start: String, // "HH:MM" when it sits on the day bar, empty = not on the bar
    #[serde(default)]
    done_dates: Vec<String>, // days ("YYYY-MM-DD") it was ticked off
    #[serde(default = "all_days")]
    days: Vec<u8>, // weekdays it happens on, 0 = Monday ... 6 = Sunday
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
        &self.specs // searchable too
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
                        self.done_dates.remove(pos); // pressing again undoes it
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

    // On every day it is planned for, from its start time for its duration.
    // Yesterday is checked too, so sleep from 23:00 shows up after midnight.
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
// 10. CALENDAR EVENTS: one events.json, shown by the agenda and calendar tabs
// ===========================================================================

#[derive(Clone, Serialize, Deserialize)]
struct CalEvent {
    id: u64,
    title: String,
    date: String,  // "YYYY-MM-DD" it starts on
    start: String, // "HH:MM"
    end: String,   // "HH:MM"; earlier than start means it ends the next day
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
// 11. DIARY: one persistent line per Enter press; #words in a line become tags
// ===========================================================================

#[derive(Clone, Serialize, Deserialize)]
struct DiaryLine {
    id: u64,
    date: String, // "YYYY-MM-DD HH:MM"
    text: String,
    #[serde(default)]
    tags: Vec<String>,
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
        })
    }

    fn label(&self, _lens: u8) -> String {
        format!("{}  {}", self.date.get(11..).unwrap_or(""), self.text)
    }

    fn detail(&self, _lens: u8) -> String {
        format!("{}\n\n{}", self.date, self.text)
    }

    fn group(&self, _lens: u8) -> String {
        self.date
            .get(..10)
            .and_then(parse_date)
            .map(day_heading)
            .unwrap_or_default()
    }

    fn order(&self, _lens: u8) -> String {
        format!("{}{:012}", self.date, self.id)
    }

    fn keys(_lens: u8) -> &'static str {
        "" // lines are written in the box, not added, edited or deleted
    }
}

// ===========================================================================
// 12. CALENDAR: a month, one row per day. The day bar shows the SELECTED day.
// ===========================================================================

struct CalendarModule {
    events: Shared<CalEvent>,
    month_offset: i32, // 0 = this month, 1 = next month, -1 = last month
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

    // events that touch one date (including ones that started the evening before)
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
        today().day0() as usize // start on today
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

    // the month as a small grid, then the events of the selected day
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
                needs_item: true, // adds to the selected day
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
// 13. THE AI TAB
//
// The assistant never changes anything on its own. It writes proposals, one
// question each. Yes runs the proposed action exactly as if you had pressed
// the key yourself (so it is logged the same way). No is remembered, so it is
// never asked again. proposals.json is a record of every yes and no.
//
// An Advisor is anything that looks at your data and proposes actions. Rules
// is the built-in one. A local model trained on your own writing plugs in here
// by implementing Advisor; nothing else has to change.
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
    key: String,    // identifies the idea, so the same one is never asked twice
    ask: String,    // the question: "Mark X as done?"
    why: String,    // what the assistant noticed
    module: String, // which tab the action runs on
    action: char,   // which key it presses there
    item: Option<u64>,
    input: Vec<String>, // what it types for that key's prompts
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
    fn advise(&self, todos: &[Todo], now: NaiveDateTime) -> Vec<Idea>;
}

// Plain rules, no model needed.
struct Rules;

impl Advisor for Rules {
    fn advise(&self, todos: &[Todo], now: NaiveDateTime) -> Vec<Idea> {
        let mut ideas = Vec::new();
        for todo in todos.iter().filter(|t| !t.is_done) {
            // a cube whose time on the timeline is over
            if let Some(end) = todo.slot_end() {
                if end <= now {
                    ideas.push(Idea {
                        key: format!("cube-ended:{}:{}", todo.id, todo.slot),
                        ask: format!("Mark \"{}\" as done?", todo.header),
                        why: format!("It was on the timeline at {} and that time is over.", todo.slot),
                        module: "todo",
                        action: 'x',
                        item: Some(todo.id),
                        input: Vec::new(),
                    });
                }
            }
            // a main todo that has sat for a month
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
        ideas
    }
}

struct AiModule {
    store: Store<Proposal>,
    todos: Shared<Todo>,
    advisors: Vec<Box<dyn Advisor>>,
    out: Vec<Request>, // approved actions waiting to be run by the app
}

impl AiModule {
    fn new(todos: &Shared<Todo>) -> Self {
        AiModule {
            store: Store::load(),
            todos: Rc::clone(todos),
            advisors: vec![Box::new(Rules)],
            out: Vec::new(),
        }
    }

    // Asks every advisor for ideas. An idea already asked before (yes, no, or
    // still waiting) is skipped. Returns how many new questions were added.
    fn think(&mut self) -> usize {
        let now = Local::now().naive_local();
        let mut ideas: Vec<Idea> = Vec::new();
        {
            let todos = self.todos.borrow();
            for advisor in &self.advisors {
                ideas.extend(advisor.advise(&todos.items, now));
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
}

impl Module for AiModule {
    fn name(&self) -> &'static str {
        "ai"
    }

    fn heading(&self) -> String {
        let waiting = self.store.items.iter().filter(|p| p.status == Verdict::Pending).count();
        format!("ai · {} waiting for you", waiting)
    }

    fn rows(&self) -> Vec<Row> {
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
                let row = Row {
                    id: p.id,
                    label: format!("[{}] {}", mark, p.ask),
                    group: group.to_string(),
                };
                (order, row)
            })
            .collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list.into_iter().map(|(_, row)| row).collect()
    }

    fn detail(&self, id: u64) -> String {
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
        vec![
            Action { key: 'y', label: "yes", needs_item: true, prompts: &[] },
            Action { key: 'n', label: "no", needs_item: true, prompts: &[] },
            Action { key: 'r', label: "think now", needs_item: false, prompts: &[] },
        ]
    }

    fn apply(&mut self, key: char, id: Option<u64>, _input: &[String], log: &mut Logger) -> String {
        match key {
            'r' => {
                let added = self.think();
                if added == 0 {
                    "Nothing new to suggest.".to_string()
                } else {
                    format!("{} new question(s).", added)
                }
            }
            'y' | 'n' => {
                let Some(id) = id else { return "Nothing selected.".to_string() };
                let (module, action, item, input, title) = {
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
                    self.out.push(Request { module, key: action, item, input });
                    log.log("ai", Kind::Approved, id, &title);
                    format!("Yes: {}", title)
                } else {
                    log.log("ai", Kind::Rejected, id, &title);
                    format!("No: {} (won't be asked again)", title)
                }
            }
            _ => String::new(),
        }
    }

    fn tick(&mut self, _log: &mut Logger) {
        self.think();
    }

    fn outbox(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.out)
    }
}

// ===========================================================================
// 14. THE RATATUI INTERFACE
// ===========================================================================

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
}

struct App {
    modules: Vec<Box<dyn Module>>,
    log: Logger,
    tab: usize, // 0..modules.len() are the parts; modules.len() is the search tab
    selected: Vec<usize>, // selected row per module
    search_query: String,
    search_hits: Vec<SearchHit>,
    search_sel: usize,
    mode: Mode,
    status: String,
    line_buffer: String, // what is being typed on a line tab (the diary)
    quit: bool,
}

// One line of inbox.jsonl, as written by the browser panel.
#[derive(Clone, Serialize, Deserialize)]
struct InboxLine {
    module: String, // "diary", "todo" or "timetable"
    text: String,
    #[serde(default)]
    at: String, // "YYYY-MM-DD HH:MM" it was typed
    #[serde(default)]
    why: String, // filled in only when the line was refused
}

// Lines that couldn't be added are kept, with the reason, instead of being lost.
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

// "14:00-15:30 Dentist #health" or "2026-10-30 14:00-15:30 Dentist"
// -> [title, date, start, end], the answers the agenda's add asks for.
// No date means today. Anything else isn't an event and returns None.
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

// the last `room` characters of what is being typed, so the cursor stays visible
fn tail(text: &str, room: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    chars[chars.len().saturating_sub(room)..].iter().collect()
}

impl App {
    fn new() -> Self {
        // Adding a tab = implement Record (or Module) for it + one line here.
        // Tabs that share a file share one store: todo, done and cubes look at
        // todos.json; agenda and calendar look at events.json.
        let todos: Shared<Todo> = share();
        let events: Shared<CalEvent> = share();
        let modules: Vec<Box<dyn Module>> = vec![
            Box::new(ListModule::new("todo", 0, &todos).with_bar()),
            Box::new(ListModule::new("done", 1, &todos).with_bar()),
            Box::new(ListModule::new("cubes", 2, &todos).with_bar()),
            Box::new(ListModule::new("daily", 0, &share::<Daily>()).with_bar()),
            Box::new(ListModule::new("habit", 0, &share::<Habit>())),
            Box::new(ListModule::new("agenda", 0, &events).with_bar()),
            Box::new(CalendarModule::new(&events)),
            Box::new(ListModule::new("diary", 0, &share::<DiaryLine>())),
            Box::new(ListModule::new("dict", 0, &share::<Page>())),
            Box::new(AiModule::new(&todos)),
        ];
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

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        self.tick_all(); // catch up on whatever happened while the app was closed
        self.check_inbox(); // and on whatever the browser panel left meanwhile
        let mut last_tick = Instant::now();
        let mut last_inbox = Instant::now();
        while !self.quit {
            terminal.draw(|f| self.draw(f))?;
            // wake up every 2s even without a key press: entries sent from the
            // browser show up, and the "now" marker moves
            let key_waiting = event::poll(Duration::from_secs(2))?;
            if last_inbox.elapsed() >= Duration::from_secs(2) {
                self.check_inbox();
                last_inbox = Instant::now();
            }
            if last_tick.elapsed() >= Duration::from_secs(30) {
                self.tick_all(); // time-based housekeeping
                last_tick = Instant::now();
            }
            if !key_waiting {
                continue;
            }
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

    // ----- the inbox: entries sent by other programs (the browser panel) -----
    //
    // The panel never touches our data files (two programs writing one file
    // would corrupt it). It only appends one line per entry to inbox.jsonl, and
    // this is the only place those lines are turned into real entries.

    fn check_inbox(&mut self) {
        let inbox = data_path("inbox.jsonl");
        let working = data_path("inbox.processing");
        // The inbox is renamed (an instant, atomic step) so lines the panel adds
        // while we work go into a fresh inbox. A leftover working file from a
        // crash is finished first.
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
                // not a time range we can read: keep it as a diary note, not lost
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

    // ----- housekeeping and requests between modules -----

    fn tick_all(&mut self) {
        for module in self.modules.iter_mut() {
            module.tick(&mut self.log);
        }
        self.flush();
        self.clamp_selection();
    }

    // Runs what modules asked other modules to do (an approved AI suggestion, say).
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

    // ----- selection helpers -----

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
            // data may have changed since the last visit, so search again
            self.search_hits = self.run_search(&self.search_query);
            self.search_sel = 0;
        } else {
            // another tab on the same file may have changed the list under us
            self.clamp_selection();
        }
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
        let count = self.modules.len() + 1; // the parts plus the search tab
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
                self.selected[self.tab] = usize::MAX; // clamped to the last row
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

    // A line tab (the diary): typing goes straight into the box at the bottom.
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

    // The search tab: typing goes straight into the query box, so letters
    // like q don't act as commands here. Leave with Tab/arrows or Esc.
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
                    self.modules[module].reveal(id); // drop any filter that hides it
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

    // ----- drawing -----

    fn draw(&self, f: &mut Frame) {
        // the day bar sits just above the key bar, only on tabs that ask for it
        let show_bar = self.modules.get(self.tab).map_or(false, |m| m.shows_day_bar());
        let bar_height = if show_bar { 7 } else { 0 };
        let [top, body, daybar, bottom] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(bar_height),
            Constraint::Length(3),
        ])
        .areas(f.area());

        // tabs
        let mut titles: Vec<&str> = self.modules.iter().map(|m| m.name()).collect();
        titles.push("search");
        let tabs = Tabs::new(titles)
            .select(self.tab)
            .block(Block::bordered().title(" life "))
            .highlight_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(tabs, top);

        // body
        match self.modules.get(self.tab) {
            None => self.draw_search(f, body),
            Some(m) if m.takes_lines() => self.draw_lines(f, body),
            Some(_) => self.draw_module(f, body),
        }
        if show_bar {
            self.draw_day_bar(f, daybar);
        }

        // bottom line: prompt, search box or key hints
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

    // The diary: history above, the line being written below, newest at the bottom.
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
        let skip = items.len().saturating_sub(room); // headings can push lines out of room
        let items: Vec<ListItem> = items.into_iter().skip(skip).collect();
        f.render_widget(List::new(items).block(Block::bordered().title(" diary ")), history);

        let space = (input_area.width as usize).saturating_sub(4);
        let input = Paragraph::new(format!("{}█", tail(&self.line_buffer, space))).block(
            Block::bordered().title(" write your story · Enter saves this line · #word makes a tag "),
        );
        f.render_widget(input, input_area);
    }

    fn draw_module(&self, f: &mut Frame, area: Rect) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(area);

        let module = &self.modules[self.tab];
        let rows = module.rows();
        let sel = self.selected[self.tab].min(rows.len().saturating_sub(1));

        // headings are extra, unselectable lines between the rows
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

    // A thick 24-hour strip like a sleep tracker. It shows the day the tab is
    // about (today, or the selected day in the calendar): time that has passed
    // is shaded, what is planned is coloured, and the current moment is marked.
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
            return; // terminal too small to draw it
        }

        let width = inner.width as usize;
        let minutes = (now.hour() * 60 + now.minute()) as usize;
        let now_col = (minutes * width / (24 * 60)).min(width - 1);

        // what is planned on that day: every tab can report blocks, in its own colour
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

        // row 0: a marker above the current time (only when looking at today)
        let marker = if is_today {
            Line::from(vec![
                Span::raw(" ".repeat(now_col)),
                Span::styled("▼", Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ])
        } else {
            Line::from("")
        };

        // rows 1-2: the bar itself, two rows thick
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

        // row 3: hour labels (every 3 hours, or every 6 when the terminal is narrow)
        let step = if width >= 32 { 3 } else { 6 };
        let mut labels = vec![' '; width];
        for h in (0..=24usize).step_by(step) {
            let col = (h * width / 24).min(width - 2);
            for (i, ch) in format!("{:02}", h).chars().enumerate() {
                labels[col + i] = ch;
            }
        }
        let labels: String = labels.into_iter().collect();

        // row 4: what the colours mean
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
        let mut state =
            ListState::default().with_selected(if hits.is_empty() { None } else { Some(self.search_sel) });
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

// ===========================================================================
// TESTS: the pure logic. Run with `cargo test`.
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
        // sleep 23:00 for 7 hours
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
        // where groups aren't wanted, @words stay in the title
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
        assert!(Group::parse("s").is_none()); // steady, secondary, serotonin
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
        assert!(parse_timetable("14:00-15:30").is_none()); // no title
        assert!(parse_timetable("25:00-26:00 Nope").is_none());
    }

    #[test]
    fn quit_habit_streak_starts_clean() {
        let input: Vec<String> = ["Smoking", "q", "", "cigarettes"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut habit = Habit::build(1, &input, None).expect("valid habit");
        assert_eq!(habit.streak(), 1); // today, nothing smoked yet
        assert!(habit.act(0, '+', &[]).is_some()); // one slip: over a limit of 0
        assert_eq!(habit.streak(), 0);
        assert!(habit.act(0, '-', &[]).is_some());
        assert_eq!(habit.streak(), 1);
    }
}
