//! The document model. See docs/architecture.md.

use chrono::{NaiveDate, NaiveTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    Lf,
    CrLf,
    None,
}

impl Ending {
    pub fn as_str(self) -> &'static str {
        match self {
            Ending::Lf => "\n",
            Ending::CrLf => "\r\n",
            Ending::None => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    High,
    Med,
    Low,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::High => "!high",
            Priority::Med => "!med",
            Priority::Low => "!low",
        }
    }

    /// The bare word, as `--prio` and `list --porcelain` spell it.
    pub fn name(self) -> &'static str {
        &self.as_str()[1..]
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Priority::High, Priority::Med, Priority::Low]
            .into_iter()
            .find(|p| p.name() == name)
    }
}

/// What is between the brackets. Three states, and the file spells each of them:
/// `- [ ]`, `- [x]`, `- [-]`.
///
/// `- [-]` is the Obsidian / Logseq convention for *decided against*, which is
/// the one thing a list cannot say when its only exit is deletion. It is out of
/// the counts and never overdue — see docs/format.md.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Open,
    Done,
    Cancelled,
}

impl State {
    /// The byte between the brackets. ASCII in all three, which is what lets the
    /// tick be a one-byte splice into `raw`.
    pub fn as_byte(self) -> u8 {
        match self {
            State::Open => b' ',
            State::Done => b'x',
            State::Cancelled => b'-',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Due {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

impl Due {
    pub fn new(date: NaiveDate) -> Self {
        Due { date, time: None }
    }

    pub fn to_file_string(self) -> String {
        match self.time {
            Some(t) => format!("@{} {}", self.date.format("%Y-%m-%d"), t.format("%H:%M")),
            None => format!("@{}", self.date.format("%Y-%m-%d")),
        }
    }
}

/// The sigil that marks a completion date, chosen to match the `✓` on screen.
///
/// Non-ASCII in the file, unlike everything else the tool writes. That is a
/// deliberate exception recorded in docs/decisions.md: the alternatives were a
/// second meaning for `@` or a `done:` word that a title could collide with.
pub const DONE_MARK: char = '✓';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub raw: String,
    pub state: State,
    /// When it was ticked — `✓2026-08-11`. `None` for anything finished before
    /// the tool started writing it, and for everything not finished.
    pub done_on: Option<NaiveDate>,
    pub title: String,
    pub due: Option<Due>,
    pub tags: Vec<String>,
    pub priority: Option<Priority>,
    pub section: Option<String>,
    /// Which file it was read from — `work.md` — and `None` when only one list
    /// is open, which is the whole of what tells the two apart. A single-file
    /// setup therefore keeps the identities, and so the calendar UIDs, it had
    /// before several files were a thing. See docs/cli.md#several-lists.
    pub file: Option<String>,
    /// While false, `raw` is authoritative and is written back untouched.
    pub dirty: bool,
    /// Byte index into `raw` of the character between the brackets.
    checkbox: usize,
}

impl Task {
    pub fn new(
        state: State,
        title: String,
        due: Option<Due>,
        tags: Vec<String>,
        priority: Option<Priority>,
    ) -> Self {
        let mut task = Task {
            raw: String::new(),
            state,
            done_on: None,
            title,
            due,
            tags,
            priority,
            section: None,
            file: None,
            dirty: false,
            checkbox: 3,
        };
        task.raw = task.render_fields();
        task
    }

    pub(crate) fn from_parts(raw: String, checkbox: usize) -> Self {
        Task {
            raw,
            state: State::Open,
            done_on: None,
            title: String::new(),
            due: None,
            tags: Vec::new(),
            priority: None,
            section: None,
            file: None,
            dirty: false,
            checkbox,
        }
    }

    pub fn done(&self) -> bool {
        self.state == State::Done
    }

    /// Still wanted and still unfinished. The one that decides the counts, the
    /// overdue test and what `ratodo done` will match — a cancelled task is out
    /// of all three.
    pub fn open(&self) -> bool {
        self.state == State::Open
    }

    // -- ALAN START: round-trip --
    // Every change here is surgery on `raw` rather than a re-render: the state
    // is one ASCII byte, and a date is one whitespace-delimited field spliced
    // into the text the user wrote. Their spacing, their field order and
    // anything the parser did not understand all survive it.
    pub fn set_state(&mut self, state: State, today: NaiveDate) {
        if self.state == state {
            return;
        }
        self.state = state;
        let mut bytes = std::mem::take(&mut self.raw).into_bytes();
        bytes[self.checkbox] = state.as_byte();
        self.raw = String::from_utf8(bytes).expect("the checkbox byte is ASCII");

        // The stamp says when it was finished, so it belongs to `Done` alone and
        // leaves again with it. Re-ticking a task restamps it with the new day.
        self.done_on = (state == State::Done).then_some(today);
        let stamp = self
            .done_on
            .map(|d| format!("{DONE_MARK}{}", d.format("%Y-%m-%d")));
        self.splice(is_done_mark, stamp.as_deref());
    }

    /// Moves the due date and nothing else.
    ///
    /// The time is a separate field and is left where it is: pushing "Friday at
    /// 09:30" out by a week is still half past nine. A task with no date at all
    /// gets one, which is the only sense `p` can make of it.
    pub fn postpone(&mut self, to: NaiveDate) {
        let time = self.due.and_then(|d| d.time);
        self.due = Some(Due { date: to, time });
        let iso = format!("@{}", to.format("%Y-%m-%d"));
        self.splice(is_due, Some(&iso));
    }

    /// One whitespace-delimited field of `raw` replaced, removed (`None`) or, if
    /// it was not there, appended.
    ///
    /// Only ever past the checkbox, so the indent and the bullet are out of
    /// reach by construction.
    ///
    /// `cargo mutants` reports `checkbox + 2` as a surviving mutant and it is an
    /// **equivalent** one, which is worth writing down rather than chasing:
    /// everything before the checkbox is fixed syntax — a bullet, spaces and
    /// `[`, none of which any `is` can match — and the window start cancels out
    /// of the absolute range on the next line. It is a guard against a future
    /// predicate, not a live constraint, and no input can tell the two apart.
    fn splice(&mut self, is: impl Fn(&str) -> bool, to: Option<&str>) {
        let from = self.checkbox + 2;
        let found = crate::capture::words(&self.raw[from..])
            .into_iter()
            .find(|&(_, word)| is(word))
            .map(|(at, word)| from + at..from + at + word.len());

        match (found, to) {
            (Some(at), Some(to)) => self.raw.replace_range(at, to),
            // One space, both ways, and deliberately not "however many are
            // there": appending always adds exactly one, so removing exactly
            // one is what puts the line back. A list written with trailing
            // spaces gets a slightly wide gap for as long as the field is on
            // it, and gets its own spacing back untouched the moment it goes.
            (Some(at), None) => {
                let start = match self.raw[from..at.start].ends_with(' ') {
                    true => at.start - 1,
                    false => at.start,
                };
                self.raw.replace_range(start.max(from)..at.end, "");
            }
            (None, Some(to)) => {
                self.raw.push(' ');
                self.raw.push_str(to);
            }
            (None, None) => {}
        }
    }

    /// Everything after the checkbox: what the input field is pre-filled with,
    /// and what an edit replaces.
    pub fn body(&self) -> &str {
        self.raw.get(self.checkbox + 2..).unwrap_or("").trim_start()
    }

    /// Swaps the body for a freshly captured one and keeps everything up to and
    /// including the checkbox byte for byte — the indentation, the bullet, and
    /// whether the box is ticked. The user retyped the line's content, not its
    /// shape, and a nested task has to stay nested.
    pub fn retype(&mut self, fields: Task) {
        self.raw = format!("{} {}", &self.raw[..self.checkbox + 2], fields.body());
        self.title = fields.title;
        self.due = fields.due;
        self.tags = fields.tags;
        self.priority = fields.priority;
    }

    pub fn line(&self) -> String {
        if self.dirty {
            self.render_fields()
        } else {
            self.raw.clone()
        }
    }
    // -- ALAN END --

    /// What makes this the same task as before: the section it sits in and its
    /// title.
    ///
    /// Deliberately **not** the raw line — a date moved or a tag added is the
    /// same task with a new line, and a calendar entry or a cursor that let go
    /// of it there would be letting go on every edit. Deliberately not the row
    /// number either, which is the thing a reload has just rearranged.
    ///
    /// `\u{1}` between the parts because it cannot occur in any of them: a
    /// section called `a` holding `b#c` must not collide with `a#c` holding `b`.
    ///
    /// The file joins them when there is more than one, or `## Work` in
    /// `work.md` and `## Work` in `2026.md` would be one section to the cursor,
    /// to `done` and to the calendar.
    pub fn identity(&self) -> String {
        let of = format!(
            "{}\u{1}{}",
            self.section.as_deref().unwrap_or(""),
            self.title
        );
        // Nothing is prefixed when there is one list, so a setup that never grows
        // a second file keeps the identities it has always had.
        match &self.file {
            Some(file) => format!("{file}\u{1}{of}"),
            None => of,
        }
    }

    /// A task that is not open is never overdue — it is finished or it is off
    /// the list, and neither is late however long ago it was due.
    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        self.open() && self.due.is_some_and(|d| d.date < today)
    }

    fn render_fields(&self) -> String {
        let mut s = String::with_capacity(self.title.len() + 32);
        s.push_str("- [");
        s.push(self.state.as_byte() as char);
        s.push_str("] ");
        s.push_str(self.title.trim());
        if let Some(due) = self.due {
            s.push(' ');
            s.push_str(&due.to_file_string());
        }
        for tag in &self.tags {
            s.push_str(" #");
            s.push_str(tag);
        }
        if let Some(p) = self.priority {
            s.push(' ');
            s.push_str(p.as_str());
        }
        if let Some(on) = self.done_on {
            s.push(' ');
            s.push(DONE_MARK);
            s.push_str(&on.format("%Y-%m-%d").to_string());
        }
        s
    }
}

/// `✓2026-08-11`, and only that exact shape.
///
/// The date is checked because a bare `✓` is something people put in their own
/// titles, and `splice` would have taken it for ours and written over it. The
/// `gnarly.md` fixture has one, which is how this was found rather than
/// shipped.
fn is_done_mark(word: &str) -> bool {
    word.strip_prefix(DONE_MARK)
        .is_some_and(|rest| NaiveDate::parse_from_str(rest, "%Y-%m-%d").is_ok())
}

fn is_due(word: &str) -> bool {
    word.strip_prefix('@')
        .is_some_and(|rest| NaiveDate::parse_from_str(rest, "%Y-%m-%d").is_ok())
}

#[cfg(test)]
mod task_tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn due_on(y: i32, m: u32, d: u32) -> Option<Due> {
        Some(Due::new(ymd(y, m, d)))
    }

    /// The boundary the whole agenda hangs off: due *today* is the TODAY group,
    /// not the OVERDUE one. See docs/design.md.
    #[test]
    fn due_today_is_not_overdue() {
        let today = ymd(2026, 8, 10);
        let mut task = Task::new(State::Open, "a".into(), due_on(2026, 8, 10), vec![], None);
        assert!(!task.is_overdue(today), "a task due today is not late yet");

        task.due = due_on(2026, 8, 9);
        assert!(task.is_overdue(today), "yesterday is late");

        task.due = due_on(2026, 8, 11);
        assert!(!task.is_overdue(today));
    }

    #[test]
    fn a_completed_task_is_never_overdue() {
        let today = ymd(2026, 8, 10);
        let mut task = Task::new(State::Open, "a".into(), due_on(2020, 1, 1), vec![], None);
        assert!(task.is_overdue(today));
        task.set_state(State::Done, today);
        assert!(!task.is_overdue(today));
    }

    #[test]
    fn an_undated_task_is_never_overdue() {
        let task = Task::new(State::Open, "a".into(), None, vec![], None);
        assert!(!task.is_overdue(ymd(2026, 8, 10)));
    }

    /// `splice` is the one thing standing between a state change and the user's
    /// line, so its awkward cases get named: nothing to replace, something that
    /// only looks like the field, and a line that already ends in space.
    #[test]
    fn a_field_is_spliced_into_the_line_and_never_over_it() {
        let today = ymd(2026, 8, 10);
        let line = |raw: &str| crate::parse::parse(raw).tasks().next().unwrap().clone();

        // Appended, because there was none — after everything, including the
        // tags and the priority the user put last.
        let mut t = line("- [ ] a @2026-08-12 #ops !high");
        t.set_state(State::Done, today);
        assert_eq!(t.raw, "- [x] a @2026-08-12 #ops !high ✓2026-08-10");

        // And taken back off with the space it was given.
        t.set_state(State::Open, today);
        assert_eq!(t.raw, "- [ ] a @2026-08-12 #ops !high");
        assert_eq!(t.done_on, None);

        // A line that already ended in space keeps every one of them.
        let mut t = line("- [ ] trailing   ");
        t.set_state(State::Done, today);
        assert_eq!(t.raw, "- [x] trailing    ✓2026-08-10");
        t.set_state(State::Open, today);
        assert_eq!(t.raw, "- [ ] trailing   ", "the user's spaces were eaten");

        // A `✓` of the user's own is not the field and is not written over.
        let mut t = line("- [ ] a ✓ tick of my own");
        t.set_state(State::Done, today);
        assert_eq!(t.raw, "- [x] a ✓ tick of my own ✓2026-08-10");

        // Cancelling never stamps, and clears a stamp it inherits.
        let mut t = line("- [x] a ✓2026-08-01");
        t.set_state(State::Cancelled, today);
        assert_eq!(t.raw, "- [-] a");
        assert_eq!(t.done_on, None);
    }

    /// `p` moves `@` and leaves the time, because pushing "Friday at 09:30" out
    /// by a week is still half past nine.
    #[test]
    fn postponing_moves_the_date_and_keeps_the_time() {
        let line = |raw: &str| crate::parse::parse(raw).tasks().next().unwrap().clone();
        let to = ymd(2026, 8, 17);

        let mut t = line("- [ ] a @2026-08-10 09:30 #ops");
        t.postpone(to);
        assert_eq!(t.raw, "- [ ] a @2026-08-17 09:30 #ops");
        assert_eq!(t.due.unwrap().date, to);
        assert_eq!(t.due.unwrap().time, "09:30".parse().ok());

        // Nothing to replace: an undated task gets the date appended.
        let mut t = line("- [ ] a #ops");
        t.postpone(to);
        assert_eq!(t.raw, "- [ ] a #ops @2026-08-17");
        assert_eq!(t.due.unwrap().date, to);

        // The indent and the bullet are before the checkbox and out of reach.
        let mut t = line("\t * [x] a @2026-08-10");
        t.postpone(to);
        assert_eq!(t.raw, "\t * [x] a @2026-08-17");
    }

    /// What the input field shows: the line's own text, not our rendering of it.
    #[test]
    fn the_body_is_whatever_follows_the_checkbox() {
        let doc = crate::parse::parse("  * [x]   wash up @2026-08-12\n- [ ] plain\n- [ ]\n");
        let bodies: Vec<&str> = doc.tasks().map(|t| t.body()).collect();
        assert_eq!(bodies, ["wash up @2026-08-12", "plain", ""]);
    }

    /// Retyping a line replaces what the user typed and nothing else. The
    /// indentation, the bullet they chose and the tick survive, because the
    /// input field was never showing them in the first place.
    #[test]
    fn retyping_keeps_the_indent_the_bullet_and_the_tick() {
        let doc = crate::parse::parse("  * [x] wash up @2026-08-12 #home\n");
        let mut task = doc.tasks().next().unwrap().clone();

        task.retype(crate::capture::capture(
            "wash up tonight #home !high",
            ymd(2026, 8, 11),
        ));

        assert_eq!(task.line(), "  * [x] wash up tonight #home !high");
        assert!(task.done(), "the tick was not the user's to retype");
        assert_eq!(task.title, "wash up tonight");
        assert_eq!(task.due, None, "the date was dropped, so it goes");
        assert_eq!(task.priority, Some(Priority::High));
    }
}

#[cfg(test)]
mod line_tests {
    /// A line index is not a task number, and the two callers that hold one — the
    /// ambiguity list `done` prints, and the tick that follows it — get `None`
    /// for anything that is not a task rather than the wrong line.
    #[test]
    fn a_line_index_only_answers_for_a_line_that_is_a_task() {
        let mut doc = crate::parse::parse("## Work\n- [ ] first\n> a note\n");

        assert_eq!(
            doc.task_at(1).map(|t| t.title.clone()),
            Some("first".to_string())
        );
        assert_eq!(
            doc.task_at_mut(1).map(|t| t.title.clone()),
            Some("first".to_string())
        );

        for not_a_task in [0, 2, 9] {
            assert!(doc.task_at(not_a_task).is_none(), "line {not_a_task}");
            assert!(doc.task_at_mut(not_a_task).is_none(), "line {not_a_task}");
        }
    }
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    /// Titles in, a document out — the line indices then mean what the enum says.
    fn doc(titles: &[(&str, bool)]) -> Doc {
        Doc {
            lines: titles
                .iter()
                .map(|&(title, done)| Line {
                    item: Item::Task(Task::new(
                        if done { State::Done } else { State::Open },
                        title.into(),
                        None,
                        vec![],
                        None,
                    )),
                    ending: Ending::Lf,
                })
                .collect(),
        }
    }

