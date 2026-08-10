//! Human-facing strings for the command line. See docs/cli.md.

use chrono::NaiveDate;

use crate::model::{Due, Task};

/// A todo.md can arrive over `git pull`. Control characters in it would be
/// acted on by the terminal rather than shown.
pub fn plain(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

/// One task, as `ratodo list` prints it.
pub fn list_line(task: &Task, today: NaiveDate) -> String {
    let mark = if task.done {
        "[x]"
    } else if task.is_overdue(today) {
        "[!]"
    } else {
        "[ ]"
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

/// The one line `ratodo add` prints before getting out of the way.
pub fn added(task: &Task, today: NaiveDate) -> String {
    let mut parts = vec![format!("added: {}", plain(&task.title))];
    if let Some(due) = task.due {
        parts.push(format!("due {}", relative(due, today)));
    }
    for tag in &task.tags {
        parts.push(format!("#{}", plain(tag)));
    }
    if let Some(p) = task.priority {
        parts.push(p.as_str().to_string());
    }
    parts.join("  ·  ")
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
        done.set_done(true);
        assert_eq!(
            list_line(&done, today()),
            "  [x] a  2026-08-08",
            "a completed task is never overdue"
        );

        let undated = capture("a", today());
        assert_eq!(list_line(&undated, today()), "  [ ] a");
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
