//! `todo.md` -> `Doc`. Syntax in docs/format.md.
//!
//! Every unrecognised line survives as `Item::Text`. Reading is more permissive
//! than writing: `- [X]`, `* [ ]` and `+ [ ]` all parse, but only `- [ ]` is
//! ever written.

use chrono::{NaiveDate, NaiveTime};

use crate::model::{DONE_MARK, Doc, Due, Ending, Item, Line, Priority, State, Task};

pub fn parse(input: &str) -> Doc {
    let mut lines = Vec::new();
    let mut section: Option<String> = None;

    for (raw, ending) in split_lines(input) {
        let item = match parse_task(raw) {
            Some(mut task) => {
                task.section = section.clone();
                Item::Task(task)
            }
            None => {
                if let Some(name) = heading(raw) {
                    section = Some(name);
                }
                Item::Text(raw.to_string())
            }
        };
        lines.push(Line { item, ending });
    }

    Doc { lines }
}

/// Not `str::lines`: that discards `\r\n` vs `\n` and cannot tell a file that
/// ends with a newline from one that does not.
fn split_lines(input: &str) -> Vec<(&str, Ending)> {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut start = 0;

    for i in 0..bytes.len() {
        if bytes[i] != b'\n' {
            continue;
        }
        let (end, ending) = if i > start && bytes[i - 1] == b'\r' {
            (i - 1, Ending::CrLf)
        } else {
            (i, Ending::Lf)
        };
        out.push((&input[start..end], ending));
        start = i + 1;
    }

    if start < input.len() {
        out.push((&input[start..], Ending::None));
    }

    out
}

fn heading(raw: &str) -> Option<String> {
    let t = raw.trim_start();
    if !t.starts_with('#') {
        return None;
    }
    let name = t.trim_start_matches('#');
    if !name.starts_with(' ') {
        return None; // `#tag` on its own line is not a heading
    }
    Some(name.trim().to_string())
}

fn parse_task(raw: &str) -> Option<Task> {
    let indent = raw.len() - raw.trim_start().len();
    let b = raw.as_bytes();
    let mut i = indent;

    if i >= b.len() || !matches!(b[i], b'-' | b'*' | b'+') {
        return None;
    }
    i += 1;

    if i >= b.len() || b[i] != b' ' {
        return None;
    }
    while i < b.len() && b[i] == b' ' {
        i += 1;
    }

    if i + 2 >= b.len() || b[i] != b'[' || b[i + 2] != b']' {
        return None;
    }
    let state = match b[i + 1] {
        b' ' => State::Open,
        b'x' | b'X' => State::Done,
        b'-' => State::Cancelled,
        _ => return None,
    };
    let checkbox = i + 1;

    let after = i + 3;
    if after < b.len() && b[after] != b' ' {
        return None;
    }

    let mut task = Task::from_parts(raw.to_string(), checkbox);
    task.state = state;
    parse_meta(&raw[after..], &mut task);
    Some(task)
}

/// Anything not understood stays in the title rather than being dropped — an
/// unparseable `@2026-13-45` is simply text.
fn parse_meta(rest: &str, task: &mut Task) {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let mut title: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];

        if let Some(rest) = word.strip_prefix('@')
            && task.due.is_none()
            && let Some(date) = parse_iso_date(rest)
        {
            let time = words.get(i + 1).and_then(|w| parse_time(w));
            task.due = Some(Due { date, time });
            i += if time.is_some() { 2 } else { 1 };
            continue;
        }

        if let Some(rest) = word.strip_prefix(DONE_MARK)
            && task.done_on.is_none()
            && let Some(date) = parse_iso_date(rest)
        {
            task.done_on = Some(date);
            i += 1;
            continue;
        }

        if let Some(tag) = word.strip_prefix('#')
            && !tag.is_empty()
        {
            task.tags.push(tag.to_string());
            i += 1;
            continue;
        }

        if let Some(p) = parse_priority(word) {
            task.priority = Some(p);
            i += 1;
            continue;
        }

        title.push(word);
        i += 1;
    }

    task.title = title.join(" ");
}

