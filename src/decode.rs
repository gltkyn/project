//! decode: reads an old todo file in ANY layout. It does not know formats. It finds how
//! the lines start (`[ ] - `, `* TODO `, `1. `, `## `, nothing at all), asks ONCE per kind of
//! start what it means, and applies that answer to every line that starts the same way.
//! It never decides alone: nothing is kept without an answer.
//!
//! Hides: how a line is split into "marker units + words", how lines are grouped by their
//! start, the shipped table of known markers, the counters that remember your corrections.
//! Must never know: ratatui, storage, the AI, `things`, `Group`. It imports only `std`.
//! Position: next to `text`, at the bottom of the arrows in PROJECT.md.
//!
//! Layers. A layer only calls layers with a lower number:
//!   1 `read_lines`                 text -> numbered lines
//!   2 `read_lead`, `read_body`     one line -> marker units in front + the words after them
//!   3 `analyze`, `discover`        lines -> groups of lines that start the same way
//!   4 `guess_meaning`              a group's start -> its likely meaning (shipped table,
//!                                  then counters from your earlier answers)
//!   5 `Question` / `Answer`        what is asked, what you may say
//!   6 `Session`                    the conversation; ends in `Draft`s
//!
//! How the existing shared functions plug in (the three `ADAPTER` points):
//!   let mut s = decode::Session::open(&file_text, &stats);
//!   while let Some(q) = s.question() {
//!       // ADAPTER 1, highlight: the shared highlight function draws `q.shown`, colouring
//!       //            each span by its `Role` (tui maps Role -> colour, not here).
//!       // ADAPTER 2, suggestion: the shared suggestion function shows `q.prompt` and, for
//!       //            `QuestionKind::Lead`, an editable label holding `guess` with
//!       //            `choices` to tap through (todo / done / heading / skip).
//!       // ADAPTER 3, input: the shared input function reads the reply and maps it:
//!       //            enter on the guess = `Answer::Yes`, a changed label =
//!       //            `Answer::Mean(m)`, no = `Answer::No`.
//!       s.answer(answer)?;
//!   }
//!   // `s.drafts()` are the approved things. Turn each into the same operation the AI's
//!   // approvals use (add todo, mark done). `s.learned()` goes back to the counters store.

use std::collections::{BTreeMap, BTreeSet};

// ───────────────────────── layer 0: vocabulary ─────────────────────────

/// What a kind of line start means. The user can always change the guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Meaning {
    /// each line is a todo to add
    Todo,
    /// each line is something already done
    Done,
    /// each line is a heading; the lines below it belong to it
    Section,
    /// skip these lines
    Ignore,
}

impl Meaning {
    pub const ALL: [Meaning; 4] = [Meaning::Todo, Meaning::Done, Meaning::Section, Meaning::Ignore];

    pub fn name(self) -> &'static str {
        match self {
            Meaning::Todo => "todo",
            Meaning::Done => "done",
            Meaning::Section => "heading",
            Meaning::Ignore => "skip",
        }
    }

    pub fn from_name(s: &str) -> Option<Meaning> {
        Meaning::ALL.into_iter().find(|m| m.name() == s)
    }

    pub fn phrase(self) -> &'static str {
        match self {
            Meaning::Todo => "a todo to add",
            Meaning::Done => "something already done",
            Meaning::Section => "a heading for the lines below it",
            Meaning::Ignore => "to be skipped",
        }
    }

    fn index(self) -> usize {
        match self {
            Meaning::Todo => 0,
            Meaning::Done => 1,
            Meaning::Section => 2,
            Meaning::Ignore => 3,
        }
    }
}

/// What a highlighted part of a line is. No colours here: `tui` decides those.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Marker,
    Priority,
    Date,
    Due,
    Tag,
    Title,
}

/// A byte range inside the shown text, plus what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub role: Role,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawLine {
    /// 1-based, as an editor shows it
    pub number: usize,
    pub text: String,
}

/// What the user approved. Plain data, ready to be turned into operations on things.
/// Dates stay text: `clock` decides whether they are real and what they mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub source_line: usize,
    pub title: String,
    pub done: bool,
    pub priority: Option<char>,
    /// dates written in front of the words, in order
    pub dates: Vec<String>,
    pub due: Option<String>,
    pub tags: Vec<String>,
    /// the heading it sat under. Not a Life-OS `Group`: the curator decides that.
    pub section: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Parsed {
    title: String,
    priority: Option<char>,
    dates: Vec<String>,
    due: Option<String>,
    tags: Vec<String>,
    /// byte ranges inside the raw line
    spans: Vec<Span>,
}

// ───────────────────────── layer 1: lines ─────────────────────────

