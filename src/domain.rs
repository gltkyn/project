//! Shared entity types. Owned by no module — used by storage, actions, ai,
//! and the modules themselves. Keep this file free of behavior beyond
//! constructors and serialization.

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::actions::Source;

// ── Diary ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub timestamp: String,
    pub content: String,
}

impl Message {
    pub fn new(id: String, content: String) -> Self {
        Self {
            id,
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            content,
        }
    }

    pub fn to_line(&self) -> String {
        format!("{} | {} | {}", self.id, self.timestamp, self.content)
    }

    pub fn from_line(line: &str) -> Option<Self> {
        if line.starts_with('#') {
            return None;
        }
        let mut parts = line.splitn(3, " | ");
        let id = parts.next()?;
        if !id.starts_with("msg-") {
            return None;
        }
        let timestamp = parts.next()?.to_string();
        let content = parts.next()?.to_string();
        Some(Self { id: id.to_string(), timestamp, content })
    }
}

// ── Todo ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub status: Status,
    pub todo: String,
    pub info: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Status { Todo, Completed }

impl TodoItem {
    pub fn new(id: String, status: Status, todo: &str, info: &str) -> Self {
        Self { id, status, todo: todo.to_string(), info: info.to_string() }
    }
}

// ── Daily ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyItem {
    pub id: String,
    pub text: String,
    pub duration_min: Option<u32>,
    pub status: DailyStatus,
    pub source: Source,
    pub created_at: String,
    #[serde(default)]
    pub last_fired_date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DailyStatus { Active, Paused }
