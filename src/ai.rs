#![expect(dead_code)] // future surface: model-backed source, daily planner
//! AI layer — a SuggestionSource. Owns no state, holds no &mut access.
//! Reads via ReadContext, produces Suggestion<Action>. The rule engine is a
//! lexicon-driven slot-filling parser: tokenize, find an intent word in a
//! table, capture the action that follows. Nothing leaves the machine.

use std::collections::HashSet;

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
        let Some(parsed) = parse(&msg.content) else { return Vec::new(); };
        let kind = parsed.kind.label();
        let sid = make_id(&msg.id, kind, &parsed.action);
        let source = Source::Ai { suggestion_id: sid.0.clone() };
        let action = match parsed.kind {
            IntentKind::Todo => Action::AddTodo {
                text: parsed.action,
                info: String::new(),
                source,
            },
            IntentKind::Daily => Action::AddDaily {
                text: parsed.action,
                duration_min: parsed.duration_min,
                source,
            },
        };
        vec![Suggestion {
            id: sid,
            action,
            reason: format!("{kind} entry {} via {}", msg.id, parsed.matched_rule),
            confidence: parsed.confidence,
            origin: Origin::RuleEngine,
        }]
    }
}

impl SuggestionSource for RuleEngine {
    fn scan(&self, ctx: &ReadContext) -> Vec<Suggestion> {
        ctx.diary_entries.iter().flat_map(|m| self.scan_entry(m)).collect()
    }
}

// ── Lexicons ────────────────────────────────────────────────────
// Everything the parser knows about English lives in these tables, matched
// after lowercasing. Teaching it a new verb, unit or idiom is one line here.

/// Obligation: "I need to X", "I ought to X". Only "need", "must" and "should"
/// may also skip the "to" (BARE_MODALS); "I have a meeting" must not match.
const INTENT_MODALS: &[&str] = &[
    "need", "needs", "should", "must", "ought", "have", "has", "got",
    "supposed", // "I'm supposed to"
    "gotta",    // "to" is part of the word, see TO_INCLUDED
];

/// Desire or plan: "I want to X", "I'm going to X", "I'd like to X".
const INTENT_WANTS: &[&str] = &[
    "want", "wanna", "plan", "intend", "going", "gonna", "aim", "hope",
    "like", // only after "would" / "I'd", see skip_lead
];

/// An action starting with one of these is a recurring practice, not a one-off.
const HABIT_VERBS: &[&str] = &[
    "learn", "practice", "practise", "play", "study", "exercise",
    "read", "write", "code", "draw", "paint", "sing", "train",
];

/// "remember to X", "remind me to X", "I keep forgetting to X".
const REMIND_TRIGGERS: &[&str] = &[
    "remember", "remind", "forget", "forgot", "forgetting", "meaning",
];

/// A leading tag turns the rest of the line into a todo: "todo: X".
/// "note" is deliberately absent: a note is not an action item.
const EXPLICIT_TAGS: &[&str] = &["todo", "task", "next"];

/// "next" is only a tag when followed by one of these ("next step: X").
const NEXT_FOLLOWERS: &[&str] = &["step", "action"];

/// Dangling words stripped from the end of the captured action ("take care of").
/// "that" is deliberately absent: trimming it turns "fix that" into "fix".
const FILLER_TOKENS: &[&str] = &[
    "to", "i", "my", "the", "a", "an", "about", "of", "for", "on",
    "i've", "i'd", "i'm",
];

const DURATION_UNITS: &[&str] = &[
    "min", "mins", "minute", "minutes", "hr", "hrs", "hour", "hours",
];

/// The subject of the sentence. Entries about other people never match.
const SUBJECTS: &[&str] = &["i", "i've", "i'd", "i'm"];

/// Words that may sit between the subject and the intent word.
const LEAD_INS: &[&str] = &[
    "really", "just", "also", "still", "always", "been", "keep", "kept", "would", "will",
];

