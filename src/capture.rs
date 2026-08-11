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

/// The first `@word` that was meant as a date and is not one.
///
/// `parts` hands such a word back as plain text, and that is the right thing to
/// do with it — a word we did not understand belongs to the user and does not
/// get eaten. What it is not is *visible*: `@2026-13-45` looks accepted right up
/// until the file has it in the title. This is the one question the preview
/// answers with an opinion rather than a readout — docs/tui.md#adding.
///
/// A bare `@` is a word somebody typed, not a failed date, and stays silent. So
/// is a date halfway typed: the preview runs on every keystroke, and one that
/// says "not a date" through all of `@2`, `@20`, `@202` is one people stop
/// reading by the time it is right.
pub fn unresolved_date(text: &str, today: NaiveDate) -> Option<&str> {
    words(text).into_iter().map(|(_, word)| word).find(|word| {
        word.strip_prefix('@').is_some_and(|rest| {
            !rest.is_empty()
                && resolve_date(rest, today).is_none()
                && !could_still_become_a_date(rest)
        })
    })
}

/// Whether more typing could still make a date of this.
///
/// The question the live preview has to ask before it opens its mouth. `@2026-0`
/// is on its way somewhere and `@2026-13` is not, and the difference is what
/// separates a warning from nagging.
fn could_still_become_a_date(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    [
        "today", "tomorrow", "mon", "tue", "wed", "thu", "fri", "sat", "sun",
    ]
    .iter()
    .any(|word| word.starts_with(&s))
        || partial_iso(&s)
}

/// `YYYY-MM-DD` with the tail not typed yet — which also covers `3d` and `2w`
/// on their way past the digits.
///
/// Field by field and not by position: `@2026-8-4` is a date, since the parser
/// does not insist on the padding, and a check counting characters would have
/// nagged at every step of typing one.
fn partial_iso(s: &str) -> bool {
    let mut fields = s.split('-');
    let year = fields.next().unwrap_or_default();
    let month = fields.next().unwrap_or_default();
    let day = fields.next().unwrap_or_default();

    if fields.next().is_some()
        || !(1..=4).contains(&year.len())
        || !year.bytes().all(|c| c.is_ascii_digit())
    {
        return false;
    }

    // All three fields filled to the brim is not a prefix of anything. It had
    // its chance to parse upstream of here and did not take it, so it is a date
    // that does not exist rather than one halfway typed.
    if (year.len(), month.len(), day.len()) == (4, 2, 2) {
        return false;
    }

    reachable(month, 12) && reachable(day, 31)
}

/// A month or a day, as far as it has been typed.
///
/// Two digits are the whole number and have to be one that exists. A single one
/// is always still going somewhere — it is either the number itself or the tens
/// of a bigger one — and an empty field has not been reached yet.
fn reachable(digits: &str, max: u32) -> bool {
    match digits.as_bytes() {
        [] => true,
        [c] => c.is_ascii_digit(),
        [_, _] => digits.parse::<u32>().is_ok_and(|n| (1..=max).contains(&n)),
        _ => false,
    }
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

    /// The word the preview is allowed to have an opinion about: an `@` that was
    /// meant as a date, is not one, and cannot become one either.
    #[test]
    fn a_word_that_can_never_be_a_date_is_named() {
        for text in [
            "call the plumber @2026-13-45",
            "call the plumber @2026-13",
            "call the plumber @2026-08-45",
            "call the plumber @2026-02-30",
            "call the plumber @2026-00",
            "call the plumber @2026-08-00",
            "call the plumber @12345-08-20",
            "call the plumber @3x",
            "call the plumber @nextweek",
        ] {
            let word = text.rsplit(' ').next().unwrap();
            assert_eq!(unresolved_date(text, today()), Some(word), "{text}");
        }
    }

    /// The preview runs on every keystroke, so every prefix on the way to a real
    /// date has to stay quiet — otherwise the warning is wrong for ten presses
    /// and right for one, and nobody reads it by then.
    #[test]
    fn a_date_halfway_typed_is_left_alone() {
        let mut typed = String::from("call the plumber ");
        for c in "@2026-08-20".chars() {
            typed.push(c);
            assert_eq!(unresolved_date(&typed, today()), None, "{typed}");
        }
        // And the unpadded form, which the parser takes and which a check
        // counting characters instead of fields would have nagged through.
        let mut typed = String::from("call the plumber ");
        for c in "@2026-8-4".chars() {
            typed.push(c);
            assert_eq!(unresolved_date(&typed, today()), None, "{typed}");
        }
        // A single digit is never finished: it is either the number itself or
        // the tens of a bigger one.
        for word in [
            "@t",
            "@to",
            "@tom",
            "@th",
            "@m",
            "@s",
            "@1",
            "@12",
            "@3",
            "@2026-1",
            "@2026-2",
            "@2026-9",
            "@2026-08-2",
            "@2026-08-3",
        ] {
            let text = format!("call the plumber {word}");
            assert_eq!(unresolved_date(&text, today()), None, "{text}");
        }
    }

    /// The other side of it: what resolves, and what was never an `@` date at
    /// all, both stay silent.
    #[test]
    fn what_is_not_a_failed_date_stays_silent() {
        for text in [
            "email a@b about it",
            "read the @ sign chapter",
            "pay the invoice @thu",
            "pay the invoice @2026-08-20",
            "pay the invoice @3d",
            "just write it down",
        ] {
            assert_eq!(unresolved_date(text, today()), None, "{text}");
        }
    }
}
