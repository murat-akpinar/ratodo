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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub raw: String,
    pub line_no: usize,
    pub done: bool,
    pub title: String,
    pub due: Option<Due>,
    pub tags: Vec<String>,
    pub priority: Option<Priority>,
    pub section: Option<String>,
    /// While false, `raw` is authoritative and is written back untouched.
    pub dirty: bool,
    /// Byte index into `raw` of the character between the brackets.
    checkbox: usize,
}

impl Task {
    pub fn new(
        done: bool,
        title: String,
        due: Option<Due>,
        tags: Vec<String>,
        priority: Option<Priority>,
    ) -> Self {
        let mut task = Task {
            raw: String::new(),
            line_no: 0,
            done,
            title,
            due,
            tags,
            priority,
            section: None,
            dirty: false,
            checkbox: 3,
        };
        task.raw = task.render_fields();
        task
    }

    pub(crate) fn from_parts(raw: String, line_no: usize, checkbox: usize) -> Self {
        Task {
            raw,
            line_no,
            done: false,
            title: String::new(),
            due: None,
            tags: Vec::new(),
            priority: None,
            section: None,
            dirty: false,
            checkbox,
        }
    }

    // -- ALAN START: round-trip --
    // Toggling replaces one ASCII byte inside `raw` instead of re-rendering the
    // line, so the user's spacing and anything we did not understand survive.
    pub fn set_done(&mut self, done: bool) {
        if self.done == done {
            return;
        }
        self.done = done;
        let mut bytes = std::mem::take(&mut self.raw).into_bytes();
        bytes[self.checkbox] = if done { b'x' } else { b' ' };
        self.raw = String::from_utf8(bytes).expect("the checkbox byte is ASCII");
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

    /// A completed task is never overdue, however long ago it was due.
    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        !self.done && self.due.is_some_and(|d| d.date < today)
    }

    fn render_fields(&self) -> String {
        let mut s = String::with_capacity(self.title.len() + 32);
        s.push_str(if self.done { "- [x] " } else { "- [ ] " });
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
        s
    }
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
        let mut task = Task::new(false, "a".into(), due_on(2026, 8, 10), vec![], None);
        assert!(!task.is_overdue(today), "a task due today is not late yet");

        task.due = due_on(2026, 8, 9);
        assert!(task.is_overdue(today), "yesterday is late");

        task.due = due_on(2026, 8, 11);
        assert!(!task.is_overdue(today));
    }

    #[test]
    fn a_completed_task_is_never_overdue() {
        let today = ymd(2026, 8, 10);
        let mut task = Task::new(false, "a".into(), due_on(2020, 1, 1), vec![], None);
        assert!(task.is_overdue(today));
        task.set_done(true);
        assert!(!task.is_overdue(today));
    }

    #[test]
    fn an_undated_task_is_never_overdue() {
        let task = Task::new(false, "a".into(), None, vec![], None);
        assert!(!task.is_overdue(ymd(2026, 8, 10)));
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
        assert!(task.done, "the tick was not the user's to retype");
        assert_eq!(task.title, "wash up tonight");
        assert_eq!(task.due, None, "the date was dropped, so it goes");
        assert_eq!(task.priority, Some(Priority::High));
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
                    item: Item::Task(Task::new(done, title.into(), None, vec![], None)),
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
        Task::new(false, title.into(), None, vec![], None)
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
                Item::Task(t) if !t.done && hit(t) => Some((i, t)),
                _ => None,
            })
            .collect();

        match open.as_slice() {
            [(line, _)] => Lookup::One(*line),
            [] => match self.tasks().find(|t| t.done && hit(t)) {
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
    /// Nothing already in the file moves, so this still never reorders. It does
    /// invalidate `line_no` for the lines it pushes down — see notes.md.
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
