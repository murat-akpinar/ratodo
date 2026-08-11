//! `todo.ics` output. See docs/calendar.md.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::model::{Priority, Task};

/// One VTODO per open, dated task. `now` is a parameter for the same reason
/// `agenda` takes `today`: a function that reads the clock cannot be tested.
pub fn calendar(tasks: &[Task], now: DateTime<Utc>) -> String {
    let stamp = now.format("%Y%m%dT%H%M%SZ").to_string();
    let mut out = String::new();

    line("BEGIN:VCALENDAR", &mut out);
    line("VERSION:2.0", &mut out);
    line("PRODID:-//ratodo//EN", &mut out);

    let mut seen = HashMap::new();
    for task in tasks {
        // Completed tasks are not exported at all in v1, and an undated one has
        // nothing to put on a calendar.
        let Some(due) = task.due.filter(|_| task.open()) else {
            continue;
        };

        line("BEGIN:VTODO", &mut out);
        line(&format!("UID:{}", uid(task, &mut seen)), &mut out);
        line(&format!("DTSTAMP:{stamp}"), &mut out);
        line(
            &match due.time {
                // Floating local time, deliberately: `@2026-08-12 16:00` in the
                // file means four in the afternoon wherever its author is, and
                // pinning it to the machine's offset today makes it wrong the
                // first time they travel. See docs/calendar.md.
                Some(t) => format!("DUE:{}T{}00", due.date.format("%Y%m%d"), t.format("%H%M")),
                None => format!("DUE;VALUE=DATE:{}", due.date.format("%Y%m%d")),
            },
            &mut out,
        );
        line(&format!("SUMMARY:{}", escape(&task.title)), &mut out);

        if !task.tags.is_empty() {
            let tags: Vec<String> = task.tags.iter().map(|t| escape(t)).collect();
            line(&format!("CATEGORIES:{}", tags.join(",")), &mut out);
        }
        if let Some(p) = task.priority {
            line(&format!("PRIORITY:{}", ics_priority(p)), &mut out);
        }
        line("STATUS:NEEDS-ACTION", &mut out);
        line("END:VTODO", &mut out);
    }

    line("END:VCALENDAR", &mut out);
    out
}

fn ics_priority(p: Priority) -> u8 {
    match p {
        Priority::High => 1,
        Priority::Med => 5,
        Priority::Low => 9,
    }
}

/// Stable across writes, which is the whole point: a UID that changes is a
/// calendar entry deleted and recreated on every save.
///
/// It comes from the title and the section rather than the raw line, so moving a
/// date or adding a tag *updates* the entry a client is already showing. Two
/// tasks that are genuinely identical get an occurrence number mixed in, because
/// two VTODOs sharing a UID is one entry as far as any client is concerned.
fn uid(task: &Task, seen: &mut HashMap<String, u32>) -> String {
    // The same identity the cursor holds on to across a reload — one definition
    // of "the same task", so the two cannot drift apart. See model.rs.
    let base = task.identity();

    let nth = seen.entry(base.clone()).or_insert(0);
    *nth += 1;

    // Counted rather than retried. Rehashing until the hash is free reads as the
    // obvious answer and is an unbounded loop: it terminates only because FNV-1a
    // happens to behave, which is not a property this function can check. The
    // first occurrence keeps the plain hash, so one task with one title never
    // depends on what came before it.
    let key = if *nth == 1 {
        base
    } else {
        format!("{base}\u{1}{nth}")
    };
    format!("{:016x}@ratodo", fnv1a(key.as_bytes()))
}

/// Written out rather than taken from `DefaultHasher`, which is explicitly
/// allowed to change between Rust releases — and a UID that changes with the
/// toolchain is exactly the bug this function exists to avoid.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// RFC 5545 §3.3.11. A `,` left unescaped in a SUMMARY silently becomes a list
/// separator, and the second half of somebody's task title disappears.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            // Anything else a control character would do here is worse.
            c if c.is_control() => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
    out
}

