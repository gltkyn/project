//! Daily module — recurring items with a cadence. Owns dailies only. When
//! a daily fires, App dispatches AddTodo through actions::dispatch; this
//! module never calls mod_todo directly.

use chrono::Local;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
};
use serde_json::json;

use crate::actions::{DispatchError, Source};
use crate::domain::{DailyItem, DailyStatus};
use crate::events::log_event;
use crate::storage::{load_dailies, save_dailies, DAILIES_FILE};
use crate::styles::SELECTED_STYLE;

pub struct DailyApp {
    pub next_id: u64,
    pub items: Vec<DailyItem>,
    pub state: ListState,
    pub last_error: Option<String>,
}

impl DailyApp {
    pub fn new(starting_next_id: u64) -> Self {
        let mut next_id = starting_next_id;
        let items = load_dailies(&mut next_id);
        Self { next_id, items, state: ListState::default(), last_error: None }
    }

    pub fn save(&mut self) {
        match save_dailies(&self.items) {
            Ok(()) => self.last_error = None,
            Err(e) => self.last_error = Some(format!("save failed: {e}")),
        }
    }

    pub fn selected_id(&self) -> Option<String> {
        self.state.selected().map(|i| self.items[i].id.clone())
    }

    pub fn select_next(&mut self) { self.state.select_next(); }
    pub fn select_previous(&mut self) { self.state.select_previous(); }
    pub fn select_first(&mut self) { self.state.select_first(); }
    pub fn select_last(&mut self) { self.state.select_last(); }
    pub fn select_none(&mut self) { self.state.select(None); }

    pub fn fix_selection(&mut self) {
        if self.items.is_empty() {
            self.state.select(None);
        } else if let Some(i) = self.state.selected()
            && i >= self.items.len()
        {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    /// Mark a daily as having fired today. Called by App after the
    /// corresponding AddTodo has been dispatched.
    pub fn mark_fired(&mut self, id: &str, today: &str) {
        if let Some(d) = self.items.iter_mut().find(|d| d.id == id) {
            d.last_fired_date = Some(today.to_string());
            log_event(json!({"type":"daily-fired","id":id,"date":today}));
        }
    }
}

// ── Handlers (called only from actions::dispatch) ───────────────

pub fn handle_add(
    daily: &mut DailyApp,
    text: String,
    duration_min: Option<u32>,
    source: Source,
) -> Result<(), DispatchError> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(DispatchError::Invalid("empty daily text".into()));
    }
    let id = format!("daily-{}", daily.next_id);
    daily.next_id += 1;
    let item = DailyItem {
        id: id.clone(),
        text: text.clone(),
        duration_min,
        status: DailyStatus::Active,
        source: source.clone(),
        created_at: Local::now().format("%Y-%m-%d").to_string(),
        last_fired_date: None,
    };
    log_event(json!({
        "type": "daily-create",
        "id": id,
        "text": text,
        "duration_min": duration_min,
        "source": source.label(),
    }));
    daily.items.push(item);
    if daily.state.selected().is_none() {
        daily.state.select_first();
    }
    daily.save();
    Ok(())
}

pub fn handle_toggle(
    daily: &mut DailyApp,
    id: &str,
    source: Source,
) -> Result<(), DispatchError> {
    let Some(item) = daily.items.iter_mut().find(|d| d.id == id) else {
        return Err(DispatchError::NotFound(id.into()));
    };
    let old = item.status;
    item.status = match old {
        DailyStatus::Active => DailyStatus::Paused,
        DailyStatus::Paused => DailyStatus::Active,
    };
    log_event(json!({
        "type": "daily-toggle",
        "id": id,
        "from": format!("{old:?}"),
        "to": format!("{:?}", item.status),
        "source": source.label(),
    }));
    daily.save();
    Ok(())
}

pub fn handle_delete(
    daily: &mut DailyApp,
    id: &str,
    source: Source,
) -> Result<(), DispatchError> {
    let Some(i) = daily.items.iter().position(|d| d.id == id) else {
        return Err(DispatchError::NotFound(id.into()));
    };
    daily.items.remove(i);
    daily.fix_selection();
    log_event(json!({"type":"daily-delete","id":id,"source":source.label()}));
    daily.save();
    Ok(())
}

// ── Rendering ───────────────────────────────────────────────────

pub fn render_tab(daily: &mut DailyApp, area: Rect, buf: &mut Buffer) {
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ]);
    let chunks = layout.split(area);
    let (header, list_area, footer) = (chunks[0], chunks[1], chunks[2]);

    let active = daily.items.iter().filter(|d| d.status == DailyStatus::Active).count();
    Paragraph::new(format!(
        "Daily — {} active / {} total ({})",
        active,
        daily.items.len(),
        DAILIES_FILE
    ))
    .bold()
    .centered()
    .render(header, buf);

    let today = Local::now().format("%Y-%m-%d").to_string();
    if daily.items.is_empty() {
        Paragraph::new(
            "No dailies yet.\n\
             Accept a suggestion like \"I want to learn guitar\" to create one.",
        )
        .centered()
        .render(list_area, buf);
    } else {
        let items: Vec<ListItem> = daily
            .items
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let fired = d.last_fired_date.as_deref() == Some(&today);
                let icon = match d.status {
                    DailyStatus::Active => if fired { "✓" } else { "○" },
                    DailyStatus::Paused => "⏸",
                };
                let dur = d.duration_min.map(|n| format!(" ({n} min)")).unwrap_or_default();
                let src = d.source.label();
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{icon} {i:>2} [{}] {}{}", d.id, d.text, dur)),
                    Span::raw("  "),
                    Span::styled(format!("[{src}]"), Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::bordered().title("Dailies"))
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, list_area, buf, &mut daily.state);
    }

    let footer_text = if let Some(err) = &daily.last_error {
        format!("⚠ {err}   j/k nav, Enter pause/resume, d delete, t tick now")
    } else {
        "j/k nav, Enter pause/resume, d delete, t tick now".to_string()
    };
    Paragraph::new(footer_text).centered().render(footer, buf);
}
