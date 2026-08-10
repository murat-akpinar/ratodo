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