pub fn read_lines(source: &str) -> Vec<RawLine> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    source
        .lines() // also strips "\r\n"
        .enumerate()
        .map(|(i, l)| RawLine {
            number: i + 1,
            text: l.trim_end().to_string(),
        })
        .collect()
}

fn is_noise(text: &str) -> bool {
    let t = text.trim();
    t.is_empty()
        || (t.len() >= 3 && t.chars().all(|c| "-=_*~".contains(c)))
        || t.starts_with("#+")
        || t.starts_with("<!--")
        || t.starts_with("//")
}

// ───────────────────────── layer 2: one line ─────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitKind {
    Box,
    Symbol,
    Number,
    Keyword,
    Date,
    Priority(char),
}

/// One marker in front of the words: `[ ]`, `-`, `1.`, `TODO`, `(A)`, a date ...
#[derive(Clone, Debug)]
struct Unit {
    start: usize,
    end: usize,
    kind: UnitKind,
    /// the form used to compare lines: `[X]` -> `[x]`, `12.` -> `#.`, a date -> `DATE`
    norm: String,
}

fn skip_spaces(text: &str, at: usize) -> usize {
    at + (text[at..].len() - text[at..].trim_start().len())
}

fn word_at(text: &str, at: usize) -> &str {
    let rest = &text[at..];
    let n = rest.find(char::is_whitespace).unwrap_or(rest.len());
    &rest[..n]
}

/// Whitespace-separated words with their byte offsets, starting at byte `from`.
fn tokens(text: &str, from: usize) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if i < from {
            continue;
        }
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push((s, &text[s..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, &text[s..]));
    }
    out
}

/// Shape only (YYYY-MM-DD). Whether the day exists is `clock`'s business.
fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, &c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn priority_inner(open: char, inner: &str) -> Option<char> {
    let mut c = inner.chars();
    if open == '(' {
        let a = c.next()?;
        (c.next().is_none() && a.is_ascii_uppercase() && a != 'X').then_some(a)
    } else {
        if c.next()? != '#' {
            return None;
        }
        let a = c.next()?;
        (c.next().is_none() && a.is_ascii_uppercase()).then_some(a)
    }
}

/// `[ ]`, `[x]`, `[-]`, `( )`, `(x)` and the priorities `(A)`, `[#A]`.
fn bracket_unit(text: &str, at: usize) -> Option<Unit> {
    let rest = &text[at..];
    let open = rest.chars().next()?;
    let close = match open {
        '[' => ']',
        '(' => ')',
        _ => return None,
    };
    let rel = rest[1..].find(close)?;
    let inner = &rest[1..1 + rel];
    let end = at + 1 + rel + 1;
    if inner.chars().count() > 2 {
        return None;
    }
    if !text[end..].is_empty() && !text[end..].starts_with(char::is_whitespace) {
        return None;
    }
    let (kind, norm) = if let Some(c) = priority_inner(open, inner) {
        let norm = if open == '(' { "(?)" } else { "[#?]" };
        (UnitKind::Priority(c), norm.to_string())
    } else {
        let t = inner.trim();
        if !(t.is_empty() || matches!(t, "x" | "X" | "-" | "✓" | "✔" | "✗" | "✘" | "*")) {
            return None;
        }
        let norm = if t.is_empty() {
            format!("{open} {close}")
        } else {
            format!("{open}{}{close}", t.to_lowercase())
        };
        (UnitKind::Box, norm)
    };
    Some(Unit { start: at, end, kind, norm })
}

fn unit_at(text: &str, at: usize) -> Option<Unit> {
    if let Some(u) = bracket_unit(text, at) {
        return Some(u);
    }
    let tok = word_at(text, at);
    if tok.is_empty() {
        return None;
    }
    let end = at + tok.len();
    let mk = |kind: UnitKind, norm: String| Some(Unit { start: at, end, kind, norm });

    if is_date(tok) {
        return mk(UnitKind::Date, "DATE".to_string());
    }
    let digits = tok.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && tok.len() == digits + 1 && matches!(tok.as_bytes()[digits], b'.' | b')' | b':') {
        return mk(UnitKind::Number, format!("#{}", &tok[digits..]));
    }
    let bare = tok.trim_end_matches(':');
    if matches!(bare, "TODO" | "DONE" | "NEXT" | "WAIT" | "WAITING")
        || matches!(tok, "todo:" | "done:" | "next:")
    {
        return mk(UnitKind::Keyword, tok.to_lowercase());
    }
    // todo.txt: a lone `x` only counts when a date follows it
    if (tok == "x" || tok == "X") && is_date(word_at(text, skip_spaces(text, end))) {
        return mk(UnitKind::Keyword, "x".to_string());
    }
    if tok != "📅" && tok.chars().count() <= 6 && !tok.chars().any(char::is_alphanumeric) {
        return mk(UnitKind::Symbol, tok.to_string());
    }
    None
}

