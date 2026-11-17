use ratatui::buffer::Buffer;
use ratatui::layout::{Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols;
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
    StatefulWidget, Widget, Wrap,
};
use serde_json::json;

use crate::actions::{DispatchError, Source};
use crate::domain::{Status, TodoItem};
use crate::events::log_event;
use crate::storage::{load_todos, save_todos, TODOS_FILE};
use crate::styles::{
    alternate_colors, display_width, info_block, COMPLETED_TEXT_FG_COLOR, NORMAL_ROW_BG,
    SELECTED_STYLE, TEXT_FG_COLOR, TODO_HEADER_STYLE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoInputMode {
    Normal,
    EditingTitle,
    EditingInfo,
    ConfirmDelete,
}

pub struct TodoList {
    pub items: Vec<TodoItem>,
    pub state: ListState,
}

pub struct TodoApp {
    pub next_id: u64,
    pub todo_list: TodoList,
    pub last_error: Option<String>,
    pub input_mode: TodoInputMode,
    pub title_buffer: String,
    pub info_buffer: String,
    pub editing_id: Option<String>,
}

impl TodoApp {
    pub fn new(starting_next_id: u64) -> Self {
        let loaded = load_todos(starting_next_id);
        if loaded.needs_save
            && let Err(e) = save_todos(&loaded.items)
        {
            log_event(json!({"type":"save-error","file":TODOS_FILE,"error":e.to_string()}));
        }
        Self {
            next_id: loaded.next_id,
            todo_list: TodoList { items: loaded.items, state: ListState::default() },
            last_error: None,
            input_mode: TodoInputMode::Normal,
            title_buffer: String::new(),
            info_buffer: String::new(),
            editing_id: None,
        }
    }

    pub fn selected_id(&self) -> Option<String> {
        self.todo_list.state.selected()
            .map(|i| self.todo_list.items[i].id.clone())
    }

    fn log_selection(&self, before: Option<String>) {
        log_event(json!({"type":"todo-select","from":before,"to":self.selected_id()}));
    }

    pub fn select_none(&mut self) {
        let b = self.selected_id();
        self.todo_list.state.select(None);
        self.log_selection(b);
    }
    pub fn select_next(&mut self) {
        let b = self.selected_id();
        self.todo_list.state.select_next();
        self.log_selection(b);
    }
    pub fn select_previous(&mut self) {
        let b = self.selected_id();
        self.todo_list.state.select_previous();
        self.log_selection(b);
    }
    pub fn select_first(&mut self) {
        let b = self.selected_id();
        self.todo_list.state.select_first();
        self.log_selection(b);
    }
    pub fn select_last(&mut self) {
        let b = self.selected_id();
        self.todo_list.state.select_last();
        self.log_selection(b);
    }

    pub fn start_new(&mut self) {
        self.editing_id = None;
        self.input_mode = TodoInputMode::EditingTitle;
        self.title_buffer.clear();
        self.info_buffer.clear();
        log_event(json!({"type": "todo-create-started"}));
    }

    pub fn start_edit(&mut self) {
        let Some(i) = self.todo_list.state.selected() else {
            log_event(json!({"type": "todo-edit-ignored", "reason": "nothing-selected"}));
            return;
        };
        let item = &self.todo_list.items[i];
        self.editing_id = Some(item.id.clone());
        self.title_buffer = item.todo.clone();
        self.info_buffer = item.info.clone();
        self.input_mode = TodoInputMode::EditingTitle;
        log_event(json!({"type": "todo-edit-started", "id": item.id}));
    }

    pub fn start_delete(&mut self) {
        if self.todo_list.state.selected().is_none() {
            log_event(json!({"type": "todo-delete-ignored", "reason": "nothing-selected"}));
            return;
        }
        self.input_mode = TodoInputMode::ConfirmDelete;
        log_event(json!({"type": "todo-delete-confirm"}));
    }

    pub fn advance_to_info(&mut self) {
        if self.title_buffer.trim().is_empty() {
            log_event(json!({"type": "todo-create-ignored", "reason": "empty-title"}));
            return;
        }
        self.input_mode = TodoInputMode::EditingInfo;
    }

    pub fn cancel_form(&mut self) {
        log_event(json!({
            "type": "todo-form-cancelled",
            "editing_id": self.editing_id,
            "title": self.title_buffer,
            "info": self.info_buffer,
        }));
        self.reset_form();
    }

    pub fn cancel_delete(&mut self) {
        self.input_mode = TodoInputMode::Normal;
        log_event(json!({"type": "todo-delete-cancelled"}));
    }

    pub fn reset_form(&mut self) {
        self.input_mode = TodoInputMode::Normal;
        self.title_buffer.clear();
        self.info_buffer.clear();
        self.editing_id = None;
    }

    pub fn cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        let max_x = area.right().saturating_sub(1);
        let max_y = area.bottom().saturating_sub(1);
        match self.input_mode {
            TodoInputMode::Normal | TodoInputMode::ConfirmDelete => None,
            TodoInputMode::EditingTitle => {
                let used = display_width("Title: ").saturating_add(display_width(&self.title_buffer));
                let x = area.x.saturating_add(1).saturating_add(used).min(max_x);
                let y = area.y.saturating_add(1).min(max_y);
                Some((x, y))
            }
            TodoInputMode::EditingInfo => {
                let used = display_width("Info:  ").saturating_add(display_width(&self.info_buffer));
                let x = area.x.saturating_add(1).saturating_add(used).min(max_x);
                let y = area.y.saturating_add(2).min(max_y);
                Some((x, y))
            }
        }
    }

    pub fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw(format!("TODO List ({TODOS_FILE})")).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        let items: Vec<ListItem> = self.todo_list.items.iter().enumerate()
            .map(|(i, item)| ListItem::from(item).bg(alternate_colors(i)))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.todo_list.state);
    }

    pub fn render_selected_item(&self, area: Rect, buf: &mut Buffer) {
        match self.input_mode {
            TodoInputMode::Normal | TodoInputMode::ConfirmDelete =>
                self.render_selected_item_info(area, buf),
            TodoInputMode::EditingTitle | TodoInputMode::EditingInfo =>
                self.render_new_todo_form(area, buf),
        }
    }

    fn render_selected_item_info(&self, area: Rect, buf: &mut Buffer) {
        let info = if let Some(i) = self.todo_list.state.selected() {
            let it = &self.todo_list.items[i];
            match it.status {
                Status::Completed => format!("✓ DONE [{}]: {}", it.id, it.info),
                Status::Todo => format!("☐ TODO [{}]: {}", it.id, it.info),
            }
        } else {
            "Nothing selected...".to_string()
        };
        Paragraph::new(info)
            .block(info_block("TODO Info"))
            .fg(TEXT_FG_COLOR)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_new_todo_form(&self, area: Rect, buf: &mut Buffer) {
        let title_style = if matches!(self.input_mode, TodoInputMode::EditingTitle) {
            Style::default().fg(Color::Yellow)
        } else { Style::default() };
        let info_style = if matches!(self.input_mode, TodoInputMode::EditingInfo) {
            Style::default().fg(Color::Yellow)
        } else { Style::default() };

        let title_line = Line::from(vec![
            ratatui::text::Span::raw("Title: "),
            ratatui::text::Span::styled(self.title_buffer.as_str(), title_style),
        ]);
        let info_line = Line::from(vec![
            ratatui::text::Span::raw("Info:  "),
            ratatui::text::Span::styled(self.info_buffer.as_str(), info_style),
        ]);

        let form_title = if self.editing_id.is_some() { "Edit Todo" } else { "New Todo" };
        Paragraph::new(vec![title_line, info_line])
            .block(info_block(form_title))
            .fg(TEXT_FG_COLOR)
            .render(area, buf);
    }
}

