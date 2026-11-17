//! Event log — append-only audit trail. AI's scan cursor, undo, and
//! post-mortem debugging all read from this.

use chrono::Local;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::storage::{COUNTERS_FILE, EVENTS_FILE};

pub const EVENTS_EVENT_VERSION: u32 = 1;

pub static NEXT_EVT: AtomicU64 = AtomicU64::new(1);
static EVENT_LOG: OnceLock<Mutex<Option<File>>> = OnceLock::new();
static LOG_ERROR_REPORTED: AtomicBool = AtomicBool::new(false);

pub fn current_evt_counter() -> u64 {
    NEXT_EVT.load(Ordering::SeqCst)
}

pub fn parse_event_id(line: &str) -> Option<u64> {
    let after = line.split_once("] evt-")?.1;
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

pub fn parse_id_num(s: &str, prefix: &str) -> Option<u64> {
    s.strip_prefix(prefix)?.parse().ok()
}

pub fn init_event_log() {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(EVENTS_FILE)
        .ok();

    let mut max_evt = 0u64;
    if let Ok(content) = fs::read_to_string(EVENTS_FILE) {
        for line in content.lines() {
            if let Some(n) = parse_event_id(line) {
                max_evt = max_evt.max(n);
            }
        }
    }
    if let Ok(content) = fs::read_to_string(COUNTERS_FILE) {
        #[derive(serde::Deserialize)]
        struct VC { next_evt: u64 }
        if let Ok(vc) = serde_json::from_str::<VC>(&content) {
            max_evt = max_evt.max(vc.next_evt.saturating_sub(1));
        }
    }
    NEXT_EVT.store(max_evt + 1, Ordering::SeqCst);
    let _ = EVENT_LOG.set(Mutex::new(file));
}

pub fn log_event(mut payload: Value) {
    if let Some(obj) = payload.as_object_mut() {
        obj.entry("ev").or_insert(serde_json::json!(EVENTS_EVENT_VERSION));
    }
    let n = NEXT_EVT.fetch_add(1, Ordering::SeqCst);
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    if let Some(mutex) = EVENT_LOG.get()
        && let Ok(mut guard) = mutex.lock()
        && let Some(file) = guard.as_mut()
    {
        let body = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());
        if let Err(e) = writeln!(file, "[{ts}] evt-{n} | {body}")
            && !LOG_ERROR_REPORTED.swap(true, Ordering::SeqCst)
        {
            eprintln!("warning: failed to write event log: {e}");
        }
    }
}

pub fn scan_events_for_max_ids() -> (u64, u64) {
    let Ok(content) = fs::read_to_string(EVENTS_FILE) else { return (0, 0); };
    let mut max_todo = 0u64;
    let mut max_msg = 0u64;
    for line in content.lines() {
        let Some((_, rest)) = line.split_once(" | ") else { continue; };
        let Ok(v) = serde_json::from_str::<Value>(rest) else { continue; };
        let Some(id) = v.get("id").and_then(|v| v.as_str()) else { continue; };
        if let Some(n) = parse_id_num(id, "todo-") { max_todo = max_todo.max(n); }
        if let Some(n) = parse_id_num(id, "msg-") { max_msg = max_msg.max(n); }
    }
    (max_todo, max_msg)
}