/// The markers in front of a line, and where the words start.
fn read_lead(text: &str) -> (Vec<Unit>, usize) {
    let mut units = Vec::new();
    let mut at = skip_spaces(text, 0);
    while units.len() < 4 && at < text.len() {
        let Some(u) = unit_at(text, at) else { break };
        at = skip_spaces(text, u.end);
        units.push(u);
    }
    (units, at)
}

/// Org-mode style trailing tags: `:home:errand:`
fn org_tags(tok: &str) -> Option<Vec<String>> {
    let ok = tok.len() > 2
        && tok.starts_with(':')
        && tok.ends_with(':')
        && tok.chars().all(|c| c.is_alphanumeric() || "_@#%:-".contains(c));
    ok.then(|| tok.split(':').filter(|s| !s.is_empty()).map(String::from).collect())
}

/// The words after the markers: tags, due dates and the title.
fn read_body(text: &str, from: usize, p: &mut Parsed) {
    let toks = tokens(text, from);
    let last = toks.len().saturating_sub(1);
    let mut title: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let (at, tok) = toks[i];
        let end = at + tok.len();
        if let Some(d) = tok.strip_prefix("due:").filter(|d| is_date(d)) {
            p.due = Some(d.to_string());
            p.spans.push(Span { start: at, end, role: Role::Due });
        } else if tok == "📅" && i + 1 < toks.len() && is_date(toks[i + 1].1) {
            let (dat, d) = toks[i + 1];
            p.due = Some(d.to_string());
            p.spans.push(Span { start: at, end: dat + d.len(), role: Role::Due });
            i += 1;
        } else if (tok.starts_with('#') || tok.starts_with('@')) && tok.len() > 1 {
            let tag = tok[1..].trim_end_matches(|c: char| ",.;:!?".contains(c));
            if !tag.is_empty() {
                p.tags.push(tag.to_string());
            }
            p.spans.push(Span { start: at, end, role: Role::Tag });
        } else if i == last && toks.len() > 1 && org_tags(tok).is_some() {
            p.tags.extend(org_tags(tok).unwrap_or_default());
            p.spans.push(Span { start: at, end, role: Role::Tag });
        } else {
            title.push(tok);
            p.spans.push(Span { start: at, end, role: Role::Title });
        }
        i += 1;
    }
    p.title = title.join(" ");
}

fn merge_title_spans(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::with_capacity(spans.len());
    for s in spans {
        let join = s.role == Role::Title && matches!(out.last(), Some(p) if p.role == Role::Title);
        if join {
            if let Some(p) = out.last_mut() {
                p.end = s.end;
            }
        } else {
            out.push(s);
        }
    }
    out
}

// ───────────────────────── layer 3: groups ─────────────────────────

struct Analyzed {
    units: Vec<String>,
    parsed: Parsed,
}

/// Splits one line into its start and its words. `None` for lines with no words.
fn analyze(text: &str) -> Option<Analyzed> {
    if is_noise(text) {
        return None;
    }
    let (units, body) = read_lead(text);
    let mut p = Parsed::default();
    for u in &units {
        let role = match u.kind {
            UnitKind::Priority(_) => Role::Priority,
            UnitKind::Date => Role::Date,
            _ => Role::Marker,
        };
        p.spans.push(Span { start: u.start, end: u.end, role });
        match u.kind {
            UnitKind::Priority(c) if p.priority.is_none() => p.priority = Some(c),
            UnitKind::Date => p.dates.push(text[u.start..u.end].to_string()),
            _ => {}
        }
    }
    read_body(text, body, &mut p);
    if p.title.is_empty() {
        return None;
    }
    p.spans.sort_by_key(|s| s.start);
    p.spans = merge_title_spans(std::mem::take(&mut p.spans));
    Some(Analyzed { units: units.into_iter().map(|u| u.norm).collect(), parsed: p })
}

#[derive(Clone, Debug)]
struct Info {
    number: usize,
    raw: String,
    group: usize,
    parsed: Parsed,
}

/// Every line that starts the same way.
#[derive(Clone, Debug)]
struct Group {
    /// the start as text, e.g. `[ ] -`; empty for lines with no marker
    shape: String,
    members: Vec<usize>,
    guess: Meaning,
    decided: Option<Meaning>,
}