    #[test]
    fn one_open_match_reports_the_line_it_is_on() {
        let d = doc(&[("pay the invoice", false), ("call the bank", false)]);
        assert_eq!(d.find_open("bank"), Lookup::One(1));
        assert_eq!(
            d.find_open("INVOICE"),
            Lookup::One(0),
            "case does not matter"
        );
        assert_eq!(
            d.find_open("the"),
            Lookup::Several(vec!["pay the invoice".into(), "call the bank".into(),])
        );
    }

    /// The index is into `lines`, not a count of tasks: a document with prose in
    /// it would otherwise tick a line seven rows above the one that matched.
    #[test]
    fn the_index_counts_every_line_not_every_task() {
        let mut d = doc(&[("first", false), ("target", false)]);
        d.lines.insert(
            1,
            Line {
                item: Item::Text("> a note".into()),
                ending: Ending::Lf,
            },
        );
        let Lookup::One(at) = d.find_open("target") else {
            panic!("expected exactly one match");
        };
        assert_eq!(at, 2);
        assert_eq!(d.lines[at].text(), "- [ ] target");
    }

    #[test]
    fn nothing_at_all_is_not_the_same_as_already_finished() {
        let d = doc(&[("close the old PRs", true)]);
        assert_eq!(
            d.find_open("old PRs"),
            Lookup::AlreadyDone("close the old PRs".into())
        );
        assert_eq!(d.find_open("something else"), Lookup::None);
    }

