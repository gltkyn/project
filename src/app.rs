use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, HighlightSpacing, List, ListItem, Paragraph, StatefulWidget, Widget};
use ratatui::{DefaultTerminal, Frame};
use serde_json::json;

use crate::actions::{self, Action, Source};
use crate::ai::{RuleEngine, SuggestionInbox};
use crate::domain::DailyStatus;
use crate::events::{current_evt_counter, log_event, scan_events_for_max_ids};
use crate::mod_daily::{self, DailyApp};
use crate::mod_diary::{self, DiaryApp, DiaryMode};
use crate::mod_todo::{self, TodoApp, TodoInputMode};
use crate::storage::{
    load_counters, load_resolved_suggestion_ids, save_counters, Counters,
};
use crate::styles::{SELECTED_STYLE, TAB_ACTIVE_STYLE, TAB_INACTIVE_STYLE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab { Todo, Diary, Daily, Suggestions }

pub struct App {
    pub should_exit: bool,
    pub active_tab: Tab,
    pub todo: TodoApp,
    pub diary: DiaryApp,
    pub daily: DailyApp,
    pub suggestions: SuggestionInbox,
}

impl App {
    pub fn new() -> Self {
        let mut counters = load_counters();
        let (evt_todo, evt_msg) = scan_events_for_max_ids();
        counters.next_todo = counters.next_todo.max(evt_todo + 1);
        counters.next_msg = counters.next_msg.max(evt_msg + 1);
        counters.next_evt = counters.next_evt.max(current_evt_counter());

        let todo = TodoApp::new(counters.next_todo);
        let diary = DiaryApp::new(counters.next_msg);
        let daily = DailyApp::new(counters.next_daily);
        let resolved = load_resolved_suggestion_ids();

        log_event(json!({
            "type": "session-start",
            "todos_loaded": todo.todo_list.items.len(),
            "messages_loaded": diary.messages.len(),
            "dailies_loaded": daily.items.len(),
            "resolved_suggestions": resolved.len(),
            "next_todo": todo.next_id,
            "next_msg": diary.next_id,
            "next_daily": daily.next_id,
            "next_evt": current_evt_counter(),
        }));

        let mut app = Self {
            should_exit: false,
            active_tab: Tab::Todo,
            todo,
            diary,
            daily,
            suggestions: SuggestionInbox::new_with_resolved(resolved),
        };

        app.tick_dailies();
        app.save_counters_now();
        app
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.render(frame))?;
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_key(key);
            }
        }
        self.save_counters_now();
        Ok(())
    }

    fn save_counters_now(&self) {
        save_counters(&Counters {
            next_todo: self.todo.next_id,
            next_msg: self.diary.next_id,
            next_daily: self.daily.next_id,
            next_evt: current_evt_counter(),
        });
    }

    // ── Daily tick ─────────────────────────────────────────────

    fn tick_dailies(&mut self) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let due: Vec<(String, String, Option<u32>)> = self
            .daily
            .items
            .iter()
            .filter(|d| d.status == DailyStatus::Active)
            .filter(|d| d.last_fired_date.as_deref() != Some(&today))
            .map(|d| (d.id.clone(), d.text.clone(), d.duration_min))
            .collect();

        let mut fired: Vec<String> = Vec::new();
        for (id, text, dur) in &due {
            let info = match dur {
                Some(n) => format!("daily {} ({n} min)", id),
                None => format!("daily {id}"),
            };
            let action = Action::AddTodo {
                text: text.clone(),
                info,
                source: Source::Daily { daily_id: id.clone() },
            };
            if actions::dispatch(action, self).is_ok() {
                fired.push(id.clone());
            }
        }
        for id in &fired {
            self.daily.mark_fired(id, &today);
        }
        if !fired.is_empty() {
            self.daily.save();
            log_event(json!({"type":"daily-tick","count":fired.len(),"date":today}));
        }
    }

    // ── Key routing ────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent) {
        let before = (
            self.todo.next_id,
            self.diary.next_id,
            self.daily.next_id,
        );

        let editing = match self.active_tab {
            Tab::Todo => !matches!(self.todo.input_mode, TodoInputMode::Normal),
            Tab::Diary => matches!(self.diary.input_mode, DiaryMode::Editing),
            Tab::Daily | Tab::Suggestions => false,
        };

        if !editing {
            match key.code {
                KeyCode::Tab => {
                    let from = self.active_tab;
                    self.active_tab = match from {
                        Tab::Todo => Tab::Diary,
                        Tab::Diary => Tab::Daily,
                        Tab::Daily => Tab::Suggestions,
                        Tab::Suggestions => Tab::Todo,
                    };
                    log_event(json!({"type":"tab-switch","from":format!("{from:?}"),"to":format!("{:?}", self.active_tab)}));
                    return;
                }
                KeyCode::Char('1') => { self.active_tab = Tab::Todo; return; }
                KeyCode::Char('2') => { self.active_tab = Tab::Diary; return; }
                KeyCode::Char('3') => { self.active_tab = Tab::Daily; return; }
                KeyCode::Char('4') => { self.active_tab = Tab::Suggestions; return; }
                _ => {}
            }
        }

        match self.active_tab {
            Tab::Todo => self.handle_todo_key(key),
            Tab::Diary => self.handle_diary_key(key),
            Tab::Daily => self.handle_daily_key(key),
            Tab::Suggestions => self.handle_suggestions_key(key),
        }

        let after = (
            self.todo.next_id,
            self.diary.next_id,
            self.daily.next_id,
        );
        if before != after {
            self.save_counters_now();
        }
    }

    fn handle_todo_key(&mut self, key: KeyEvent) {
        match self.todo.input_mode {
            TodoInputMode::Normal => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
                KeyCode::Char('n') => self.todo.start_new(),
                KeyCode::Char('e') => self.todo.start_edit(),
                KeyCode::Char('d') => self.todo.start_delete(),
                KeyCode::Char('h') | KeyCode::Left => self.todo.select_none(),
                KeyCode::Char('j') | KeyCode::Down => self.todo.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.todo.select_previous(),
                KeyCode::Char('g') | KeyCode::Home => self.todo.select_first(),
                KeyCode::Char('G') | KeyCode::End => self.todo.select_last(),
                KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                    if let Some(id) = self.todo.selected_id() {
                        match actions::dispatch(
                            Action::ToggleTodo { id, source: Source::Human },
                            self,
                        ) {
                            Ok(()) => self.todo.last_error = None,
                            Err(e) => self.todo.last_error = Some(e.to_string()),
                        }
                    }
                }
                _ => {}
            },
            TodoInputMode::EditingTitle => match key.code {
                KeyCode::Enter => self.todo.advance_to_info(),
                KeyCode::Esc => self.todo.cancel_form(),
                KeyCode::Char(c) => self.todo.title_buffer.push(c),
                KeyCode::Backspace => { self.todo.title_buffer.pop(); }
                _ => {}
            },
            TodoInputMode::EditingInfo => match key.code {
                KeyCode::Enter => {
                    let title = self.todo.title_buffer.trim().to_string();
                    let info = self.todo.info_buffer.trim().to_string();
                    let editing_id = self.todo.editing_id.clone();
                    self.todo.reset_form();
                    if !title.is_empty() {
                        let action = match editing_id {
                            Some(id) => Action::UpdateTodo {
                                id, text: title, info, source: Source::Human,
                            },
                            None => Action::AddTodo {
                                text: title, info, source: Source::Human,
                            },
                        };
                        match actions::dispatch(action, self) {
                            Ok(()) => self.todo.last_error = None,
                            Err(e) => self.todo.last_error = Some(e.to_string()),
                        }
                    }
                }
                KeyCode::Esc => self.todo.cancel_form(),
                KeyCode::Char(c) => self.todo.info_buffer.push(c),
                KeyCode::Backspace => { self.todo.info_buffer.pop(); }
                _ => {}
            },
            TodoInputMode::ConfirmDelete => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let selected = self.todo.selected_id();
                    self.todo.input_mode = TodoInputMode::Normal;
                    if let Some(id) = selected {
                        match actions::dispatch(
                            Action::DeleteTodo { id, source: Source::Human },
                            self,
                        ) {
                            Ok(()) => {
                                mod_todo::fix_selection_after_delete(&mut self.todo);
                                self.todo.last_error = None;
                            }
                            Err(e) => self.todo.last_error = Some(e.to_string()),
                        }
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.todo.cancel_delete();
                }
                _ => {}
            },
        }
    }

    fn handle_diary_key(&mut self, key: KeyEvent) {
        match self.diary.input_mode {
            DiaryMode::Normal => match key.code {
                KeyCode::Char('e') => {
                    self.diary.input_mode = DiaryMode::Editing;
                    log_event(json!({"type":"mode-switch","from":"Normal","to":"Editing"}));
                }
                KeyCode::Char('q') => self.should_exit = true,
                _ => {}
            },
            DiaryMode::Editing => match key.code {
                KeyCode::Enter => {
                    if !self.diary.input.is_empty() {
                        let body = std::mem::take(&mut self.diary.input);
                        self.diary.reset_cursor();
                        match actions::dispatch(
                            Action::AddDiaryEntry { body, source: Source::Human },
                            self,
                        ) {
                            Ok(()) => {
                                self.diary.last_error = None;
                                // Scan the just-added entry.
                                if let Some(msg) = self.diary.messages.last().cloned() {
                                    let suggs = RuleEngine.scan_entry(&msg);
                                    self.suggestions.push_all(suggs);
                                }
                            }
                            Err(e) => self.diary.last_error = Some(e.to_string()),
                        }
                        log_event(json!({"type": "input-reset"}));
                    }
                }
                KeyCode::Char(c) => self.diary.enter_char(c),
                KeyCode::Backspace => self.diary.delete_char(),
                KeyCode::Left => self.diary.move_cursor_left(),
                KeyCode::Right => self.diary.move_cursor_right(),
                KeyCode::Esc => {
                    self.diary.input_mode = DiaryMode::Normal;
                    log_event(json!({"type":"mode-switch","from":"Editing","to":"Normal"}));
                }
                _ => {}
            },
        }
    }

    fn handle_daily_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
            KeyCode::Char('h') | KeyCode::Left => self.daily.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.daily.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.daily.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.daily.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.daily.select_last(),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                if let Some(id) = self.daily.selected_id() {
                    let _ = actions::dispatch(
                        Action::ToggleDaily { id, source: Source::Human },
                        self,
                    );
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self.daily.selected_id() {
                    let _ = actions::dispatch(
                        Action::DeleteDaily { id, source: Source::Human },
                        self,
                    );
                }
            }
            KeyCode::Char('t') => {
                self.tick_dailies();
            }
            _ => {}
        }
    }

    fn handle_suggestions_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
            KeyCode::Char('h') | KeyCode::Left => self.suggestions.state.select(None),
            KeyCode::Char('j') | KeyCode::Down => self.suggestions.state.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.suggestions.state.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.suggestions.state.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.suggestions.state.select_last(),
            KeyCode::Enter | KeyCode::Char('a') => {
                if let Some(i) = self.suggestions.state.selected() {
                    self.accept_suggestion(i);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                if let Some(i) = self.suggestions.state.selected() {
                    self.dismiss_suggestion(i);
                }
            }
            KeyCode::Char('A') => {
                let mut safety = 1000;
                while !self.suggestions.items.is_empty() && safety > 0 {
                    if !self.accept_suggestion(0) { break; }
                    safety -= 1;
                }
            }
            _ => {}
        }
    }

    fn accept_suggestion(&mut self, idx: usize) -> bool {
        if idx >= self.suggestions.items.len() { return false; }
        let sugg = self.suggestions.items[idx].clone();
        let sid = sugg.id.clone();
        match actions::dispatch(sugg.action, self) {
            Ok(()) => {
                self.suggestions.items.remove(idx);
                self.suggestions.resolved.insert(sid.0.clone());
                log_event(json!({"type":"suggestion-accepted","suggestion_id":sid.0}));
                if self.suggestions.items.is_empty() {
                    self.suggestions.state.select(None);
                } else if idx >= self.suggestions.items.len() {
                    self.suggestions
                        .state
                        .select(Some(self.suggestions.items.len() - 1));
                }
                self.save_counters_now();
                true
            }
            Err(e) => {
                log_event(json!({
                    "type":"suggestion-dispatch-failed",
                    "suggestion_id":sid.0,
                    "error":e.to_string(),
                }));
                false
            }
        }
    }

    fn dismiss_suggestion(&mut self, idx: usize) {
        if idx >= self.suggestions.items.len() { return; }
        let s = self.suggestions.items.remove(idx);
        self.suggestions.resolved.insert(s.id.0.clone());
        log_event(json!({"type":"suggestion-dismissed","suggestion_id":s.id.0}));
        if self.suggestions.items.is_empty() {
            self.suggestions.state.select(None);
        } else if idx >= self.suggestions.items.len() {
            self.suggestions
                .state
                .select(Some(self.suggestions.items.len() - 1));
        }
    }

    // ── Render ─────────────────────────────────────────────────

    fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let chunks = layout.split(frame.area());
        let (tabs_area, content_area) = (chunks[0], chunks[1]);

        self.render_tab_bar(tabs_area, frame.buffer_mut());

        match self.active_tab {
            Tab::Todo => self.render_todo_tab(content_area, frame),
            Tab::Diary => mod_diary::render_diary_tab(&self.diary, content_area, frame),
            Tab::Daily => mod_daily::render_tab(&mut self.daily, content_area, frame.buffer_mut()),
            Tab::Suggestions => self.render_suggestions_tab(content_area, frame),
        }
    }

    fn render_tab_bar(&self, area: Rect, buf: &mut Buffer) {
        let active = self.active_tab;
        let style = |t: Tab| if active == t { TAB_ACTIVE_STYLE } else { TAB_INACTIVE_STYLE };

        let sugg_label = if self.suggestions.items.is_empty() {
            " 4: Suggestions ".to_string()
        } else {
            format!(" 4: Suggestions ({}) ", self.suggestions.items.len())
        };

        let line = Line::from(vec![
            Span::styled(" 1: Todo ", style(Tab::Todo)),
            Span::raw(" "),
            Span::styled(" 2: Diary ", style(Tab::Diary)),
            Span::raw(" "),
            Span::styled(" 3: Daily ", style(Tab::Daily)),
            Span::raw(" "),
            Span::styled(sugg_label, style(Tab::Suggestions)),
            Span::raw("   "),
            Span::raw("(Tab / 1-4)")
                .fg(ratatui::style::palette::tailwind::SLATE.c500),
        ]);
        Paragraph::new(line).render(area, buf);
    }

    fn render_todo_tab(&mut self, area: Rect, frame: &mut Frame) {
        let main_layout = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ]);
        let chunks = main_layout.split(area);
        let (header_area, content_area, footer_area) = (chunks[0], chunks[1], chunks[2]);

        let content_layout = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]);
        let chunks = content_layout.split(content_area);
        let (list_area, item_area) = (chunks[0], chunks[1]);

        Paragraph::new("Ratatui Todo List Example")
            .bold()
            .centered()
            .render(header_area, frame.buffer_mut());

        let footer_line = if matches!(self.todo.input_mode, TodoInputMode::ConfirmDelete) {
            Line::from(Span::styled(
                "Delete this todo? (y/n)",
                ratatui::style::Style::default()
                    .fg(Color::Red)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ))
        } else if let Some(err) = &self.todo.last_error {
            Line::from(vec![
                Span::styled(
                    format!("⚠ {err}"),
                    ratatui::style::Style::default().fg(Color::Red),
                ),
                Span::raw("   "),
                Span::raw("↓↑ move, ← unselect, → toggle, n new, e edit, d delete."),
            ])
        } else {
            Line::from("↓↑ move, ← unselect, → toggle, n new, e edit, d delete.")
        };
        Paragraph::new(footer_line).centered().render(footer_area, frame.buffer_mut());

        self.todo.render_list(list_area, frame.buffer_mut());
        self.todo.render_selected_item(item_area, frame.buffer_mut());

        if let Some((x, y)) = self.todo.cursor_position(item_area) {
            frame.set_cursor_position(Position::new(x, y));
        }
    }

    fn render_suggestions_tab(&mut self, area: Rect, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]);
        let chunks = layout.split(area);
        let (header, list_area, footer) = (chunks[0], chunks[1], chunks[2]);

        Paragraph::new(format!(
            "Suggestions — {} pending",
            self.suggestions.items.len()
        ))
        .bold()
        .centered()
        .render(header, frame.buffer_mut());

        if self.suggestions.items.is_empty() {
            Paragraph::new(
                "No pending suggestions.\n\
                 Write a diary entry like \"I need to clear my room.\" \
                 or \"I want to learn guitar.\"",
            )
            .centered()
            .render(list_area, frame.buffer_mut());
        } else {
            let items: Vec<ListItem> = self
                .suggestions
                .items
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let action_str = match &s.action {
                        Action::AddTodo { text, .. } => format!("+ todo: {text}"),
                        Action::AddDaily { text, duration_min, .. } => match duration_min {
                            Some(n) => format!("+ daily: {text} ({n} min)"),
                            None => format!("+ daily: {text}"),
                        },
                        _ => "(other)".to_string(),
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(format!(" {i:>2}. ")),
                        Span::styled(action_str, ratatui::style::Style::default().fg(Color::Cyan)),
                        Span::raw("   "),
                        Span::styled(
                            format!("{} {}", s.reason, (s.confidence * 100.0) as u8),
                            ratatui::style::Style::default().fg(Color::DarkGray),
                        ),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(Block::bordered().title("Pending"))
                .highlight_style(SELECTED_STYLE)
                .highlight_symbol(">")
                .highlight_spacing(HighlightSpacing::Always);

            StatefulWidget::render(
                list,
                list_area,
                frame.buffer_mut(),
                &mut self.suggestions.state,
            );
        }

        Paragraph::new("j/k nav  Enter/a accept  d/x dismiss  A accept all")
            .centered()
            .render(footer, frame.buffer_mut());
    }
}
