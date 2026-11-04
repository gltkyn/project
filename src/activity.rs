//! The one thing. Activities are todos, cubes, dailies, events, habits —
//! the same underlying shape in different states. This file is the model.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{Entry, Group};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Tier {
    Requirement,
    Developing,
    Optional,
    Quitting,
}

impl Default for Tier {
    fn default() -> Self {
        Tier::Developing
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum Throwable {
    Never,
    UnderPressure,
    #[default]
    Anytime,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Wants {
    pub date: NaiveDate,
    pub start_min: u32,
    pub minutes: u32,
    #[serde(default)]
    pub actual_minutes: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum State {
    #[default]
    Open,
    Placed { date: NaiveDate, start: u32, length: u32 },
    Waiting { for_date: NaiveDate },
    Deferred { until: NaiveDate },
    Skipped { on: NaiveDate, why: String },
    Done { on: NaiveDate, at: String, actual: u32 },
    Failed { on: NaiveDate, scheduled_at: String },
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum ActivityKind {
    #[default]
    OneOff,
    Repeating { days: Vec<u8> },
    Quitting { replacement: Option<u64> },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Activity {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,

    #[serde(default)]
    pub tier: Tier,
    #[serde(default)]
    pub tier_locked: bool,
    #[serde(default)]
    pub group: Option<Group>,

    #[serde(default)]
    pub kind: ActivityKind,

    #[serde(default)]
    pub wants: Option<Wants>,
    #[serde(default)]
    pub floor: u32,

    #[serde(default)]
    pub throwable: Throwable,
    #[serde(default)]
    pub check_in: bool,

    #[serde(default)]
    pub state: State,

    #[serde(default)]
    pub created: String,
}

impl Entry for Activity {
    const FILE: &'static str = "activities.json";
    fn id(&self) -> u64 { self.id }
    fn title(&self) -> &str { &self.title }
    fn body(&self) -> &str { &self.notes }
    fn tags(&self) -> &[String] { &self.tags }
}

pub fn group_to_tier(g: Group) -> Tier {
    match g {
        Group::Main | Group::Steady | Group::Resurface => Tier::Developing,
        Group::Quick | Group::Secondary | Group::Serotonin => Tier::Optional,
    }
}

pub fn group_to_throwable(g: Group) -> Throwable {
    match g {
        Group::Main | Group::Steady | Group::Resurface => Throwable::Never,
        Group::Quick | Group::Secondary | Group::Serotonin => Throwable::Anytime,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn activity_round_trips() {
        let a = Activity {
            id: 5,
            title: "test".into(),
            tags: vec!["x".into()],
            notes: String::new(),
            tier: Tier::Developing,
            tier_locked: false,
            group: Some(Group::Main),
            kind: ActivityKind::OneOff,
            wants: Some(Wants {
                date: d("2026-11-03"),
                start_min: 540,
                minutes: 30,
                actual_minutes: None,
            }),
            floor: 20,
            throwable: Throwable::Never,
            check_in: false,
            state: State::Placed {
                date: d("2026-11-03"),
                start: 540,
                length: 30,
            },
            created: "2026-11-03".into(),
        };
        let text = serde_json::to_string(&a).unwrap();
        let back: Activity = serde_json::from_str(&text).unwrap();
        assert_eq!(back.id, 5);
        assert_eq!(back.title, "test");
        assert!(matches!(back.state, State::Placed { start, .. } if start == 540));
    }

    #[test]
    fn empty_activity_round_trips() {
        let a = Activity {
            id: 1,
            title: "t".into(),
            tags: vec![],
            notes: String::new(),
            tier: Tier::Optional,
            tier_locked: false,
            group: None,
            kind: ActivityKind::OneOff,
            wants: None,
            floor: 30,
            throwable: Throwable::Anytime,
            check_in: false,
            state: State::Open,
            created: String::new(),
        };
        let text = serde_json::to_string(&a).unwrap();
        let back: Activity = serde_json::from_str(&text).unwrap();
        assert!(matches!(back.state, State::Open));
    }

    #[test]
    fn group_maps_to_tier() {
        assert!(matches!(group_to_tier(Group::Main), Tier::Developing));
        assert!(matches!(group_to_tier(Group::Quick), Tier::Optional));
    }
}
