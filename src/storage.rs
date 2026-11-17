//! File I/O, versioned schemas, migrations, counters.

use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::domain::{Message, TodoItem};
use crate::events::log_event;

pub const TODOS_FILE: &str = "todos.json";
pub const LOG_FILE: &str = "messages.log";
pub const EVENTS_FILE: &str = "events.log";
pub const COUNTERS_FILE: &str = "counters.json";

pub const DAILIES_FILE: &str = "dailies.json";
pub const DAILIES_VERSION: u32 = 1;

pub const TODOS_VERSION: u32 = 1;
pub const MESSAGES_VERSION: u32 = 1;
pub const COUNTERS_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct VersionedCounters {
    pub version: u32,
    pub next_todo: u64,
    pub next_msg: u64,
    #[serde(default)]
    pub next_daily: u64,
    pub next_evt: u64,
}

#[derive(Deserialize)]
pub struct LegacyCounters {
    pub next_todo: u64,
    pub next_msg: u64,
    #[serde(default)]
    pub next_daily: u64,
    pub next_evt: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Counters {
    pub next_todo: u64,
    pub next_msg: u64,
    pub next_daily: u64,
    pub next_evt: u64,
}

#[derive(Serialize, Deserialize)]
pub struct VersionedDailies {
    pub version: u32,
    pub items: Vec<crate::domain::DailyItem>,
}

// ── Atomic write + backups ──────────────────────────────────────

pub fn atomic_write(path: &str, content: &str) -> std::io::Result<()> {
    let tmp = format!("{path}.tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = Path::new(path).parent()
        && let Ok(dir) = File::open(parent)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

pub fn backup_file(path: &str, version: u32) -> std::io::Result<()> {
    let dst = format!("{path}.v{version}.bak");
    if !Path::new(&dst).exists() { fs::copy(path, &dst)?; }
    Ok(())
}

pub fn backup_corrupt(path: &str) -> std::io::Result<()> {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let dst = format!("{path}.corrupt-{ts}.bak");
    fs::copy(path, &dst)?;
    Ok(())
}

// ── Versioned shapes ────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct VersionedTodos {
    pub version: u32,
    pub items: Vec<TodoItem>,
}


pub fn detect_messages_file_version(content: &str) -> u32 {
    match content.lines().next() {
        Some(line) if line.starts_with("# version: ") => {
            line["# version: ".len()..].trim().parse().unwrap_or(0)
        }
        _ => 0,
    }
}

// ── Version check ───────────────────────────────────────────────

pub fn check_versions() -> Result<(), String> {
    if let Ok(content) = fs::read_to_string(TODOS_FILE)
        && let Ok(vt) = serde_json::from_str::<VersionedTodos>(&content)
        && vt.version > TODOS_VERSION
    {
        return Err(format!("{TODOS_FILE} is version {}, this app supports up to {}",
            vt.version, TODOS_VERSION));
    }
    if let Ok(content) = fs::read_to_string(COUNTERS_FILE)
        && let Ok(vc) = serde_json::from_str::<VersionedCounters>(&content)
        && vc.version > COUNTERS_VERSION
    {
        return Err(format!("{COUNTERS_FILE} is version {}, this app supports up to {}",
            vc.version, COUNTERS_VERSION));
    }
    if let Ok(content) = fs::read_to_string(LOG_FILE) {
        let v = detect_messages_file_version(&content);
        if v > MESSAGES_VERSION {
            return Err(format!("{LOG_FILE} is version {v}, this app supports up to {MESSAGES_VERSION}"));
        }
    }
    if let Ok(content) = fs::read_to_string(DAILIES_FILE)
        && let Ok(vd) = serde_json::from_str::<VersionedDailies>(&content)
        && vd.version > DAILIES_VERSION
    {
        return Err(format!(
            "{DAILIES_FILE} is version {}, this app supports up to {}",
            vd.version, DAILIES_VERSION
        ));
    }
    Ok(())
}

// ── Migrations ──────────────────────────────────────────────────

#[derive(Debug)]
pub enum MigrateTodosResult {
    NoChange,
    Migrated { new_content: String, count: usize },
    Corrupt,
}

pub fn migrate_todos_str(content: &str) -> MigrateTodosResult {
    if content.trim().is_empty() { return MigrateTodosResult::NoChange; }
    if let Ok(vt) = serde_json::from_str::<VersionedTodos>(content)
        && vt.version >= TODOS_VERSION
    {
        return MigrateTodosResult::NoChange;
    }
    if let Ok(items) = serde_json::from_str::<Vec<TodoItem>>(content) {
        let count = items.len();
        let vt = VersionedTodos { version: TODOS_VERSION, items };
        return match serde_json::to_string_pretty(&vt) {
            Ok(new_content) => MigrateTodosResult::Migrated { new_content, count },
            Err(_) => MigrateTodosResult::Corrupt,
        };
    }
    MigrateTodosResult::Corrupt
}

fn migrate_todos() -> Result<(), String> {
    let Ok(content) = fs::read_to_string(TODOS_FILE) else { return Ok(()); };
    match migrate_todos_str(&content) {
        MigrateTodosResult::NoChange => Ok(()),
        MigrateTodosResult::Migrated { new_content, count } => {
            backup_file(TODOS_FILE, 0).map_err(|e| format!("backup: {e}"))?;
            atomic_write(TODOS_FILE, &new_content).map_err(|e| format!("write: {e}"))?;
            log_event(json!({"type":"data-migrated","file":TODOS_FILE,"from_version":0,"to_version":TODOS_VERSION,"count":count}));
            Ok(())
        }
        MigrateTodosResult::Corrupt => {
            let _ = backup_corrupt(TODOS_FILE);
            log_event(json!({"type":"data-corrupt","file":TODOS_FILE}));
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum MigrateMessagesResult {
    NoChange,
    WriteHeaderOnly,
    Migrated(String),
}

pub fn migrate_messages_str(content: &str) -> MigrateMessagesResult {
    if content.trim().is_empty() { return MigrateMessagesResult::WriteHeaderOnly; }
    let v = detect_messages_file_version(content);
    if v >= MESSAGES_VERSION { return MigrateMessagesResult::NoChange; }
    MigrateMessagesResult::Migrated(format!("# version: {MESSAGES_VERSION}\n{content}"))
}

fn migrate_messages() -> Result<(), String> {
    let content = match fs::read_to_string(LOG_FILE) {
        Ok(c) => c,
        Err(_) => {
            atomic_write(LOG_FILE, &format!("# version: {MESSAGES_VERSION}\n"))
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    };
    match migrate_messages_str(&content) {
        MigrateMessagesResult::NoChange => Ok(()),
        MigrateMessagesResult::WriteHeaderOnly => {
            atomic_write(LOG_FILE, &format!("# version: {MESSAGES_VERSION}\n"))
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        MigrateMessagesResult::Migrated(new_content) => {
            backup_file(LOG_FILE, 0).map_err(|e| e.to_string())?;
            atomic_write(LOG_FILE, &new_content).map_err(|e| e.to_string())?;
            log_event(json!({"type":"data-migrated","file":LOG_FILE,"from_version":0,"to_version":MESSAGES_VERSION}));
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum MigrateCountersResult {
    NoChange,
    Migrated { new_content: String },
    Corrupt,
}

pub fn migrate_counters_str(content: &str) -> MigrateCountersResult {
    if content.trim().is_empty() { return MigrateCountersResult::NoChange; }
    if let Ok(vc) = serde_json::from_str::<VersionedCounters>(content)
        && vc.version >= COUNTERS_VERSION
    {
        return MigrateCountersResult::NoChange;
    }
    if let Ok(lc) = serde_json::from_str::<LegacyCounters>(content) {
        let vc = VersionedCounters {
            version: COUNTERS_VERSION,
            next_todo: lc.next_todo,
            next_msg: lc.next_msg,
            next_daily: lc.next_daily,
            next_evt: lc.next_evt,
        };
        return match serde_json::to_string_pretty(&vc) {
            Ok(new_content) => MigrateCountersResult::Migrated { new_content },
            Err(_) => MigrateCountersResult::Corrupt,
        };
    }
    MigrateCountersResult::Corrupt
}

fn migrate_counters() -> Result<(), String> {
    let Ok(content) = fs::read_to_string(COUNTERS_FILE) else { return Ok(()); };
    match migrate_counters_str(&content) {
        MigrateCountersResult::NoChange => Ok(()),
        MigrateCountersResult::Migrated { new_content } => {
            backup_file(COUNTERS_FILE, 0).map_err(|e| e.to_string())?;
            atomic_write(COUNTERS_FILE, &new_content).map_err(|e| e.to_string())?;
            log_event(json!({"type":"data-migrated","file":COUNTERS_FILE,"from_version":0,"to_version":COUNTERS_VERSION}));
            Ok(())
        }
        MigrateCountersResult::Corrupt => {
            let _ = backup_corrupt(COUNTERS_FILE);
            log_event(json!({"type":"data-corrupt","file":COUNTERS_FILE}));
            Ok(())
        }
    }
}

pub fn migrate_all() -> Result<(), String> {
    migrate_todos()?;
    migrate_messages()?;
    migrate_counters()?;
    Ok(())
}

pub fn load_counters() -> Counters {
    let Ok(content) = fs::read_to_string(COUNTERS_FILE) else { return Counters::default(); };
    if let Ok(vc) = serde_json::from_str::<VersionedCounters>(&content) {
        return Counters {
            next_todo: vc.next_todo,
            next_msg: vc.next_msg,
            next_daily: vc.next_daily,
            next_evt: vc.next_evt,
        };
    }
    if let Ok(lc) = serde_json::from_str::<LegacyCounters>(&content) {
        return Counters {
            next_todo: lc.next_todo,
            next_msg: lc.next_msg,
            next_daily: lc.next_daily,
            next_evt: lc.next_evt,
        };
    }
    Counters::default()
}

pub fn save_counters(c: &Counters) {
    let vc = VersionedCounters {
        version: COUNTERS_VERSION,
        next_todo: c.next_todo,
        next_msg: c.next_msg,
        next_daily: c.next_daily,
        next_evt: c.next_evt,
    };
    if let Ok(json) = serde_json::to_string_pretty(&vc) {
        let _ = atomic_write(COUNTERS_FILE, &json);
    }
}

// ── Todos ───────────────────────────────────────────────────────

pub fn seed_todos(_start_id: u64) -> Vec<TodoItem> { Vec::new() }

pub struct LoadedTodos {
    pub items: Vec<TodoItem>,
    pub next_id: u64,
    pub needs_save: bool,
}

pub fn load_todos(starting_next_id: u64) -> LoadedTodos {
    let mut next_id = starting_next_id;
    let Ok(content) = fs::read_to_string(TODOS_FILE) else {
        let items = seed_todos(next_id);
        next_id += items.len() as u64;
        log_event(json!({"type":"todo-load","source":"seed","count":items.len()}));
        return LoadedTodos { items, next_id, needs_save: true };
    };
    if let Ok(vt) = serde_json::from_str::<VersionedTodos>(&content) {
        for item in &vt.items {
            if let Some(n) = crate::events::parse_id_num(&item.id, "todo-") {
                next_id = next_id.max(n + 1);
            }
        }
        log_event(json!({"type":"todo-load","source":TODOS_FILE,"version":vt.version,"count":vt.items.len()}));
        return LoadedTodos { items: vt.items, next_id, needs_save: false };
    }
    if let Ok(items) = serde_json::from_str::<Vec<TodoItem>>(&content) {
        for item in &items {
            if let Some(n) = crate::events::parse_id_num(&item.id, "todo-") {
                next_id = next_id.max(n + 1);
            }
        }
        log_event(json!({"type":"todo-load","source":"legacy","count":items.len()}));
        return LoadedTodos { items, next_id, needs_save: false };
    }
    log_event(json!({"type":"todo-load","source":"corrupt","action":"seed"}));
    let items = seed_todos(next_id);
    next_id += items.len() as u64;
    LoadedTodos { items, next_id, needs_save: true }
}

pub fn save_todos(items: &[TodoItem])
    -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let vt = VersionedTodos { version: TODOS_VERSION, items: items.to_vec() };
    let json = serde_json::to_string_pretty(&vt)?;
    match atomic_write(TODOS_FILE, &json) {
        Ok(()) => {
            log_event(json!({"type":"save-todos","count":items.len()}));
            Ok(())
        }
        Err(e) => {
            log_event(json!({"type":"save-error","file":TODOS_FILE,"error":e.to_string()}));
            Err(Box::new(e))
        }
    }
}

// ── Messages ────────────────────────────────────────────────────

pub fn load_messages(next_msg: &mut u64) -> Vec<Message> {
    match fs::read_to_string(LOG_FILE) {
        Ok(content) => {
            let msgs: Vec<Message> = content.lines().filter_map(Message::from_line).collect();
            for m in &msgs {
                if let Some(n) = crate::events::parse_id_num(&m.id, "msg-") {
                    *next_msg = (*next_msg).max(n + 1);
                }
            }
            log_event(json!({"type":"msg-load","count":msgs.len()}));
            msgs
        }
        Err(_) => Vec::new(),
    }
}

pub fn append_message(msg: &Message) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)?;
    writeln!(file, "{}", msg.to_line())
}

pub fn load_dailies(next_id: &mut u64) -> Vec<crate::domain::DailyItem> {
    let Ok(content) = fs::read_to_string(DAILIES_FILE) else { return Vec::new(); };
    let Ok(vd) = serde_json::from_str::<VersionedDailies>(&content) else {
        log_event(json!({"type":"daily-load","source":"corrupt","action":"empty"}));
        return Vec::new();
    };
    for d in &vd.items {
        if let Some(n) = crate::events::parse_id_num(&d.id, "daily-") {
            *next_id = (*next_id).max(n + 1);
        }
    }
    log_event(json!({"type":"daily-load","source":DAILIES_FILE,"count":vd.items.len()}));
    vd.items
}

pub fn save_dailies(
    items: &[crate::domain::DailyItem],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let vd = VersionedDailies { version: DAILIES_VERSION, items: items.to_vec() };
    let json = serde_json::to_string_pretty(&vd)?;
    match atomic_write(DAILIES_FILE, &json) {
        Ok(()) => {
            log_event(json!({"type":"save-dailies","count":items.len()}));
            Ok(())
        }
        Err(e) => {
            log_event(json!({"type":"save-error","file":DAILIES_FILE,"error":e.to_string()}));
            Err(Box::new(e))
        }
    }
}

/// Suggestion IDs that have already been accepted or dismissed, so we
/// don't re-surface them when the same diary entry is re-scanned.
pub fn load_resolved_suggestion_ids() -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let Ok(content) = fs::read_to_string(EVENTS_FILE) else { return out; };
    for line in content.lines() {
        let Some((_, rest)) = line.split_once(" | ") else { continue; };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(rest) else { continue; };
        let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if (ty == "suggestion-accepted" || ty == "suggestion-dismissed")
            && let Some(id) = v.get("suggestion_id").and_then(|s| s.as_str())
        {
            out.insert(id.to_string());
        }
    }
    out
}
