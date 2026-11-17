//! Action vocabulary — the ONLY way state gets mutated.

use crate::app::App;
use crate::mod_daily;
use crate::mod_diary;
use crate::mod_todo;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Source {
    Human,
    Ai { suggestion_id: String },
    Daily { daily_id: String },
    Import,
}

impl Source {
    pub fn label(&self) -> &'static str {
        match self {
            Source::Human => "human",
            Source::Ai { .. } => "ai",
            Source::Daily { .. } => "daily",
            Source::Import => "import",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    AddTodo { text: String, info: String, source: Source },
    UpdateTodo { id: String, text: String, info: String, source: Source },
    ToggleTodo { id: String, source: Source },
    DeleteTodo { id: String, source: Source },
    AddDiaryEntry { body: String, source: Source },
    AddDaily { text: String, duration_min: Option<u32>, source: Source },
    ToggleDaily { id: String, source: Source },
    DeleteDaily { id: String, source: Source },
}

#[derive(Debug)]
pub enum DispatchError {
    NotFound(String),
    Save(String),
    Invalid(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::NotFound(s) => write!(f, "not found: {s}"),
            DispatchError::Save(s) => write!(f, "save failed: {s}"),
            DispatchError::Invalid(s) => write!(f, "invalid: {s}"),
        }
    }
}

impl std::error::Error for DispatchError {}

pub fn dispatch(action: Action, app: &mut App) -> Result<(), DispatchError> {
    match action {
        Action::AddTodo { text, info, source } =>
            mod_todo::handle_add(&mut app.todo, text, info, source),
        Action::UpdateTodo { id, text, info, source } =>
            mod_todo::handle_update(&mut app.todo, &id, text, info, source),
        Action::ToggleTodo { id, source } =>
            mod_todo::handle_toggle(&mut app.todo, &id, source),
        Action::DeleteTodo { id, source } =>
            mod_todo::handle_delete(&mut app.todo, &id, source),
        Action::AddDiaryEntry { body, source } =>
            mod_diary::handle_add(&mut app.diary, body, source),
        Action::AddDaily { text, duration_min, source } =>
            mod_daily::handle_add(&mut app.daily, text, duration_min, source),
        Action::ToggleDaily { id, source } =>
            mod_daily::handle_toggle(&mut app.daily, &id, source),
        Action::DeleteDaily { id, source } =>
            mod_daily::handle_delete(&mut app.daily, &id, source),

    }
}
