//! The one thing. Every managed entity is an Item. Tasks, events, dailies,
//! habits, notes, projects — same struct, different `kind`. One file:
//! items.json. Views are filters over items; the AI is the manager.
//!
//! `Item` is what the AI reasons about and proposes changes to. The diary is
//! the raw input the AI reads to understand the user; it is not something the
//! AI edits.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::Entry;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,

    pub kind: Kind,
    #[serde(default)]
    pub state: State,
    #[serde(default)]
    pub tier: Tier,
    #[serde(default)]
    pub throwable: Throwable,

    #[serde(default)]
    pub when: Option<When>,
    #[serde(default)]
    pub minutes: u32,
    #[serde(default)]
    pub floor: u32,

    #[serde(default)]
    pub progress: BTreeMap<NaiveDate, u32>,

    #[serde(default)]
    pub parent_id: Option<u64>,

    #[serde(default)]
    pub ai_notes: String,
    #[serde(default)]
    pub tier_locked: bool,
    #[serde(default)]
    pub check_in: bool,

    #[serde(default)]
    pub created: String,
}

impl Entry for Item {
    const FILE: &'static str = "items.json";
    fn id(&self) -> u64 {
        self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn body(&self) -> &str {
        &self.body
    }
    fn tags(&self) -> &[String] {
        &self.tags
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    Task,
    Event,
    Daily { days: Vec<u8> },
    HabitMake { target: u32 },
    HabitQuit {
        target: u32,
        #[serde(default)]
        replacement: Option<u64>,
    },
    Note,
    Project,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum State {
    #[default]
    Open,
    Done { on: NaiveDate },
    Failed { on: NaiveDate, why: String },
    Waiting { for_date: NaiveDate },
    Deferred { until: NaiveDate },
    Skipped { on: NaiveDate },
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Tier {
    Requirement,
    #[default]
    Developing,
    Optional,
    Quitting,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Throwable {
    Never,
    UnderPressure,
    #[default]
    Anytime,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct When {
    pub date: NaiveDate,
    pub start: u32,
    pub minutes: u32,
    #[serde(default)]
    pub actual: Option<u32>,
}

// ---------------------------------------------------------------------------
// Small conveniences. Used by views and the AI.
// ---------------------------------------------------------------------------

impl Item {
    pub fn day(&self) -> Option<NaiveDate> {
        self.when.as_ref().map(|w| w.date)
    }

    pub fn effective_minutes(&self) -> u32 {
        if let Some(w) = &self.when {
            if let Some(actual) = w.actual {
                return actual;
            }
            return self.minutes.max(w.minutes);
        }
        self.minutes
    }

    pub fn is_open(&self) -> bool {
        matches!(self.state, State::Open | State::Waiting { .. })
    }

    pub fn is_habit(&self) -> bool {
        matches!(self.kind, Kind::HabitMake { .. } | Kind::HabitQuit { .. })
    }

    pub fn is_task_like(&self) -> bool {
        matches!(self.kind, Kind::Task | Kind::Event | Kind::Daily { .. })
    }

    pub fn target(&self) -> u32 {
        match self.kind {
            Kind::HabitMake { target } | Kind::HabitQuit { target, .. } => target,
            _ => 0,
        }
    }

    pub fn count_on(&self, day: NaiveDate) -> u32 {
        self.progress.get(&day).copied().unwrap_or(0)
    }

    pub fn habit_good_on(&self, day: NaiveDate) -> bool {
        match self.kind {
            Kind::HabitMake { target } => self.count_on(day) >= target.max(1),
            Kind::HabitQuit { target, .. } => self.count_on(day) <= target,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn task(id: u64, title: &str) -> Item {
        Item {
            id,
            title: title.into(),
            body: String::new(),
            tags: vec![],
            kind: Kind::Task,
            state: State::Open,
            tier: Tier::Developing,
            throwable: Throwable::Anytime,
            when: None,
            minutes: 0,
            floor: 0,
            progress: BTreeMap::new(),
            parent_id: None,
            ai_notes: String::new(),
            tier_locked: false,
            check_in: false,
            created: "2026-11-04".into(),
        }
    }

    #[test]
    fn bare_task_round_trips() {
        let t = task(5, "buy milk");
        let text = serde_json::to_string(&t).unwrap();
        let back: Item = serde_json::from_str(&text).unwrap();
        assert_eq!(back.id, 5);
        assert_eq!(back.title, "buy milk");
        assert!(matches!(back.kind, Kind::Task));
        assert!(matches!(back.state, State::Open));
        assert_eq!(back.tier, Tier::Developing);
        assert!(back.when.is_none());
    }

    #[test]
    fn scheduled_event_round_trips() {
        let mut e = task(10, "dentist");
        e.kind = Kind::Event;
        e.tier = Tier::Requirement;
        e.throwable = Throwable::Never;
        e.when = Some(When {
            date: d("2026-11-04"),
            start: 14 * 60,
            minutes: 90,
            actual: None,
        });
        e.minutes = 90;
        e.floor = 90;

        let text = serde_json::to_string(&e).unwrap();
        let back: Item = serde_json::from_str(&text).unwrap();
        assert_eq!(back.id, 10);
        assert!(matches!(back.kind, Kind::Event));
        assert_eq!(back.tier, Tier::Requirement);
        assert_eq!(back.throwable, Throwable::Never);
        let w = back.when.unwrap();
        assert_eq!(w.date, d("2026-11-04"));
        assert_eq!(w.start, 840);
        assert_eq!(w.minutes, 90);
        assert_eq!(w.actual, None);
    }

    #[test]
    fn daily_carries_weekdays() {
        let mut dly = task(3, "read");
        dly.kind = Kind::Daily { days: vec![0, 1, 2, 3, 4] };
        dly.when = Some(When {
            date: d("2026-11-04"),
            start: 21 * 60,
            minutes: 20,
            actual: None,
        });
        dly.minutes = 20;
        dly.floor = 20;

        let text = serde_json::to_string(&dly).unwrap();
        assert!(text.contains(r#""kind":"daily""#));
        assert!(text.contains(r#""days":[0,1,2,3,4]"#));

        let back: Item = serde_json::from_str(&text).unwrap();
        match back.kind {
            Kind::Daily { days } => assert_eq!(days, vec![0, 1, 2, 3, 4]),
            _ => panic!("wrong kind"),
        }
    }

    #[test]
    fn habit_make_counts_good_when_target_reached() {
        let mut h = task(7, "read 20 pages");
        h.kind = Kind::HabitMake { target: 20 };
        h.progress.insert(d("2026-11-04"), 15);
        assert!(!h.habit_good_on(d("2026-11-04")));
        h.progress.insert(d("2026-11-04"), 20);
        assert!(h.habit_good_on(d("2026-11-04")));
        h.progress.insert(d("2026-11-04"), 25);
        assert!(h.habit_good_on(d("2026-11-04")));
    }

    #[test]
    fn habit_quit_good_when_under_limit() {
        let mut h = task(8, "no cigarettes");
        h.kind = Kind::HabitQuit { target: 0, replacement: None };
        h.progress.insert(d("2026-11-04"), 0);
        assert!(h.habit_good_on(d("2026-11-04")));
        h.progress.insert(d("2026-11-04"), 1);
        assert!(!h.habit_good_on(d("2026-11-04")));
    }

    #[test]
    fn habit_quit_with_replacement_round_trips() {
        let mut h = task(9, "no smoking");
        h.kind = Kind::HabitQuit { target: 0, replacement: Some(42) };
        let text = serde_json::to_string(&h).unwrap();
        let back: Item = serde_json::from_str(&text).unwrap();
        match back.kind {
            Kind::HabitQuit { target, replacement } => {
                assert_eq!(target, 0);
                assert_eq!(replacement, Some(42));
            }
            _ => panic!("wrong kind"),
        }
    }

    #[test]
    fn failed_state_carries_reason() {
        let mut t = task(11, "sleep");
        t.state = State::Failed {
            on: d("2026-11-04"),
            why: "slot expired".into(),
        };
        let text = serde_json::to_string(&t).unwrap();
        let back: Item = serde_json::from_str(&text).unwrap();
        match back.state {
            State::Failed { on, why } => {
                assert_eq!(on, d("2026-11-04"));
                assert_eq!(why, "slot expired");
            }
            _ => panic!("wrong state"),
        }
    }

    #[test]
    fn effective_minutes_uses_actual_when_present() {
        let mut t = task(1, "commute");
        t.minutes = 40;
        t.when = Some(When {
            date: d("2026-11-04"),
            start: 8 * 60,
            minutes: 40,
            actual: None,
        });
        assert_eq!(t.effective_minutes(), 40);

        t.when.as_mut().unwrap().actual = Some(20);
        assert_eq!(t.effective_minutes(), 20);
    }

    #[test]
    fn tier_locked_round_trips() {
        let mut t = task(2, "write report");
        t.tier_locked = true;
        t.tier = Tier::Requirement;
        let text = serde_json::to_string(&t).unwrap();
        let back: Item = serde_json::from_str(&text).unwrap();
        assert!(back.tier_locked);
        assert_eq!(back.tier, Tier::Requirement);
    }

    #[test]
    fn unknown_kind_does_not_silently_succeed() {
        let bad = r#"{"id":1,"title":"x","kind":"not_a_kind"}"#;
        let parsed: Result<Item, _> = serde_json::from_str(bad);
        assert!(parsed.is_err());
    }

    #[test]
    fn ai_notes_round_trip() {
        let mut t = task(4, "groceries");
        t.ai_notes = "user has deferred this three times".into();
        let text = serde_json::to_string(&t).unwrap();
        let back: Item = serde_json::from_str(&text).unwrap();
        assert_eq!(back.ai_notes, "user has deferred this three times");
    }
}
