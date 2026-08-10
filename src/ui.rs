//! ratatui drawing. See docs/tui.md.

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
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

/// `○ ✓ !` or `[ ] [x] [!]`. See docs/design.md#rules — an ASCII fallback is
/// mandatory, and no meaning is ever carried by colour alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyphs {
    Unicode,
    Ascii,
}

impl Glyphs {
    /// Read from the locale, and **not** from `NO_COLOR`: whether a terminal can
    /// draw `○` and whether the user wants colour are two different questions,
    /// however often the same terminal answers no to both.
    pub fn for_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(l) if l.to_ascii_lowercase().replace('-', "").contains("utf8") => Glyphs::Unicode,
            // Including `None`: an unset locale is C, which is not UTF-8.
            _ => Glyphs::Ascii,
        }
    }

    fn mark(self, task: &Task, today: NaiveDate) -> &'static str {
        match (self, task.done, task.is_overdue(today)) {
            (Glyphs::Unicode, true, _) => "✓",
            (Glyphs::Unicode, false, true) => "!",
            (Glyphs::Unicode, false, false) => "○",
            (Glyphs::Ascii, true, _) => "[x]",
            (Glyphs::Ascii, false, true) => "[!]",
            (Glyphs::Ascii, false, false) => "[ ]",
        }
    }

    fn cursor(self) -> &'static str {
        match self {
            Glyphs::Unicode => "▌ ",
            Glyphs::Ascii => "> ",
        }
    }

    fn rule(self) -> char {
        match self {
            Glyphs::Unicode => '─',
            Glyphs::Ascii => '-',
        }
    }

    /// The frame too. A fallback that leaves box-drawing characters in the
    /// border is not a fallback — it is the same broken screen with tidier
    /// checkboxes.
    fn border(self) -> ratatui::symbols::border::Set<'static> {
        match self {
            Glyphs::Unicode => ratatui::symbols::border::PLAIN,
            Glyphs::Ascii => ratatui::symbols::border::Set {
                top_left: "+",
                top_right: "+",
                bottom_left: "+",
                bottom_right: "+",
                vertical_left: "|",
                vertical_right: "|",
                horizontal_top: "-",
                horizontal_bottom: "-",
            },
        }
    }

    /// The dash between the name and the counts, and the one between the counts.
    fn punctuation(self) -> (&'static str, &'static str) {
        match self {
            Glyphs::Unicode => ("—", "·"),
            Glyphs::Ascii => ("-", "/"),
        }
    }
}

/// How much of a row fits. A pane in a tiling layout is narrow as the normal
/// case, not the edge case — docs/tui.md#width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size {
    /// Under 34 columns: the frame goes entirely and only rows are left.
    Bare,
    /// 34–59: spacers and tags go, the date shortens.
    Narrow,
    /// 60 and up: everything.
    Wide,
}

impl Size {
    pub fn of(columns: u16) -> Self {
        match columns {
            0..=33 => Size::Bare,
            34..=59 => Size::Narrow,
            _ => Size::Wide,
        }
    }
}

/// Everything the drawing needs that is not the list itself.
#[derive(Debug, Clone, Copy)]
pub struct Render {
    pub colours: Theme,
    pub glyphs: Glyphs,
    pub today: NaiveDate,
}

/// Display columns — not bytes, and not characters. `ş` is one column and `🚀`
/// is two, and a list that counts either of them wrong draws a ragged right
/// edge. The fixtures carry both on purpose.
fn columns(text: &str) -> usize {
    Span::raw(text).width()
}