/// Groups all lines by their start. Biggest group first, then by first appearance.
fn discover(lines: &[RawLine], stats: &Stats) -> (Vec<Info>, Vec<Group>) {
    let mut infos: Vec<Info> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut by_shape: BTreeMap<String, usize> = BTreeMap::new();
    for l in lines {
        let Some(a) = analyze(&l.text) else { continue };
        let shape = a.units.join(" ");
        let gi = match by_shape.get(&shape) {
            Some(&g) => g,
            None => {
                groups.push(Group {
                    guess: guess_meaning(&a.units, &shape, stats),
                    shape: shape.clone(),
                    members: Vec::new(),
                    decided: None,
                });
                by_shape.insert(shape, groups.len() - 1);
                groups.len() - 1
            }
        };
        groups[gi].members.push(infos.len());
        infos.push(Info { number: l.number, raw: l.text.clone(), group: gi, parsed: a.parsed });
    }
    let mut order: Vec<usize> = (0..groups.len()).collect();
    order.sort_by(|&a, &b| groups[b].members.len().cmp(&groups[a].members.len()).then(a.cmp(&b)));
    let mut rank = vec![0; groups.len()];
    for (new, &old) in order.iter().enumerate() {
        rank[old] = new;
    }
    let mut slots: Vec<Option<Group>> = groups.into_iter().map(Some).collect();
    let groups: Vec<Group> = order.iter().filter_map(|&o| slots[o].take()).collect();
    for info in &mut infos {
        info.group = rank[info.group];
    }
    (infos, groups)
}

// ───────────────────────── layer 4: guessing ─────────────────────────

/// The shipped half of the guessing: a table, not code. A new row teaches the app a
/// new marker. Keys are the normalised unit forms (see `Unit::norm`).
const LIBRARY: &[(&str, Meaning)] = &[
    ("[ ]", Meaning::Todo),
    ("( )", Meaning::Todo),
    ("☐", Meaning::Todo),
    ("todo", Meaning::Todo),
    ("todo:", Meaning::Todo),
    ("next", Meaning::Todo),
    ("next:", Meaning::Todo),
    ("wait", Meaning::Todo),
    ("waiting", Meaning::Todo),
    ("[x]", Meaning::Done),
    ("(x)", Meaning::Done),
    ("[✓]", Meaning::Done),
    ("[✔]", Meaning::Done),
    ("☑", Meaning::Done),
    ("✓", Meaning::Done),
    ("✔", Meaning::Done),
    ("done", Meaning::Done),
    ("done:", Meaning::Done),
    ("x", Meaning::Done),
    ("[-]", Meaning::Ignore),
    ("[✗]", Meaning::Ignore),
    ("[✘]", Meaning::Ignore),
    ("#", Meaning::Section),
    ("##", Meaning::Section),
    ("###", Meaning::Section),
    ("####", Meaning::Section),
    ("#####", Meaning::Section),
    ("######", Meaning::Section),
];

/// What a kind of start probably means: your earlier answers first, then the table,
/// and a bullet nobody knows is "a todo to add".
fn guess_meaning(units: &[String], shape: &str, stats: &Stats) -> Meaning {
    if let Some(m) = stats.best(shape) {
        return m;
    }
    units
        .iter()
        .find_map(|u| LIBRARY.iter().find(|e| e.0 == u.as_str()).map(|e| e.1))
        .unwrap_or(Meaning::Todo)
}

/// The learning half: how often each kind of start was given each meaning. Plain
/// counters, saved by `storage` (see `to_text` / `from_text` for a tiny format).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    by_shape: BTreeMap<String, [u32; 4]>,
}

impl Stats {
    pub fn record(&mut self, shape: &str, m: Meaning) {
        self.by_shape.entry(shape.to_string()).or_insert([0; 4])[m.index()] += 1;
    }

    pub fn counts(&self, shape: &str) -> [u32; 4] {
        self.by_shape.get(shape).copied().unwrap_or([0; 4])
    }

    /// The meaning this start was given most often, if it was ever answered.
    pub fn best(&self, shape: &str) -> Option<Meaning> {
        let c = self.counts(shape);
        let most = *c.iter().max()?;
        if most == 0 {
            return None;
        }
        Meaning::ALL.into_iter().find(|m| c[m.index()] == most)
    }

    pub fn merge(&mut self, other: &Stats) {
        for (k, v) in &other.by_shape {
            let c = self.by_shape.entry(k.clone()).or_insert([0; 4]);
            for i in 0..4 {
                c[i] += v[i];
            }
        }
    }

    /// One line per start: `todo done heading skip<TAB>shape`.
    pub fn to_text(&self) -> String {
        self.by_shape
            .iter()
            .map(|(k, c)| format!("{} {} {} {}\t{}\n", c[0], c[1], c[2], c[3], k))
            .collect()
    }

    pub fn from_text(s: &str) -> Stats {
        let mut st = Stats::default();
        for line in s.lines() {
            let Some((nums, shape)) = line.split_once('\t') else { continue };
            let v: Vec<u32> = nums.split_whitespace().filter_map(|n| n.parse().ok()).collect();
            if v.len() == 4 {
                st.by_shape.insert(shape.to_string(), [v[0], v[1], v[2], v[3]]);
            }
        }
        st
    }
}