/// RFC 5545 §3.1: CRLF endings, and no line longer than 75 octets — a
/// continuation begins with one space, which counts against the next line's 75.
///
/// The split lands on a character boundary. Folding through the middle of a
/// UTF-8 sequence produces a file that is not text, and `şğüöç` is in the
/// fixtures to make sure that is noticed.
fn line(text: &str, out: &mut String) {
    let mut rest = text;
    let mut budget = 75;

    while rest.len() > budget {
        let mut at = budget;
        while !rest.is_char_boundary(at) {
            at -= 1;
        }
        out.push_str(&rest[..at]);
        out.push_str("\r\n ");
        rest = &rest[at..];
        budget = 74;
    }

    out.push_str(rest);
    out.push_str("\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::capture;
    use crate::model::State;
    use chrono::{NaiveDate, TimeZone};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 10, 9, 30, 0).unwrap()
    }

    fn task(text: &str) -> Task {
        capture(text, today())
    }

    fn lines(ics: &str) -> Vec<&str> {
        ics.split("\r\n").filter(|l| !l.is_empty()).collect()
    }

    #[test]
    fn the_shape_documented_in_calendar_md() {
        let ics = calendar(&[task("pay the invoice @2026-08-12 #home !high")], now());
        let lines = lines(&ics);

        assert_eq!(lines[0], "BEGIN:VCALENDAR");
        assert_eq!(lines[1], "VERSION:2.0");
        assert_eq!(lines[2], "PRODID:-//ratodo//EN");
        assert_eq!(lines[3], "BEGIN:VTODO");
        assert!(lines[4].starts_with("UID:"), "{lines:?}");
        assert_eq!(lines[5], "DTSTAMP:20260810T093000Z");
        assert_eq!(lines[6], "DUE;VALUE=DATE:20260812");
        assert_eq!(lines[7], "SUMMARY:pay the invoice");
        assert_eq!(lines[8], "CATEGORIES:home");
        assert_eq!(lines[9], "PRIORITY:1");
        assert_eq!(lines[10], "STATUS:NEEDS-ACTION");
        assert_eq!(lines[11], "END:VTODO");
        assert_eq!(lines[12], "END:VCALENDAR");
        assert_eq!(lines.len(), 13);
    }

    #[test]
    fn an_empty_list_is_still_a_valid_calendar() {
        assert_eq!(
            calendar(&[], now()),
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//ratodo//EN\r\nEND:VCALENDAR\r\n"
        );
    }

    /// v1 exports open, dated tasks and nothing else. A completed task on a
    /// calendar is noise, and an undated one has no place to be drawn.
    #[test]
    fn only_open_dated_tasks_are_exported() {
        let mut done = task("finished @2026-08-12");
        done.set_state(State::Done, today());
        let tasks = [done, task("undated"), task("kept @2026-08-12")];

        let ics = calendar(&tasks, now());
        assert_eq!(ics.matches("BEGIN:VTODO").count(), 1, "{ics}");
        assert!(ics.contains("SUMMARY:kept"), "{ics}");
    }

    #[test]
    fn a_priority_or_a_tag_that_is_absent_leaves_out_its_line() {
        let ics = calendar(&[task("bare @2026-08-12")], now());
        assert!(!ics.contains("PRIORITY"), "{ics}");
        assert!(!ics.contains("CATEGORIES"), "{ics}");
    }

    #[test]
    fn the_three_priorities_map_to_the_documented_numbers() {
        for (word, number) in [("!high", 1), ("!med", 5), ("!low", 9)] {
            let ics = calendar(&[task(&format!("a @2026-08-12 {word}"))], now());
            assert!(ics.contains(&format!("PRIORITY:{number}\r\n")), "{word}");
        }
    }

    #[test]
    fn a_time_becomes_a_floating_date_time_and_a_bare_date_stays_a_date() {
        let timed = calendar(&[task("standup @2026-08-12 09:30")], now());
        assert!(timed.contains("DUE:20260812T093000\r\n"), "{timed}");
        assert!(
            !timed.contains("DUE:20260812T093000Z"),
            "a Z would pin the user's 09:30 to today's offset: {timed}"
        );

        let dated = calendar(&[task("someday @2026-08-12")], now());
        assert!(dated.contains("DUE;VALUE=DATE:20260812\r\n"), "{dated}");
    }

    /// The property the whole feature rests on: run it twice, get the same UID.
    #[test]
    fn a_uid_does_not_move_between_writes() {
        let tasks = [task("pay the invoice @2026-08-12")];
        let first = calendar(&tasks, now());
        let later = calendar(&tasks, Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap());

        let uid_of = |ics: &str| {
            lines(ics)
                .into_iter()
                .find(|l| l.starts_with("UID:"))
                .unwrap()
                .to_string()
        };
        assert_eq!(uid_of(&first), uid_of(&later));
    }

    /// Not `DefaultHasher`, whose output the standard library reserves the right
    /// to change. A diff in these numbers means every entry in every user's
    /// calendar was deleted and recreated.
    ///
    /// The values come from an independent FNV-1a run, not from this code — a
    /// test that pins whatever the implementation happens to print pins nothing.
    #[test]
    fn the_hash_is_pinned_not_borrowed() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"ratodo"), 0x84cf_cabf_1dd1_fa84);
    }

    #[test]
    fn a_date_change_updates_the_entry_rather_than_replacing_it() {
        let before = calendar(&[task("pay the invoice @2026-08-12")], now());
        let after = calendar(&[task("pay the invoice @2026-09-01 #new !low")], now());

        let uid = |ics: &str| {
            lines(ics)
                .into_iter()
                .find(|l| l.starts_with("UID:"))
                .unwrap()
                .to_string()
        };
        assert_eq!(uid(&before), uid(&after), "the date is not the identity");
    }

    /// The same title in two files is two tasks, and two calendar entries. It is
    /// the identity that carries that, which is why there is only one of them.
    #[test]
    fn the_same_task_in_two_files_gets_two_entries() {
        let mut tasks = [
            task("fix the tap @2026-08-12"),
            task("fix the tap @2026-08-12"),
        ];
        for (task, file) in tasks.iter_mut().zip(["work.md", "home.md"]) {
            task.file = Some(file.to_string());
        }

        let ics = calendar(&tasks, now());
        let uids = uids(&ics);
        assert_eq!(uids.len(), 2, "{ics}");
        assert_ne!(uids[0], uids[1], "one entry for two files: {uids:?}");
    }

    /// Two VTODOs with one UID is one entry to every client, so the second task
    /// would simply vanish.
    fn uids(ics: &str) -> Vec<String> {
        lines(ics)
            .into_iter()
            .filter(|l| l.starts_with("UID:"))
            .map(str::to_string)
            .collect()
    }

    /// Two VTODOs with one UID is one entry to every client, so the duplicates
    /// would simply vanish. Three of them, not two: with two, a counter that
    /// never advances past its first step still looks correct.
    #[test]
    fn identical_tasks_still_get_one_entry_each() {
        let one = task("standup @2026-08-12");
        let ics = calendar(&[one.clone(), one.clone(), one], now());

        let uids = uids(&ics);
        assert_eq!(uids.len(), 3);
        assert_eq!(
            uids.iter().collect::<std::collections::HashSet<_>>().len(),
            3,
            "{ics}"
        );
    }

    /// The first occurrence is hashed from its own text alone, so adding a
    /// duplicate later does not move the entry a client is already showing.
    #[test]
    fn a_duplicate_appearing_later_leaves_the_first_uid_alone() {
        let one = task("standup @2026-08-12");
        let alone = uids(&calendar(std::slice::from_ref(&one), now()));
        let crowded = uids(&calendar(&[one.clone(), one], now()));

        assert_eq!(alone[0], crowded[0]);
        assert_ne!(crowded[0], crowded[1]);
    }

    #[test]
    fn the_same_title_under_two_headings_is_two_tasks() {
        let mut work = task("standup @2026-08-12");
        work.section = Some("Work".into());
        let mut home = task("standup @2026-08-12");
        home.section = Some("Home".into());

        let ics = calendar(&[work, home], now());
        let uids: Vec<&str> = lines(&ics)
            .into_iter()
            .filter(|l| l.starts_with("UID:"))
            .collect();
        assert_ne!(uids[0], uids[1]);
    }

    #[test]
    fn the_characters_rfc_5545_treats_as_syntax() {
        let mut t = task("a @2026-08-12");
        t.title = "buy milk, bread; and \\ a note".into();
        let ics = calendar(&[t], now());
        assert!(
            ics.contains("SUMMARY:buy milk\\, bread\\; and \\\\ a note"),
            "{ics}"
        );
    }

    #[test]
    fn a_control_character_never_reaches_the_file() {
        let mut t = task("a @2026-08-12");
        t.title = "wipe\x1b[2J".into();
        let ics = calendar(&[t], now());
        assert!(!ics.contains('\x1b'), "{ics}");
        assert!(ics.contains('\u{fffd}'), "{ics}");
    }

    /// A newline inside a title would end the property and turn the rest of the
    /// title into a line the parser has to guess at.
    #[test]
    fn a_newline_in_a_title_cannot_start_a_new_property() {
        let mut t = task("a @2026-08-12");
        t.title = "first\nSUMMARY:injected".into();
        let ics = calendar(&[t], now());
        assert!(ics.contains("SUMMARY:first\\nSUMMARY:injected"), "{ics}");
        assert_eq!(ics.matches("\r\nSUMMARY:").count(), 1, "{ics}");
    }

    #[test]
    fn no_line_is_longer_than_seventy_five_octets() {
        let mut t = task("a @2026-08-12");
        t.title = "ş".repeat(200);
        let ics = calendar(&[t], now());

        for line in ics.split("\r\n") {
            assert!(line.len() <= 75, "{} octets: {line:?}", line.len());
        }
    }

    /// Exactly 75 is allowed. One octet more is not. Off by one here produces a
    /// fold with an empty continuation line, which unfolds to the right text and
    /// looks wrong to anybody reading the file.
    #[test]
    fn seventy_five_is_the_limit_and_not_one_less() {
        let mut fits = String::new();
        line(&"x".repeat(75), &mut fits);
        assert_eq!(fits, format!("{}\r\n", "x".repeat(75)), "75 got folded");

        let mut over = String::new();
        line(&"x".repeat(76), &mut over);
        assert_eq!(over, format!("{}\r\n x\r\n", "x".repeat(75)), "76 did not");
    }

    /// Folding through the middle of a UTF-8 sequence produces a file that is not
    /// text at all. `ş` is two octets, so a naive split at 75 lands inside one.
    #[test]
    fn folding_never_cuts_a_character_in_half() {
        let title = "şğüöç 🚀".repeat(20);
        let mut t = task("a @2026-08-12");
        t.title = title.clone();
        let ics = calendar(&[t], now());

        // Unfolding is CRLF-plus-one-space removed; what comes back must be the
        // title we put in.
        let unfolded = ics.replace("\r\n ", "");
        assert!(unfolded.contains(&title), "{unfolded}");
    }

    #[test]
    fn every_line_ends_crlf_and_none_ends_bare_lf() {
        let ics = calendar(&[task("a @2026-08-12"), task("b @2026-08-13")], now());
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert_eq!(
            ics.matches('\n').count(),
            ics.matches("\r\n").count(),
            "a bare LF got in: {ics:?}"
        );
    }
}