/// Cuts to `limit` columns, ending in `…`. The title is the last thing to be
/// shortened and never goes below twelve columns: a row you cannot identify is
/// not a row, it is noise.
fn shorten(text: &str, limit: usize) -> String {
    if columns(text) <= limit {
        return text.to_string();
    }
    if limit == 0 {
        return String::new();
    }

    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = columns(c.encode_utf8(&mut [0u8; 4]));
        // One column is held back for the ellipsis.
        if used + w > limit - 1 {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('…');
    out
}

/// The right-hand date column. Near dates read as words and far ones as
/// numbers; the narrow forms are what docs/tui.md#width drops to.
fn when(task: &Task, today: NaiveDate, size: Size) -> String {
    let Some(due) = task.due else {
        return String::new();
    };
    let days = (due.date - today).num_days();
    let time = due.time.map(|t| t.format("%H:%M").to_string());

    match (days, size) {
        (d, _) if d < 0 => format!("{}d ago", -d),
        (0, _) => time.unwrap_or_else(|| "today".to_string()),
        (1..=6, Size::Wide) => match time {
            Some(t) => format!("{} {t}", due.date.format("%a")),
            None => due.date.format("%a").to_string(),
        },
        (1..=6, _) => due.date.format("%a").to_string(),
        _ => due.date.format("%b %-d").to_string(),
    }
}

/// One task, laid out: mark, title, then whatever still fits on the right.
///
/// The drop order is the one in docs/tui.md#width — tags, then priority, then
/// the date shortens, then the title is cut. Tags go before dates because a date
/// is actionable and a tag is a filter.
fn task_line(task: &Task, width: usize, render: Render, size: Size) -> Line<'static> {
    let colour = task_colour(task, render.today, render.colours);
    let mark = render.glyphs.mark(task, render.today);

    let mut right: Vec<Span<'static>> = Vec::new();
    let date = when(task, render.today, size);
    if !date.is_empty() {
        right.push(Span::styled(date, Style::default().fg(render.colours.dim)));
    }
    if size == Size::Wide {
        if let Some(p) = task.priority {
            right.push(Span::styled(
                format!("  {}", p.as_str()),
                Style::default().fg(render.colours.dim),
            ));
        }
        for tag in &task.tags {
            right.push(Span::styled(
                format!("  #{}", text::plain(tag)),
                Style::default().fg(render.colours.tag),
            ));
        }
    }

    let right_width: usize = right.iter().map(|s| columns(&s.content)).sum();
    let mark_width = columns(mark) + 1;
    // Twelve columns of title, always. Everything on the right gives way first.
    let for_title = width
        .saturating_sub(mark_width + right_width + 2)
        .max(12.min(width.saturating_sub(mark_width)));

    let title = shorten(&text::plain(&task.title), for_title);
    let gap = width.saturating_sub(mark_width + columns(&title) + right_width);

    let mut spans = vec![
        Span::styled(format!("{mark} "), Style::default().fg(colour)),
        Span::styled(title, Style::default().fg(colour)),
        Span::raw(" ".repeat(gap)),
    ];
    spans.extend(right);
    Line::from(spans)
}

/// A group heading with a rule out to the right edge. In a narrow pane the eye
/// needs a horizontal anchor to find where a group starts; a bare word does not
/// give it — docs/tui.md.
fn header_line(title: &str, width: usize, render: Render) -> Line<'static> {
    let name = text::plain(title);
    let rule = width.saturating_sub(columns(&name) + 2);
    Line::from(vec![
        Span::styled(name, Style::default().fg(render.colours.accent).bold()),
        Span::styled(
            format!(" {}", render.glyphs.rule().to_string().repeat(rule)),
            Style::default().fg(render.colours.border),
        ),
    ])
}