    /// A completed task must not make an open one ambiguous — that is the whole
    /// reason the search is over open tasks only.
    #[test]
    fn a_finished_task_is_not_a_candidate() {
        let d = doc(&[("write the report", true), ("send the report", false)]);
        assert_eq!(d.find_open("report"), Lookup::One(1));
    }

    /// `ratodo done ''` matches every task by substring rules, and on a list with
    /// one open task that would read as a correct guess.
    #[test]
    fn an_empty_search_matches_nothing_rather_than_everything() {
        let d = doc(&[("the only task", false)]);
        assert_eq!(d.find_open(""), Lookup::None);
        assert_eq!(d.find_open("   "), Lookup::None);
    }

    #[test]
    fn surrounding_space_is_not_part_of_the_search() {
        let d = doc(&[("pay the invoice", false)]);
        assert_eq!(d.find_open("  invoice  "), Lookup::One(0));
    }

    #[test]
    fn a_search_never_changes_anything() {
        let before = doc(&[("a", false), ("b", true)]);
        let after = before.clone();
        let _ = after.find_open("a");
        assert_eq!(before, after);
    }
}

#[cfg(test)]
mod push_tests {
    use super::*;

    fn task(title: &str) -> Task {
        Task::new(State::Open, title.into(), None, vec![], None)
    }

