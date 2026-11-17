use ratatui::style::palette::tailwind::{BLUE, GREEN, SLATE};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Padding};

pub const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
pub const NORMAL_ROW_BG: Color = SLATE.c950;
pub const ALT_ROW_BG_COLOR: Color = SLATE.c900;
pub const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
pub const TEXT_FG_COLOR: Color = SLATE.c200;
pub const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;

pub const TAB_ACTIVE_STYLE: Style = Style::new()
    .fg(SLATE.c100)
    .bg(BLUE.c800)
    .add_modifier(Modifier::BOLD);
pub const TAB_INACTIVE_STYLE: Style = Style::new().fg(SLATE.c400).bg(SLATE.c950);

pub fn info_block(title: &'static str) -> Block<'static> {
    Block::new()
        .title(Line::raw(title).centered())
        .borders(Borders::TOP)
        .border_set(symbols::border::EMPTY)
        .border_style(TODO_HEADER_STYLE)
        .bg(NORMAL_ROW_BG)
        .padding(Padding::horizontal(1))
}

pub fn display_width(s: &str) -> u16 {
    u16::try_from(Line::raw(s).width()).unwrap_or(u16::MAX)
}

pub const fn alternate_colors(i: usize) -> Color {
    if i.is_multiple_of(2) { NORMAL_ROW_BG } else { ALT_ROW_BG_COLOR }
}