pub fn draw(frame: &mut Frame, screen: &mut Screen, counts: Counts, render: Render) {
    let area = frame.area();
    let size = Size::of(area.width);

    // Under 34 columns the frame is two of them, which is a tenth of the pane.
    let (dash, _) = render.glyphs.punctuation();
    let block = (size > Size::Bare).then(|| {
        Block::bordered()
            .border_set(render.glyphs.border())
            .border_style(Style::default().fg(render.colours.border))
            .title(format!(
                " ratodo {dash} {} ",
                title_counts(counts, size, render.glyphs)
            ))
    });
    let inner = block.as_ref().map_or(area, |b| b.inner(area));

    // The selection marker is drawn into the row, so the width the layout gets
    // is what is left after it.
    let cursor = render.glyphs.cursor();
    let width = (inner.width as usize).saturating_sub(columns(cursor));

    let items: Vec<ListItem> = screen
        .rows
        .iter()
        .filter(|row| !(size < Size::Wide && matches!(row, Row::Spacer)))
        .map(|row| match row {
            Row::Task(t) => ListItem::new(task_line(t, width, render, size)),
            Row::Header(title) => ListItem::new(header_line(title, width, render)),
            Row::Spacer => ListItem::new(""),
        })
        .collect();

    let mut list = List::new(items)
        .style(Style::default().bg(render.colours.background))
        .highlight_symbol(cursor)
        // Background only. Setting a foreground here would repaint the selected
        // row in the accent colour, and an overdue task would stop being red the
        // moment you moved the cursor onto it — which is the one row you are
        // most likely to be looking at. docs/design.md: red only ever means late.
        .highlight_style(Style::default().bg(render.colours.selection));
    if let Some(block) = block {
        list = list.block(block);
    }

    frame.render_stateful_widget(list, area, &mut screen.state);
}