    fn text(s: &str) -> Line {
        Line {
            item: Item::Text(s.to_string()),
            ending: Ending::Lf,
        }
    }

    fn rendered(doc: &Doc) -> Vec<String> {
        doc.lines.iter().map(|l| l.text()).collect()
    }

    /// The reason this is not a plain append: a hand-written list very often ends
    /// with something that is not a task, and a captured task must not land under it.
    #[test]
    fn a_task_lands_above_the_trailing_prose() {
        let mut doc = Doc {
            lines: vec![
                text("## Work"),
                Line {
                    item: Item::Task(task("first")),
                    ending: Ending::Lf,
                },
                text(""),
                text("---"),
                text("a closing note"),
            ],
        };
        doc.push_task(task("second"));

        assert_eq!(
            rendered(&doc),
            [
                "## Work",
                "- [ ] first",
                "- [ ] second",
                "",
                "---",
                "a closing note"
            ]
        );
    }

    #[test]
    fn a_file_with_no_tasks_at_all_appends_at_the_end() {
        let mut doc = Doc {
            lines: vec![text("# My list"), text("just a paragraph")],
        };
        doc.push_task(task("first"));
        assert_eq!(
            rendered(&doc),
            ["# My list", "just a paragraph", "- [ ] first"]
        );
    }

