//! Agenda grouping. See docs/design.md#agenda-grouping-rules-v1.

use chrono::{Datelike, Days, NaiveDate};

use crate::model::{Priority, Task};

/// What `list --tag` and `--prio` narrow the file down to. See docs/cli.md.
///
/// The agenda only has something to say about dated tasks, and most of a
/// developer's list is undated — this is how the rest of it stays reachable.
#[derive(Debug, Clone, Copy, Default)]
pub struct Filter<'a> {
    /// Empty means no tag filter at all. Several tags mean *or*.
    pub tags: &'a [String],
    pub prio: Option<Priority>,
}

impl Filter<'_> {
    pub fn matches(&self, task: &Task) -> bool {
        let tagged = self.tags.is_empty()
            || self
                .tags
                .iter()
                .any(|wanted| task.tags.iter().any(|t| same_tag(t, wanted)));
        tagged && self.prio.is_none_or(|p| task.priority == Some(p))
    }
}

/// `#Ops` and `#ops` are the same tag to everyone except a byte comparison.
/// Unicode-aware lowercasing, which still gets Turkish dotted/dotless I wrong —
/// `İş` and `iş` will not match. Noted rather than solved: the fix is a locale
/// the tool does not otherwise need.
fn same_tag(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// What a status bar asks for. See docs/cli.md#status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub open: usize,
    pub today: usize,
    pub overdue: usize,
    /// Only the TUI's title bar reads this. `status` and `--json` are an
    /// interface other people's status bars are already parsing, and they say
    /// what they have always said.
    pub done: usize,
}

impl Counts {
    pub fn of(tasks: &[Task], today: NaiveDate) -> Self {
        let open = tasks.iter().filter(|t| t.open());
        Counts {
            open: open.clone().count(),
            today: open
                .clone()
                .filter(|t| t.due.is_some_and(|d| d.date == today))
                .count(),
            overdue: tasks.iter().filter(|t| t.is_overdue(today)).count(),
            done: tasks.iter().filter(|t| t.done()).count(),
        }
    }

    /// The field waybar and eww key their CSS off, so these three words are an
    /// interface: renaming one silently unstyles somebody's bar.
    pub fn class(&self) -> &'static str {
        if self.overdue > 0 {
            "overdue"
        } else if self.today > 0 {
            "due"
        } else {
            "ok"
        }
    }
}

