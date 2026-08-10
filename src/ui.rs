//! ratatui drawing. See docs/tui.md.

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, List, ListItem, ListState};

use crate::agenda::{Counts, Group};
use crate::model::Task;
use crate::text;
use crate::theme::Theme;

/// What a keypress means. Separated from reading events so the keymap — the
/// most user-visible surface in the tool — can be tested without a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Move(isize),
    Top,
    Bottom,
    Ignore,
}

/// The list-mode keys from docs/tui.md#keys. Note what is deliberately absent:
/// `esc` is `Ignore`, never `Quit` — someone pressing it out of habit must not
/// lose the pane.
pub fn action(key: KeyEvent) -> Action {
    // Windows reports a release for every press; without this every key acts
    // twice. A `Repeat` is a key held down and has to keep scrolling, so the
    // test is against `Release` alone rather than for `Press`.
    if key.kind == KeyEventKind::Release {
        return Action::Ignore;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char('c') if ctrl => Action::Quit,
        // Only the bare letter: ctrl-q is a terminal flow-control key on some setups.
        KeyCode::Char('q') if !ctrl => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::Move(1),
        KeyCode::Char('k') | KeyCode::Up => Action::Move(-1),
        KeyCode::Char('g') => Action::Top,
        KeyCode::Char('G') => Action::Bottom,
        _ => Action::Ignore,
    }
}

/// One line of the list. Only a `Task` can hold the selection; the rest is
/// scenery the cursor moves over.
///
/// Owned rather than borrowed, so that a reload can swap the whole list out
/// without the screen still pointing into the document it replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Header(String),
    Task(Task),
    Spacer,
}