    #[test]
    fn the_missing_final_newline_is_supplied_only_when_appending_past_the_end() {
        let mut doc = Doc {
            lines: vec![Line {
                item: Item::Text("no trailing newline".into()),
                ending: Ending::None,
            }],
        };
        doc.push_task(task("first"));
        assert_eq!(doc.lines[0].ending, Ending::Lf);
        assert_eq!(doc.lines[1].ending, Ending::Lf);
    }

    /// Inserting in the middle must not give the file a trailing newline it did
    /// not have.
    #[test]
    fn a_file_without_a_final_newline_keeps_not_having_one() {
        let mut doc = Doc {
            lines: vec![
                Line {
                    item: Item::Task(task("first")),
                    ending: Ending::Lf,
                },
                Line {
                    item: Item::Text("closing note".into()),
                    ending: Ending::None,
                },
            ],
        };
        doc.push_task(task("second"));
        assert_eq!(doc.lines.last().unwrap().ending, Ending::None);
    }

    #[test]
    fn removing_a_task_takes_one_line_and_leaves_the_rest_alone() {
        let mut doc = Doc {
            lines: vec![
                text("## Work"),
                Line {
                    item: Item::Task(task("first")),
                    ending: Ending::Lf,
                },
                Line {
                    item: Item::Task(task("second")),
                    ending: Ending::Lf,
                },
                text("> a note"),
            ],
        };

        let gone = doc.remove_task(1).expect("a task was there");
        assert_eq!(gone.title, "first");
        assert_eq!(rendered(&doc), ["## Work", "- [ ] second", "> a note"]);
    }