// ───────────────────────── layer 5: questions and answers ─────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestionKind {
    /// "All 100 lines start with `[ ] -`. I think each is a todo. Right?"
    Lead {
        /// the start as text; empty when the lines have no marker
        lead: String,
        count: usize,
        of: usize,
        guess: Meaning,
        /// what the editable label can be changed to
        choices: Vec<Meaning>,
    },
    /// "`a5` appears 2 times. Add it once?"
    Duplicate { title: String, count: usize, lines: Vec<usize> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub kind: QuestionKind,
    /// the sentence to ask
    pub prompt: String,
    /// sample lines to show, joined by newlines, highlighted by `spans`
    pub shown: String,
    pub spans: Vec<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// accept the guess (meaning question) / add it once (duplicate question)
    Yes,
    /// skip these lines (meaning question) / add every copy (duplicate question)
    No,
    /// meaning question: the label was changed, accept it with this meaning
    Mean(Meaning),
    /// finish now; kinds of start that were not answered yet are not added
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerError {
    /// there is nothing left to ask
    NoQuestion,
    /// that answer does not belong to this question
    NotAllowed,
}

impl std::fmt::Display for AnswerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AnswerError::NoQuestion => "nothing left to answer",
            AnswerError::NotAllowed => "that answer does not fit this question",
        })
    }
}

fn lead_prompt(lead: &str, count: usize, of: usize, guess: Meaning) -> String {
    let which = if lead.is_empty() {
        "Lines with no marker in front".to_string()
    } else {
        format!("Lines that start with \"{lead}\"")
    };
    format!("{which} ({count} of {of}). I think each one is {}. Right?", guess.phrase())
}

/// Joins sample lines with newlines and moves their spans to match.
fn show(infos: &[&Info]) -> (String, Vec<Span>) {
    let mut shown = String::new();
    let mut spans = Vec::new();
    for (n, i) in infos.iter().enumerate() {
        if n > 0 {
            shown.push('\n');
        }
        let off = shown.len();
        shown.push_str(&i.raw);
        for s in &i.parsed.spans {
            spans.push(Span { start: s.start + off, end: s.end + off, role: s.role });
        }
    }
    (shown, spans)
}

// ───────────────────────── layer 6: the conversation ─────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Leads,
    Dupes,
    Done,
}

#[derive(Clone, Debug)]
struct Dupe {
    key: String,
    /// indices into `infos`
    members: Vec<usize>,
}

/// One import. Ask `question()`, give `answer()`, repeat until `question()` is `None`.
/// It only proposes: a line is added only after its kind of start got an answer.
#[derive(Clone, Debug)]
pub struct Session {
    infos: Vec<Info>,
    groups: Vec<Group>,
    stage: Stage,
    cursor: usize,
    dupes: Vec<Dupe>,
    once: BTreeSet<String>,
    learned: Stats,
}

impl Session {
    pub fn open(source: &str, stats: &Stats) -> Session {
        let (infos, groups) = discover(&read_lines(source), stats);
        let stage = if infos.is_empty() { Stage::Done } else { Stage::Leads };
        Session {
            infos,
            groups,
            stage,
            cursor: 0,
            dupes: Vec::new(),
            once: BTreeSet::new(),
            learned: Stats::default(),
        }
    }

    pub fn question(&self) -> Option<Question> {
        match self.stage {
            Stage::Done => None,
            Stage::Leads => {
                let g = &self.groups[self.cursor];
                let sample: Vec<&Info> = g.members.iter().take(3).map(|&i| &self.infos[i]).collect();
                let (shown, spans) = show(&sample);
                let of = self.infos.len();
                Some(Question {
                    prompt: lead_prompt(&g.shape, g.members.len(), of, g.guess),
                    kind: QuestionKind::Lead {
                        lead: g.shape.clone(),
                        count: g.members.len(),
                        of,
                        guess: g.guess,
                        choices: Meaning::ALL.to_vec(),
                    },
                    shown,
                    spans,
                })
            }
            Stage::Dupes => {
                let d = &self.dupes[self.cursor];
                let all: Vec<&Info> = d.members.iter().map(|&i| &self.infos[i]).collect();
                let title = all[0].parsed.title.clone();
                let lines: Vec<usize> = all.iter().map(|i| i.number).collect();
                let list: Vec<String> = lines.iter().map(|n| n.to_string()).collect();
                let sample: Vec<&Info> = all.iter().take(4).copied().collect();
                let (shown, spans) = show(&sample);
                Some(Question {
                    prompt: format!(
                        "\"{title}\" appears {} times (lines {}). Add it once?",
                        lines.len(),
                        list.join(", ")
                    ),
                    kind: QuestionKind::Duplicate { title, count: lines.len(), lines },
                    shown,
                    spans,
                })
            }
        }
    }