/// Intent words that already contain their "to" ("gotta" = "got to").
const TO_INCLUDED: &[&str] = &["wanna", "gonna", "gotta"];

/// Modals that need no "to": "I should call mom", "I need milk".
const BARE_MODALS: &[&str] = &["need", "must", "should"];

/// After "to" a verb must follow. "going to the gym" is travel, not a plan.
const DETERMINERS: &[&str] = &[
    "the", "a", "an", "my", "our", "your", "his", "her", "their", "this",
    "that", "these", "those",
];

/// "I want to learn X" / "I want to start X" are daily practice of X.
const DROPPED_VERBS: &[&str] = &["learn", "start"];

const DEFAULT_DAILY_MIN: u32 = 15;

// ── Parser ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntentKind { Todo, Daily }

impl IntentKind {
    fn label(self) -> &'static str {
        match self {
            IntentKind::Todo => "todo",
            IntentKind::Daily => "daily",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ParseResult {
    kind: IntentKind,
    action: String,
    duration_min: Option<u32>,
    /// Which rule fired: "tag:todo", "modal:should", "want+habit", ...
    matched_rule: String,
    confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family { Tag, Remind, Modal, Want }

/// One word of the entry: `text` as written (shown to the user), `lc`
/// normalized for table lookups.
#[derive(Debug)]
struct Tok {
    text: String,
    lc: String,
}

/// Where the intent word was found and where the action starts.
struct Hit {
    family: Family,
    rule: String,
    /// Index of the first action token.
    start: usize,
    /// The action follows "to", so it must begin with a verb.
    infinitive: bool,
    min_tokens: usize,
}

fn parse(text: &str) -> Option<ParseResult> {
    let toks = tokenize(text);
    let hit = locate(&toks)?;
    let mut span: Vec<&Tok> = toks.get(hit.start..)?.iter().collect();

    if hit.infinitive && span.first().is_some_and(|t| DETERMINERS.contains(&t.lc.as_str())) {
        return None;
    }

    let mut kind = IntentKind::Todo;
    let mut promoted = false;

    if hit.family == Family::Want
        && span.first().is_some_and(|t| DROPPED_VERBS.contains(&t.lc.as_str()))
    {
        span.remove(0);
        if span.first().is_some_and(|t| t.lc == "to") {
            span.remove(0);
        }
        kind = IntentKind::Daily;
        promoted = true;
    }
    if kind == IntentKind::Todo
        && span.first().is_some_and(|t| HABIT_VERBS.contains(&t.lc.as_str()))
    {
        kind = IntentKind::Daily;
        promoted = true;
    }

    let duration_min = match kind {
        IntentKind::Daily => Some(take_duration(&mut span).unwrap_or(DEFAULT_DAILY_MIN)),
        IntentKind::Todo => None,
    };

    trim_filler(&mut span);
    if span.len() < hit.min_tokens {
        return None;
    }

    let confidence = if promoted {
        0.75
    } else {
        match hit.family {
            Family::Tag => 0.95,
            Family::Remind => 0.9,
            Family::Modal => 0.85,
            Family::Want => 0.8,
        }
    };
    let matched_rule = if promoted { format!("{}+habit", hit.rule) } else { hit.rule };
    let action = span.iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ");

    Some(ParseResult { kind, action, duration_min, matched_rule, confidence })
}

/// Split into words. Trailing sentence punctuation is dropped; the words keep
/// their original casing for display.
fn tokenize(raw: &str) -> Vec<Tok> {
    raw.trim()
        .trim_end_matches(['.', '!', '?'])
        .split_whitespace()
        .map(|w| Tok { text: w.to_string(), lc: normalize(w) })
        .collect()
}

fn normalize(word: &str) -> String {
    word.to_lowercase()
        .replace('\u{2019}', "'")
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

/// Find the intent word, in priority order: tag, reminder, modal, want.
fn locate(toks: &[Tok]) -> Option<Hit> {
    let lc = |i: usize| toks.get(i).map(|t| t.lc.as_str());

    let first = lc(0)?;
    if EXPLICIT_TAGS.contains(&first) {
        let start = match first {
            "next" if lc(1).is_some_and(|w| NEXT_FOLLOWERS.contains(&w)) => 2,
            "next" => return None,
            _ => 1,
        };
        return Some(Hit {
            family: Family::Tag,
            rule: format!("tag:{first}"),
            start,
            infinitive: false,
            min_tokens: 1,
        });
    }

    let (i, polite) = skip_lead(toks);
    let head = lc(i)?;

    if REMIND_TRIGGERS.contains(&head) {
        let mut j = i + 1;
        if head == "remind" && lc(j) == Some("me") {
            j += 1;
        }
        if lc(j) != Some("to") {
            return None;
        }
        return Some(Hit {
            family: Family::Remind,
            rule: format!("remind:{head}"),
            start: j + 1,
            infinitive: true,
            min_tokens: 1,
        });
    }

    if INTENT_MODALS.contains(&head) {
        let (start, infinitive) = if TO_INCLUDED.contains(&head) {
            (i + 1, true)
        } else if lc(i + 1) == Some("to") {
            (i + 2, true)
        } else if BARE_MODALS.contains(&head) {
            (i + 1, false)
        } else {
            // "I have a meeting": a modal word without "to" is not an obligation.
            return None;
        };
        return Some(Hit {
            family: Family::Modal,
            rule: format!("modal:{head}"),
            start,
            infinitive,
            min_tokens: 1,
        });
    }

    if INTENT_WANTS.contains(&head) {
        if head == "like" && !polite {
            return None; // "I like to read" is a statement, "I would like to" is a wish
        }
        let start = if TO_INCLUDED.contains(&head) {
            i + 1
        } else if lc(i + 1) == Some("to") {
            i + 2
        } else {
            return None;
        };
        // "I'm going to work" is a commute, "I'm going to fix the bike" is a plan.
        let min_tokens = if head == "going" { 2 } else { 1 };
        return Some(Hit {
            family: Family::Want,
            rule: "want".to_string(),
            start,
            infinitive: true,
            min_tokens,
        });
    }

    None
}

/// Skip the subject and lead-in words. Returns the index of the intent word
/// and whether it was introduced politely ("would", "I'd").
fn skip_lead(toks: &[Tok]) -> (usize, bool) {
    let mut i = 0;
    let mut polite = false;
    let mut seen_subject = false;
    while let Some(t) = toks.get(i) {
        let w = t.lc.as_str();
        let next = toks.get(i + 1).map(|n| n.lc.as_str());
        let skip = SUBJECTS.contains(&w)
            || LEAD_INS.contains(&w)
            || (matches!(w, "have" | "has") && next == Some("been"))
            || (matches!(w, "don't" | "dont") && next == Some("forget") && !seen_subject);
        if !skip {
            break;
        }
        seen_subject |= SUBJECTS.contains(&w);
        polite |= w == "would" || w == "i'd";
        i += 1;
    }
    (i, polite)
}

/// Remove "<number> <unit>" (or "30min") and a preceding "for" from the span.
fn take_duration(span: &mut Vec<&Tok>) -> Option<u32> {
    for k in 0..span.len() {
        let Some((minutes, used)) = duration_at(span, k) else { continue; };
        let from = if k > 0 && span[k - 1].lc == "for" { k - 1 } else { k };
        span.drain(from..k + used);
        return Some(minutes);
    }
    None
}

/// Minutes and tokens consumed if a duration starts at `span[k]`.
fn duration_at(span: &[&Tok], k: usize) -> Option<(u32, usize)> {
    let word = span[k].lc.as_str();
    let digits = word.find(|c: char| !c.is_ascii_digit()).unwrap_or(word.len());
    if digits == 0 {
        return None;
    }
    let n: u32 = word[..digits].parse().ok()?;
    let (unit, used) = if digits == word.len() {
        (span.get(k + 1)?.lc.as_str(), 2)
    } else {
        (&word[digits..], 1)
    };
    if !DURATION_UNITS.contains(&unit) {
        return None;
    }
    let minutes = if unit.starts_with('h') { n.checked_mul(60)? } else { n };
    Some((minutes, used))
}

/// Leading words are kept on purpose: "I need my keys" is "my keys".
fn trim_filler(span: &mut Vec<&Tok>) {
    while span.last().is_some_and(|t| FILLER_TOKENS.contains(&t.lc.as_str())) {
        span.pop();
    }
}

fn make_id(msg_id: &str, kind: &str, text: &str) -> SuggestionId {
    SuggestionId(format!(
        "sugg:{}:{}:{}",
        msg_id,
        kind,
        text.to_lowercase().replace(' ', "_")
    ))
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
    use IntentKind::{Daily, Todo};

    fn msg(id: &str, content: &str) -> Message {
        Message { id: id.into(), timestamp: "t".into(), content: content.into() }
    }

    // ── preserved from the regex engine ─────────────────────────

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
    fn parse_strips_duration() {
        for text in [
            "I want to practice guitar for 30 minutes",
            "I want to practice guitar for 1 hour",
            "I want to practice guitar",
        ] {
            assert_eq!(parse(text).unwrap().action, "practice guitar", "{text}");
        }
    }

    // ── phrasing the regexes could not handle ───────────────────

    fn check(text: &str, kind: IntentKind, action: &str, rule: &str) {
        let p = parse(text).unwrap_or_else(|| panic!("no match for {text:?}"));
        assert_eq!((p.kind, p.action.as_str(), p.matched_rule.as_str()), (kind, action, rule));
    }

    #[test]
    fn have_to_vs_have_a() {
        check("I have to call mom", Todo, "call mom", "modal:have");
        assert!(parse("I have a meeting").is_none());
    }

    #[test]
    fn want_play() {
        check("I would like to play guitar", Daily, "play guitar", "want+habit");
    }

    #[test]
    fn meaning_to() {
        check("I've been meaning to fix the sink", Todo, "fix the sink", "remind:meaning");
    }

    #[test]
    fn keep_forgetting() {
        check("I keep forgetting to water plants", Todo, "water plants", "remind:forgetting");
    }

    #[test]
    fn ought_to() {
        check("I ought to sleep earlier", Todo, "sleep earlier", "modal:ought");
    }

    #[test]
    fn tag_todo() {
        check("todo: buy milk", Todo, "buy milk", "tag:todo");
    }

    #[test]
    fn tag_next_step() {
        check("next step: file taxes", Todo, "file taxes", "tag:next");
    }

    #[test]
    fn remind_me() {
        check("remind me to check the mail", Todo, "check the mail", "remind:remind");
    }

    // ── behaviour the regex engine had and the parser keeps ─────

    #[test]
    fn keeps_original_casing() {
        let p = parse("I need to email Sarah about the Q3 report.").unwrap();
        assert_eq!(p.action, "email Sarah about the Q3 report");
    }

    #[test]
    fn bare_modals_need_no_to() {
        check("I need milk", Todo, "milk", "modal:need");
        check("I must file my taxes", Todo, "file my taxes", "modal:must");
        check("I should call mom", Todo, "call mom", "modal:should");
        // only need/must/should may skip "to"
        assert!(parse("I ought call mom").is_none());
    }

    #[test]
    fn want_to_start_is_daily() {
        check("I want to start running", Daily, "running", "want+habit");
    }

    #[test]
    fn habit_promotion_looks_at_the_first_word_only() {
        // "read" later in the sentence must not turn a one-off into a habit.
        check("I need to buy a book to read", Todo, "buy a book to read", "modal:need");
        check("I need to learn French", Daily, "learn French", "modal:need+habit");
    }

    #[test]
    fn duration_only_for_dailies() {
        let p = parse("I need to call mom for 30 minutes").unwrap();
        assert_eq!((p.kind, p.duration_min), (Todo, None));
        assert_eq!(p.action, "call mom for 30 minutes");
        assert_eq!(parse("I want to practice piano 30min").unwrap().duration_min, Some(30));
    }

    #[test]
    fn curly_apostrophes() {
        check("Don\u{2019}t forget to call mom", Todo, "call mom", "remind:forget");
    }

    #[test]
    fn dangling_fillers_are_trimmed() {
        check("I need to fix that", Todo, "fix that", "modal:need");
        check("I need to take care of", Todo, "take care", "modal:need");
    }

    #[test]
    fn negation_and_other_people_do_not_match() {
        assert!(parse("I don't need to call mom").is_none());
        assert!(parse("She has to call the plumber").is_none());
        assert!(parse("I don't forget to call mom").is_none());
    }

    // ── ids and reasons ─────────────────────────────────────────

    #[test]
    fn suggestion_id_is_stable_and_readable() {
        assert_eq!(
            make_id("msg-7", "todo", "Clear My Room"),
            SuggestionId("sugg:msg-7:todo:clear_my_room".into())
        );
        let s = RuleEngine.scan_entry(&msg("msg-9", "I need to clear my room."));
        assert_eq!(s[0].id.0, "sugg:msg-9:todo:clear_my_room");
    }

    #[test]
    fn reason_names_the_rule() {
        let s = RuleEngine.scan_entry(&msg("msg-7", "I should call mom"));
        assert_eq!(s[0].reason, "todo entry msg-7 via modal:should");
        assert_eq!(s[0].confidence, 0.85);
    }

    // ── negatives ───────────────────────────────────────────────

    #[test]
    fn empty_and_punctuation_only() {
        for text in ["", "   ", "...", "?!"] {
            assert!(parse(text).is_none(), "{text:?}");
        }
    }

    #[test]
    fn incomplete_intents() {
        for text in ["I want", "I want to", "I want to learn", "todo:", "I need to", "remember to"] {
            assert!(parse(text).is_none(), "{text:?}");
        }
    }

    // ── golden corpus ───────────────────────────────────────────
    // (input, expected (kind, action, duration_min, matched_rule) or None)

    type Row = (&'static str, Option<(IntentKind, &'static str, Option<u32>, &'static str)>);

    const CORPUS: &[Row] = &[
        // INTENT_MODALS
        ("I need to clear my room", Some((Todo, "clear my room", None, "modal:need"))),
        ("Needs to renew passport", Some((Todo, "renew passport", None, "modal:needs"))),
        ("I should call mom", Some((Todo, "call mom", None, "modal:should"))),
        ("I must file my taxes", Some((Todo, "file my taxes", None, "modal:must"))),
        ("Has to renew insurance", Some((Todo, "renew insurance", None, "modal:has"))),
        ("I ought to sleep earlier", Some((Todo, "sleep earlier", None, "modal:ought"))),
        ("I have to call mom", Some((Todo, "call mom", None, "modal:have"))),
        ("She has to call the plumber", None),
        ("I've got to book flights", Some((Todo, "book flights", None, "modal:got"))),
        ("I'm supposed to email Sarah", Some((Todo, "email Sarah", None, "modal:supposed"))),
        ("I gotta pay rent", Some((Todo, "pay rent", None, "modal:gotta"))),
        // INTENT_WANTS
        ("I want to buy a car", Some((Todo, "buy a car", None, "want"))),
        ("I wanna fix my bike", Some((Todo, "fix my bike", None, "want"))),
        ("I plan to visit grandma", Some((Todo, "visit grandma", None, "want"))),
        ("I intend to finish the report", Some((Todo, "finish the report", None, "want"))),
        ("I'm going to repaint the fence", Some((Todo, "repaint the fence", None, "want"))),
        ("I'm gonna call the dentist", Some((Todo, "call the dentist", None, "want"))),
        ("I aim to wake up earlier", Some((Todo, "wake up earlier", None, "want"))),
        ("I hope to finish the book", Some((Todo, "finish the book", None, "want"))),
        ("I would like to play guitar", Some((Daily, "play guitar", Some(15), "want+habit"))),
        ("I'd like to read before bed", Some((Daily, "read before bed", Some(15), "want+habit"))),
        ("I like to read", None),
        // HABIT_VERBS and DURATION_UNITS
        ("I want to learn guitar for 30 minutes", Some((Daily, "guitar", Some(30), "want+habit"))),
        ("I want to learn to play my guitar", Some((Daily, "play my guitar", Some(15), "want+habit"))),
        ("I want to start running", Some((Daily, "running", Some(15), "want+habit"))),
        ("I want to practice piano for 30 mins", Some((Daily, "practice piano", Some(30), "want+habit"))),
        ("I want to practise violin for 20 minutes", Some((Daily, "practise violin", Some(20), "want+habit"))),
        ("I want to study Spanish for 2 hrs", Some((Daily, "study Spanish", Some(120), "want+habit"))),
        ("I want to exercise", Some((Daily, "exercise", Some(15), "want+habit"))),
        ("I want to read for 45 min", Some((Daily, "read", Some(45), "want+habit"))),
        ("I need to write for 1 hour", Some((Daily, "write", Some(60), "modal:need+habit"))),
        ("I want to code for 2 hours", Some((Daily, "code", Some(120), "want+habit"))),
        ("I want to draw for 1 minute", Some((Daily, "draw", Some(1), "want+habit"))),
        ("I want to paint for 90 minutes", Some((Daily, "paint", Some(90), "want+habit"))),
        ("I want to sing for 10 min", Some((Daily, "sing", Some(10), "want+habit"))),
        ("I want to train for 1 hr", Some((Daily, "train", Some(60), "want+habit"))),
        ("I want to practice piano 30min", Some((Daily, "practice piano", Some(30), "want+habit"))),
        ("I want to practice piano for about 30 minutes", Some((Daily, "practice piano", Some(30), "want+habit"))),
        // REMIND_TRIGGERS
        ("Remember to buy milk", Some((Todo, "buy milk", None, "remind:remember"))),
        ("Remind me to check the mail", Some((Todo, "check the mail", None, "remind:remind"))),
        ("Don't forget to call mom", Some((Todo, "call mom", None, "remind:forget"))),
        ("I forgot to lock the door", Some((Todo, "lock the door", None, "remind:forgot"))),
        ("I keep forgetting to water plants", Some((Todo, "water plants", None, "remind:forgetting"))),
        ("I've been meaning to fix the sink", Some((Todo, "fix the sink", None, "remind:meaning"))),
        // EXPLICIT_TAGS
        ("todo: buy milk", Some((Todo, "buy milk", None, "tag:todo"))),
        ("Task: renew passport", Some((Todo, "renew passport", None, "tag:task"))),
        ("next step: file taxes", Some((Todo, "file taxes", None, "tag:next"))),
        ("Next action: email Sarah about the Q3 report", Some((Todo, "email Sarah about the Q3 report", None, "tag:next"))),
        // FILLER_TOKENS
        ("I need to take care of", Some((Todo, "take care", None, "modal:need"))),
        ("I need to work on", Some((Todo, "work", None, "modal:need"))),
        ("I need to look into the issue about", Some((Todo, "look into the issue", None, "modal:need"))),
        ("I need to find an apartment", Some((Todo, "find an apartment", None, "modal:need"))),
        // lead-ins, bare objects, original casing
        ("I need milk", Some((Todo, "milk", None, "modal:need"))),
        ("need to buy eggs", Some((Todo, "buy eggs", None, "modal:need"))),
        ("I really need to call the bank", Some((Todo, "call the bank", None, "modal:need"))),
        ("I need to learn French", Some((Daily, "learn French", Some(15), "modal:need+habit"))),
        // must not match
        ("I have a meeting", None),
        ("I don't need to call mom", None),
        ("Today was nice.", None),
        ("I'm going to bed", None),
        ("I'm going to the gym", None),
        ("I remember my childhood", None),
        ("Next week I go to Paris", None),
        ("Note: gas is cheap", None),
        ("", None),
    ];

    #[test]
    fn golden_corpus() {
        assert!(CORPUS.len() >= 40);
        for &(input, expected) in CORPUS {
            let got = parse(input).map(|p| (p.kind, p.action, p.duration_min, p.matched_rule));
            let want = expected.map(|(k, a, d, r)| (k, a.to_string(), d, r.to_string()));
            assert_eq!(got, want, "corpus row {input:?}");
        }
    }

    #[test]
    fn every_lexicon_entry_appears_in_the_corpus() {
        // Only rows that produce a suggestion count as exercising an entry.
        let used: HashSet<String> = CORPUS
            .iter()
            .filter(|row| row.1.is_some())
            .flat_map(|&(input, _)| tokenize(input))
            .map(|t| t.lc)
            .collect();
        let tables: [(&str, &[&str]); 7] = [
            ("INTENT_MODALS", INTENT_MODALS),
            ("INTENT_WANTS", INTENT_WANTS),
            ("HABIT_VERBS", HABIT_VERBS),
            ("REMIND_TRIGGERS", REMIND_TRIGGERS),
            ("EXPLICIT_TAGS", EXPLICIT_TAGS),
            ("FILLER_TOKENS", FILLER_TOKENS),
            ("DURATION_UNITS", DURATION_UNITS),
        ];
        for (name, table) in tables {
            for entry in table {
                assert!(used.contains(*entry), "{name}: no corpus row uses {entry:?}");
            }
        }
    }

    /// The helper tables are too many-entried for one corpus row each, so
    /// every entry is driven through a template instead.
    #[test]
    fn every_helper_table_entry_is_exercised() {
        for s in SUBJECTS {
            check(&format!("{s} need to call mom"), Todo, "call mom", "modal:need");
        }
        for w in LEAD_INS {
            check(&format!("I {w} need to call mom"), Todo, "call mom", "modal:need");
        }
        check("I have been meaning to fix the sink", Todo, "fix the sink", "remind:meaning");
        for d in DETERMINERS {
            let text = format!("I'm going to {d} gym");
            assert!(parse(&text).is_none(), "{text:?}");
        }
        for m in BARE_MODALS {
            check(&format!("I {m} call mom"), Todo, "call mom", &format!("modal:{m}"));
        }
        for t in TO_INCLUDED {
            let text = format!("I {t} call mom");
            assert!(parse(&text).is_some(), "{text:?}");
        }
        for f in NEXT_FOLLOWERS {
            check(&format!("next {f}: call mom"), Todo, "call mom", "tag:next");
        }
        for v in DROPPED_VERBS {
            check(&format!("I want to {v} chess"), Daily, "chess", "want+habit");
        }
    }

    #[test]
    fn lexicon_entries_are_lowercase() {
        let all: [&[&str]; 14] = [
            INTENT_MODALS, INTENT_WANTS, HABIT_VERBS, REMIND_TRIGGERS, EXPLICIT_TAGS,
            NEXT_FOLLOWERS, FILLER_TOKENS, DURATION_UNITS, SUBJECTS, LEAD_INS,
            TO_INCLUDED, BARE_MODALS, DETERMINERS, DROPPED_VERBS,
        ];
        for table in all {
            for entry in table {
                assert_eq!(*entry, entry.to_lowercase());
            }
        }
    }
}