/// Flattens the agenda into lines. The blank row between groups is half of the
/// design — see docs/design.md#rules — so it is a row, not a margin.
pub fn rows(groups: &[Group<'_>]) -> Vec<Row> {
    let mut out = Vec::new();
    for group in groups {
        if !out.is_empty() {
            out.push(Row::Spacer);
        }
        if let Some(title) = group.kind.title() {
            out.push(Row::Header(title.to_string()));
        }
        out.extend(group.tasks.iter().map(|t| Row::Task((*t).clone())));
    }
    out
}

#[derive(Default)]
pub struct Screen {
    rows: Vec<Row>,
    /// Holds the scroll offset as well as the selection, so the selected row
    /// stays on screen without this module doing viewport arithmetic.
    state: ListState,
}

impl Screen {
    pub fn new(rows: Vec<Row>) -> Self {
        let mut screen = Screen::default();
        screen.replace(rows);
        screen
    }

    /// Swaps the list for a freshly read one and tries to leave the cursor where
    /// it was. Matching on the raw line is not the identity tracking docs/tui.md
    /// asks for — that is step 6 — but it covers the case that actually happens:
    /// `ratodo add` in another pane pushing rows around underneath you.
    pub fn replace(&mut self, rows: Vec<Row>) {
        let was = self.task().map(|t| t.raw.clone());
        let kept = was.and_then(|raw| {
            rows.iter()
                .position(|r| matches!(r, Row::Task(t) if t.raw == raw))
        });

        self.rows = rows;
        let first = (0..self.rows.len()).find(|&i| self.is_task(i));
        self.state.select(kept.or(first));
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn task(&self) -> Option<&Task> {
        match self.rows.get(self.state.selected()?) {
            Some(Row::Task(t)) => Some(t),
            _ => None,
        }
    }

    fn is_task(&self, i: usize) -> bool {
        matches!(self.rows.get(i), Some(Row::Task(_)))
    }

    /// Moves `n` task rows, skipping headers and blanks, and **stops at the
    /// ends rather than wrapping**: a list that wraps costs you your place every
    /// time you overshoot.
    pub fn move_by(&mut self, n: isize) {
        let Some(mut at) = self.state.selected() else {
            return;
        };
        // The sign is read once, outside the loop: `n > 0` inside it is a
        // condition no test can pin down, because at `n == 0` the loop never runs
        // and both branches are the same answer.
        let forward = n.is_positive();
        for _ in 0..n.unsigned_abs() {
            let next = if forward {
                (at + 1..self.rows.len()).find(|&i| self.is_task(i))
            } else {
                (0..at).rev().find(|&i| self.is_task(i))
            };
            match next {
                Some(i) => at = i,
                None => break,
            }
        }
        self.state.select(Some(at));
    }

    pub fn top(&mut self) {
        self.jump(0..self.rows.len());
    }

    pub fn bottom(&mut self) {
        self.jump((0..self.rows.len()).rev());
    }

    fn jump(&mut self, mut order: impl Iterator<Item = usize>) {
        if let Some(i) = order.find(|&i| self.is_task(i)) {
            self.state.select(Some(i));
        }
    }
}

/// The dumb version: the rows, a border and the counts. The design in
/// docs/tui.md lands in step 6 — this is the one that proves the loop runs.
pub fn draw(
    frame: &mut Frame,
    screen: &mut Screen,
    counts: Counts,
    today: NaiveDate,
    colours: Theme,
) {
    let items: Vec<ListItem> = screen
        .rows
        .iter()
        .map(|row| match row {
            // The CLI's own two-space indent would fight the selection marker.
            Row::Task(t) => ListItem::new(text::list_line(t, today).trim_start().to_string())
                .style(Style::default().fg(task_colour(t, today, colours))),
            Row::Header(title) => {
                ListItem::new(text::plain(title)).style(Style::default().fg(colours.accent).bold())
            }
            Row::Spacer => ListItem::new(""),
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(colours.border))
                .title(format!(" ratodo — {} ", text::status_line(counts))),
        )
        .style(Style::default().bg(colours.background))
        .highlight_symbol("▌ ")
        // Background only. Setting a foreground here would repaint the selected
        // row in the accent colour, and an overdue task would stop being red the
        // moment you moved the cursor onto it — which is the one row you are
        // most likely to be looking at. docs/design.md: red only ever means late.
        .highlight_style(Style::default().bg(colours.selection));

    frame.render_stateful_widget(list, frame.area(), &mut screen.state);
}

/// Red is only for overdue and green only for done — docs/design.md#rules — so
/// this is the whole of the colour logic and there is nowhere else to add to it.
fn task_colour(task: &Task, today: NaiveDate, colours: Theme) -> Color {
    if task.done {
        colours.done_text
    } else if task.is_overdue(today) {
        colours.overdue
    } else if task.due.is_some_and(|d| d.date == today) {
        colours.today
    } else {
        colours.foreground
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agenda::agenda;
    use crate::capture::capture;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()
    }

    fn tasks(specs: &[&str]) -> Vec<Task> {
        specs.iter().map(|s| capture(s, today())).collect()
    }

    fn in_section(specs: &[(&str, &str)]) -> Vec<Task> {
        specs
            .iter()
            .map(|(text, section)| {
                let mut t = capture(text, today());
                t.section = Some(section.to_string());
                t
            })
            .collect()
    }

    fn titles(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Header(t) => format!("# {t}"),
                Row::Task(t) => t.title.clone(),
                Row::Spacer => String::new(),
            })
            .collect()
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn every_key_the_dumb_list_answers_to() {
        let cases = [
            (KeyCode::Char('q'), Action::Quit),
            (KeyCode::Char('j'), Action::Move(1)),
            (KeyCode::Down, Action::Move(1)),
            (KeyCode::Char('k'), Action::Move(-1)),
            (KeyCode::Up, Action::Move(-1)),
            (KeyCode::Char('g'), Action::Top),
            (KeyCode::Char('G'), Action::Bottom),
        ];
        for (code, want) in cases {
            assert_eq!(action(press(code)), want, "{code:?}");
        }
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::Quit
        );
    }

    /// The modifier is the whole of the difference. A bare `c` quitting would
    /// close the pane on a typo, and `ctrl-q` is flow control on some terminals
    /// rather than a keystroke we should claim.
    #[test]
    fn the_control_key_is_not_optional_and_not_ignorable() {
        assert_eq!(action(press(KeyCode::Char('c'))), Action::Ignore);
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            Action::Ignore
        );
    }

    /// The keys docs/tui.md leaves unbound on purpose. `esc` is the one that
    /// matters: pressed out of habit, it must not take the pane down with it.
    #[test]
    fn the_deliberately_unbound_keys_do_nothing() {
        for code in [
            KeyCode::Esc,
            KeyCode::Char('x'),
            KeyCode::Char(':'),
            KeyCode::Char('/'),
            KeyCode::Char('Q'),
        ] {
            assert_eq!(action(press(code)), Action::Ignore, "{code:?}");
        }
    }

    /// Windows sends a release for every press. Acting on both moves the cursor
    /// two rows for one keystroke.
    #[test]
    fn a_key_being_let_go_is_not_a_second_press() {
        let mut key = press(KeyCode::Char('j'));
        assert_eq!(action(key), Action::Move(1));
        key.kind = KeyEventKind::Release;
        assert_eq!(action(key), Action::Ignore);
        key.kind = KeyEventKind::Repeat;
        assert_eq!(action(key), Action::Move(1), "held down still scrolls");
    }

    #[test]
    fn the_groups_flatten_into_headers_tasks_and_one_blank_between() {
        let tasks = tasks(&["late @2026-08-01", "now @2026-08-10", "also @2026-08-10"]);
        let groups = agenda(&tasks, today());
        assert_eq!(
            titles(&rows(&groups)),
            ["# OVERDUE", "late", "", "# TODAY", "now", "also"]
        );
    }

    /// No leading blank row: a pane that opens with an empty first line looks
    /// like it failed to draw.
    #[test]
    fn the_first_group_gets_no_spacer_above_it() {
        let tasks = tasks(&["a @2026-08-10"]);
        assert_eq!(
            rows(&agenda(&tasks, today()))[0],
            Row::Header("TODAY".to_string())
        );
    }

    /// An untitled group is the run of tasks above the file's first heading, and
    /// it still needs its tasks — just not a header row.
    #[test]
    fn an_untitled_group_contributes_only_tasks() {
        let tasks = tasks(&["a", "b"]);
        assert_eq!(titles(&rows(&agenda(&tasks, today()))), ["a", "b"]);
    }

    #[test]
    fn the_selection_starts_on_the_first_task_not_the_header() {
        let tasks = tasks(&["a @2026-08-10"]);
        let groups = agenda(&tasks, today());
        let screen = Screen::new(rows(&groups));
        assert_eq!(screen.selected(), Some(1));
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("a"));
    }

    #[test]
    fn a_list_with_nothing_in_it_has_no_selection() {
        let screen = Screen::new(rows(&[]));
        assert_eq!(screen.selected(), None);
        assert!(screen.task().is_none());
    }

    /// The reason `move_by` is not `selected += 1`: between two groups there are
    /// two rows that cannot hold the cursor.
    #[test]
    fn moving_steps_over_the_headers_and_the_blanks() {
        let tasks = tasks(&["late @2026-08-01", "now @2026-08-10"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("late"));
        screen.move_by(1);
        assert_eq!(
            screen.task().map(|t| t.title.as_str()),
            Some("now"),
            "one press crossed a blank and a header"
        );
    }

    #[test]
    fn moving_stops_at_the_ends_instead_of_wrapping() {
        let tasks = tasks(&["a @2026-08-10", "b @2026-08-10"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        screen.move_by(-1);
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("a"));
        screen.move_by(99);
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("b"));
        screen.move_by(1);
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("b"));
    }

    #[test]
    fn a_half_page_lands_where_that_many_single_steps_would() {
        let tasks = in_section(&[("a", "S"), ("b", "S"), ("c", "S"), ("d", "S")]);
        let groups = agenda(&tasks, today());

        let mut jumped = Screen::new(rows(&groups));
        jumped.move_by(3);
        let mut stepped = Screen::new(rows(&groups));
        for _ in 0..3 {
            stepped.move_by(1);
        }
        assert_eq!(jumped.selected(), stepped.selected());
        assert_eq!(jumped.task().map(|t| t.title.as_str()), Some("d"));
    }

    /// `ratodo add` in the next pane pushes rows around. A cursor that jumps to
    /// the top every time makes the pane unusable as something you leave open.
    #[test]
    fn a_reload_leaves_the_cursor_on_the_task_it_was_on() {
        let before = in_section(&[("one", "S"), ("two", "S"), ("three", "S")]);
        let groups = agenda(&before, today());
        let mut screen = Screen::new(rows(&groups));
        screen.move_by(2);
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("three"));

        let after = in_section(&[
            ("inserted at the top", "S"),
            ("one", "S"),
            ("two", "S"),
            ("three", "S"),
        ]);
        let groups = agenda(&after, today());
        screen.replace(rows(&groups));

        assert_eq!(
            screen.task().map(|t| t.title.as_str()),
            Some("three"),
            "the cursor followed the row number instead of the task"
        );
    }

    #[test]
    fn a_reload_that_takes_the_selected_task_away_falls_back_to_the_top() {
        let before = in_section(&[("one", "S"), ("doomed", "S")]);
        let groups = agenda(&before, today());
        let mut screen = Screen::new(rows(&groups));
        screen.bottom();

        let after = in_section(&[("one", "S")]);
        let groups = agenda(&after, today());
        screen.replace(rows(&groups));
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("one"));

        screen.replace(rows(&[]));
        assert_eq!(screen.selected(), None, "an emptied list selects nothing");
    }

    #[test]
    fn top_and_bottom_reach_the_first_and_last_task() {
        let tasks = tasks(&["late @2026-08-01", "now @2026-08-10", "soon @2026-08-12"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        screen.bottom();
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("soon"));
        screen.top();
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("late"));
    }

    fn rendered(width: u16, height: u16, tasks: &[Task]) -> Vec<String> {
        let groups = agenda(tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let counts = Counts::of(tasks, today());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| draw(f, &mut screen, counts, today(), crate::theme::MOCHA))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .chunks(width as usize)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
            .collect()
    }

    #[test]
    fn the_screen_shows_the_counts_the_groups_and_the_marker() {
        let tasks = tasks(&["late @2026-08-01", "now @2026-08-10"]);
        let screen = rendered(46, 8, &tasks);

        assert!(
            screen[0].contains("ratodo — 2 open · 1 overdue"),
            "{screen:?}"
        );
        assert!(screen[1].contains("OVERDUE"), "{screen:?}");
        assert!(screen[2].contains("▌ [!] late"), "{screen:?}");
        assert!(screen[4].contains("TODAY"), "{screen:?}");
        assert!(screen[5].contains("[ ] now"), "{screen:?}");
        assert!(!screen[5].contains('▌'), "two rows drawn as selected");
    }

    /// A pane in a tiling layout is routinely shorter than the list. If the
    /// selection can leave the viewport the tool looks broken at row eleven.
    #[test]
    fn the_selection_stays_on_screen_in_a_pane_too_short_for_the_list() {
        let specs: Vec<String> = (0..30).map(|i| format!("task{i}")).collect();
        let tasks: Vec<Task> = specs.iter().map(|s| capture(s, today())).collect();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        screen.bottom();

        let counts = Counts::of(&tasks, today());
        let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
        terminal
            .draw(|f| draw(f, &mut screen, counts, today(), crate::theme::MOCHA))
            .unwrap();

        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            text.contains("task29"),
            "the last task scrolled off: {text}"
        );
        assert!(!text.contains("task0 "), "the view did not scroll at all");
    }

    /// Reads the colour of the cell a word starts in.
    fn colour_of(width: u16, height: u16, tasks: &[Task], word: &str, colours: Theme) -> Color {
        let groups = agenda(tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let counts = Counts::of(tasks, today());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| draw(f, &mut screen, counts, today(), colours))
            .unwrap();

        // Cell by cell, not by searching a flattened string: `│` and `─` are
        // multi-byte, so a byte offset into the joined text is not a cell index.
        let buffer = terminal.backend().buffer().clone();
        let cells = buffer.content();
        let symbols: Vec<&str> = cells.iter().map(|c| c.symbol()).collect();
        let wanted: Vec<String> = word.chars().map(|c| c.to_string()).collect();

        let at = symbols
            .windows(wanted.len())
            .position(|run| run.iter().zip(&wanted).all(|(a, b)| *a == b))
            .unwrap_or_else(|| panic!("{word} was never drawn"));
        cells[at].fg
    }

    /// The selected row keeps its own colour. Painting it in the accent would
    /// mean an overdue task stops being red exactly when the cursor is on it,
    /// and docs/design.md#rules says red only ever means late.
    #[test]
    fn selecting_an_overdue_task_does_not_take_its_red_away() {
        let tasks = tasks(&["late @2026-08-01", "fine"]);
        let colours = crate::theme::MOCHA;

        assert_eq!(
            colour_of(40, 8, &tasks, "late", colours),
            colours.overdue,
            "the selection repainted the row"
        );
        assert_eq!(
            colour_of(40, 8, &tasks, "fine", colours),
            colours.foreground
        );
    }

    #[test]
    fn the_theme_decides_every_colour_on_the_screen() {
        let mut done = capture("finished", today());
        done.set_done(true);
        let tasks = [
            capture("late @2026-08-01", today()),
            capture("now @2026-08-10", today()),
            done,
        ];

        for (_, colours) in crate::theme::BUILT_IN {
            assert_eq!(colour_of(40, 12, &tasks, "late", colours), colours.overdue);
            assert_eq!(colour_of(40, 12, &tasks, "now", colours), colours.today);
            assert_eq!(
                colour_of(40, 12, &tasks, "finished", colours),
                colours.done_text
            );
            assert_eq!(
                colour_of(40, 12, &tasks, "OVERDUE", colours),
                colours.accent
            );
        }
    }

    /// `NO_COLOR=1` leaves the symbols doing the work. Nothing on the screen may
    /// carry a colour, or the flag is a lie.
    #[test]
    fn no_colour_means_no_colour_anywhere() {
        let tasks = tasks(&["late @2026-08-01", "now @2026-08-10"]);
        let plain = crate::theme::Theme::plain();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let counts = Counts::of(&tasks, today());

        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|f| draw(f, &mut screen, counts, today(), plain))
            .unwrap();

        for cell in terminal.backend().buffer().content() {
            assert_eq!(cell.fg, Color::Reset, "{:?}", cell.symbol());
            assert_eq!(cell.bg, Color::Reset, "{:?}", cell.symbol());
        }
    }

    /// A title from a file that arrived over `git pull` must not be able to
    /// drive the terminal it is drawn into.
    #[test]
    fn a_control_character_never_reaches_the_buffer() {
        let mut task = capture("innocent title", today());
        task.title = "wipe\x1b[2J".into();
        let screen = rendered(40, 5, &[task]);
        assert!(screen.iter().all(|r| !r.contains('\x1b')), "{screen:?}");
        assert!(screen.iter().any(|r| r.contains('\u{fffd}')), "{screen:?}");
    }
}
