//! Free text -> `Task`, for `ratodo add`.
//!
//! Input is flexible, storage is strict: `@tomorrow` is accepted here and
//! resolved to an ISO date before anything reaches the file. docs/format.md.

use chrono::{Datelike, Days, NaiveDate, NaiveTime, Weekday};

use crate::model::{Due, Priority, State, Task};

/// What `capture` made of one word.
///
/// The input field colours by this rather than by the leading character, so a
/// `@notaday` stays plain on screen exactly as it will in the file — a colour
/// that promises more than the parser delivers teaches the wrong syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Text,
    Date,
    /// The `HH:MM` after a date, which the date has already taken.
    Time,
    Tag,
    Priority,
}

/// Every word of `text` as a byte range, paired with what it means.
///
/// The one tokenizer: `capture` builds the task out of this and the input field
/// colours it, so the screen cannot drift from the parse. See docs/tui.md#adding.
pub fn parts(text: &str, today: NaiveDate) -> Vec<(std::ops::Range<usize>, Part)> {
    let words = words(text);
    let mut out = Vec::with_capacity(words.len());
    let mut dated = false;
    let mut i = 0;

    while i < words.len() {
        let (at, word) = words[i];
        let range = at..at + word.len();

        if let Some(rest) = word.strip_prefix('@')
            && !dated
            && resolve_date(rest, today).is_some()
        {
            dated = true;
            out.push((range, Part::Date));
            i += 1;
            // A time only counts as one directly after a date. On its own it is
            // words in a title, and `09:30 standup` is a title somebody typed.
            if let Some(&(at, next)) = words.get(i)
                && parse_time(next).is_some()
            {
                out.push((at..at + next.len(), Part::Time));
                i += 1;
            }
            continue;
        }

        let part = match word {
            w if w.strip_prefix('#').is_some_and(|tag| !tag.is_empty()) => Part::Tag,
            w if parse_priority(w).is_some() => Part::Priority,
            _ => Part::Text,
        };
        out.push((range, part));
        i += 1;
    }
    out
}

/// `split_whitespace` with the offsets kept, which is the whole difference: the
/// field has to know *where* a word is to colour it.
pub(crate) fn words(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (c.is_whitespace(), start) {
            (true, Some(s)) => {
                out.push((s, &text[s..i]));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, &text[s..]));
    }
    out
}

pub fn capture(text: &str, today: NaiveDate) -> Task {
    let mut title: Vec<&str> = Vec::new();
    let mut due: Option<Due> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut priority: Option<Priority> = None;

    for (range, part) in parts(text, today) {
        let word = &text[range];
        match part {
            Part::Text => title.push(word),
            Part::Date => {
                due = resolve_date(&word[1..], today).map(|date| Due { date, time: None })
            }
            Part::Time => {
                if let Some(due) = due.as_mut() {
                    due.time = parse_time(word);
                }
            }
            Part::Tag => tags.push(word[1..].to_string()),
            Part::Priority => priority = parse_priority(word),
        }
    }

    Task::new(State::Open, title.join(" "), due, tags, priority)
}