    #[test]
    fn removing_something_that_is_not_a_task_removes_nothing() {
        let mut doc = Doc {
            lines: vec![text("> a note")],
        };
        assert!(doc.remove_task(0).is_none());
        assert!(doc.remove_task(9).is_none());
        assert_eq!(rendered(&doc), ["> a note"]);
    }

    /// The file's last line is the only one that can lack an ending. Deleting it
    /// must hand that absence on, or a file with no trailing newline grows one.
    #[test]
    fn removing_the_last_line_does_not_add_a_trailing_newline() {
        let mut doc = Doc {
            lines: vec![
                Line {
                    item: Item::Task(task("keep")),
                    ending: Ending::Lf,
                },
                Line {
                    item: Item::Task(task("drop")),
                    ending: Ending::None,
                },
            ],
        };

        doc.remove_task(1);
        assert_eq!(doc.lines.last().unwrap().ending, Ending::None);
    }

    #[test]
    fn removing_from_the_middle_leaves_the_final_ending_where_it_was() {
        let mut doc = Doc {
            lines: vec![
                Line {
                    item: Item::Task(task("drop")),
                    ending: Ending::Lf,
                },
                Line {
                    item: Item::Text("last".into()),
                    ending: Ending::None,
                },
            ],
        };

        doc.remove_task(0);
        assert_eq!(doc.lines.last().unwrap().ending, Ending::None);
        assert_eq!(rendered(&doc), ["last"]);
    }