impl From<&TodoItem> for ListItem<'_> {
    fn from(value: &TodoItem) -> Self {
        let line = match value.status {
            Status::Todo =>
                Line::styled(format!(" ☐ [{}] {}", value.id, value.todo), TEXT_FG_COLOR),
            Status::Completed =>
                Line::styled(format!(" ✓ [{}] {}", value.id, value.todo), COMPLETED_TEXT_FG_COLOR),
        };
        ListItem::new(line)
    }
}

// ── Handlers (called only by actions::dispatch) ─────────────────

pub fn fix_selection_after_delete(todo: &mut TodoApp) {
    if todo.todo_list.items.is_empty() {
        todo.todo_list.state.select(None);
    } else if let Some(i) = todo.todo_list.state.selected()
        && i >= todo.todo_list.items.len()
    {
        todo.todo_list.state.select(Some(todo.todo_list.items.len() - 1));
    }
}

pub fn handle_add(
    todo: &mut TodoApp,
    text: String,
    info: String,
    source: Source,
) -> Result<(), DispatchError> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(DispatchError::Invalid("empty todo text".into()));
    }
    let info = info.trim().to_string();
    let id = format!("todo-{}", todo.next_id);
    todo.next_id += 1;
    log_event(json!({
        "type": "todo-create",
        "id": id,
        "status": "Todo",
        "todo": text,
        "info": info,
        "source": source.label(),
    }));
    todo.todo_list.items.push(TodoItem::new(id, Status::Todo, &text, &info));
    todo.todo_list.state.select(Some(todo.todo_list.items.len() - 1));
    save_todos(&todo.todo_list.items).map_err(|e| DispatchError::Save(e.to_string()))
}

pub fn handle_update(
    todo: &mut TodoApp,
    id: &str,
    text: String,
    info: String,
    source: Source,
) -> Result<(), DispatchError> {
    let Some(item) = todo.todo_list.items.iter_mut().find(|t| t.id == id) else {
        return Err(DispatchError::NotFound(id.into()));
    };
    item.todo = text;
    item.info = info;
    log_event(json!({"type":"todo-update","id":id,"source":source.label()}));
    save_todos(&todo.todo_list.items).map_err(|e| DispatchError::Save(e.to_string()))
}

pub fn handle_toggle(
    todo: &mut TodoApp,
    id: &str,
    source: Source,
) -> Result<(), DispatchError> {
    let Some(item) = todo.todo_list.items.iter_mut().find(|t| t.id == id) else {
        return Err(DispatchError::NotFound(id.into()));
    };
    let old = item.status;
    item.status = match old {
        Status::Completed => Status::Todo,
        Status::Todo => Status::Completed,
    };
    log_event(json!({
        "type": "todo-toggle",
        "id": id,
        "from": format!("{old:?}"),
        "to": format!("{:?}", item.status),
        "source": source.label(),
    }));
    save_todos(&todo.todo_list.items).map_err(|e| DispatchError::Save(e.to_string()))
}

pub fn handle_delete(
    todo: &mut TodoApp,
    id: &str,
    source: Source,
) -> Result<(), DispatchError> {
    let Some(i) = todo.todo_list.items.iter().position(|t| t.id == id) else {
        return Err(DispatchError::NotFound(id.into()));
    };
    todo.todo_list.items.remove(i);
    log_event(json!({"type":"todo-delete","id":id,"source":source.label()}));
    save_todos(&todo.todo_list.items).map_err(|e| DispatchError::Save(e.to_string()))
}