    pub fn answer(&mut self, a: Answer) -> Result<(), AnswerError> {
        match (self.stage, a) {
            (Stage::Done, _) => Err(AnswerError::NoQuestion),
            (_, Answer::Stop) => {
                self.stage = Stage::Done;
                Ok(())
            }
            (Stage::Leads, Answer::Yes) => {
                let m = self.groups[self.cursor].guess;
                self.decide(m);
                Ok(())
            }
            (Stage::Leads, Answer::No) => {
                self.decide(Meaning::Ignore);
                Ok(())
            }
            (Stage::Leads, Answer::Mean(m)) => {
                self.decide(m);
                Ok(())
            }
            (Stage::Dupes, Answer::Yes) => {
                let key = self.dupes[self.cursor].key.clone();
                self.once.insert(key);
                self.next_dupe();
                Ok(())
            }
            (Stage::Dupes, Answer::No) => {
                self.next_dupe();
                Ok(())
            }
            (Stage::Dupes, Answer::Mean(_)) => Err(AnswerError::NotAllowed),
        }
    }

    fn decide(&mut self, m: Meaning) {
        let g = &mut self.groups[self.cursor];
        g.decided = Some(m);
        let shape = g.shape.clone();
        self.learned.record(&shape, m);
        self.cursor += 1;
        if self.cursor >= self.groups.len() {
            self.start_dupes();
        }
    }

    /// After the meanings are settled: find todos that were written more than once.
    fn start_dupes(&mut self) {
        let mut by_key: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, info) in self.infos.iter().enumerate() {
            if matches!(self.groups[info.group].decided, Some(Meaning::Todo) | Some(Meaning::Done)) {
                by_key.entry(info.parsed.title.to_lowercase()).or_default().push(i);
            }
        }
        let mut dupes: Vec<Dupe> = by_key
            .into_iter()
            .filter(|(_, m)| m.len() > 1)
            .map(|(key, members)| Dupe { key, members })
            .collect();
        dupes.sort_by_key(|d| d.members[0]);
        self.cursor = 0;
        self.stage = if dupes.is_empty() { Stage::Done } else { Stage::Dupes };
        self.dupes = dupes;
    }

    fn next_dupe(&mut self) {
        self.cursor += 1;
        if self.cursor >= self.dupes.len() {
            self.stage = Stage::Done;
        }
    }

    pub fn finished(&self) -> bool {
        self.stage == Stage::Done
    }

    /// (questions answered, questions known so far). Duplicate questions only appear
    /// once every kind of start has an answer, so the total can grow.
    pub fn progress(&self) -> (usize, usize) {
        let g = self.groups.len();
        match self.stage {
            Stage::Leads => (self.cursor, g),
            Stage::Dupes => (g + self.cursor, g + self.dupes.len()),
            Stage::Done => (g + self.dupes.len(), g + self.dupes.len()),
        }
    }

    /// What the user has approved, in file order. Call it when `finished()`.
    pub fn drafts(&self) -> Vec<Draft> {
        let mut out = Vec::new();
        let mut section: Option<String> = None;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for info in &self.infos {
            let Some(m) = self.groups[info.group].decided else { continue };
            match m {
                Meaning::Ignore => {}
                Meaning::Section => section = Some(info.parsed.title.clone()),
                Meaning::Todo | Meaning::Done => {
                    let key = info.parsed.title.to_lowercase();
                    if self.once.contains(&key) && !seen.insert(key) {
                        continue;
                    }
                    let p = &info.parsed;
                    out.push(Draft {
                        source_line: info.number,
                        title: p.title.clone(),
                        done: m == Meaning::Done,
                        priority: p.priority,
                        dates: p.dates.clone(),
                        due: p.due.clone(),
                        tags: p.tags.clone(),
                        section: section.clone(),
                    });
                }
            }
        }
        out
    }

    /// Counter changes from this import, to merge into the saved ones.
    pub fn learned(&self) -> &Stats {
        &self.learned
    }
}

