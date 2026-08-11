//! Human-facing strings for the command line. See docs/cli.md.

use chrono::NaiveDate;

use crate::agenda::Counts;
use crate::capture::Part;
use crate::model::{Due, State, Task};

/// A todo.md can arrive over `git pull`. Control characters in it would be
/// acted on by the terminal rather than shown.
pub fn plain(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

/// One task, as `ratodo list` prints it.
pub fn list_line(task: &Task, today: NaiveDate) -> String {
    let mark = match task.state {
        State::Done => "[x]",
        State::Cancelled => "[-]",
        State::Open if task.is_overdue(today) => "[!]",
        State::Open => "[ ]",
    };

    let mut line = format!("  {mark} {}", plain(&task.title));
    if let Some(due) = task.due {
        line.push_str("  ");
        line.push_str(due.to_file_string().trim_start_matches('@'));
    }
    for tag in &task.tags {
        line.push_str("  #");
        line.push_str(&plain(tag));
    }
    if let Some(p) = task.priority {
        line.push_str("  ");
        line.push_str(p.as_str());
    }
    line
}

/// One task for a script: five tab-separated fields, always all five, so
/// `cut -f5` means the same thing on every line. See docs/cli.md#list---porcelain.
///
/// `plain` is doing real work here — a tab inside a title would otherwise invent
/// a sixth field and shift everything after it.
pub fn porcelain_line(task: &Task) -> String {
    // Field one is what a script branches on, so a third state has to be its own
    // word rather than folded into `done` — docs/cli.md#list---porcelain.
    let state = match task.state {
        State::Open => "open",
        State::Done => "done",
        State::Cancelled => "cancelled",
    };
    let date = task
        .due
        .map(|d| d.date.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let tags: Vec<String> = task.tags.iter().map(|t| plain(t)).collect();
    let prio = task.priority.map_or("", |p| p.name());
    format!(
        "{state}\t{date}\t{}\t{}\t{prio}",
        plain(&task.title),
        tags.join(",")
    )
}

/// The summary under `list`, and the whole of `status`.
pub fn status_line(counts: Counts) -> String {
    format!("{} open · {} overdue", counts.open, counts.overdue)
}

/// One line for waybar or eww. See docs/cli.md#status.
///
/// Hand-formatted, which is only safe because every value in here is a number
/// or one of three fixed words — no text from the user's file reaches it. Put a
/// task title in the tooltip and this needs escaping first.
pub fn status_json(counts: Counts) -> String {
    let text = if counts.overdue > 0 {
        format!("{} ○ {}!", counts.open, counts.overdue)
    } else {
        format!("{} ○", counts.open)
    };

    let mut tooltip = vec![format!("{} open", counts.open)];
    if counts.today > 0 {
        tooltip.push(format!("{} due today", counts.today));
    }
    if counts.overdue > 0 {
        tooltip.push(format!("{} overdue", counts.overdue));
    }

    format!(
        r#"{{"text":"{text}","tooltip":"{}","class":"{}"}}"#,
        tooltip.join(", "),
        counts.class()
    )
}

/// Everything a capture understood except the title: the date resolved, the
/// tags, the priority. `ratodo add` prints it after the title, and the TUI's
/// input preview shows it on its own while the sentence is still being typed.
///
/// The separator is the caller's, because the two callers do not agree on it:
/// the TUI has to fall back to ASCII when the locale is not UTF-8, and stdout
/// does not — docs/tui.md#no-colour-no-nerd-font.
pub fn fields(task: &Task, today: NaiveDate, dot: &str) -> String {
    let joined: Vec<String> = field_parts(task, today)
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    joined.join(&format!("  {dot}  "))
}

/// The same fields, still paired with what each one is, for the caller that can
/// colour them. The TUI's preview reads this and stdout reads `fields`, so the
/// two cannot drift into saying different things about the same task.
pub fn field_parts(task: &Task, today: NaiveDate) -> Vec<(Part, String)> {
    let mut parts = Vec::new();
    if let Some(due) = task.due {
        parts.push((Part::Date, format!("due {}", relative(due, today))));
    }
    for tag in &task.tags {
        parts.push((Part::Tag, format!("#{}", plain(tag))));
    }
    if let Some(p) = task.priority {
        parts.push((Part::Priority, p.as_str().to_string()));
    }
    parts
}

/// The one line `ratodo add` prints before getting out of the way.
pub fn added(task: &Task, today: NaiveDate) -> String {
    let title = format!("added: {}", plain(&task.title));
    match fields(task, today, "·") {
        rest if rest.is_empty() => title,
        rest => format!("{title}  ·  {rest}"),
    }
}

/// The one line `ratodo done` prints. Deliberately the same shape as `added`.
pub fn marked_done(task: &Task) -> String {
    format!("done: {}", plain(&task.title))
}

/// Why nothing was changed, and what to type instead. Ends by saying the file
/// was left alone — the user has to be able to trust that without checking.
pub fn ambiguous(text: &str, candidates: &[String]) -> String {
    let mut out = format!("'{}' matches {} tasks:", plain(text), candidates.len());
    for title in candidates {
        out.push_str("\n  ");
        out.push_str(&plain(title));
    }
    out.push_str("\nbe more specific — nothing was changed");
    out
}

/// Near dates read better as words, far ones as numbers. The absolute date is
/// always shown too — this is the moment the shorthand gets confirmed, so
/// "tomorrow" alone would hide exactly what the user wants to check.
pub fn relative(due: Due, today: NaiveDate) -> String {
    let days = (due.date - today).num_days();
    let stamp = match due.time {
        Some(t) => format!("{} {}", due.date.format("%Y-%m-%d"), t.format("%H:%M")),
        None => due.date.format("%Y-%m-%d").to_string(),
    };

    let word = match days {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        2..=6 => due.date.format("%A").to_string(),
        _ => return stamp,
    };
    format!("{word} ({stamp})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agenda::Counts;
    use crate::capture::capture;
    use crate::model::Priority;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()
    }

    fn due(y: i32, m: u32, d: u32) -> Due {
        Due::new(NaiveDate::from_ymd_opt(y, m, d).unwrap())
    }

    #[test]
    fn control_characters_never_reach_the_terminal() {
        assert_eq!(plain("clean"), "clean");
        assert_eq!(plain("\x1b[2Jwipe"), "\u{fffd}[2Jwipe");
        assert_eq!(plain("bell\x07"), "bell\u{fffd}");
        assert_eq!(plain("şğüöç 🚀"), "şğüöç 🚀");
    }

    #[test]
    fn relative_dates_read_as_words_when_they_are_near() {
        assert_eq!(relative(due(2026, 8, 10), today()), "today (2026-08-10)");
        assert_eq!(relative(due(2026, 8, 11), today()), "tomorrow (2026-08-11)");
        assert_eq!(
            relative(due(2026, 8, 12), today()),
            "Wednesday (2026-08-12)"
        );
        assert_eq!(relative(due(2026, 8, 16), today()), "Sunday (2026-08-16)");
    }

    #[test]
    fn distant_and_past_dates_read_as_numbers() {
        assert_eq!(relative(due(2026, 8, 17), today()), "2026-08-17");
        assert_eq!(relative(due(2026, 8, 9), today()), "2026-08-09");
        assert_eq!(relative(due(2020, 1, 1), today()), "2020-01-01");
    }

    #[test]
    fn a_time_is_carried_through() {
        let d = Due {
            date: NaiveDate::from_ymd_opt(2026, 8, 11).unwrap(),
            time: chrono::NaiveTime::from_hms_opt(16, 0, 0),
        };
        assert_eq!(relative(d, today()), "tomorrow (2026-08-11 16:00)");
    }

    #[test]
    fn the_added_line() {
        let t = capture("pay the invoice @tomorrow #home !high", today());
        assert_eq!(
            added(&t, today()),
            "added: pay the invoice  ·  due tomorrow (2026-08-11)  ·  #home  ·  !high"
        );

        let bare = capture("something", today());
        assert_eq!(added(&bare, today()), "added: something");
    }

    #[test]
    fn list_lines_mark_state() {
        let open = capture("a @2026-08-12", today());
        assert_eq!(list_line(&open, today()), "  [ ] a  2026-08-12");

        let late = capture("a @2026-08-08", today());
        assert_eq!(list_line(&late, today()), "  [!] a  2026-08-08");

        let mut done = capture("a @2026-08-08", today());
        done.set_state(State::Done, today());
        assert_eq!(
            list_line(&done, today()),
            "  [x] a  2026-08-08",
            "a completed task is never overdue"
        );

        let undated = capture("a", today());
        assert_eq!(list_line(&undated, today()), "  [ ] a");
    }

    #[test]
    fn the_status_line_is_the_same_string_list_ends_with() {
        let counts = Counts {
            open: 3,
            today: 0,
            overdue: 1,
            ..Counts::default()
        };
        assert_eq!(status_line(counts), "3 open · 1 overdue");
        assert_eq!(status_line(Counts::default()), "0 open · 0 overdue");
    }

    /// The shape in docs/cli.md, asserted whole: waybar reads `text`, shows
    /// `tooltip` and styles off `class`, so all three are an interface.
    #[test]
    fn the_status_json_is_what_waybar_expects() {
        let counts = Counts {
            open: 3,
            today: 0,
            overdue: 1,
            ..Counts::default()
        };
        assert_eq!(
            status_json(counts),
            r#"{"text":"3 ○ 1!","tooltip":"3 open, 1 overdue","class":"overdue"}"#
        );
    }

    #[test]
    fn a_quiet_list_gets_no_exclamation_mark_and_no_overdue_clause() {
        let counts = Counts {
            open: 2,
            today: 1,
            overdue: 0,
            ..Counts::default()
        };
        assert_eq!(
            status_json(counts),
            r#"{"text":"2 ○","tooltip":"2 open, 1 due today","class":"due"}"#
        );
        assert_eq!(
            status_json(Counts::default()),
            r#"{"text":"0 ○","tooltip":"0 open","class":"ok"}"#
        );
    }

    /// The field count is the contract: `cut -f5` has to mean priority on every
    /// line, including the ones with no tags and no date.
    #[test]
    fn a_porcelain_line_always_has_five_fields() {
        let cases = [
            "everything @2026-08-12 #ops #home !high",
            "bare",
            "dated @2026-08-12",
            "tagged #ops",
        ];
        for text in cases {
            let line = porcelain_line(&capture(text, today()));
            assert_eq!(line.split('\t').count(), 5, "{text} → {line:?}");
        }
    }

    #[test]
    fn the_porcelain_fields_are_state_date_title_tags_priority() {
        let t = capture(
            "write the deploy plan @2026-08-12 #ops #home !high",
            today(),
        );
        assert_eq!(
            porcelain_line(&t),
            "open\t2026-08-12\twrite the deploy plan\tops,home\thigh"
        );

        assert_eq!(
            porcelain_line(&capture("call the bank", today())),
            "open\t\tcall the bank\t\t"
        );
    }

    #[test]
    fn porcelain_says_done_and_drops_the_overdue_distinction() {
        let mut t = capture("close the old PRs @2026-08-09 #ops", today());
        assert!(t.is_overdue(today()), "the fixture is meant to be late");
        assert!(
            porcelain_line(&t).starts_with("open\t"),
            "overdue is a display state; a script reads the date itself"
        );

        t.set_state(State::Done, today());
        assert_eq!(
            porcelain_line(&t),
            "done\t2026-08-09\tclose the old PRs\tops\t"
        );
    }

    /// The time is deliberately not in there. Field 2 is a date a script can
    /// compare or hand to `date -d`; if the time is ever wanted it arrives as a
    /// sixth column, which costs nobody their `cut -f3`.
    #[test]
    fn a_time_does_not_reach_the_date_field() {
        let t = capture("standup @2026-08-12 09:30", today());
        assert_eq!(porcelain_line(&t).split('\t').nth(1), Some("2026-08-12"));
    }

    #[test]
    fn a_tab_in_a_title_cannot_invent_a_field() {
        let mut t = capture("harmless", today());
        t.title = "two\tcolumns".into();
        let line = porcelain_line(&t);
        assert_eq!(line.split('\t').count(), 5, "{line:?}");
        assert!(line.contains("two\u{fffd}columns"), "{line:?}");
    }

    #[test]
    fn list_lines_carry_every_field() {
        let t = capture("a @2026-08-12 16:00 #x #y !low", today());
        assert_eq!(
            list_line(&t, today()),
            "  [ ] a  2026-08-12 16:00  #x  #y  !low"
        );
        assert_eq!(t.priority, Some(Priority::Low));
    }
}
