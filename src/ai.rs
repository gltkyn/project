#![expect(dead_code)] // future surface: model-backed source, daily planner
//! AI layer — a SuggestionSource. Owns no state, holds no &mut access.
//! Reads via ReadContext, produces Suggestion<Action>. Rules are pure
//! regex + heuristics; nothing leaves the machine.

use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use ratatui::widgets::ListState;

use crate::actions::{Action, Source};
use crate::domain::{DailyItem, Message, TodoItem};

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub id: SuggestionId,
    pub action: Action,
    pub reason: String,
    pub confidence: f32,
    pub origin: Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin { Ai, DailyPlanner, RuleEngine }

impl Origin {
    pub fn label(&self) -> &'static str {
        match self {
            Origin::Ai => "ai",
            Origin::DailyPlanner => "daily",
            Origin::RuleEngine => "rule",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SuggestionId(pub String);

pub trait SuggestionSource {
    fn scan(&self, ctx: &ReadContext) -> Vec<Suggestion>;
}

pub struct ReadContext<'a> {
    pub diary_entries: &'a [Message],
    pub todos: &'a [TodoItem],
    pub dailies: &'a [DailyItem],
}

// ── RuleEngine ──────────────────────────────────────────────────

pub struct RuleEngine;

impl RuleEngine {
    /// Scan a single diary entry for todo/daily intent.
    pub fn scan_entry(&self, msg: &Message) -> Vec<Suggestion> {
        let Some(extracted) = extract(msg.content.trim()) else { return Vec::new(); };
        let (kind, body, duration_min) = match extracted {
            Extracted::Todo { text } => ("todo", text, None),
            Extracted::Daily { text, duration_min } => ("daily", text, duration_min),
        };
        let sid = make_id(&msg.id, kind, &body);
        let source = Source::Ai { suggestion_id: sid.0.clone() };
        let action = match kind {
            "todo" => Action::AddTodo { text: body, info: String::new(), source },
            _ => Action::AddDaily { text: body, duration_min, source },
        };
        vec![Suggestion {
            id: sid,
            action,
            reason: format!("{} entry {}", kind, msg.id),
            confidence: if kind == "todo" { 0.85 } else { 0.75 },
            origin: Origin::RuleEngine,
        }]
    }
}

impl SuggestionSource for RuleEngine {
    fn scan(&self, ctx: &ReadContext) -> Vec<Suggestion> {
        ctx.diary_entries.iter().flat_map(|m| self.scan_entry(m)).collect()
    }
}

// ── Extraction ──────────────────────────────────────────────────

enum Extracted {
    Todo { text: String },
    Daily { text: String, duration_min: Option<u32> },
}

fn extract(text: &str) -> Option<Extracted> {
    // Daily first — learning/practicing overrides a bare "I want to X".
    for re in daily_patterns() {
        if let Some(caps) = re.captures(text) {
            let raw = if caps.len() > 2 {
                format!("{} {}", &caps[1], &caps[2]).trim().to_string()
            } else {
                caps.get(1)?.as_str().trim().to_string()
            };
            let body = strip_duration(&raw);
            if body.is_empty() { continue; }
            let dur = extract_duration(text).or(Some(15));
            return Some(Extracted::Daily { text: body, duration_min: dur });
        }
    }
    for re in todo_patterns() {
        if let Some(caps) = re.captures(text) {
            let body = caps.get(1)?.as_str().trim().to_string();
            if body.is_empty() { continue; }
            if is_daily_verb(&body) {
                return Some(Extracted::Daily { text: body, duration_min: Some(15) });
            }
            return Some(Extracted::Todo { text: body });
        }
    }
    None
}

fn is_daily_verb(s: &str) -> bool {
    let first = s.split_whitespace().next().unwrap_or("").to_lowercase();
    matches!(
        first.as_str(),
        "learn" | "practice" | "play" | "study" | "exercise" | "read" | "write"
    )
}

fn make_id(msg_id: &str, kind: &str, text: &str) -> SuggestionId {
    let mut h = DefaultHasher::new();
    msg_id.hash(&mut h);
    kind.hash(&mut h);
    text.to_lowercase().hash(&mut h);
    SuggestionId(format!("sugg-{:x}", h.finish()))
}

fn todo_patterns() -> &'static [Regex] {
    static R: OnceLock<Vec<Regex>> = OnceLock::new();
    R.get_or_init(|| {
        vec![
            Regex::new(r"(?i)^\s*i\s+(?:need|should|must|have)\s+to\s+(.+?)[.!?]*\s*$").unwrap(),
            Regex::new(r"(?i)^\s*(?:don'?t\s+forget|remember)\s+to\s+(.+?)[.!?]*\s*$").unwrap(),
            Regex::new(r"(?i)^\s*(?:todo|task|next\s+step)[\s:]+(.+?)[.!?]*\s*$").unwrap(),
            Regex::new(r"(?i)^\s*i\s+(?:need|must)\s+(.+?)[.!?]*\s*$").unwrap(),
        ]
    })
}

