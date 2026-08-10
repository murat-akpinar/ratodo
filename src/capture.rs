//! Free text -> `Task`, for `ratodo add`.
//!
//! Input is flexible, storage is strict: `@tomorrow` is accepted here and
//! resolved to an ISO date before anything reaches the file. docs/format.md.

use chrono::{Datelike, Days, NaiveDate, NaiveTime, Weekday};

use crate::model::{Due, Priority, Task};

pub fn capture(text: &str, today: NaiveDate) -> Task {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut title: Vec<&str> = Vec::new();
    let mut due: Option<Due> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut priority: Option<Priority> = None;
    let mut i = 0;

    while i < words.len() {
        let word = words[i];

        if let Some(rest) = word.strip_prefix('@')
            && due.is_none()
            && let Some(date) = resolve_date(rest, today)
        {
            let time = words.get(i + 1).and_then(|w| parse_time(w));
            due = Some(Due { date, time });
            i += if time.is_some() { 2 } else { 1 };
            continue;
        }

        if let Some(tag) = word.strip_prefix('#')
            && !tag.is_empty()
        {
            tags.push(tag.to_string());
            i += 1;
            continue;
        }

        if let Some(p) = parse_priority(word) {
            priority = Some(p);
            i += 1;
            continue;
        }

        title.push(word);
        i += 1;
    }

    Task::new(false, title.join(" "), due, tags, priority)
}

/// `2026-08-12`, `today`, `tomorrow`, `mon`..`sun`, `3d`, `2w`.
fn resolve_date(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }

    let s = s.to_ascii_lowercase();
    match s.as_str() {
        "today" => return Some(today),
        "tomorrow" => return today.checked_add_days(Days::new(1)),
        _ => {}
    }

    if let Some(w) = weekday(&s) {
        return next_weekday(today, w);
    }

    let mut chars = s.chars();
    let unit = chars.next_back()?;
    let n: u64 = chars.as_str().parse().ok()?;
    match unit {
        'd' => today.checked_add_days(Days::new(n)),
        'w' => today.checked_add_days(Days::new(n * 7)),
        _ => None,
    }
}

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "mon" => Weekday::Mon,
        "tue" => Weekday::Tue,
        "wed" => Weekday::Wed,
        "thu" => Weekday::Thu,
        "fri" => Weekday::Fri,
        "sat" => Weekday::Sat,
        "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// Strictly in the future: `@mon` typed on a Monday means the next one.
fn next_weekday(today: NaiveDate, target: Weekday) -> Option<NaiveDate> {
    let ahead = (target.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
    let ahead = if ahead == 0 { 7 } else { ahead };
    today.checked_add_days(Days::new(ahead as u64))
}

fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}

fn parse_priority(word: &str) -> Option<Priority> {
    let rest = word.strip_prefix('!')?;
    match rest.to_ascii_lowercase().as_str() {
        "high" => Some(Priority::High),
        "med" => Some(Priority::Med),
        "low" => Some(Priority::Low),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Monday, matching the reference date the docs use throughout.
    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()
    }

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn the_examples_from_the_docs() {
        assert_eq!(
            capture("pay the invoice @tomorrow", today()).raw,
            "- [ ] pay the invoice @2026-08-11"
        );
        assert_eq!(
            capture("report @mon !high", today()).raw,
            "- [ ] report @2026-08-17 !high"
        );
        assert_eq!(
            capture("run a backup @3d", today()).raw,
            "- [ ] run a backup @2026-08-13"
        );
    }

    #[test]
    fn shorthand_never_reaches_the_file() {
        for text in ["a @today", "a @tomorrow", "a @sun", "a @2w", "a @1d"] {
            let raw = capture(text, today()).raw;
            assert!(!raw.contains("@to"), "{raw}");
            assert!(raw.contains("@2026-"), "{raw}");
        }
    }

    #[test]
    fn weekday_is_always_in_the_future() {
        assert_eq!(resolve_date("mon", today()), Some(ymd(2026, 8, 17)));
        assert_eq!(resolve_date("tue", today()), Some(ymd(2026, 8, 11)));
        assert_eq!(resolve_date("sun", today()), Some(ymd(2026, 8, 16)));
    }

    #[test]
    fn offsets() {
        assert_eq!(resolve_date("today", today()), Some(ymd(2026, 8, 10)));
        assert_eq!(resolve_date("0d", today()), Some(ymd(2026, 8, 10)));
        assert_eq!(resolve_date("2w", today()), Some(ymd(2026, 8, 24)));
        assert_eq!(resolve_date("30d", today()), Some(ymd(2026, 9, 9)));
    }

    #[test]
    fn nonsense_dates_stay_in_the_title() {
        let t = capture("a @whenever b @2026-13-45", today());
        assert!(t.due.is_none());
        assert_eq!(t.title, "a @whenever b @2026-13-45");
    }

    #[test]
    fn full_line() {
        let t = capture("call the accountant @thu 09:30 #work #money !high", today());
        assert_eq!(
            t.raw,
            "- [ ] call the accountant @2026-08-13 09:30 #work #money !high"
        );
    }

    #[test]
    fn an_iso_date_passes_straight_through() {
        assert_eq!(capture("a @2026-12-01", today()).raw, "- [ ] a @2026-12-01");
    }

    #[test]
    fn a_capture_round_trips_through_the_parser() {
        let t = capture("fatura öde @tomorrow #ev !med", today());
        let parsed = crate::parse::parse(&t.raw);
        let back = parsed.tasks().next().unwrap();
        assert_eq!(back.title, "fatura öde");
        assert_eq!(back.due, t.due);
        assert_eq!(back.tags, t.tags);
        assert_eq!(back.priority, t.priority);
    }
}