/// `5 open · 1 overdue` while it fits, `5 · 1!` when it does not — and the same
/// numbers a waybar module shows, in the same words. One source.
fn title_counts(counts: Counts, size: Size, glyphs: Glyphs) -> String {
    let (_, dot) = glyphs.punctuation();
    match size {
        Size::Wide => format!("{} open {dot} {} overdue", counts.open, counts.overdue),
        _ => format!("{} {dot} {}!", counts.open, counts.overdue),
    }
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

    fn render(colours: Theme) -> Render {
        Render {
            colours,
            glyphs: Glyphs::Unicode,
            today: today(),
        }
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
            .draw(|f| draw(f, &mut screen, counts, render(crate::theme::MOCHA)))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .chunks(width as usize)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
            .collect()
    }

    /// The layout arithmetic, pinned. Every other rendering test asks whether
    /// something is *present*, which leaves the padding free to be a column out
    /// in either direction — six mutants lived in exactly that gap. This one
    /// asks where each character actually is, and it is meant to be updated by
    /// hand when the design changes on purpose.
    #[test]
    fn the_wide_screen_exactly() {
        let mut work = capture("write the plan", today());
        work.section = Some("Work".into());
        let tasks = [capture("late @2026-08-09 #ops", today()), work];

        assert_eq!(
            rendered(62, 7, &tasks),
            [
                "┌ ratodo — 2 open · 1 overdue ───────────────────────────────┐",
                "│  OVERDUE ───────────────────────────────────────────────── │",
                "│▌ ! late                                        1d ago  #ops│",
                "│                                                            │",
                "│  Work ──────────────────────────────────────────────────── │",
                "│  ○ write the plan                                          │",
                "└────────────────────────────────────────────────────────────┘",
            ]
        );
    }

    /// Where the title gets cut, pinned to the column. The two snapshots above
    /// use short titles, which leaves the gap arithmetic free to be wrong by a
    /// column in either direction without anything noticing.
    #[test]
    fn a_cut_title_stops_two_columns_short_of_the_date() {
        let mut long = capture("a @2026-08-09 #ops", today());
        long.title = "an extremely long task title that will not fit".into();

        assert_eq!(
            rendered(50, 5, &[long]),
            [
                "┌ ratodo — 1 · 1! ───────────────────────────────┐",
                "│  OVERDUE ───────────────────────────────────── │",
                "│▌ ! an extremely long task title that w…  1d ago│",
                "│                                                │",
                "└────────────────────────────────────────────────┘",
            ]
        );
    }

    /// The same list one breakpoint down: short counts, no tags, and the blank
    /// row between the groups gone.
    #[test]
    fn the_narrow_screen_exactly() {
        let mut work = capture("write the plan", today());
        work.section = Some("Work".into());
        let tasks = [capture("late @2026-08-09 #ops", today()), work];

        assert_eq!(
            rendered(46, 7, &tasks),
            [
                "┌ ratodo — 2 · 1! ───────────────────────────┐",
                "│  OVERDUE ───────────────────────────────── │",
                "│▌ ! late                              1d ago│",
                "│  Work ──────────────────────────────────── │",
                "│  ○ write the plan                          │",
                "│                                            │",
                "└────────────────────────────────────────────┘",
            ]
        );
    }

    #[test]
    fn the_screen_shows_the_counts_the_groups_and_the_marker() {
        let tasks = tasks(&[
            "late @2026-08-01",
            "now @2026-08-10 16:00",
            "soon #ops !low",
        ]);
        let screen = rendered(62, 10, &tasks);

        assert!(
            screen[0].contains("ratodo — 3 open · 1 overdue"),
            "{screen:?}"
        );
        assert!(screen[1].starts_with("│  OVERDUE ────"), "{screen:?}");
        assert!(screen[2].contains("▌ ! late"), "{screen:?}");
        assert!(screen[2].contains("9d ago"), "{screen:?}");
        assert!(screen[4].contains("TODAY"), "{screen:?}");
        assert!(screen[5].contains("○ now"), "{screen:?}");
        assert!(screen[5].contains("16:00"), "{screen:?}");
        assert!(!screen[5].contains('▌'), "two rows drawn as selected");
        assert!(screen[7].contains("○ soon"), "{screen:?}");
        assert!(
            screen[7].contains("!low") && screen[7].contains("#ops"),
            "{screen:?}"
        );
    }

    /// The right-hand column is right-aligned, which is the whole reason it is
    /// a column: the eye reads down it.
    #[test]
    fn the_date_column_ends_at_the_right_edge() {
        let tasks = tasks(&["short @2026-08-01", "a much longer title here @2026-08-01"]);
        let screen = rendered(62, 8, &tasks);

        let ends: Vec<&str> = screen[2..4]
            .iter()
            .map(|row| row.trim_end_matches('│').trim_end())
            .collect();
        for row in &ends {
            assert!(row.ends_with("9d ago"), "{row:?}");
        }
    }

    /// The three breakpoints from docs/tui.md#width, and what each gives up.
    #[test]
    fn the_width_breakpoints() {
        let tasks = tasks(&["late @2026-08-01", "now @2026-08-10 #ops !high"]);

        let wide = rendered(62, 10, &tasks);
        assert!(wide[0].contains("3 · ") || wide[0].contains("2 open · 1 overdue"));
        assert!(wide.iter().any(|r| r.contains("#ops")), "{wide:?}");
        assert!(wide.iter().any(|r| r.contains("!high")), "{wide:?}");
        assert!(
            wide.iter()
                .any(|r| r.trim_matches(['│', ' ']).is_empty() && r.contains('│')),
            "a wide pane keeps its spacer rows: {wide:?}"
        );

        let narrow = rendered(40, 10, &tasks);
        assert!(narrow[0].contains("2 · 1!"), "{narrow:?}");
        assert!(
            !narrow.iter().any(|r| r.contains("#ops")),
            "tags survived: {narrow:?}"
        );
        assert!(!narrow.iter().any(|r| r.contains("!high")), "{narrow:?}");
        assert!(narrow.iter().any(|r| r.contains("late")), "{narrow:?}");

        let bare = rendered(30, 8, &tasks);
        assert!(!bare[0].contains('┌'), "the frame survived: {bare:?}");
        assert!(bare.iter().any(|r| r.contains("late")), "{bare:?}");
    }

    /// A row you cannot identify is not a row, it is noise. The title is the
    /// last thing shortened and never goes below twelve columns.
    #[test]
    fn a_long_title_is_cut_last_and_never_to_nothing() {
        let mut long = capture("a @2026-08-01", today());
        long.title = "an extremely long task title that will not fit anywhere".into();

        for width in [30u16, 40, 62] {
            let screen = rendered(width, 6, &[long.clone()]);
            let row = screen
                .iter()
                .find(|r| r.contains('…'))
                .unwrap_or_else(|| panic!("nothing was truncated at {width}: {screen:?}"));
            let shown: String = row
                .chars()
                .skip_while(|c| *c != 'a')
                .take_while(|c| *c != '…')
                .collect();
            assert!(shown.chars().count() >= 11, "{width}: {shown:?}");
        }
    }

    /// `ş` is one column and `🚀` is two. A layout that counts bytes or chars
    /// draws a ragged right edge, which is why both are in the fixtures.
    #[test]
    fn the_right_edge_holds_with_wide_and_accented_characters() {
        let mut task = capture("a @2026-08-01", today());
        task.title = "şğüöç 🚀 iş listesi".into();
        let screen = rendered(50, 6, &[task]);

        // Not `chars().count()`: a double-width character takes two cells and
        // ratatui leaves the second one empty, so the row's character count is
        // shorter than the row. What matters is that the frame still closes.
        let last = screen.len() - 1;
        for row in &screen[1..last] {
            assert!(row.ends_with('│'), "the right edge broke: {screen:?}");
        }
        assert!(screen[0].ends_with('┐') && screen[last].ends_with('┘'));
        assert!(screen[2].ends_with("9d ago│"), "{screen:?}");
    }

    #[test]
    fn the_ascii_fallback_replaces_every_glyph() {
        let tasks = tasks(&["late @2026-08-01", "fine"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let counts = Counts::of(&tasks, today());
        let render = Render {
            colours: crate::theme::MOCHA,
            glyphs: Glyphs::Ascii,
            today: today(),
        };

        let mut terminal = Terminal::new(TestBackend::new(62, 8)).unwrap();
        terminal
            .draw(|f| draw(f, &mut screen, counts, render))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(text.contains("> [!] late"), "{text}");
        assert!(text.contains("[ ] fine"), "{text}");
        // The strong form: not "the checkboxes are ASCII" but "the screen is".
        // A fallback that leaves the frame in box-drawing characters is the same
        // broken screen with tidier checkboxes.
        assert!(
            text.is_ascii(),
            "something non-ASCII reached the screen: {text}"
        );
    }

    #[test]
    fn the_locale_decides_the_glyphs() {
        for utf8 in ["en_US.UTF-8", "tr_TR.utf8", "C.UTF-8", "en_GB.UTF8"] {
            assert_eq!(Glyphs::for_locale(Some(utf8)), Glyphs::Unicode, "{utf8}");
        }
        for plain in ["C", "POSIX", "en_US", "en_US.ISO-8859-1"] {
            assert_eq!(Glyphs::for_locale(Some(plain)), Glyphs::Ascii, "{plain}");
        }
        assert_eq!(
            Glyphs::for_locale(None),
            Glyphs::Ascii,
            "an unset locale is C, which is not UTF-8"
        );
    }

    #[test]
    fn the_date_column_shortens_before_the_title_does() {
        let task = capture("a @2026-08-12 09:30", today());
        assert_eq!(when(&task, today(), Size::Wide), "Wed 09:30");
        assert_eq!(when(&task, today(), Size::Narrow), "Wed");

        let late = capture("a @2026-08-08", today());
        assert_eq!(when(&late, today(), Size::Wide), "2d ago");

        let far = capture("a @2026-09-20", today());
        assert_eq!(when(&far, today(), Size::Wide), "Sep 20");

        let undated = capture("a", today());
        assert_eq!(when(&undated, today(), Size::Wide), "");
    }

    #[test]
    fn shortening_counts_columns_not_bytes() {
        assert_eq!(shorten("hello", 10), "hello");
        assert_eq!(shorten("hello there", 8), "hello t…");
        assert_eq!(shorten("şşşşş", 3), "şş…");
        assert_eq!(shorten("🚀🚀🚀", 5), "🚀🚀…");
        assert_eq!(shorten("anything", 0), "");
        assert_eq!(columns("şğüöç"), 5);
        assert_eq!(columns("🚀"), 2);
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
            .draw(|f| draw(f, &mut screen, counts, render(crate::theme::MOCHA)))
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
            .draw(|f| draw(f, &mut screen, counts, render(colours)))
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
            .draw(|f| draw(f, &mut screen, counts, render(plain)))
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