/// What `p` takes: everything `@` takes, plus a bare number meaning days.
///
/// The bare number is only accepted here. `@2` in a sentence is somebody typing
/// about the number two, but a box that has just asked *how long* has no other
/// reading of it — and "1 day, 2 days" is how the question gets answered.
pub fn later(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let text = text.trim();
    match text.parse::<u64>() {
        Ok(days) => today.checked_add_days(Days::new(days)),
        Err(_) => resolve_date(text, today),
    }
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
        'w' => today.checked_add_days(Days::new(n.checked_mul(7)?)),
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

    /// What `p` accepts. The bare number is the whole reason this is not just
    /// `resolve_date`: the box asked how long, so `2` has one reading.
    #[test]
    fn how_long_takes_a_bare_number_of_days_as_well() {
        assert_eq!(later("2", today()), Some(ymd(2026, 8, 12)));
        assert_eq!(
            later("0", today()),
            Some(today()),
            "today, and not an error"
        );
        assert_eq!(later("  3  ", today()), Some(ymd(2026, 8, 13)));

        // And everything `@` already took.
        assert_eq!(later("3d", today()), Some(ymd(2026, 8, 13)));
        assert_eq!(later("1w", today()), Some(ymd(2026, 8, 17)));
        assert_eq!(later("fri", today()), Some(ymd(2026, 8, 14)));
        assert_eq!(later("tomorrow", today()), Some(ymd(2026, 8, 11)));
        assert_eq!(later("2026-09-01", today()), Some(ymd(2026, 9, 1)));

        for nonsense in ["", "   ", "3x", "-1", "soon", "2026-13-45"] {
            assert_eq!(later(nonsense, today()), None, "{nonsense:?}");
        }
    }

    /// The bare number is `later`'s alone. `@2` in a sentence is somebody typing
    /// about the number two, and giving it a date would rewrite their title.
    #[test]
    fn a_bare_number_is_not_a_date_in_a_sentence() {
        let task = capture("chapter @2 of the book", today());
        assert_eq!(task.due, None);
        assert_eq!(task.title, "chapter @2 of the book");
    }

    #[test]
    fn weekday_is_always_in_the_future() {
        for (name, expected) in [
            ("mon", ymd(2026, 8, 17)),
            ("tue", ymd(2026, 8, 11)),
            ("wed", ymd(2026, 8, 12)),
            ("thu", ymd(2026, 8, 13)),
            ("fri", ymd(2026, 8, 14)),
            ("sat", ymd(2026, 8, 15)),
            ("sun", ymd(2026, 8, 16)),
        ] {
            assert_eq!(resolve_date(name, today()), Some(expected), "@{name}");
        }
    }

    /// Every other test here starts from a Monday, where `today`'s weekday
    /// index is zero and the offset arithmetic cannot be wrong in the usual
    /// way. Starting mid-week is what actually exercises it.
    #[test]
    fn weekdays_from_the_middle_of_the_week() {
        let wednesday = ymd(2026, 8, 12);
        for (name, expected) in [
            ("thu", ymd(2026, 8, 13)),
            ("fri", ymd(2026, 8, 14)),
            ("sun", ymd(2026, 8, 16)),
            ("mon", ymd(2026, 8, 17)),
            ("tue", ymd(2026, 8, 18)),
            ("wed", ymd(2026, 8, 19)),
        ] {
            assert_eq!(resolve_date(name, wednesday), Some(expected), "@{name}");
        }

        let sunday = ymd(2026, 8, 16);
        assert_eq!(resolve_date("mon", sunday), Some(ymd(2026, 8, 17)));
        assert_eq!(resolve_date("sat", sunday), Some(ymd(2026, 8, 22)));
        assert_eq!(resolve_date("sun", sunday), Some(ymd(2026, 8, 23)));
    }

    #[test]
    fn every_priority_is_recognised() {
        for (word, expected) in [
            ("!high", Priority::High),
            ("!med", Priority::Med),
            ("!low", Priority::Low),
        ] {
            assert_eq!(parse_priority(word), Some(expected), "{word}");
        }
        assert_eq!(parse_priority("!urgent"), None);
        assert_eq!(parse_priority("high"), None);
    }

    #[test]
    fn offsets() {
        assert_eq!(resolve_date("today", today()), Some(ymd(2026, 8, 10)));
        assert_eq!(resolve_date("0d", today()), Some(ymd(2026, 8, 10)));
        assert_eq!(resolve_date("2w", today()), Some(ymd(2026, 8, 24)));
        assert_eq!(resolve_date("30d", today()), Some(ymd(2026, 9, 9)));
    }

    /// Each of these takes a different route out: the count does not fit a u64,
    /// the weeks-to-days multiply overflows, and the date lands outside the
    /// calendar. None of them may panic.
    #[test]
    fn absurd_offsets_do_not_panic() {
        for s in [
            "99999999999999999999d",
            "2635249153387078802w",
            "999999999d",
        ] {
            assert_eq!(resolve_date(s, today()), None, "{s}");
        }
    }

    #[test]
    fn a_large_but_real_offset_still_works() {
        assert_eq!(
            resolve_date("9999999d", today()),
            NaiveDate::from_ymd_opt(29405, 9, 4)
        );
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
