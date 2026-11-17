use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use serde_json::json;

use crate::actions::{DispatchError, Source};
use crate::domain::Message;
use crate::events::log_event;
use crate::storage::{append_message, load_messages, LOG_FILE};
use crate::styles::display_width;

#[derive(Debug)]
pub enum DiaryMode { Normal, Editing }

pub struct DiaryApp {
    pub next_id: u64,
    pub input: String,
    pub character_index: usize,
    pub input_mode: DiaryMode,
    pub messages: Vec<Message>,
    pub last_error: Option<String>,
}

impl DiaryApp {
    pub fn new(starting_next_id: u64) -> Self {
        let mut next_id = starting_next_id;
        let messages = load_messages(&mut next_id);
        Self {
            next_id,
            input: String::new(),
            character_index: 0,
            input_mode: DiaryMode::Normal,
            messages,
            last_error: None,
        }
    }

    pub fn move_cursor_left(&mut self) {
        let n = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(n);
    }

    pub fn move_cursor_right(&mut self) {
        let n = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(n);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
        log_event(json!({
            "type": "input-char",
            "char": new_char.to_string(),
            "buf_len": self.input.chars().count(),
            "cursor": self.character_index,
        }));
    }

    pub fn byte_index(&self) -> usize {
        self.input.char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    pub fn delete_char(&mut self) {
        if self.character_index == 0 {
            log_event(json!({"type":"input-backspace-ignored","reason":"at-start"}));
            return;
        }
        let current_index = self.character_index;
        let from_left = current_index - 1;
        let removed = self.input.chars().nth(from_left);
        let before = self.input.chars().take(from_left);
        let after = self.input.chars().skip(current_index);
        self.input = before.chain(after).collect();
        self.move_cursor_left();
        log_event(json!({
            "type": "input-backspace",
            "removed": removed.map(|c| c.to_string()),
            "buf_len": self.input.chars().count(),
            "cursor": self.character_index,
        }));
    }

    pub fn clamp_cursor(&self, n: usize) -> usize {
        n.clamp(0, self.input.chars().count())
    }

    pub fn reset_cursor(&mut self) {
        self.character_index = 0;
    }
}

pub fn handle_add(
    diary: &mut DiaryApp,
    body: String,
    source: Source,
) -> Result<(), DispatchError> {
    if body.is_empty() {
        return Err(DispatchError::Invalid("empty entry".into()));
    }
    let id = format!("msg-{}", diary.next_id);
    diary.next_id += 1;
    let msg = Message::new(id, body);
    let append_result = match append_message(&msg) {
        Ok(()) => {
            log_event(json!({
                "type": "msg-submit",
                "id": msg.id,
                "timestamp": msg.timestamp,
                "content": msg.content,
                "source": source.label(),
            }));
            log_event(json!({"type": "save-message", "id": msg.id}));
            Ok(())
        }
        Err(e) => {
            log_event(json!({"type":"save-error","file":LOG_FILE,"error":e.to_string()}));
            Err(DispatchError::Save(e.to_string()))
        }
    };
    diary.messages.push(msg);
    append_result
}

// ── Render (called from app.rs) ─────────────────────────────────

pub fn render_diary_tab(diary: &DiaryApp, area: Rect, frame: &mut ratatui::Frame) {
    use ratatui::layout::{Constraint, Layout};
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(1),
    ]);
    let chunks = layout.split(area);
    let (help_area, input_area, messages_area) = (chunks[0], chunks[1], chunks[2]);

    let (msg, style) = match diary.input_mode {
        DiaryMode::Normal => (
            vec!["Press ".into(), "e".bold(), " to start editing, ".into(), "q".bold(), " to exit.".bold()],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        DiaryMode::Editing => (
            vec!["Press ".into(), "Esc".bold(), " to stop editing, ".into(), "Enter".bold(), " to save".into()],
            Style::default(),
        ),
    };
    let mut spans = msg;
    if let Some(err) = &diary.last_error {
        spans.push("   ".into());
        spans.push(Span::styled(format!("⚠ {err}"), Style::default().fg(Color::Red)));
    }
    let text = Text::from(Line::from(spans)).patch_style(style);
    frame.render_widget(Paragraph::new(text), help_area);

    let input = Paragraph::new(diary.input.as_str())
        .style(match diary.input_mode {
            DiaryMode::Normal => Style::default(),
            DiaryMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::bordered().title("Input"));
    frame.render_widget(input, input_area);

    if matches!(diary.input_mode, DiaryMode::Editing) {
        let text_before = &diary.input[..diary.byte_index()];
        let used = display_width(text_before);
        let max_x = input_area.right().saturating_sub(1);
        let max_y = input_area.bottom().saturating_sub(1);
        let x = input_area.x.saturating_add(1).saturating_add(used).min(max_x);
        let y = input_area.y.saturating_add(1).min(max_y);
        frame.set_cursor_position(Position::new(x, y));
    }

    let messages: Vec<ListItem> = diary.messages.iter().enumerate()
        .map(|(i, m)| ListItem::new(Line::from(vec![
            Span::raw(format!("{i}: ")),
            Span::styled(format!("[{}]", m.timestamp), Style::default().fg(ratatui::style::palette::tailwind::SLATE.c500)),
            Span::raw(" "),
            Span::styled(format!("{} ", m.id), Style::default().fg(ratatui::style::palette::tailwind::GREEN.c400)),
            Span::raw(m.content.as_str()),
        ])))
        .collect();
    let messages = List::new(messages)
        .block(Block::bordered().title(format!("Messages ({LOG_FILE})")));
    frame.render_widget(messages, messages_area);
}