/// Strict ISO. Shorthand like `@tomorrow` is an input convenience resolved in
/// `capture`, and never appears in the file.
fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
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
    use crate::model::Item;

    fn only_task(s: &str) -> Task {
        let doc = parse(s);
        doc.tasks().next().expect("expected one task").clone()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn plain_open_task() {
        let t = only_task("- [ ] pay the invoice");
        assert!(!t.done());
        assert_eq!(t.title, "pay the invoice");
        assert!(t.due.is_none());
        assert!(t.tags.is_empty());
        assert!(t.priority.is_none());
    }

    #[test]
    fn all_metadata() {
        let t = only_task("- [ ] rotate the keys @2026-08-08 #ops #home !high");
        assert_eq!(t.title, "rotate the keys");
        assert_eq!(t.due.unwrap().date, date(2026, 8, 8));
        assert_eq!(t.tags, vec!["ops", "home"]);
        assert_eq!(t.priority, Some(Priority::High));
    }

    #[test]
    fn due_with_time() {
        let t = only_task("- [ ] review the PR @2026-08-10 16:00 #work");
        let due = t.due.unwrap();
        assert_eq!(due.date, date(2026, 8, 10));
        assert_eq!(due.time, Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()));
        assert_eq!(t.title, "review the PR");
    }

    #[test]
    fn permissive_bullets_and_capital_x() {
        for line in ["- [x] done", "* [x] done", "+ [x] done", "- [X] done"] {
            let t = only_task(line);
            assert!(t.done(), "{line} should parse as done");
            assert_eq!(t.title, "done");
        }
    }

    /// The third state, and the field that goes with the second.
    #[test]
    fn cancelled_tasks_and_completion_stamps() {
        let t = only_task("- [-] decided against @2026-08-12 #ops");
        assert_eq!(t.state, State::Cancelled);
        assert!(!t.open(), "a cancelled task is not open");
        assert!(!t.done(), "and it is not finished either");
        assert_eq!(t.title, "decided against");
        assert_eq!(t.due.unwrap().date, date(2026, 8, 12));

        let t = only_task("- [x] shipped it @2026-08-08 ✓2026-08-10 #ops");
        assert_eq!(t.done_on, Some(date(2026, 8, 10)));
        assert_eq!(t.due.unwrap().date, date(2026, 8, 8));
        assert_eq!(t.title, "shipped it", "the stamp is not part of the title");
        assert_eq!(t.tags, vec!["ops"]);
    }

    /// The stamp is `✓` plus an ISO date and nothing else. A bare tick, or one
    /// with something that is not a date after it, is somebody's own text —
    /// `gnarly.md` has one — and stays in the title where they put it.
    #[test]
    fn a_tick_that_is_not_a_stamp_stays_in_the_title() {
        for line in [
            "- [x] a ✓ tick",
            "- [x] a ✓maybe",
            "- [x] a ✓2026-13-45",
            "- [x] a ✓2026-08",
        ] {
            let t = only_task(line);
            assert_eq!(t.done_on, None, "{line}");
            assert!(t.title.contains('✓'), "{line} lost the user's tick");
        }
    }

    /// The second stamp on a line is text, exactly like the second `@date`.
    #[test]
    fn only_the_first_stamp_counts() {
        let t = only_task("- [x] a ✓2026-08-10 ✓2026-08-11");
        assert_eq!(t.done_on, Some(date(2026, 8, 10)));
        assert_eq!(t.title, "a ✓2026-08-11");
    }

    #[test]
    fn indented_task_is_still_a_task() {
        let t = only_task("  - [ ] an indented subtask");
        assert_eq!(t.title, "an indented subtask");
        assert_eq!(t.raw, "  - [ ] an indented subtask");
    }

    #[test]
    fn invalid_date_stays_in_the_title() {
        let t = only_task("- [ ] invalid date @2026-13-45");
        assert!(t.due.is_none());
        assert_eq!(t.title, "invalid date @2026-13-45");
    }

    #[test]
    fn bare_at_and_hash_are_text() {
        let t = only_task("- [ ] junk @ and # on their own");
        assert!(t.due.is_none());
        assert!(t.tags.is_empty());
        assert_eq!(t.title, "junk @ and # on their own");
    }

    #[test]
    fn empty_checkbox_line_is_a_task() {
        let t = only_task("- [ ]");
        assert_eq!(t.title, "");
        assert!(!t.done());
    }

    #[test]
    fn non_tasks_are_never_tasks() {
        for line in [
            "# My list",
            "## Work",
            "> a quote",
            "| a | table |",
            "---",
            "",
            "just a paragraph",
            "-[ ] no space after the bullet",
            "- [] empty brackets",
            "- [?] not a checkbox",
            "-- [ ] two bullets",
            // A closing bracket in the right place with no opening one. Each of
            // these fails a different clause of the box check.
            "- ax] rest",
            "- a ] rest",
            "- [x rest",
            "- x] rest",
        ] {
            assert_eq!(parse(line).task_count(), 0, "{line:?} must not be a task");
        }
    }

    #[test]
    fn sections_are_attached() {
        let doc = parse("## Work\n- [ ] a\n\n## Home\n- [ ] b\n");
        let tasks: Vec<_> = doc.tasks().collect();
        assert_eq!(tasks[0].section.as_deref(), Some("Work"));
        assert_eq!(tasks[1].section.as_deref(), Some("Home"));
    }

    #[test]
    fn a_tag_on_its_own_line_is_not_a_heading() {
        let doc = parse("#ops\n- [ ] a\n");
        assert_eq!(doc.tasks().next().unwrap().section, None);
    }

    #[test]
    fn line_endings_and_final_newline_are_remembered() {
        assert_eq!(parse("a\nb\n").lines.last().unwrap().ending, Ending::Lf);
        assert_eq!(parse("a\nb").lines.last().unwrap().ending, Ending::None);
        assert_eq!(parse("a\r\nb\r\n").lines[0].ending, Ending::CrLf);
        assert_eq!(parse("").lines.len(), 0);
        assert_eq!(parse("\n").lines.len(), 1);
    }

    #[test]
    fn blank_lines_are_kept_as_text() {
        let doc = parse("- [ ] a\n\n\n- [ ] b\n");
        assert_eq!(doc.lines.len(), 4);
        assert_eq!(doc.lines[1].item, Item::Text(String::new()));
        assert_eq!(doc.lines[2].item, Item::Text(String::new()));
    }

    #[test]
    fn second_date_is_left_in_the_title() {
        let t = only_task("- [ ] a @2026-08-08 b @2026-09-09");
        assert_eq!(t.due.unwrap().date, date(2026, 8, 8));
        assert_eq!(t.title, "a b @2026-09-09");
    }

    #[test]
    fn priority_is_case_insensitive_when_read() {
        assert_eq!(only_task("- [ ] a !HIGH").priority, Some(Priority::High));
        assert_eq!(only_task("- [ ] a !nope").priority, None);
        assert_eq!(only_task("- [ ] a !nope").title, "a !nope");
    }

    #[test]
    fn non_ascii_survives() {
        let t = only_task("- [ ] fatura öde şğüöçİI 🚀 @2026-08-17 #ev");
        assert_eq!(t.title, "fatura öde şğüöçİI 🚀");
        assert_eq!(t.tags, vec!["ev"]);
        assert_eq!(t.due.unwrap().date, date(2026, 8, 17));
    }
}