// ───────────────────────── tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "[ ] - a1\n[ ] - a2\n[ ] - a3\n[ ] - a4\n[ ] - a5\n[ ] - a5\n[ ] - a6\n[ ] - a7\n";

    fn titles(d: &[Draft]) -> Vec<&str> {
        d.iter().map(|d| d.title.as_str()).collect()
    }

    fn lead_guess(q: &Question) -> Meaning {
        match &q.kind {
            QuestionKind::Lead { guess, .. } => *guess,
            other => panic!("not a lead question: {other:?}"),
        }
    }

    fn check_spans(q: &Question) {
        let mut prev = 0;
        for s in &q.spans {
            assert!(s.start < s.end && s.end <= q.shown.len(), "{s:?} in {:?}", q.shown);
            assert!(q.shown.is_char_boundary(s.start) && q.shown.is_char_boundary(s.end));
            assert!(s.start >= prev, "spans overlap in {:?}", q.shown);
            prev = s.end;
        }
    }

    #[test]
    fn one_question_for_the_whole_old_list() {
        let mut s = Session::open(OLD, &Stats::default());
        let q = s.question().unwrap();
        assert_eq!(
            q.kind,
            QuestionKind::Lead {
                lead: "[ ] -".into(),
                count: 8,
                of: 8,
                guess: Meaning::Todo,
                choices: Meaning::ALL.to_vec(),
            }
        );
        assert!(q.prompt.contains("[ ] -") && q.prompt.contains("8 of 8"));
        assert_eq!(q.shown, "[ ] - a1\n[ ] - a2\n[ ] - a3");
        let first: Vec<(&str, Role)> =
            q.spans.iter().take(3).map(|s| (&q.shown[s.start..s.end], s.role)).collect();
        assert_eq!(first, vec![("[ ]", Role::Marker), ("-", Role::Marker), ("a1", Role::Title)]);

        s.answer(Answer::Yes).unwrap();
        // the one repeated line is asked about; the other seven are not
        let q = s.question().unwrap();
        assert!(matches!(&q.kind, QuestionKind::Duplicate { title, count: 2, lines } if title == "a5" && lines == &vec![5, 6]));
        s.answer(Answer::Yes).unwrap();
        assert!(s.finished());
        assert_eq!(titles(&s.drafts()), vec!["a1", "a2", "a3", "a4", "a5", "a6", "a7"]);
        assert!(s.drafts().iter().all(|d| !d.done));
    }

    #[test]
    fn duplicates_can_all_be_kept() {
        let mut s = Session::open(OLD, &Stats::default());
        s.answer(Answer::Yes).unwrap();
        s.answer(Answer::No).unwrap();
        assert_eq!(s.drafts().len(), 8);
    }

    #[test]
    fn user_changes_the_guess_and_the_app_learns_it() {
        let src = "[x] - a1\n[x] - a2\n";
        let mut s = Session::open(src, &Stats::default());
        assert_eq!(lead_guess(&s.question().unwrap()), Meaning::Done);
        s.answer(Answer::Mean(Meaning::Todo)).unwrap();
        assert!(s.finished());
        assert!(s.drafts().iter().all(|d| !d.done));

        let mut saved = Stats::default();
        saved.merge(s.learned());
        let saved = Stats::from_text(&saved.to_text());
        let s2 = Session::open(src, &saved);
        assert_eq!(lead_guess(&s2.question().unwrap()), Meaning::Todo);
        // the library is still used for starts that were never answered
        let s3 = Session::open("[ ] - a1\n", &saved);
        assert_eq!(lead_guess(&s3.question().unwrap()), Meaning::Todo);
    }

    #[test]
    fn different_starts_get_their_own_question_and_headings_group_lines() {
        let src = "## Home\n[ ] - a1\n[ ] - a2\n[x] - b1\nplain line\n";
        let mut s = Session::open(src, &Stats::default());
        let mut asked = Vec::new();
        while let Some(q) = s.question() {
            check_spans(&q);
            if let QuestionKind::Lead { lead, count, guess, .. } = &q.kind {
                asked.push((lead.clone(), *count, *guess));
            }
            s.answer(Answer::Yes).unwrap();
        }
        assert_eq!(
            asked,
            vec![
                ("[ ] -".to_string(), 2, Meaning::Todo),
                ("##".to_string(), 1, Meaning::Section),
                ("[x] -".to_string(), 1, Meaning::Done),
                (String::new(), 1, Meaning::Todo),
            ]
        );
        let d = s.drafts();
        assert_eq!(titles(&d), vec!["a1", "a2", "b1", "plain line"]);
        assert_eq!(d.iter().map(|x| x.done).collect::<Vec<_>>(), vec![false, false, true, false]);
        assert!(d.iter().all(|x| x.section.as_deref() == Some("Home")));
    }

    #[test]
    fn skip_and_stop_add_nothing_for_those_lines() {
        let mut s = Session::open("[ ] - a1\n[x] - b1\n", &Stats::default());
        s.answer(Answer::No).unwrap(); // first kind of start: skip
        assert!(s.question().is_some());
        s.answer(Answer::Stop).unwrap(); // second kind: never answered
        assert!(s.finished());
        assert!(s.drafts().is_empty());
    }

    #[test]
    fn reads_priority_dates_tags_and_due() {
        let src = "- [ ] 2026-11-01 (A) call dentist #health due:2026-11-25\n";
        let mut s = Session::open(src, &Stats::default());
        let q = s.question().unwrap();
        assert!(matches!(&q.kind, QuestionKind::Lead { lead, .. } if lead == "- [ ] DATE (?)"));
        check_spans(&q);
        s.answer(Answer::Yes).unwrap();
        let d = &s.drafts()[0];
        assert_eq!(d.title, "call dentist");
        assert_eq!(d.priority, Some('A'));
        assert_eq!(d.dates, vec!["2026-11-01"]);
        assert_eq!(d.due.as_deref(), Some("2026-11-25"));
        assert_eq!(d.tags, vec!["health"]);
        assert!(!d.done);
    }

    #[test]
    fn numbered_lists_share_one_start() {
        let mut s = Session::open("1. milk\n2. bread\n10. eggs\n", &Stats::default());
        let q = s.question().unwrap();
        assert!(matches!(&q.kind, QuestionKind::Lead { lead, count: 3, .. } if lead == "#."));
        s.answer(Answer::Yes).unwrap();
        assert_eq!(titles(&s.drafts()), vec!["milk", "bread", "eggs"]);
    }

    #[test]
    fn org_and_todotxt_markers_come_from_the_table() {
        let mut s = Session::open("* TODO [#B] clean desk :tidy:\n** DONE file taxes\n", &Stats::default());
        while let Some(q) = s.question() {
            check_spans(&q);
            s.answer(Answer::Yes).unwrap();
        }
        let d = s.drafts();
        assert_eq!(titles(&d), vec!["clean desk", "file taxes"]);
        assert_eq!((d[0].done, d[0].priority, d[0].tags.clone()), (false, Some('B'), vec!["tidy".to_string()]));
        assert!(d[1].done);

        let mut s = Session::open("x 2026-11-10 buy milk\nx 2026-11-11 call mom\n", &Stats::default());
        assert_eq!(lead_guess(&s.question().unwrap()), Meaning::Done);
        s.answer(Answer::Yes).unwrap();
        let d = s.drafts();
        assert_eq!(titles(&d), vec!["buy milk", "call mom"]);
        assert_eq!(d[0].dates, vec!["2026-11-10"]);
    }

    #[test]
    fn spans_stay_valid_for_every_sample() {
        for src in [OLD, "## Home\n[ ] - a1\n[x] - b1\nplain line\n", "- [ ] 2026-11-01 (A) call #h due:2026-11-25\n", "- [x] bread 📅 2026-11-22 #home\n"] {
            let mut s = Session::open(src, &Stats::default());
            while let Some(q) = s.question() {
                check_spans(&q);
                s.answer(Answer::Yes).unwrap();
            }
        }
        let mut s = Session::open("- [x] bread 📅 2026-11-22 #home\n", &Stats::default());
        let q = s.question().unwrap();
        let shown: Vec<(&str, Role)> = q.spans.iter().map(|s| (&q.shown[s.start..s.end], s.role)).collect();
        assert_eq!(
            shown,
            vec![("-", Role::Marker), ("[x]", Role::Marker), ("bread", Role::Title), ("📅 2026-11-22", Role::Due), ("#home", Role::Tag)]
        );
        s.answer(Answer::Yes).unwrap();
        assert_eq!(s.drafts()[0].due.as_deref(), Some("2026-11-22"));
    }

    #[test]
    fn answers_must_fit_and_empty_files_ask_nothing() {
        let mut s = Session::open(OLD, &Stats::default());
        s.answer(Answer::Yes).unwrap();
        assert_eq!(s.answer(Answer::Mean(Meaning::Done)), Err(AnswerError::NotAllowed));
        s.answer(Answer::Yes).unwrap();
        assert_eq!(s.answer(Answer::Yes), Err(AnswerError::NoQuestion));
        assert!(Session::open("", &Stats::default()).question().is_none());
        assert!(Session::open("\n \n---\n[ ] -\n", &Stats::default()).question().is_none());
    }

    #[test]
    fn counters_round_trip_and_pick_the_most_common_answer() {
        let mut st = Stats::default();
        st.record("[ ] -", Meaning::Done);
        st.record("[ ] -", Meaning::Todo);
        st.record("[ ] -", Meaning::Todo);
        st.record("", Meaning::Ignore);
        assert_eq!(st.best("[ ] -"), Some(Meaning::Todo));
        assert_eq!(st.best("nope"), None);
        assert_eq!(Stats::from_text(&st.to_text()), st);
        let mut both = Stats::default();
        both.merge(&st);
        both.merge(&st);
        assert_eq!(both.counts("[ ] -"), [4, 2, 0, 0]);
    }

    #[test]
    fn progress_counts_questions() {
        let mut s = Session::open(OLD, &Stats::default());
        assert_eq!(s.progress(), (0, 1));
        s.answer(Answer::Yes).unwrap();
        assert_eq!(s.progress(), (1, 2));
        s.answer(Answer::No).unwrap();
        assert_eq!(s.progress(), (2, 2));
        assert_eq!(Meaning::from_name("heading"), Some(Meaning::Section));
    }
}
