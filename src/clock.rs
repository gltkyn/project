//! Dates and times. Hides the clock library (chrono), the date format, and how a span that
//! crosses midnight is cut into days. Knows nothing about todos, habits or the terminal.

use chrono::{Local, NaiveDate, NaiveTime, Timelike};

pub(crate) const DATE_FMT: &str = "%Y-%m-%d";

pub(crate) fn today() -> NaiveDate {
    Local::now().date_naive()
}
pub(crate) fn today_text() -> String {
    today().format(DATE_FMT).to_string()
}
pub(crate) fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), DATE_FMT).ok()
}
pub(crate) fn minutes_of(text: &str) -> Option<u32> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M")
        .ok()
        .map(|t| t.hour() * 60 + t.minute())
}
pub(crate) fn clock(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
}
pub(crate) fn day_heading(date: NaiveDate) -> String {
    let t = today();
    let relative = if date == t {
        " · today"
    } else if t.succ_opt() == Some(date) {
        " · tomorrow"
    } else if t.pred_opt() == Some(date) {
        " · yesterday"
    } else {
        ""
    };
    format!("{}{}", date.format("%a %d %b"), relative)
}
pub(crate) fn span_on(from: NaiveDate, start: u32, len: u32, on: NaiveDate) -> Option<(u32, u32)> {
    let a = (on - from).num_days() * 1440;
    if a < 0 {
        return None;
    }
    let lo = (start as i64).max(a);
    let hi = ((start + len) as i64).min(a + 1440);
    if lo >= hi {
        None
    } else {
        Some(((lo - a) as u32, (hi - a) as u32))
    }
}
pub(crate) fn parse_when(text: &str) -> Result<String, String> {
    let text = text.trim();
    let bad = || "Use HH:MM (today) or YYYY-MM-DD HH:MM.".to_string();
    let (date, time) = match text.split_once(' ') {
        Some((d, t)) => (parse_date(d).ok_or_else(bad)?, t),
        None => (today(), text),
    };
    let minutes = minutes_of(time).ok_or_else(bad)?;
    Ok(format!("{} {}", date.format(DATE_FMT), clock(minutes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> NaiveDate {
        parse_date(text).expect("test date")
    }

    #[test]
    fn spans_cross_midnight() {
        let from = d("2026-10-24");
        assert_eq!(span_on(from, 23 * 60, 420, from), Some((1380, 1440)));
        assert_eq!(span_on(from, 23 * 60, 420, d("2026-10-25")), Some((0, 360)));
        assert_eq!(span_on(from, 23 * 60, 420, d("2026-10-26")), None);
        assert_eq!(span_on(from, 23 * 60, 420, d("2026-10-23")), None);
    }
}