fn daily_patterns() -> &'static [Regex] {
    static R: OnceLock<Vec<Regex>> = OnceLock::new();
    R.get_or_init(|| {
        vec![
            Regex::new(r"(?i)^\s*i\s+want\s+to\s+learn\s+(?:to\s+)?(.+?)[.!?]*\s*$").unwrap(),
            Regex::new(r"(?i)^\s*i\s+want\s+to\s+start\s+(.+?)[.!?]*\s*$").unwrap(),
            Regex::new(
                r"(?i)^\s*i\s+(?:want|would\s+like)\s+to\s+(practice|play|study|exercise|read|write)\s+(.+?)[.!?]*\s*$",
            )
            .unwrap(),
        ]
    })
}

fn duration_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?i)(\d+)\s*(minutes?|min|hours?|hrs?)").unwrap()
    })
}

fn extract_duration(text: &str) -> Option<u32> {
    let caps = duration_regex().captures(text)?;
    let n: u32 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2)?.as_str().to_lowercase();
    Some(if unit.starts_with("h") { n * 60 } else { n })
}

fn strip_duration(text: &str) -> String {
    let stripped = duration_regex().replace_all(text, "");
    let stripped = stripped.trim();
    let stripped = stripped.strip_suffix("for").unwrap_or(stripped).trim();
    stripped.to_string()
}

// ── Inbox ───────────────────────────────────────────────────────

#[derive(Default)]
pub struct SuggestionInbox {
    pub items: Vec<Suggestion>,
    pub state: ListState,
    pub resolved: HashSet<String>,
}

impl SuggestionInbox {
    pub fn new_with_resolved(resolved: HashSet<String>) -> Self {
        Self { items: Vec::new(), state: ListState::default(), resolved }
    }

    pub fn push_all(&mut self, suggestions: Vec<Suggestion>) {
        for s in suggestions {
            if self.resolved.contains(&s.id.0) { continue; }
            if self.items.iter().any(|x| x.id == s.id) { continue; }
            self.items.push(s);
        }
        if self.state.selected().is_none() && !self.items.is_empty() {
            self.state.select_first();
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str, content: &str) -> Message {
        Message { id: id.into(), timestamp: "t".into(), content: content.into() }
    }

    #[test]
    fn todo_simple() {
        let s = RuleEngine.scan_entry(&msg("msg-1", "I need to clear my room."));
        assert_eq!(s.len(), 1);
        match &s[0].action {
            Action::AddTodo { text, .. } => assert_eq!(text, "clear my room"),
            _ => panic!("expected AddTodo"),
        }
    }

    #[test]
    fn todo_dont_forget() {
        let s = RuleEngine.scan_entry(&msg("msg-2", "Don't forget to call mom."));
        match &s[0].action {
            Action::AddTodo { text, .. } => assert_eq!(text, "call mom"),
            _ => panic!(),
        }
    }

    #[test]
    fn daily_learn() {
        let s = RuleEngine.scan_entry(&msg("msg-3", "I want to learn to play my guitar."));
        match &s[0].action {
            Action::AddDaily { text, duration_min, .. } => {
                assert!(text.to_lowercase().contains("play"));
                assert!(text.to_lowercase().contains("guitar"));
                assert_eq!(*duration_min, Some(15));
            }
            _ => panic!("expected AddDaily"),
        }
    }

    #[test]
    fn daily_with_duration() {
        let s = RuleEngine.scan_entry(&msg("msg-4", "I want to learn guitar for 30 minutes."));
        match &s[0].action {
            Action::AddDaily { text, duration_min, .. } => {
                assert_eq!(text, "guitar");
                assert_eq!(*duration_min, Some(30));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn daily_hour_conversion() {
        let s = RuleEngine.scan_entry(&msg("msg-5", "I want to practice piano for 1 hour."));
        match &s[0].action {
            Action::AddDaily { duration_min, .. } => assert_eq!(*duration_min, Some(60)),
            _ => panic!(),
        }
    }

    #[test]
    fn no_match() {
        assert!(RuleEngine.scan_entry(&msg("msg-6", "Today was nice.")).is_empty());
    }

    #[test]
    fn dedupe_by_id() {
        let mut inbox = SuggestionInbox::default();
        let s = RuleEngine.scan_entry(&msg("msg-7", "I need to x."));
        inbox.push_all(s.clone());
        inbox.push_all(s);
        assert_eq!(inbox.items.len(), 1);
    }

    #[test]
    fn resolved_filtered() {
        let s = RuleEngine.scan_entry(&msg("msg-8", "I need to x."));
        let id = s[0].id.0.clone();
        let mut inbox = SuggestionInbox::default();
        inbox.resolved.insert(id);
        inbox.push_all(s);
        assert!(inbox.items.is_empty());
    }

    #[test]
    fn strip_duration_works() {
        assert_eq!(strip_duration("guitar for 30 minutes"), "guitar");
        assert_eq!(strip_duration("guitar for 1 hour"), "guitar");
        assert_eq!(strip_duration("guitar"), "guitar");
    }
}