    #[test]
    fn nothing_that_was_already_there_moves_relative_to_anything_else() {
        let before = vec![text("## A"), text("> quote"), text("| table |")];
        let mut doc = Doc {
            lines: before.clone(),
        };
        doc.push_task(task("new"));

        let after: Vec<String> = rendered(&doc)
            .into_iter()
            .filter(|l| l != "- [ ] new")
            .collect();
        assert_eq!(after, rendered(&Doc { lines: before }));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Task(Task),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub item: Item,
    pub ending: Ending,
}

impl Line {
    pub fn text(&self) -> String {
        match &self.item {
            Item::Task(t) => t.line(),
            Item::Text(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Doc {
    pub lines: Vec<Line>,
}

/// What `Doc::find_open` found. See docs/cli.md#done-matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// The index into `Doc::lines`, not a task number: the caller has to reach
    /// the same line to change it.
    One(usize),
    None,
    /// The candidate titles, in file order.
    Several(Vec<String>),
    /// Matched, but there is nothing left to do to it.
    AlreadyDone(String),
}

impl Doc {
    pub fn tasks(&self) -> impl Iterator<Item = &Task> {
        self.lines.iter().filter_map(|l| match &l.item {
            Item::Task(t) => Some(t),
            Item::Text(_) => None,
        })
    }

    pub fn tasks_mut(&mut self) -> impl Iterator<Item = &mut Task> {
        self.lines.iter_mut().filter_map(|l| match &mut l.item {
            Item::Task(t) => Some(t),
            Item::Text(_) => None,
        })
    }

    pub fn task_count(&self) -> usize {
        self.tasks().count()
    }

    pub fn task_at(&self, line: usize) -> Option<&Task> {
        match &self.lines.get(line)?.item {
            Item::Task(t) => Some(t),
            Item::Text(_) => None,
        }
    }

    pub fn task_at_mut(&mut self, line: usize) -> Option<&mut Task> {
        match &mut self.lines.get_mut(line)?.item {
            Item::Task(t) => Some(t),
            Item::Text(_) => None,
        }
    }

    /// Looks a task up the way `ratodo done '<text>'` does: case-insensitive
    /// substring, over the open tasks only. See docs/cli.md#done-matching.
    ///
    /// Ambiguity is reported rather than resolved. Ticking the wrong line is the
    /// exact trust break the round-trip guarantee exists to prevent, and a
    /// "closest match" heuristic is how that happens.
    pub fn find_open(&self, text: &str) -> Lookup {
        let needle = text.trim().to_lowercase();
        if needle.is_empty() {
            // `done ''` would otherwise match every task, and on a list with one
            // open task that reads as a successful guess.
            return Lookup::None;
        }
        let hit = |t: &Task| t.title.to_lowercase().contains(&needle);

        let open: Vec<(usize, &Task)> = self
            .lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match &l.item {
                Item::Task(t) if t.open() && hit(t) => Some((i, t)),
                _ => None,
            })
            .collect();

        match open.as_slice() {
            [(line, _)] => Lookup::One(*line),
            [] => match self.tasks().find(|t| !t.open() && hit(t)) {
                // Otherwise "no task matches" is a lie the user cannot act on.
                Some(t) => Lookup::AlreadyDone(t.title.clone()),
                None => Lookup::None,
            },
            several => Lookup::Several(several.iter().map(|(_, t)| t.title.clone()).collect()),
        }
    }

    /// Takes one line out and returns the task that was on it.
    ///
    /// Deleting is the one place the tool removes something the user wrote, so
    /// it removes exactly one line and touches nothing else. The file's last
    /// line is the only one that can lack an ending, so when it is the one
    /// going, the ending it did not have is handed to whatever is now last —
    /// otherwise a file with no trailing newline quietly grows one.
    pub fn remove_task(&mut self, line: usize) -> Option<Task> {
        let removed = match self.lines.get(line) {
            Some(Line {
                item: Item::Task(_),
                ..
            }) => self.lines.remove(line),
            _ => return None,
        };

        if line == self.lines.len()
            && removed.ending == Ending::None
            && let Some(last) = self.lines.last_mut()
        {
            last.ending = Ending::None;
        }

        match removed.item {
            Item::Task(t) => Some(t),
            Item::Text(_) => None,
        }
    }

    /// Inserts after the last task rather than at the end of the file. A list
    /// that ends with a table, a `---` or a paragraph would otherwise collect
    /// captured tasks below all of it, outside every `##` section.
    ///
    /// Nothing already in the file moves, so this still never reorders.
    pub fn push_task(&mut self, task: Task) {
        let at = self
            .lines
            .iter()
            .rposition(|l| matches!(l.item, Item::Task(_)))
            .map_or(self.lines.len(), |i| i + 1);

        // Only the final line can lack an ending, and appending after it needs one.
        if let Some(prev) = at.checked_sub(1).and_then(|i| self.lines.get_mut(i))
            && prev.ending == Ending::None
        {
            prev.ending = Ending::Lf;
        }

        self.lines.insert(
            at,
            Line {
                item: Item::Task(task),
                ending: Ending::Lf,
            },
        );
    }
}