/// How many tasks were finished on each day of `today`'s week, Monday first.
///
/// `today` is a parameter and there is no clock in here, for the same reason
/// `agenda` has none: a function that asks the calendar what day it is cannot be
/// tested on any other day.
///
/// Read off the `✓` completion stamps in the file, so it says nothing about a
/// task ticked before the stamp existed. That is a real hole and the screen owes
/// the reader an answer about it rather than a quietly short bar —
/// docs/format.md#the-completion-stamp.
/// Takes an iterator rather than a slice so the TUI can hand it the tasks it
/// already holds, folded groups included, without collecting the list again on
/// every keystroke.
pub fn week<'a>(tasks: impl IntoIterator<Item = &'a Task>, today: NaiveDate) -> [usize; 7] {
    let monday = today - Days::new(today.weekday().num_days_from_monday() as u64);
    let mut out = [0usize; 7];
    for task in tasks {
        let Some(on) = task.done_on.filter(|_| task.done()) else {
            continue;
        };
        let day = (on - monday).num_days();
        if (0..7).contains(&day) {
            out[day as usize] += 1;
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind<'a> {
    Overdue,
    Today,
    ThisWeek,
    Later,
    /// Undated, under the file's own heading. `None` is the run of tasks above
    /// the first one, and `file` is set only when more than one list is open.
    Section {
        file: Option<&'a str>,
        name: Option<&'a str>,
    },
}

impl<'a> Kind<'a> {
    /// The heading as it is shown: the name, and the file it came out of while
    /// several lists are open. `None` when there is nothing to say — the run of
    /// tasks above a single file's first heading gets no heading, rather than a
    /// "(no section)" nobody wrote.
    ///
    /// One composition for the screen and the printed list, or `## Work` from
    /// two files would be told apart in one of them and not the other.
    pub fn heading(self) -> Option<String> {
        match self {
            Kind::Section { file, name } => match (file, name) {
                (Some(file), Some(name)) => Some(format!("{name} ({file})")),
                // A file's tasks above its own first heading still need saying
                // whose they are, or they read as more of the file before them.
                (Some(file), None) => Some(format!("({file})")),
                (None, Some(name)) => Some(name.to_string()),
                (None, None) => None,
            },
            other => other.title().map(str::to_string),
        }
    }

    /// `None` for tasks above the first heading: a file with no headings gets no
    /// headings, rather than a "(no section)" nobody wrote.
    pub fn title(self) -> Option<&'a str> {
        match self {
            Kind::Overdue => Some("OVERDUE"),
            Kind::Today => Some("TODAY"),
            Kind::ThisWeek => Some("THIS WEEK"),
            Kind::Later => Some("LATER"),
            Kind::Section { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group<'a> {
    pub kind: Kind<'a>,
    pub tasks: Vec<&'a Task>,
}

/// `today` is a parameter and there is no clock in here, which is the only
/// reason any of this is testable.
///
/// Membership of `Overdue` is positional — a completed task keeps the date it
/// had — so it is not the same question as `Task::is_overdue`, which asks
/// whether something still needs attention. Both words are right; they are
/// answering different things.
pub fn agenda<'a>(tasks: &'a [Task], today: NaiveDate) -> Vec<Group<'a>> {
    // A `today` near the end of the calendar must not panic the agenda.
    let horizon = today
        .checked_add_days(Days::new(7))
        .unwrap_or(NaiveDate::MAX);

    let mut dated: [Vec<&Task>; 4] = Default::default();
    let mut sections: Vec<Group<'a>> = Vec::new();

    for task in tasks {
        let Some(due) = task.due else {
            // Contiguous runs, never merged across the file: two `## Work`
            // headings stay two groups rather than one that pulls tasks upwards.
            let kind = Kind::Section {
                file: task.file.as_deref(),
                name: task.section.as_deref(),
            };
            match sections.last_mut() {
                Some(group) if group.kind == kind => group.tasks.push(task),
                _ => sections.push(Group {
                    kind,
                    tasks: vec![task],
                }),
            }
            continue;
        };

        let slot = if due.date < today {
            0
        } else if due.date == today {
            1
        } else if due.date <= horizon {
            2
        } else {
            3
        };
        dated[slot].push(task);
    }

    let mut out: Vec<Group<'a>> = Vec::new();
    let kinds = [Kind::Overdue, Kind::Today, Kind::ThisWeek, Kind::Later];
    for (kind, mut tasks) in kinds.into_iter().zip(dated) {
        if tasks.is_empty() {
            continue;
        }
        // Stable, so tasks that tie on date keep the order the file had them in.
        tasks.sort_by_key(|t| (!t.open(), t.due.map(|d| (d.date, d.time))));
        out.push(Group { kind, tasks });
    }

    // Undated last, and unsorted: docs/design.md is explicit that the user's own
    // arrangement under their own headings is not ours to rearrange.
    out.extend(sections);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::capture;
    use crate::model::State;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()
    }

    /// The seven cells behind the sparkline on the main screen and the
    /// histogram on the stats one. Monday first, and `today` is Monday the 10th
    /// of August 2026, so the week runs the 10th to the 16th.
    #[test]
    fn the_week_counts_completions_by_the_day_they_were_stamped() {
        let stamped = |title: &str, y, m, d| {
            let mut task = capture(title, today());
            task.set_state(State::Done, NaiveDate::from_ymd_opt(y, m, d).unwrap());
            task
        };
        let tasks = [
            stamped("mon", 2026, 8, 10),
            stamped("wed", 2026, 8, 12),
            stamped("wed again", 2026, 8, 12),
            stamped("sun", 2026, 8, 16),
            // Last week and next week are somebody else's bar.
            stamped("last sunday", 2026, 8, 9),
            stamped("next monday", 2026, 8, 17),
            // Open, and never mind that it has a stamp on it.
            capture("still open", today()),
        ];

        assert_eq!(week(&tasks, today()), [1, 0, 2, 0, 0, 0, 1]);
        assert_eq!(week(&[], today()), [0; 7]);
    }

    /// The week is `today`'s week wherever in it `today` falls, so the answer
    /// does not move between Monday and Sunday.
    #[test]
    fn the_week_is_the_same_week_from_any_day_in_it() {
        let mut task = capture("done", today());
        task.set_state(State::Done, NaiveDate::from_ymd_opt(2026, 8, 12).unwrap());

        for day in 10..=16 {
            let from = NaiveDate::from_ymd_opt(2026, 8, day).unwrap();
            assert_eq!(
                week(std::slice::from_ref(&task), from),
                [0, 0, 1, 0, 0, 0, 0],
                "{from}"
            );
        }
    }

    /// A task ticked before the completion stamp existed has no `done_on`, so it
    /// is in no day's bar. The screen says so rather than quietly under-reporting
    /// — docs/tui.md#main-screen.
    #[test]
    fn a_completion_with_no_stamp_lands_in_no_day() {
        let mut task = capture("done long ago", today());
        task.set_state(State::Done, today());
        task.done_on = None;

        assert_eq!(week(std::slice::from_ref(&task), today()), [0; 7]);
    }

    /// Undated tasks group by heading, and with several lists open the heading
    /// is not the whole answer: `## Work` in two files is two headings, or one
    /// file's tasks would be pulled up under the other's.
    #[test]
    fn the_same_heading_in_two_files_stays_two_groups() {
        let mut tasks = [capture("review", today()), capture("deploy", today())];
        for (task, file) in tasks.iter_mut().zip(["work.md", "2026.md"]) {
            task.section = Some("Work".into());
            task.file = Some(file.to_string());
        }

        let groups = agenda(&tasks, today());
        assert_eq!(groups.len(), 2, "{groups:?}");
        assert_eq!(
            groups.iter().map(|g| g.kind).collect::<Vec<_>>(),
            [
                Kind::Section {
                    file: Some("work.md"),
                    name: Some("Work")
                },
                Kind::Section {
                    file: Some("2026.md"),
                    name: Some("Work")
                }
            ]
        );

        // And one list is still one group, however many headings say the same.
        for task in &mut tasks {
            task.file = None;
        }
        assert_eq!(agenda(&tasks, today()).len(), 1);
    }

    fn task(text: &str) -> Task {
        capture(text, today())
    }

    fn in_section(text: &str, section: &str) -> Task {
        let mut t = task(text);
        t.section = Some(section.to_string());
        t
    }

    /// Group titles paired with the task titles under them — the whole result in
    /// the shape the assertions want to read. An untitled group reads as `""`.
    fn shape(tasks: &[Task], today: NaiveDate) -> Vec<(String, Vec<String>)> {
        agenda(tasks, today)
            .iter()
            .map(|g| {
                (
                    g.kind.title().unwrap_or_default().to_string(),
                    g.tasks.iter().map(|t| t.title.clone()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn every_group_in_documented_order() {
        let tasks = [
            task("later @2026-09-01"),
            task("undated"),
            task("late @2026-08-01"),
            task("now @2026-08-10"),
            task("soon @2026-08-12"),
        ];
        assert_eq!(
            shape(&tasks, today()),
            [
                ("OVERDUE".into(), vec!["late".to_string()]),
                ("TODAY".into(), vec!["now".into()]),
                ("THIS WEEK".into(), vec!["soon".into()]),
                ("LATER".into(), vec!["later".into()]),
                ("".into(), vec!["undated".into()]),
            ]
        );
    }

    #[test]
    fn an_empty_group_is_not_reported_at_all() {
        assert_eq!(shape(&[], today()), []);
        assert_eq!(
            shape(&[task("only one @2026-08-10")], today()),
            [("TODAY".to_string(), vec!["only one".to_string()])]
        );
    }

    /// The boundaries the whole thing hangs off. Each of these is one day away
    /// from a different answer.
    #[test]
    fn the_day_boundaries() {
        let cases = [
            ("@2026-08-09", "OVERDUE"),
            ("@2026-08-10", "TODAY"),
            ("@2026-08-11", "THIS WEEK"),
            ("@2026-08-17", "THIS WEEK"),
            ("@2026-08-18", "LATER"),
        ];
        for (date, group) in cases {
            let tasks = [task(&format!("a {date}"))];
            assert_eq!(shape(&tasks, today())[0].0, group, "{date}");
        }
    }

    /// Midnight is the start of the day, not the end of the one before it.
    #[test]
    fn a_time_never_changes_which_day_a_task_belongs_to() {
        for time in ["00:00", "23:59"] {
            let tasks = [task(&format!("a @2026-08-10 {time}"))];
            assert_eq!(shape(&tasks, today())[0].0, "TODAY", "{time}");
        }
    }

    #[test]
    fn a_date_years_back_is_still_just_overdue() {
        let tasks = [task("ancient @1999-01-01")];
        assert_eq!(shape(&tasks, today())[0].0, "OVERDUE");
    }

    /// `@2026-02-30` is not a date, so `parse` never produces a `Due` for it and
    /// the task arrives here undated. Asserted because the alternative — a
    /// silently dropped task — would be invisible.
    #[test]
    fn a_task_whose_date_does_not_exist_is_undated_not_missing() {
        let t = task("a @2026-02-30");
        assert_eq!(t.due, None);
        assert_eq!(
            shape(&[t], today()),
            [("".to_string(), vec!["a @2026-02-30".to_string()])]
        );
    }

    #[test]
    fn the_last_week_of_the_calendar_does_not_panic() {
        let end = NaiveDate::MAX;
        let tasks = [task("a @2026-08-10")];
        assert_eq!(shape(&tasks, end)[0].0, "OVERDUE");
    }

    #[test]
    fn within_a_dated_group_the_earliest_is_first_and_the_done_are_last() {
        let mut done = task("done @2026-08-01");
        done.set_state(State::Done, today());
        let tasks = [
            done,
            task("at nine @2026-08-08 09:00"),
            task("first @2026-08-01"),
            task("all day @2026-08-08"),
        ];
        assert_eq!(
            shape(&tasks, today())[0].1,
            ["first", "all day", "at nine", "done"],
            "an all-day task heads its own day, the way a calendar shows it, \
             and completed sinks below the lot"
        );
    }

    #[test]
    fn tasks_on_the_same_day_keep_the_order_the_file_had() {
        let tasks = [
            task("b @2026-08-12"),
            task("a @2026-08-12"),
            task("c @2026-08-12"),
        ];
        assert_eq!(shape(&tasks, today())[0].1, ["b", "a", "c"]);
    }

    #[test]
    fn undated_tasks_are_grouped_by_their_heading() {
        let tasks = [
            in_section("deploy", "Work"),
            in_section("invoice", "Work"),
            in_section("plumber", "Home"),
        ];
        assert_eq!(
            shape(&tasks, today()),
            [
                ("Work".to_string(), vec!["deploy".into(), "invoice".into()]),
                ("Home".to_string(), vec!["plumber".into()]),
            ]
        );
    }

    /// The rule that beats "completed sinks to the bottom": under the user's own
    /// heading the file's order is the answer, done or not.
    #[test]
    fn an_undated_group_is_never_reordered() {
        let mut done = in_section("finished", "Work");
        done.set_state(State::Done, today());
        let tasks = [done, in_section("open", "Work")];
        assert_eq!(shape(&tasks, today())[0].1, ["finished", "open"]);
    }

    /// Merging them would lift the second run's tasks up the page, which is a
    /// reorder however tidy it looks.
    #[test]
    fn a_heading_used_twice_stays_two_groups() {
        let tasks = [
            in_section("a", "Work"),
            in_section("b", "Home"),
            in_section("c", "Work"),
        ];
        let titles: Vec<&str> = agenda(&tasks, today())
            .iter()
            .filter_map(|g| g.kind.title())
            .collect();
        assert_eq!(titles, ["Work", "Home", "Work"]);
    }

    #[test]
    fn the_counts_a_bar_asks_for() {
        let mut done = task("finished @2026-08-01");
        done.set_state(State::Done, today());
        let tasks = [
            task("late @2026-08-01"),
            task("now @2026-08-10"),
            task("soon @2026-08-12"),
            task("undated"),
            done,
        ];
        assert_eq!(
            Counts::of(&tasks, today()),
            Counts {
                open: 4,
                today: 1,
                overdue: 1,
                done: 1,
            }
        );
    }

    /// A completed task is neither open nor overdue however late it was — the
    /// count a bar shows must not include work that is finished. It is counted
    /// as *done*, which is the one place finished work does get to appear.
    #[test]
    fn completing_something_late_empties_the_counts() {
        let mut t = task("late @2026-08-01");
        t.set_state(State::Done, today());
        assert_eq!(
            Counts::of(&[t], today()),
            Counts {
                done: 1,
                ..Counts::default()
            }
        );
    }

    #[test]
    fn the_class_is_the_worst_thing_the_list_contains() {
        let quiet = task("soon @2026-08-12");
        let due = task("now @2026-08-10");
        let late = task("late @2026-08-01");

        assert_eq!(Counts::of(&[], today()).class(), "ok");
        assert_eq!(
            Counts::of(std::slice::from_ref(&quiet), today()).class(),
            "ok"
        );
        assert_eq!(
            Counts::of(std::slice::from_ref(&due), today()).class(),
            "due"
        );
        assert_eq!(
            Counts::of(std::slice::from_ref(&late), today()).class(),
            "overdue"
        );
        assert_eq!(
            Counts::of(&[quiet, due, late], today()).class(),
            "overdue",
            "overdue outranks a task merely due today"
        );
    }

    fn matching<'a>(tasks: &'a [Task], filter: Filter<'_>) -> Vec<&'a str> {
        tasks
            .iter()
            .filter(|t| filter.matches(t))
            .map(|t| t.title.as_str())
            .collect()
    }

    #[test]
    fn the_empty_filter_is_not_a_filter() {
        let tasks = [task("a #ops !high"), task("b")];
        assert_eq!(matching(&tasks, Filter::default()), ["a", "b"]);
    }

    #[test]
    fn a_tag_filter_keeps_only_what_carries_it() {
        let tasks = [task("a #ops"), task("b #home"), task("c")];
        let ops = [String::from("ops")];
        assert_eq!(
            matching(
                &tasks,
                Filter {
                    tags: &ops,
                    ..Filter::default()
                }
            ),
            ["a"]
        );
    }

    /// Repeats are or, not and — nobody means "carries both" by listing two.
    #[test]
    fn several_tags_widen_the_result() {
        let tasks = [task("a #ops"), task("b #home"), task("c #other")];
        let two = [String::from("ops"), String::from("home")];
        assert_eq!(
            matching(
                &tasks,
                Filter {
                    tags: &two,
                    ..Filter::default()
                }
            ),
            ["a", "b"]
        );
    }

    #[test]
    fn a_tag_matches_whatever_case_the_file_wrote_it_in() {
        let tasks = [task("a #Ops"), task("b #OPS")];
        let ops = [String::from("oPs")];
        assert_eq!(
            matching(
                &tasks,
                Filter {
                    tags: &ops,
                    ..Filter::default()
                }
            ),
            ["a", "b"]
        );
    }

    /// `--prio high` means high, not "high and above": there is no ranking in
    /// the file format to read one off.
    #[test]
    fn a_priority_filter_is_an_exact_level() {
        let tasks = [task("a !high"), task("b !med"), task("c")];
        let high = Filter {
            prio: Some(Priority::High),
            ..Filter::default()
        };
        assert_eq!(matching(&tasks, high), ["a"]);
    }

    #[test]
    fn a_tag_and_a_priority_together_narrow_rather_than_widen() {
        let tasks = [
            task("both #ops !high"),
            task("tag only #ops"),
            task("prio only !high"),
        ];
        let ops = [String::from("ops")];
        let filter = Filter {
            tags: &ops,
            prio: Some(Priority::High),
        };
        assert_eq!(matching(&tasks, filter), ["both"]);
    }

    /// A dated task between two undated ones must not split the undated run: the
    /// grouping keys off the heading, not off adjacency in the file.
    #[test]
    fn a_dated_task_in_between_does_not_split_a_section() {
        let mut dated = in_section("dated", "Work");
        dated.due = task("x @2026-08-12").due;
        let tasks = [in_section("a", "Work"), dated, in_section("b", "Work")];
        assert_eq!(
            shape(&tasks, today()),
            [
                ("THIS WEEK".to_string(), vec!["dated".to_string()]),
                ("Work".to_string(), vec!["a".into(), "b".into()]),
            ]
        );
    }
}
