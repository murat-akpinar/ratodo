//! ratatui drawing. See docs/tui.md.

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph};

use crate::agenda::{Counts, Group, Kind};
use crate::model::{Priority, Task};
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
    Toggle,
    /// `a` `o` — open the input on an empty line. `o` because a vim user will
    /// reach for it to open a new one.
    Add,
    /// `⏎` — the same input, pre-filled with the selected task.
    Change,
    /// `h` `l` `z` — collapse or open the group under the cursor.
    Fold(Fold),
    /// `d` — immediately, with `u` to take it back.
    Delete,
    /// `u` — put the last change back.
    Undo,
    /// Hand the terminal to `$EDITOR`. The escape hatch for everything the
    /// tool cannot do — docs/product.md#product-decisions.
    Edit,
    Reload,
    /// Opens the key help, and closes it again — the only overlay in the
    /// product, and the only place a popup is the right answer.
    Help,
    /// `esc`: closes the overlay, and does nothing at all otherwise. It must
    /// never quit — somebody pressing it out of habit keeps their pane.
    Close,
    /// A key that is bound to nothing on purpose but still owes an answer.
    /// Silence reads as a broken program — docs/tui.md#deliberately-unbound.
    Say(&'static str),
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
        // `gg` in vim is two keys; here the first one already did it and the
        // second is a harmless no-op, so there is no pending-key state to hold.
        KeyCode::Char('g') => Action::Top,
        KeyCode::Char('G') => Action::Bottom,
        KeyCode::Char('d') if ctrl => Action::Move(10),
        KeyCode::Char('u') if ctrl => Action::Move(-10),
        KeyCode::Char('d') => Action::Delete,
        KeyCode::Char('u') => Action::Undo,
        KeyCode::Char(' ') => Action::Toggle,
        KeyCode::Char('a') | KeyCode::Char('o') => Action::Add,
        KeyCode::Enter => Action::Change,
        KeyCode::Char('h') | KeyCode::Left => Action::Fold(Fold::Close),
        KeyCode::Char('l') | KeyCode::Right => Action::Fold(Fold::Open),
        // `z` is the vim fold prefix, and here it is the whole of it.
        KeyCode::Char('z') => Action::Fold(Fold::Toggle),
        KeyCode::Char('e') => Action::Edit,
        KeyCode::Char('r') => Action::Reload,
        KeyCode::Char('?') => Action::Help,
        // Bound to an answer rather than to nothing. A key that appears broken
        // is worse than one that explains itself.
        // `esc` never quits, but while the overlay is up it is the obvious way
        // to put it down, so the loop reads it as a second `?`.
        KeyCode::Esc => Action::Close,
        KeyCode::Char(':') => Action::Say("no command mode - ? for keys"),
        KeyCode::Char('/') => Action::Say("search comes in v2"),
        _ => Action::Ignore,
    }
}

/// What `h`, `l` and `z` ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fold {
    Close,
    Open,
    Toggle,
}

/// The other mode, and the whole of it: a line being typed, and what it is for.
///
/// It only ever exists because `a`, `o` or `⏎` made it — docs/tui.md#two-modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub text: String,
    /// Where the next character goes, as a byte index into `text`. Always on a
    /// char boundary — every move steps by a whole `char`.
    pub at: usize,
    /// The raw line being rewritten. `None` is a new task.
    pub editing: Option<String>,
}

impl Input {
    /// The only way to build one: the caret starts at the end of `text`, which
    /// is where a retype begins.
    pub fn new(text: String, editing: Option<String>) -> Self {
        Input {
            at: text.len(),
            text,
            editing,
        }
    }

    pub fn adding() -> Self {
        Input::new(String::new(), None)
    }

    /// Pre-filled with the task's text as it stands in the file, so an edit
    /// starts from what is actually written there rather than from our reading
    /// of it.
    pub fn editing(task: &Task) -> Self {
        Input::new(task.body().to_string(), Some(task.raw.clone()))
    }

    pub fn insert(&mut self, c: char) {
        self.text.insert(self.at, c);
        self.at += c.len_utf8();
    }

    /// Backspace: the character *before* the caret.
    pub fn back(&mut self) {
        if let Some(c) = self.text[..self.at].chars().next_back() {
            self.at -= c.len_utf8();
            self.text.remove(self.at);
        }
    }

    /// Delete: the character *under* the caret; the caret does not move.
    pub fn delete(&mut self) {
        if self.at < self.text.len() {
            self.text.remove(self.at);
        }
    }

    pub fn left(&mut self) {
        if let Some(c) = self.text[..self.at].chars().next_back() {
            self.at -= c.len_utf8();
        }
    }

    pub fn right(&mut self) {
        if let Some(c) = self.text[self.at..].chars().next() {
            self.at += c.len_utf8();
        }
    }

    pub fn home(&mut self) {
        self.at = 0;
    }

    pub fn end(&mut self) {
        self.at = self.text.len();
    }
}

/// What a keypress means while the input is open. A second, much smaller keymap
/// rather than a branch inside `action`: it is what makes "nothing else can open
/// it" true by construction — an `a` in here is a letter, not a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    Char(char),
    Back,
    Delete,
    Left,
    Right,
    Home,
    End,
    Save,
    /// `esc` **and** `ctrl-c`. Somebody half-way through a sentence who reaches
    /// for the universal "stop that" key loses the sentence, not the session.
    Cancel,
    Ignore,
}

pub fn typing(key: KeyEvent) -> Typed {
    if key.kind == KeyEventKind::Release {
        return Typed::Ignore;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char('c') if ctrl => Typed::Cancel,
        KeyCode::Esc => Typed::Cancel,
        KeyCode::Enter => Typed::Save,
        KeyCode::Backspace => Typed::Back,
        KeyCode::Delete => Typed::Delete,
        // A field you can only append to is not a field. The caret moves, and
        // the line scrolls to keep up — docs/tui.md#adding.
        KeyCode::Left => Typed::Left,
        KeyCode::Right => Typed::Right,
        KeyCode::Home => Typed::Home,
        KeyCode::End => Typed::End,
        // Every other modified key is left alone: `ctrl-v`, `alt-f` and the rest
        // mean things in a terminal that a one-line field has no business
        // claiming, and a stray control character in a task title is a file the
        // user cannot read back.
        KeyCode::Char(c) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => Typed::Char(c),
        _ => Typed::Ignore,
    }
}

/// One line of the list. Only a `Task` can hold the selection; the rest is
/// scenery the cursor moves over.
///
/// Owned rather than borrowed, so that a reload can swap the whole list out
/// without the screen still pointing into the document it replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Header {
        title: String,
        /// How many tasks are hidden under it, when it is folded. A collapsed
        /// group that does not say how much it is hiding is a dead end —
        /// docs/tui.md.
        hidden: Option<usize>,
    },
    Task(Task),
    Spacer,
}

impl Row {
    fn header(title: &str) -> Self {
        Row::Header {
            title: title.to_string(),
            hidden: None,
        }
    }
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
            // `OVERDUE` is ours and `## Work` is the user's, and until now they
            // were the same bold word plus the same rule. The markdown marker
            // the heading already carries in the file is what tells them apart
            // — it costs no colour and no third level of hierarchy, and it says
            // "this line is yours" to anyone who has seen the file
            // — docs/tui.md#main-screen.
            match group.kind {
                Kind::Section(_) => out.push(Row::header(&format!("## {title}"))),
                _ => out.push(Row::header(title)),
            }
        }
        out.extend(group.tasks.iter().map(|t| Row::Task((*t).clone())));
    }
    out
}

#[derive(Default)]
pub struct Screen {
    /// Every row the document produces, folded or not.
    all: Vec<Row>,
    /// The titles of the groups that are collapsed. Kept across a reload on
    /// purpose: a group you folded stays folded when `ratodo add` fires the
    /// watcher, or folding would undo itself every time anything touched the
    /// file.
    ///
    /// Keyed by title, so a file with the same heading twice folds both at once.
    /// That is the price of surviving a reload — an index would not — and two
    /// `## Work` headings are the user's own doing.
    folded: std::collections::HashSet<String>,
    /// What is on screen — `all` with the folded groups taken out. Derived, and
    /// the thing `state` indexes into.
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

    /// Swaps the list for a freshly read one and leaves the cursor on the task
    /// it was on — by identity, not by row, so `ratodo add` in another pane
    /// pushing four rows in above it does not move it, and neither does the task
    /// itself changing. A tool that loses your place while you are reading it is
    /// not usable as a side pane — docs/tui.md#write-conflict.
    pub fn replace(&mut self, rows: Vec<Row>) {
        self.all = rows;
        self.refresh();
    }

    /// Rebuilds the visible rows from `all` and the folded set, and puts the
    /// cursor back on the task it was on.
    ///
    /// When that task is no longer visible — because the group it lived in was
    /// just folded — the cursor holds its **position** instead and snaps to the
    /// nearest task. Sending it to the top of the list would be the one thing a
    /// side pane must not do.
    fn refresh(&mut self) {
        let was_on = self.task().map(Task::identity);
        let was_at = self.state.selected().unwrap_or(0);

        self.rows = Vec::with_capacity(self.all.len());
        let mut skipping: Option<usize> = None;

        for row in &self.all {
            match row {
                Row::Header { title, .. } if self.folded.contains(title) => {
                    skipping = Some(0);
                    self.rows.push(Row::Header {
                        title: title.clone(),
                        hidden: Some(0),
                    });
                }
                Row::Header { title, .. } => {
                    skipping = None;
                    self.rows.push(Row::header(title));
                }
                Row::Task(t) => match skipping {
                    Some(_) => {
                        // Count it against the header that is hiding it.
                        if let Some(Row::Header {
                            hidden: Some(n), ..
                        }) = self.rows.last_mut()
                        {
                            *n += 1;
                        }
                    }
                    None => self.rows.push(Row::Task(t.clone())),
                },
                Row::Spacer => self.rows.push(Row::Spacer),
            }
        }

        // The nearest match, not the first. Two tasks with the same title in the
        // same section share an identity, and the one that was under the cursor
        // is the one closest to where the cursor was — picking the first would
        // send it up the screen every time somebody has two `call the bank`s.
        let kept = was_on.and_then(|id| {
            self.rows
                .iter()
                .enumerate()
                .filter(|(_, r)| matches!(r, Row::Task(t) if t.identity() == id))
                .map(|(i, _)| i)
                .min_by_key(|i| i.abs_diff(was_at))
        });
        let near = (was_at..self.rows.len())
            .find(|&i| self.is_selectable(i))
            .or_else(|| {
                (0..was_at.min(self.rows.len()))
                    .rev()
                    .find(|&i| self.is_selectable(i))
            });

        self.state.select(kept.or(near));
    }

    /// The heading the cursor is under — or sitting on, when the group is
    /// folded. A run of tasks above the file's first `##` has no header to
    /// collapse into and gets `None`.
    fn group_at_cursor(&self) -> Option<String> {
        let at = self.state.selected()?;
        self.rows[..=at.min(self.rows.len().saturating_sub(1))]
            .iter()
            .rev()
            .find_map(|r| match r {
                Row::Header { title, .. } => Some(title.clone()),
                _ => None,
            })
    }

    /// `h` collapses, `l` opens, `z` does whichever is the opposite of now —
    /// the muscle memory `lf`, `ranger` and `yazi` arrive with.
    ///
    /// `None` means it happened. `Some(complaint)` means it did not and here is
    /// what to put on the bottom line: silence would read as a key that does not
    /// work. The wording lives here rather than in the event loop so that it can
    /// be tested without a terminal.
    pub fn fold(&mut self, want: Fold) -> Option<&'static str> {
        let Some(title) = self.group_at_cursor() else {
            return Some("no group to fold here");
        };
        let folded = self.folded.contains(&title);
        let should = match want {
            Fold::Close => true,
            Fold::Open => false,
            Fold::Toggle => !folded,
        };
        if should == folded {
            return Some(if folded {
                "already folded"
            } else {
                "nothing folded here"
            });
        }

        if should {
            self.folded.insert(title.clone());
            self.refresh();
            // Land on the header that just swallowed the group. Anywhere else
            // and there is no way back: `l` needs a cursor on the thing it opens.
            if let Some(at) = self
                .rows
                .iter()
                .position(|r| matches!(r, Row::Header { title: t, hidden: Some(_) } if *t == title))
            {
                self.state.select(Some(at));
            }
        } else {
            self.folded.remove(&title);
            // `refresh` already steps inside: the header stops being selectable
            // the moment it opens, so holding the position lands on the first
            // task of the group that was just revealed.
            self.refresh();
        }
        None
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

    /// Rewrites the selected row and **nothing else**. A task ticked done marks
    /// in place; it does not jump to the end of its group until the next
    /// reload. Watching a row you just touched fly somewhere else is
    /// disorienting — docs/tui.md, the first of the side-pane rules.
    pub fn update_selected(&mut self, task: Task) {
        if let Some(Row::Task(row)) = self.state.selected().and_then(|i| self.rows.get_mut(i)) {
            *row = task;
        }
    }

    /// A task, or the header of a folded group.
    ///
    /// A collapsed group is one thing on screen and behaves like one: the cursor
    /// lands on it, which is also the only way `l` can ever open it again. This
    /// is what `lf` and `ranger` do with a closed directory, and the reason `h`
    /// and `l` are the keys.
    fn is_selectable(&self, i: usize) -> bool {
        matches!(
            self.rows.get(i),
            Some(Row::Task(_))
                | Some(Row::Header {
                    hidden: Some(_),
                    ..
                })
        )
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
                (at + 1..self.rows.len()).find(|&i| self.is_selectable(i))
            } else {
                (0..at).rev().find(|&i| self.is_selectable(i))
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
        if let Some(i) = order.find(|&i| self.is_selectable(i)) {
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

    /// Every mark in a set is the same width, and the column arithmetic is done
    /// before there is a task to ask: `○` is one column and `[ ]` is three. A
    /// test pins this against `mark` so the two cannot drift — get it wrong and
    /// the ASCII screen budgets two columns it does not have, which shows up as
    /// tags silently disappearing, not as a broken frame.
    fn mark_width(self) -> usize {
        match self {
            Glyphs::Unicode => 1,
            Glyphs::Ascii => 3,
        }
    }

    fn cursor(self) -> &'static str {
        match self {
            Glyphs::Unicode => "▌ ",
            Glyphs::Ascii => "> ",
        }
    }

    /// The `⏎` in the hints and on the input line. It is a key name, so it goes
    /// the way every other glyph goes when the locale is not UTF-8.
    fn enter(self) -> &'static str {
        match self {
            Glyphs::Unicode => "⏎",
            Glyphs::Ascii => "ret",
        }
    }

    /// The arrow keys, as the help overlay names them. Two words in ASCII: a
    /// key help that shows a key the terminal cannot draw is the one screen
    /// where the fallback matters most.
    fn arrows(self) -> &'static str {
        match self {
            Glyphs::Unicode => "↓ ↑",
            Glyphs::Ascii => "down up",
        }
    }

    /// What a cut title ends in. Three columns in ASCII rather than one, which
    /// `shorten` has to hold back rather than assume.
    fn ellipsis(self) -> &'static str {
        match self {
            Glyphs::Unicode => "…",
            Glyphs::Ascii => "...",
        }
    }

    /// The tick on its own, for the count in a narrow title bar.
    fn tick(self) -> &'static str {
        match self {
            Glyphs::Unicode => "✓",
            Glyphs::Ascii => "x",
        }
    }

    /// Filled and empty, for the progress bar.
    fn bar(self) -> (&'static str, &'static str) {
        match self {
            Glyphs::Unicode => ("▰", "▱"),
            Glyphs::Ascii => ("#", "-"),
        }
    }

    /// The bar the typed text sits behind.
    fn field(self) -> &'static str {
        match self {
            Glyphs::Unicode => "▏",
            Glyphs::Ascii => "|",
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
pub struct Render<'a> {
    pub colours: Theme,
    pub glyphs: Glyphs,
    pub today: NaiveDate,
    /// Shown on the empty screen. The promise of this product is that the file
    /// is yours, so you get told where it is on day one.
    pub path: &'a str,
}

/// Display columns — not bytes, and not characters. `ş` is one column and `🚀`
/// is two, and a list that counts either of them wrong draws a ragged right
/// edge. The fixtures carry both on purpose.
fn columns(text: &str) -> usize {
    Span::raw(text).width()
}

/// Cuts to `limit` columns, ending in the ellipsis. The title is the last thing
/// to be shortened and never goes below twelve columns: a row you cannot
/// identify is not a row, it is noise.
///
/// The marker is a glyph like any other — `...` costs three columns where `…`
/// costs one, so what is held back is measured rather than assumed.
fn shorten(text: &str, limit: usize, glyphs: Glyphs) -> String {
    if columns(text) <= limit {
        return text.to_string();
    }
    let ellipsis = glyphs.ellipsis();
    // Not even room for the marker, so there is nothing to mark: a cell of dots
    // says less than a cell of the title does.
    if limit <= columns(ellipsis) {
        return lead(text, limit);
    }
    let mut out = lead(text, limit - columns(ellipsis));
    out.push_str(ellipsis);
    out
}

/// The last `limit` columns of a string. The input field scrolls rather than
/// truncating: what you are typing is under the caret, and a capture box that
/// hides it is not a capture box.
fn tail(text: &str, limit: usize) -> String {
    if columns(text) <= limit {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars().rev() {
        let w = columns(c.encode_utf8(&mut [0u8; 4]));
        if used + w > limit {
            break;
        }
        out.insert(0, c);
        used += w;
    }
    out
}

/// The first `limit` columns, and no ellipsis: the other half of the scrolling
/// input field — what sits after the caret, in whatever room is left.
fn lead(text: &str, limit: usize) -> String {
    if columns(text) <= limit {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = columns(c.encode_utf8(&mut [0u8; 4]));
        if used + w > limit {
            break;
        }
        out.push(c);
        used += w;
    }
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
        // Lateness is a claim about work still owed. A ticked line saying "2d
        // ago" contradicts the tick — and the counts, which already leave
        // finished work out of `overdue`. It falls through to the plain date,
        // which is still true: that is when it was for.
        (d, _) if d < 0 && !task.done => format!("{}d ago", -d),
        (0, _) => time.unwrap_or_else(|| "today".to_string()),
        (1..=6, Size::Wide) => match time {
            Some(t) => format!("{} {t}", due.date.format("%a")),
            None => due.date.format("%a").to_string(),
        },
        (1..=6, _) => due.date.format("%a").to_string(),
        _ => due.date.format("%b %-d").to_string(),
    }
}

/// The gap between two columns. Two, because one reads as a typo and three
/// pulls the eye across a distance it does not need to travel.
const GAP: usize = 2;

/// Where the columns sit. Every entry is as wide as the widest thing in it, so
/// a list with no priorities spends no width on a priority column, and the eye
/// gets a straight edge to read down — docs/tui.md#width.
///
/// Computed once per draw from the **whole** list rather than the viewport: a
/// column that changes width as you scroll past a long title is not a column.
///
/// `Columns::default()` — every field zero — is the narrow-pane signal, and it
/// means "no columns, push the right-hand block to the edge the old way".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Columns {
    /// Excludes the mark; the date and priority widths include their own gap.
    title: usize,
    date: usize,
    prio: usize,
}

/// The fourth breakpoint, in columns of **row** — the frame and the selection
/// marker are already off it, so it is four short of the terminal.
///
/// Columns have to be paid for: an empty priority column costs every row its
/// width whether or not anything on screen uses one. Below this there is not
/// enough row to buy alignment with, the old right-aligned block packs more
/// onto it, and packing wins when there is not much to pack into.
const COLUMNS_AT: usize = 76;

impl Columns {
    /// `size` is not consulted for the breakpoint: `COLUMNS_AT` is above the
    /// widest pane either of the narrow sizes can be, so the width settles it
    /// and a second test on the same fact would be a second thing to keep true.
    fn of(rows: &[Row], width: usize, render: Render<'_>, size: Size) -> Self {
        if width < COLUMNS_AT {
            return Self::default();
        }
        let today = render.today;
        let tasks = || {
            rows.iter().filter_map(|r| match r {
                Row::Task(t) => Some(t),
                _ => None,
            })
        };
        let title = tasks()
            .map(|t| columns(&text::plain(&t.title)))
            .max()
            .unwrap_or(0);
        let date = tasks()
            .map(|t| columns(&when(t, today, size)))
            .max()
            .unwrap_or(0);
        let prio = tasks()
            .map(|t| t.priority.map_or(0, |p| columns(p.as_str())))
            .max()
            .unwrap_or(0);
        // An empty column takes no gap either, or a list nobody tagged would
        // still be drawn around a tag column that is not there.
        let with_gap = |w: usize| if w == 0 { 0 } else { w + GAP };
        let (date, prio) = (with_gap(date), with_gap(prio));

        // The mark and its space — three columns wider under the ASCII
        // fallback, and budgeting the Unicode figure there spends width the row
        // does not have.
        let mark = render.glyphs.mark_width() + 1;
        // Tags get no reservation. They are last and ragged, so nothing lines
        // up after them, and reserving the widest row's worth would cut every
        // title to pay for tags most rows do not have — the exact inversion of
        // the drop order in docs/tui.md#width. task_line spends what is left.
        let room = width.saturating_sub(mark + date + prio);
        Self {
            title: title.min(room).max(12.min(width.saturating_sub(mark))),
            date,
            prio,
        }
    }
}

/// One task, laid out: mark, title, then the date, priority and tags. Past
/// `COLUMNS_AT` those last three are fixed and left-aligned, so the dates line
/// up under each other; below it there is not enough width to spend on
/// alignment and the right-hand block is pushed to the edge instead.
///
/// The drop order is the one in docs/tui.md#width — tags, then priority, then
/// the date shortens, then the title is cut. Tags go before dates because a date
/// is actionable and a tag is a filter.
fn task_line(
    task: &Task,
    width: usize,
    cols: Columns,
    render: Render<'_>,
    size: Size,
) -> Line<'static> {
    let colour = task_colour(task, render.today, render.colours);
    let mark = render.glyphs.mark(task, render.today);
    let mark_width = columns(mark) + 1;

    let date = when(task, render.today, size);
    let prio = task.priority.map(|p| p.as_str().to_string());

    let mut right: Vec<Span<'static>> = Vec::new();
    // A column pads to its own width; without one the entry carries its gap.
    let mut push = |text: String, column: usize, style: Style| {
        if column == 0 {
            if !text.is_empty() {
                right.push(Span::styled(format!("{}{text}", " ".repeat(GAP)), style));
            }
            return;
        }
        right.push(Span::raw(" ".repeat(GAP)));
        let pad = column.saturating_sub(columns(&text) + GAP);
        right.push(Span::styled(text, style));
        right.push(Span::raw(" ".repeat(pad)));
    };

    // No guard on either: `push` draws nothing for an empty entry outside a
    // column, and pads the full width for one inside it, which is exactly what
    // an undated task in a dated list needs.
    let dim = Style::default().fg(render.colours.dim);
    // The date is where the lateness actually is, and it was the one field
    // saying so in grey while the title beside it went red. It borrows the row's
    // own colour on the two rows where it means something — late, and due today
    // — and stays dim everywhere else: `Fri` is a fact, not a warning, and a
    // finished task is neither. No new theme role: `overdue` and `today` are
    // already the two the title uses — docs/tui.md#main-screen.
    let pressing = task.is_overdue(render.today)
        || (!task.done && task.due.is_some_and(|d| d.date == render.today));
    let date_style = if pressing {
        Style::default().fg(colour)
    } else {
        dim
    };
    push(date, cols.date, date_style);
    if size == Size::Wide {
        // `!high` is the one field the user typed to mean *urgent*, and dim
        // beside the tags is the screen saying it back in a whisper. Weight
        // rather than a twelfth theme colour: nothing on this screen may be
        // carried by colour alone — docs/design.md#rules. `!med` and `!low` stay
        // where they were, or three loud rows teach nothing about which is which,
        // and a ticked task is not urgent however it was filed.
        let urgent = task.priority == Some(Priority::High) && !task.done;
        let style = if urgent {
            Style::default().fg(colour).bold()
        } else {
            dim
        };
        push(prio.unwrap_or_default(), cols.prio, style);
        // What is left of the row after the columns. A tag that does not fit is
        // dropped whole rather than cut: `#hea…` is not a filter, it is a
        // riddle. Tags go before the title — docs/tui.md#width.
        let mut room = match cols.title {
            0 => usize::MAX,
            title => width.saturating_sub(mark_width + title + cols.date + cols.prio),
        };
        for tag in &task.tags {
            let span = format!("  #{}", text::plain(tag));
            let Some(left) = room.checked_sub(columns(&span)) else {
                break;
            };
            room = left;
            right.push(Span::styled(span, Style::default().fg(render.colours.tag)));
        }
    }

    let (for_title, pad_to) = if cols.title > 0 {
        (cols.title, cols.title)
    } else {
        // No `+ GAP` here: without a column to pad to, the first entry carries
        // its own gap, and subtracting it a second time cuts the title two
        // columns short of where docs/tui.md#width says it stops.
        let right_width: usize = right.iter().map(|s| columns(&s.content)).sum();
        let room = width
            .saturating_sub(mark_width + right_width)
            .max(12.min(width.saturating_sub(mark_width)));
        (room, width.saturating_sub(mark_width + right_width))
    };

    let title = shorten(&text::plain(&task.title), for_title, render.glyphs);
    let gap = pad_to.saturating_sub(columns(&title));

    let mut spans = vec![
        Span::styled(format!("{mark} "), Style::default().fg(colour)),
        Span::styled(title, Style::default().fg(colour)),
        Span::raw(" ".repeat(gap)),
    ];
    spans.extend(right);
    Line::from(spans)
}

/// A group heading with a rule after it. In a narrow pane the eye needs a
/// horizontal anchor to find where a group starts; a bare word does not give it
/// — docs/tui.md.
///
/// Where the rule **stops** is the title column once there is one: past
/// `COLUMNS_AT` a rule to the right edge is the heaviest thing on the screen
/// and says nothing, while one that ends with the titles draws the column
/// instead. Below it there is no column to end at, so it runs to the edge.
fn header_line(
    title: &str,
    hidden: Option<usize>,
    width: usize,
    cols: Columns,
    render: Render<'_>,
) -> Line<'static> {
    // A collapsed group says how much it is hiding and which key opens it.
    // One that does not is a dead end — docs/tui.md.
    let name = match hidden {
        Some(n) => format!("{} ({n})", text::plain(title)),
        None => text::plain(title),
    };
    let tail = if hidden.is_some() { " l" } else { "" };

    // Plus the mark the tasks below carry, so the rule ends with their titles
    // rather than short of them — and it is `[ ]` under the ASCII fallback,
    // which is two columns more than `○`.
    let end = match cols.title {
        0 => width,
        title => (title + render.glyphs.mark_width() + 1).min(width),
    };
    let rule = end.saturating_sub(columns(&name) + columns(tail) + 2);
    Line::from(vec![
        Span::styled(name, Style::default().fg(render.colours.accent).bold()),
        Span::styled(
            format!(" {}", render.glyphs.rule().to_string().repeat(rule)),
            Style::default().fg(render.colours.border),
        ),
        Span::styled(tail, Style::default().fg(render.colours.dim)),
    ])
}

/// What the one reserved line under the frame is saying. One line, and it is
/// the only part of the screen that changes shape — docs/tui.md#the-bottom-line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// The default: which keys do what.
    Hints,
    /// Just after an action.
    Said(String),
    /// A write that was refused. The one thing worth interrupting for.
    Warned(String),
}

impl Notice {
    fn line(&self, size: Size, height: u16, glyphs: Glyphs, colours: Theme) -> Line<'static> {
        let (text, colour) = match self {
            // Only the keys that do something. A hint bar advertising a key that
            // is not implemented yet is a worse lie than no hint bar.
            Notice::Hints if height < 10 => (" ?".to_string(), colours.dim),
            // Six keys, not the whole keymap: the bar has to fit the narrowest
            // pane that still counts as wide, which is sixty columns. `d` and
            // `e` gave up their slots to the capture keys when those arrived —
            // adding a task is what the tool is for, and `?` lists the rest.
            Notice::Hints if size == Size::Wide => (
                format!(
                    " j k move   spc done   a add   {} edit   ? keys   q quit",
                    glyphs.enter()
                ),
                colours.dim,
            ),
            Notice::Hints => (" j k  spc  a  d  e  ?  q".to_string(), colours.dim),
            Notice::Said(text) => (format!(" {text}"), colours.dim),
            Notice::Warned(text) => {
                let mark = match glyphs {
                    Glyphs::Unicode => "⚠",
                    Glyphs::Ascii => "!",
                };
                (format!(" {mark} {text}"), colours.overdue)
            }
        };
        Line::from(Span::styled(text, Style::default().fg(colour)))
    }
}

/// The input field, and under it what the typed text will actually become.
///
/// The preview is the most valuable ten lines in the TUI: `@thu` resolving to a
/// real date while you type teaches the syntax without anyone opening
/// docs/format.md, catches the typo before it reaches the file, and proves the
/// shorthand did what you meant. Nothing parseable leaves it empty rather than
/// showing an error — plain text is a perfectly good task.
///
/// Returns the column the cursor belongs in as well, because it is the same
/// arithmetic.
fn input_lines(input: &Input, width: usize, render: Render<'_>) -> (Vec<Line<'static>>, usize) {
    let dim = Style::default().fg(render.colours.dim);
    let head = format!(
        " {} {}",
        if input.editing.is_some() {
            "edit"
        } else {
            "add"
        },
        render.glyphs.field()
    );
    // The window is anchored on the caret, not on the end of the line: what is
    // before it fills the field from the right, and whatever room is left shows
    // what comes after.
    let room = width.saturating_sub(columns(&head));
    let before = tail(&input.text[..input.at], room);
    let after = lead(
        &input.text[input.at..],
        room.saturating_sub(columns(&before)),
    );
    let at = columns(&head) + columns(&before);

    // What is on screen, as byte offsets into the whole line, so the colouring
    // below can be asked about the text rather than about the window.
    let (from, to) = (input.at - before.len(), input.at + after.len());
    let plain = Style::default().fg(render.colours.foreground);

    // Coloured while it is typed, and coloured by what the parser **took**: a
    // `@notaday` stays plain here exactly as it will in the file. This is the
    // preview's job done a second way — the preview says what it understood, and
    // this says where — docs/tui.md#adding.
    let mut spans = vec![Span::styled(head, dim)];
    let mut cut = from;
    for (word, part) in crate::capture::parts(&input.text, render.today) {
        if part == crate::capture::Part::Text || word.end <= from || word.start >= to {
            continue;
        }
        let (start, end) = (word.start.max(from), word.end.min(to));
        if start > cut {
            spans.push(Span::styled(input.text[cut..start].to_string(), plain));
        }
        spans.push(Span::styled(
            input.text[start..end].to_string(),
            match part {
                crate::capture::Part::Tag => Style::default().fg(render.colours.tag),
                crate::capture::Part::Priority => plain.bold(),
                _ => Style::default().fg(render.colours.accent),
            },
        ));
        cut = end;
    }
    if cut < to {
        spans.push(Span::styled(input.text[cut..to].to_string(), plain));
    }
    let field = Line::from(spans);

    let parsed = crate::capture::capture(&input.text, render.today);
    let (_, dot) = render.glyphs.punctuation();
    let preview = Line::from(Span::styled(
        format!("      {}", crate::text::fields(&parsed, render.today, dot)),
        Style::default().fg(render.colours.accent),
    ));

    (vec![field, preview], at.min(width.saturating_sub(1)))
}

pub fn draw(
    frame: &mut Frame,
    screen: &mut Screen,
    counts: Counts,
    render: Render<'_>,
    notice: &Notice,
    helping: bool,
    input: Option<&Input>,
) {
    let whole = frame.area();
    // One row held back, always. The screen never changes shape now: the input
    // opens as a box over the middle of the list rather than growing this line
    // — docs/decisions.md#reversed.
    //
    // Written with `Rect::new` rather than struct update syntax: a mutation that
    // drops the `height:` field from `Rect { height: 1, ..whole }` produces a
    // rectangle that is clipped back to the same one row, so it is a change no
    // test can ever object to. Positional arguments leave nothing to drop.
    //
    // Never the whole pane: somebody dragging a splitter past the point of
    // usefulness keeps a row of list, because a lone hint bar helps less than a
    // lone task does.
    let reserved = 1.min(whole.height.saturating_sub(1));
    let area = Rect::new(whole.x, whole.y, whole.width, whole.height - reserved);
    let bottom = (reserved > 0).then(|| {
        Rect::new(
            whole.x,
            whole.y + whole.height - reserved,
            whole.width,
            reserved,
        )
    });
    let size = Size::of(area.width);

    // Under 34 columns the frame is two of them, which is a tenth of the pane.
    let (dash, _) = render.glyphs.punctuation();
    let block = (size > Size::Bare).then(|| {
        let name = format!(
            " ratodo {dash} {} ",
            title_counts(counts, size, render.glyphs)
        );
        let bar = (size == Size::Wide)
            .then(|| progress(counts, area.width as usize, columns(&name), render))
            .flatten();

        let block = Block::bordered()
            .border_set(render.glyphs.border())
            .border_style(Style::default().fg(render.colours.border))
            .title(name);
        match bar {
            Some(bar) => block.title(bar.right_aligned()),
            None => block,
        }
    });
    let inner = block.as_ref().map_or(area, |b| b.inner(area));

    // The selection marker is drawn into the row, so the width the layout gets
    // is what is left after it.
    let cursor = render.glyphs.cursor();
    let width = (inner.width as usize).saturating_sub(columns(cursor));
    let cols = Columns::of(&screen.rows, width, render, size);

    if screen.rows.iter().all(|r| !matches!(r, Row::Task(_))) {
        empty(frame, area, block, render);
    } else {
        let items: Vec<ListItem> = screen
            .rows
            .iter()
            .filter(|row| !(size < Size::Wide && matches!(row, Row::Spacer)))
            .map(|row| match row {
                Row::Task(t) => ListItem::new(task_line(t, width, cols, render, size)),
                Row::Header { title, hidden } => {
                    ListItem::new(header_line(title, *hidden, width, cols, render))
                }
                Row::Spacer => ListItem::new(""),
            })
            .collect();

        let mut list = List::new(items)
            .style(Style::default().bg(render.colours.background))
            .highlight_symbol(cursor)
            // Background only. Setting a foreground here would repaint the
            // selected row in the accent colour, and an overdue task would stop
            // being red the moment you moved the cursor onto it — which is the
            // one row you are most likely to be looking at. docs/design.md: red
            // only ever means late.
            .highlight_style(Style::default().bg(render.colours.selection));
        if let Some(block) = block {
            list = list.block(block);
        }

        frame.render_stateful_widget(list, area, &mut screen.state);
    }

    if helping {
        help(frame, area, render);
    }
    if let Some(input) = input {
        input_box(frame, area, input, render);
    }

    let Some(bottom) = bottom else { return };
    // While the input is open the line names the two keys that end it, and
    // nothing else: the list keys under it are letters until `esc`, so
    // advertising them there would be a lie.
    let line = match input {
        Some(_) => Line::from(Span::styled(
            format!(" {} save   esc cancel", render.glyphs.enter()),
            Style::default().fg(render.colours.dim),
        )),
        None => notice.line(size, whole.height, render.glyphs, render.colours),
    };

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(render.colours.background)),
        bottom,
    );
}

/// The input, in a box over the middle of the list.
///
/// It lived on the bottom line until it did not: in a pane in the corner of a
/// tiling layout that line is at the bottom of the screen, and looking down
/// there to type is exactly the interruption the fixed line existed to avoid.
/// The box lands where the eye already is — docs/decisions.md#reversed.
///
/// Two rows inside it: the field, and the live parse under it. The keys that end
/// it stay on the bottom line, so they are in the same place whether or not the
/// box is open.
fn input_box(frame: &mut Frame, area: Rect, input: &Input, render: Render<'_>) {
    // Four columns short of the pane at most, so the frame underneath stays
    // visible on both sides. A box flush with the border reads as the screen
    // having changed shape, which is the one thing it must not do.
    let width = 70.min(area.width.saturating_sub(4));
    // Border, field, preview — and the preview is what goes first, exactly as it
    // did on the bottom line. Under three rows there is nothing to draw at all:
    // two of them are border and the field would have nowhere to sit, so the
    // pane keeps its tasks and the bottom line still names the keys.
    let height = 4.min(area.height);
    if height < 3 {
        return;
    }
    let box_area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );

    let (lines, at) = input_lines(input, (width as usize).saturating_sub(2), render);

    frame.render_widget(Clear, box_area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .border_set(render.glyphs.border())
                .border_style(Style::default().fg(render.colours.accent))
                .style(Style::default().bg(render.colours.background)),
        ),
        box_area,
    );
    // The terminal's own cursor, not a drawn block: it blinks the way every
    // other text field the user has ever typed into does, and it costs a line.
    frame.set_cursor_position((box_area.x + 1 + at as u16, box_area.y + 1));
}

/// The keys, in a box over the middle of the list. This is the one overlay in
/// the product and the only place a popup is the right answer: you asked for it,
/// and it covers nothing you were half-way through reading — docs/tui.md#help.
///
/// Only the keys that do something. A help screen listing keys that are not
/// built yet teaches the wrong thing twice.
fn help(frame: &mut Frame, area: Rect, render: Render<'_>) {
    // Two of these carry a glyph, and the overlay is the one screen where a
    // character the terminal cannot draw does the most damage: it is the screen
    // somebody opens *because* they are lost — docs/tui.md#no-colour-no-nerd-font.
    let keys: [(String, &str); 10] = [
        (format!("j k  {}", render.glyphs.arrows()), "move"),
        ("g G".to_string(), "top / bottom"),
        ("ctrl-d ctrl-u".to_string(), "half page"),
        ("spc".to_string(), "toggle done"),
        (format!("a o  {}", render.glyphs.enter()), "add / edit"),
        ("d  u".to_string(), "delete / undo"),
        ("h l  z".to_string(), "fold this group"),
        // Two keys to a row, so that the box still fits a fourteen-row pane.
        // At twelve rows of keys the border takes `q  ctrl-c` off the bottom,
        // and a help screen that cuts off at quit is worse than none.
        ("e  r".to_string(), "$EDITOR / re-read"),
        (":  /".to_string(), "answer, for now"),
        ("q  ctrl-c".to_string(), "quit"),
    ];

    let width = 40.min(area.width);
    // Two for the border, and not a row more: at ten keys a spare blank line is
    // the difference between the box fitting a 14-row pane and `q  ctrl-c` being
    // the line that falls off it. A help screen that cuts off at quit is worse
    // than no help screen.
    let height = (keys.len() as u16 + 2).min(area.height);
    // Centred, and clamped: on a pane smaller than the box the box wins the
    // space it has rather than drawing outside it.
    let box_area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );

    let mut lines = Vec::with_capacity(keys.len());
    for (keys, what) in keys {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {keys:<15}"),
                Style::default().fg(render.colours.accent),
            ),
            Span::styled(what, Style::default().fg(render.colours.foreground)),
        ]));
    }

    frame.render_widget(Clear, box_area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .border_set(render.glyphs.border())
                .border_style(Style::default().fg(render.colours.accent))
                .title(" keys ")
                // On the border, where it costs no row: the way out of the
                // overlay is the one thing that must never be the line that
                // falls off the bottom of a short pane.
                .title_bottom(Line::from(" esc or ? to close ").centered())
                .style(Style::default().bg(render.colours.background)),
        ),
        box_area,
    );
}

/// The first thing a new user sees, so it teaches rather than apologises. The
/// worked example is doing the real work: it shows `@` and `#` in use, which
/// lands faster than any syntax table — docs/tui.md#empty.
fn empty(frame: &mut Frame, area: Rect, block: Option<Block<'_>>, render: Render<'_>) {
    let inner = block.as_ref().map_or(area, |b| b.inner(area));
    if let Some(block) = block {
        frame.render_widget(block, area);
    }

    let dim = Style::default().fg(render.colours.dim);
    let mut lines = vec![
        Line::raw(""),
        Line::styled(
            "  Nothing here yet.",
            Style::default().fg(render.colours.foreground),
        ),
        Line::raw(""),
        Line::styled("  a          add your first task", dim),
        // Shortened rather than left to run into the frame: a `--file` path can
        // be arbitrarily long, and a broken right edge on the very first screen
        // somebody sees is a poor introduction.
        Line::styled(
            format!(
                "  e          open {} in $EDITOR",
                shorten(
                    render.path,
                    (inner.width as usize).saturating_sub(31),
                    render.glyphs,
                )
            ),
            dim,
        ),
        Line::raw(""),
    ];
    // Four rows for the box under the six above it. Where they do not fit, the
    // example stays a line of text: it is the part that teaches, so it is the
    // last thing a short pane is allowed to lose.
    let room = inner.height >= 10 && inner.width >= 34;
    if !room {
        lines.push(Line::styled(
            format!("  Try:  a  then  {EXAMPLE}"),
            Style::default().fg(render.colours.accent),
        ));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(render.colours.background)),
        inner,
    );

    if room {
        example(frame, inner, render);
    }
}

/// The one line of syntax anybody needs, and the shorthand in it is the point.
const EXAMPLE: &str = "buy milk @tomorrow #home";

/// The example in the box it will actually be typed into: the same field `a`
/// opens, drawn by the same code, with the live parse under it already turning
/// `@tomorrow` into a date. Nothing has to be typed to find that out.
///
/// The frame's own colour rather than the accent, because the accent border is
/// what marks the box that has the keyboard — this one is a picture of it.
fn example(frame: &mut Frame, inner: Rect, render: Render<'_>) {
    let width = 48.min(inner.width.saturating_sub(4));
    let area = Rect::new(inner.x + 2, inner.y + 6, width, 4);
    let (lines, _) = input_lines(
        &Input::new(EXAMPLE.to_string(), None),
        (width as usize).saturating_sub(2),
        render,
    );

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .border_set(render.glyphs.border())
                .border_style(Style::default().fg(render.colours.border)),
        ),
        area,
    );
}

/// `5 open · 1 overdue` while it fits, `5 · 1!` when it does not — and the same
/// numbers a waybar module shows, in the same words. One source.
///
/// A narrow pane has no room for the bar, so what it has finished is a count on
/// the end instead, and only once there is something to say.
fn title_counts(counts: Counts, size: Size, glyphs: Glyphs) -> String {
    let (_, dot) = glyphs.punctuation();
    match size {
        Size::Wide => format!("{} open {dot} {} overdue", counts.open, counts.overdue),
        _ if counts.done > 0 => format!(
            "{} {dot} {}! {dot} {}{}",
            counts.open,
            counts.overdue,
            counts.done,
            glyphs.tick()
        ),
        _ => format!("{} {dot} {}!", counts.open, counts.overdue),
    }
}

/// How many cells of the bar are filled. Eight cells, and the two ends are the
/// two things a reader will not forgive being wrong: a bar showing nothing when
/// something is done, and a full one with work left.
const BAR: usize = 8;

fn filled(done: usize, total: usize) -> usize {
    let cells = done * BAR / total;
    // Only the low end needs saying. The high end takes care of itself: with
    // anything left, `done * 8` is short of `total * 8`, so the division cannot
    // reach eight — and a clamp for it would be a line no test could ever fail.
    if done > 0 { cells.max(1) } else { cells }
}

/// What is finished, on the right of the title rule. `None` when there is
/// nothing to say — nothing done yet, or a pane with no room for it.
///
/// An empty bar is not information: "2 open" on the left already says you are at
/// the start, and `0/2` says it again in a second alphabet. So it appears when
/// the first task is ticked and not before.
///
/// Green, because docs/design.md#rules says green is only ever for completed and
/// this is the only other thing in the product that means completed. Spending a
/// second colour on it would dilute an earned meaning.
fn progress(
    counts: Counts,
    width: usize,
    left: usize,
    render: Render<'_>,
) -> Option<Line<'static>> {
    let total = counts.open + counts.done;
    let text = format!(" {}/{total} ", counts.done);
    // Four columns of rule between the two titles, or they read as one label.
    if counts.done == 0 || left + BAR + columns(&text) + 4 > width {
        return None;
    }

    let (on, off) = render.glyphs.bar();
    let cells = filled(counts.done, total);
    Some(Line::from(vec![
        Span::raw(" "),
        Span::styled(on.repeat(cells), Style::default().fg(render.colours.done)),
        Span::styled(
            off.repeat(BAR - cells),
            Style::default().fg(render.colours.border),
        ),
        Span::styled(text, Style::default().fg(render.colours.dim)),
    ]))
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

    fn render(colours: Theme) -> Render<'static> {
        Render {
            colours,
            glyphs: Glyphs::Unicode,
            today: today(),
            path: "~/.config/ratodo/todo.md",
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
                Row::Header { title, .. } => format!("# {title}"),
                Row::Task(t) => t.title.clone(),
                Row::Spacer => String::new(),
            })
            .collect()
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// `OVERDUE` is ours, `## Work` came out of the user's file, and the same
    /// bold word plus the same rule said nothing about which was which. Only
    /// the marker separates them: no second colour, no third level of
    /// hierarchy — docs/tui.md#main-screen.
    #[test]
    fn the_users_own_headings_keep_the_markdown_marker_and_ours_do_not() {
        let mut mixed = in_section(&[("write the plan", "Work")]);
        mixed.extend(tasks(&["late @2026-08-08"]));

        let groups = agenda(&mixed, today());
        assert_eq!(
            titles(&rows(&groups)),
            ["# OVERDUE", "late", "", "# ## Work", "write the plan"]
        );
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
        for code in [KeyCode::Char('x'), KeyCode::Char('Q'), KeyCode::Char('w')] {
            assert_eq!(action(press(code)), Action::Ignore, "{code:?}");
        }
        // `esc` is the one that matters. It closes the overlay and does nothing
        // else — never, under any circumstances, quit.
        assert_eq!(action(press(KeyCode::Esc)), Action::Close);
    }

    /// `:` and `/` are unbound but not silent. A key that appears broken is
    /// worse than one that explains itself — docs/tui.md#deliberately-unbound.
    #[test]
    fn the_keys_with_nothing_behind_them_still_answer() {
        assert_eq!(
            action(press(KeyCode::Char(':'))),
            Action::Say("no command mode - ? for keys")
        );
        assert_eq!(
            action(press(KeyCode::Char('/'))),
            Action::Say("search comes in v2")
        );
    }

    #[test]
    fn the_help_overlay_opens_and_closes_on_the_keys_that_should() {
        assert_eq!(action(press(KeyCode::Char('?'))), Action::Help);
        assert_eq!(action(press(KeyCode::Esc)), Action::Close);
    }

    /// The overlay covers the list and gives it back. Anything left behind on
    /// the second draw is a smear the user has to `r` away.
    #[test]
    fn the_overlay_covers_the_list_and_leaves_no_trace() {
        let tasks = tasks(&["pay the invoice @2026-08-01", "call the bank"]);
        let groups = agenda(&tasks, today());
        let counts = Counts::of(&tasks, today());

        let paint_help = |helping: bool| {
            let mut screen = Screen::new(rows(&groups));
            let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
            terminal
                .draw(|f| {
                    draw(
                        f,
                        &mut screen,
                        counts,
                        render(crate::theme::MOCHA),
                        &Notice::Hints,
                        helping,
                        None,
                    )
                })
                .unwrap();
            terminal
                .backend()
                .buffer()
                .content()
                .chunks(60)
                .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
                .collect::<Vec<String>>()
        };

        let plain = paint_help(false);
        let helped = paint_help(true);

        assert!(helped.iter().any(|r| r.contains("keys")), "{helped:?}");
        assert!(
            helped.iter().any(|r| r.contains("toggle done")),
            "{helped:?}"
        );
        assert_ne!(plain, helped, "the overlay drew nothing");
        assert_eq!(
            plain,
            paint_help(false),
            "closing it did not give the list back"
        );
    }

    /// Where the box sits, pinned. Centring is four pieces of arithmetic and
    /// every one of them survived a mutation while the tests only asked whether
    /// the words were on screen somewhere.
    #[test]
    fn the_help_box_is_centred_exactly() {
        let tasks = tasks(&["a @2026-08-01"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let mut terminal = Terminal::new(TestBackend::new(48, 14)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::default(),
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    true,
                    None,
                )
            })
            .unwrap();

        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content()
            .chunks(48)
            .map(|r| r.iter().map(|c| c.symbol()).collect())
            .collect();

        assert_eq!(
            rows,
            [
                "┌ ra┌ keys ────────────────────────────────┐───┐",
                "│  O│  j k  ↓ ↑       move                 │── │",
                "│▌ !│  g G            top / bottom         │ago│",
                "│   │  ctrl-d ctrl-u  half page            │   │",
                "│   │  spc            toggle done          │   │",
                "│   │  a o  ⏎         add / edit           │   │",
                "│   │  d  u           delete / undo        │   │",
                "│   │  h l  z         fold this group      │   │",
                "│   │  e  r           $EDITOR / re-read    │   │",
                "│   │  :  /           answer, for now      │   │",
                "│   │  q  ctrl-c      quit                 │   │",
                "│   └───────── esc or ? to close ──────────┘   │",
                "└──────────────────────────────────────────────┘",
                " j k  spc  a  d  e  ?  q                        ",
            ]
        );
    }

    /// The snapshot above is one pane height, and at that height the vertical
    /// centring happens to come out as zero — which leaves the arithmetic free
    /// to be anything. This walks it down three panes.
    #[test]
    fn the_help_box_sits_lower_as_the_pane_gets_taller() {
        let tasks = tasks(&["a @2026-08-01"]);
        let groups = agenda(&tasks, today());

        for (height, top) in [(15u16, 1usize), (17, 2), (21, 4)] {
            let mut screen = Screen::new(rows(&groups));
            let mut terminal = Terminal::new(TestBackend::new(48, height)).unwrap();
            terminal
                .draw(|f| {
                    draw(
                        f,
                        &mut screen,
                        Counts::default(),
                        render(crate::theme::MOCHA),
                        &Notice::Hints,
                        true,
                        None,
                    )
                })
                .unwrap();

            let at = terminal
                .backend()
                .buffer()
                .content()
                .chunks(48)
                .map(|r| r.iter().map(|c| c.symbol()).collect::<String>())
                .position(|r| r.contains(" keys "))
                .unwrap_or_else(|| panic!("no overlay at {height} rows"));
            assert_eq!(at, top, "at {height} rows");
        }
    }

    /// Only the keys that work. A help screen listing an unimplemented key
    /// teaches the wrong thing and then breaks the promise it just made.
    #[test]
    fn the_help_lists_nothing_that_is_not_built() {
        let tasks = tasks(&["a"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::default(),
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    true,
                    None,
                )
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        for built in ["add / edit", "$EDITOR", "delete / undo"] {
            assert!(text.contains(built), "{built} is built but not listed");
        }
        for unbuilt in ["search", "sort", "filter", "$VISUAL"] {
            assert!(
                !text.contains(unbuilt),
                "{unbuilt} is advertised but absent"
            );
        }
    }

    /// `h` and `l` collapse and open what is under the cursor — the muscle
    /// memory `lf`, `ranger` and `yazi` arrive with — and `z` is the vim fold
    /// prefix doing the whole job in one key.
    #[test]
    fn the_fold_keys() {
        assert_eq!(action(press(KeyCode::Char('h'))), Action::Fold(Fold::Close));
        assert_eq!(action(press(KeyCode::Left)), Action::Fold(Fold::Close));
        assert_eq!(action(press(KeyCode::Char('l'))), Action::Fold(Fold::Open));
        assert_eq!(action(press(KeyCode::Right)), Action::Fold(Fold::Open));
        assert_eq!(
            action(press(KeyCode::Char('z'))),
            Action::Fold(Fold::Toggle)
        );
    }

    #[test]
    fn the_keys_that_change_the_list() {
        assert_eq!(action(press(KeyCode::Char(' '))), Action::Toggle);
        assert_eq!(action(press(KeyCode::Char('r'))), Action::Reload);
        assert_eq!(action(press(KeyCode::Char('e'))), Action::Edit);
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Action::Move(10)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            Action::Move(-10)
        );
        // Without the modifier they are delete and undo. The pair is easy to
        // cross: `ctrl-d` scrolling and `d` deleting share a letter on purpose,
        // because that is what vim does.
        assert_eq!(action(press(KeyCode::Char('d'))), Action::Delete);
        assert_eq!(action(press(KeyCode::Char('u'))), Action::Undo);
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
        assert_eq!(rows(&agenda(&tasks, today()))[0], Row::header("TODAY"));
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

    /// The task itself changed — somebody ran `ratodo done` in the next pane, or
    /// a `git pull` brought a tag with it. The line is not the task, so the
    /// cursor has no business letting go of it.
    #[test]
    fn a_reload_holds_on_to_a_task_whose_line_changed() {
        let before = in_section(&[("one", "S"), ("two", "S"), ("three", "S")]);
        let groups = agenda(&before, today());
        let mut screen = Screen::new(rows(&groups));
        screen.move_by(1);
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("two"));

        // Ticked and tagged from outside, and pushed down the list by an insert.
        let mut ticked = capture("two #ops", today());
        ticked.section = Some("S".into());
        ticked.set_done(true);
        let mut after = in_section(&[("inserted", "S"), ("one", "S")]);
        after.push(ticked);
        after.extend(in_section(&[("three", "S")]));

        screen.replace(rows(&agenda(&after, today())));
        assert_eq!(
            screen.task().map(|t| t.title.as_str()),
            Some("two"),
            "the cursor followed the raw line instead of the task"
        );
        assert!(screen.task().unwrap().done);
    }

    /// Two tasks can be the same task as far as identity goes. The cursor stays
    /// with the nearer of them rather than jumping to the first — picking the
    /// first would walk it up the screen every time somebody keeps two
    /// `call the bank`s in one section.
    #[test]
    fn a_duplicated_title_keeps_the_cursor_on_the_nearer_one() {
        let tasks = in_section(&[
            ("call the bank", "S"),
            ("filler", "S"),
            ("call the bank", "S"),
        ]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        screen.bottom();
        let was = screen.selected();

        screen.replace(rows(&agenda(&tasks, today())));
        assert_eq!(
            screen.selected(),
            was,
            "the cursor jumped to the other one of the pair"
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
        rendered_with(width, height, tasks, |_| {})
    }

    fn rendered_notice(width: u16, height: u16, tasks: &[Task], notice: &Notice) -> Vec<String> {
        paint(width, height, tasks, notice, |_| {})
    }

    /// Renders after doing something to the screen first — a toggle, a move.
    fn rendered_with(
        width: u16,
        height: u16,
        tasks: &[Task],
        act: impl FnOnce(&mut Screen),
    ) -> Vec<String> {
        paint(width, height, tasks, &Notice::Hints, act)
    }

    fn paint(
        width: u16,
        height: u16,
        tasks: &[Task],
        notice: &Notice,
        act: impl FnOnce(&mut Screen),
    ) -> Vec<String> {
        let groups = agenda(tasks, today());
        let mut screen = Screen::new(rows(&groups));
        act(&mut screen);
        let counts = Counts::of(tasks, today());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render(crate::theme::MOCHA),
                    notice,
                    false,
                    None,
                )
            })
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
            rendered(62, 10, &tasks),
            [
                "┌ ratodo — 2 open · 1 overdue ───────────────────────────────┐",
                "│  OVERDUE ───────────────────────────────────────────────── │",
                "│▌ ! late                                        1d ago  #ops│",
                "│                                                            │",
                "│  ## Work ───────────────────────────────────────────────── │",
                "│  ○ write the plan                                          │",
                "│                                                            │",
                "│                                                            │",
                "└────────────────────────────────────────────────────────────┘",
                " j k move   spc done   a add   ⏎ edit   ? keys   q quit       ",
            ]
        );
    }

    /// The progress bar, drawn exactly: green for what is finished, the rule
    /// between the two titles, and the count flush to the corner.
    #[test]
    fn the_title_bar_shows_what_is_finished() {
        let mut done = capture("migrate the server", today());
        done.set_done(true);
        let tasks = [capture("late @2026-08-09 #ops", today()), done];

        assert_eq!(
            rendered(62, 5, &tasks),
            [
                "┌ ratodo — 1 open · 1 overdue ───────────────── ▰▰▰▰▱▱▱▱ 1/2 ┐",
                "│  OVERDUE ───────────────────────────────────────────────── │",
                "│▌ ! late                                        1d ago  #ops│",
                "└────────────────────────────────────────────────────────────┘",
                " ?                                                            ",
            ]
        );
    }

    /// Nothing finished is not a fact worth two symbols: `2 open` on the left
    /// already says you are at the start.
    #[test]
    fn an_untouched_list_gets_no_bar_at_all() {
        let tasks = tasks(&["a @2026-08-09", "b"]);
        let screen = rendered(62, 5, &tasks);
        assert!(!screen[0].contains('▱'), "{screen:?}");
        assert!(!screen[0].contains('/'), "{screen:?}");
    }

    /// A narrow pane has no room for the bar and says the number instead — and
    /// under 34 columns there is no title bar to say it in.
    #[test]
    fn the_bar_gives_way_before_the_counts_do() {
        let mut done = capture("finished", today());
        done.set_done(true);
        let tasks = [capture("late @2026-08-09", today()), done];

        let narrow = rendered(46, 5, &tasks);
        assert!(
            narrow[0].starts_with("┌ ratodo — 1 · 1! · 1✓"),
            "{narrow:?}"
        );
        assert!(!narrow[0].contains('▰'), "the bar did not fit: {narrow:?}");

        assert!(
            !rendered(30, 5, &tasks)[0].contains('✓'),
            "there is no frame under 34 columns to put a count in"
        );
    }

    /// The title grows with the counts, and at some width it wants the room the
    /// bar is standing in. Pinned to the exact column, because "it fits" and "it
    /// fits by one" are the same assertion until one of them is not.
    #[test]
    fn the_bar_stands_down_when_the_title_needs_the_room() {
        let counts = Counts {
            open: 5,
            done: 3,
            ..Counts::default()
        };
        let render = render(crate::theme::MOCHA);
        // Eight cells, ` 3/8 `, and four columns of rule between the two titles:
        // seventeen, so a 43-column title is the last one that leaves room.
        assert!(progress(counts, 60, 43, render).is_some());
        assert!(
            progress(counts, 60, 44, render).is_none(),
            "the bar overran the title"
        );
        assert!(
            progress(counts, 20, 10, render).is_none(),
            "a pane this narrow has no room for either"
        );
    }

    /// The two ends are the ones a reader will not forgive. Everything between
    /// them is proportional.
    #[test]
    fn the_bar_never_reads_empty_with_work_done_or_full_with_work_left() {
        assert_eq!(filled(0, 8), 0);
        assert_eq!(filled(8, 8), BAR);
        assert_eq!(filled(4, 8), 4);
        assert_eq!(
            filled(1, 100),
            1,
            "one task in a hundred still moved the bar"
        );
        assert_eq!(
            filled(99, 100),
            BAR - 1,
            "one task left is not a finished list"
        );
    }

    /// The whole screen goes ASCII together, and the bar is on the screen.
    #[test]
    fn the_bar_has_an_ascii_form() {
        let mut done = capture("finished", today());
        done.set_done(true);
        let tasks = [capture("late @2026-08-09", today()), done];
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let render = Render {
            glyphs: Glyphs::Ascii,
            ..render(crate::theme::MOCHA)
        };

        let mut terminal = Terminal::new(TestBackend::new(62, 5)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::of(&tasks, today()),
                    render,
                    &Notice::Hints,
                    false,
                    None,
                )
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(text.contains("#---- 1/2"), "{text}");
        assert!(text.is_ascii(), "something non-ASCII reached the screen");
    }

    /// The fallback used to stop at the edge of the overlay: `↓ ↑` and `⏎` were
    /// written into it as literals, and the buffer test above never caught it
    /// because it does not open the overlay. So this one opens **everything** —
    /// the help box, the input with its preview, and a title long enough to be
    /// cut, since the ellipsis is a glyph like any other.
    #[test]
    fn the_ascii_fallback_reaches_the_overlay_and_everything_under_it() {
        let mut long = capture("a @2026-08-09 !high #ops", today());
        long.title = "an extremely long task title that will not fit in the pane".into();
        let tasks = [long];
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let render = Render {
            glyphs: Glyphs::Ascii,
            ..render(crate::theme::MOCHA)
        };
        // Two fields, so the preview line has to put a separator between them.
        // (The overlay covers the cut title itself; `shorten` is tested with
        // both glyph sets on its own.)
        let input = Input::new("buy milk @thu #home".to_string(), None);

        let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::of(&tasks, today()),
                    render,
                    &Notice::Hints,
                    true,
                    Some(&input),
                )
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(text.contains("down up"), "the arrow keys: {text}");
        assert!(text.contains("ret"), "the enter key: {text}");
        assert!(text.is_ascii(), "something non-ASCII reached the screen");
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
                "└────────────────────────────────────────────────┘",
                " ?                                                ",
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
            rendered(46, 8, &tasks),
            [
                "┌ ratodo — 2 · 1! ───────────────────────────┐",
                "│  OVERDUE ───────────────────────────────── │",
                "│▌ ! late                              1d ago│",
                "│  ## Work ───────────────────────────────── │",
                "│  ○ write the plan                          │",
                "│                                            │",
                "└────────────────────────────────────────────┘",
                " ?                                            ",
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

    /// The whole reason a column is a column: the eye reads *down* it. Two
    /// titles of very different lengths have to put their dates in the same
    /// place, or it is a ragged list wearing a table's name.
    #[test]
    fn the_date_column_starts_at_the_same_place_on_every_row() {
        let tasks = tasks(&["short @2026-08-01", "a much longer title here @2026-08-01"]);
        let screen = rendered(90, 8, &tasks);

        let at: Vec<usize> = screen[2..4]
            .iter()
            .map(|row| at_column(row, "9d ago"))
            .collect();
        assert_eq!(at[0], at[1], "the dates do not line up: {screen:?}");

        // And the column is the width of the widest title, not of the pane: a
        // date shoved to the right edge is what this replaced.
        assert!(at[0] < 40, "the date is still at the edge: {screen:?}");
    }

    /// Columns are the fourth breakpoint, not the third. A pane too narrow to
    /// afford an empty priority column keeps the packed right-aligned block:
    /// otherwise alignment is bought with the title, and docs/tui.md#width
    /// calls the title sacred.
    #[test]
    fn a_pane_too_narrow_for_columns_does_not_get_them() {
        let tasks = tasks(&["short @2026-08-01", "a much longer title here @2026-08-01"]);
        // Two rows with the same right-hand block line up under either layout,
        // so equality proves nothing here. Where the date *sits* does: packed
        // against the right edge, or one gap past the longest title.
        let at = |terminal: u16| at_column(&rendered(terminal, 8, &tasks)[2], "9d ago");

        // Four columns of frame and selection marker sit between the terminal
        // and the row that COLUMNS_AT measures.
        let packed = COLUMNS_AT as u16 + 3;
        assert!(
            at(packed) > COLUMNS_AT - 10,
            "columns one column too early: {:?}",
            rendered(packed, 8, &tasks)
        );
        assert_eq!(
            at(packed + 1),
            1 + 2 + 2 + columns("a much longer title here") + GAP,
            "columns one column too late: {:?}",
            rendered(packed + 1, 8, &tasks)
        );
    }

    /// The date column is the easy half: it starts right after the title, so
    /// it lines up even if the padding after it is wrong. What proves the
    /// padding is the column *behind* it — tags on rows whose dates and
    /// priorities are all different lengths, including rows that have neither.
    #[test]
    fn every_column_pads_to_its_own_width_so_the_one_behind_it_lines_up() {
        let tasks = tasks(&[
            "long date and a priority @2026-08-14 09:30 !high #alpha",
            "short date, no priority @2026-08-01 #bravo",
            "no date at all !low #charlie",
        ]);
        let screen = rendered(90, 10, &tasks);

        let at: Vec<usize> = ["#alpha", "#bravo", "#charlie"]
            .iter()
            .map(|tag| {
                let row = screen
                    .iter()
                    .find(|r| r.contains(tag))
                    .unwrap_or_else(|| panic!("{tag} is not on screen: {screen:?}"));
                at_column(row, tag)
            })
            .collect();

        assert_eq!(at[0], at[1], "a shorter date moved the tags: {screen:?}");
        assert_eq!(at[1], at[2], "a missing date moved the tags: {screen:?}");
    }

    /// The column widths themselves, not the difference between two of them: a
    /// budget that is wrong by the same amount everywhere still lines up.
    #[test]
    fn the_column_widths_are_what_the_arithmetic_says() {
        let mut long = capture("a @2026-08-01 !high", today());
        long.title = "x".repeat(80);
        let rows = rows(&agenda(&[long], today()));
        let cols = Columns::of(&rows, 86, render(crate::theme::MOCHA), Size::Wide);

        // `9d ago` and `!high`, each plus its gap.
        assert_eq!(cols.date, 6 + GAP, "the date column");
        assert_eq!(cols.prio, 5 + GAP, "the priority column");
        // 86 less the mark and its space, less both of those. The title asked
        // for 80 and the row has this much to give it.
        assert_eq!(
            cols.title,
            86 - 2 - (6 + GAP) - (5 + GAP),
            "the title column"
        );
    }

    /// A list where nothing has a priority spends no width on a priority
    /// column — otherwise every screen pays for the busiest one it might ever
    /// show.
    #[test]
    fn an_empty_column_costs_nothing() {
        let bare = tasks(&["one @2026-08-01", "two @2026-08-01"]);
        let cols = Columns::of(
            &rows(&agenda(&bare, today())),
            80,
            render(crate::theme::MOCHA),
            Size::Wide,
        );
        assert_eq!(cols.prio, 0, "a priority column nobody filled");

        let some = tasks(&["one @2026-08-01 !high", "two @2026-08-01"]);
        let cols = Columns::of(
            &rows(&agenda(&some, today())),
            80,
            render(crate::theme::MOCHA),
            Size::Wide,
        );
        assert_eq!(
            cols.prio,
            5 + GAP,
            "one task with a priority buys the column"
        );
    }

    /// Tags are the first thing to go when the row runs out, and they go whole:
    /// `#hea…` is not a filter, it is a riddle. What must never happen is the
    /// inverse — a title cut to pay for tags the row does not have room for.
    #[test]
    fn tags_give_way_before_the_title_does() {
        // Both a date and a priority, so every term of the budget is non-zero
        // and none of them can be dropped without the answer moving.
        let mut task = capture("a @2026-08-01 !high #alpha #bravo #charlie #delta", today());
        task.title = "a title long enough to leave the last tags nowhere to go".into();
        let screen = rendered(96, 6, &[task]);

        assert!(
            screen[2].contains("a title long enough to leave the last tags nowhere to go"),
            "the title was cut to pay for tags: {screen:?}"
        );
        assert!(
            !screen[2].contains('…'),
            "a tag was cut in half: {screen:?}"
        );

        // Where the budget runs out, to the column. The row is 92 wide; the
        // mark, the 56-column title, the date column and the priority column
        // spend 73 of it, and `#alpha` and `#bravo` are what the rest buys.
        let title = columns("a title long enough to leave the last tags nowhere to go");
        let budget = 92 - (2 + title + (6 + GAP) + (5 + GAP));
        assert_eq!(
            budget,
            columns("  #alpha  #bravo") + 3,
            "the tag budget moved"
        );
        assert!(screen[2].contains("#alpha"), "{screen:?}");
        assert!(screen[2].contains("#bravo"), "{screen:?}");
        for missing in ["#charlie", "#delta"] {
            assert!(
                !screen[2].contains(missing),
                "{missing} was drawn past the budget: {screen:?}"
            );
        }
    }

    /// The date column carried the fact and the title carried the colour: a red
    /// title next to a grey `3d ago` puts the warning one field away from the
    /// thing that is actually late. Only the two that press get it — a `Fri` is
    /// a fact, and a ticked task is neither late nor due.
    #[test]
    fn the_date_goes_loud_only_when_it_is_late_or_today() {
        let colours = crate::theme::MOCHA;
        let style_of = |spec: &str, done: bool| {
            let mut task = capture(spec, today());
            task.set_done(done);
            let rows = [Row::Task(task.clone())];
            let cols = Columns::of(&rows, 86, render(colours), Size::Wide);
            let line = task_line(&task, 86, cols, render(colours), Size::Wide);
            // The date is the first styled entry after the mark and the title.
            line.spans[3..]
                .iter()
                .find(|s| !s.content.trim().is_empty())
                .expect("the date is on the row")
                .style
        };

        assert_eq!(style_of("a @2026-08-08", false).fg, Some(colours.overdue));
        assert_eq!(style_of("a @2026-08-10", false).fg, Some(colours.today));
        assert_eq!(
            style_of("a @2026-08-10 16:00", false).fg,
            Some(colours.today),
            "a time today is the one most worth seeing"
        );

        // Neither late nor today, so it is a fact like any other.
        assert_eq!(style_of("a @2026-08-14", false).fg, Some(colours.dim));
        // And a finished task is neither, however far past its date it is.
        assert_eq!(style_of("a @2026-08-08", true).fg, Some(colours.dim));
        assert_eq!(style_of("a @2026-08-10", true).fg, Some(colours.dim));
    }

    /// `!high` is the one field the user typed to mean *urgent*, and it sat in
    /// the same grey as the date and the tags. Weight says so without spending a
    /// twelfth theme colour, and without leaning on colour at all
    /// — docs/design.md#rules.
    #[test]
    fn high_priority_carries_weight_and_the_other_two_stay_quiet() {
        let colours = crate::theme::MOCHA;
        let style_of = |spec: &str, done: bool| {
            let mut task = capture(spec, today());
            task.set_done(done);
            let rows = [Row::Task(task.clone())];
            let cols = Columns::of(&rows, 86, render(colours), Size::Wide);
            let line = task_line(&task, 86, cols, render(colours), Size::Wide);
            line.spans
                .iter()
                .find(|s| s.content.starts_with('!'))
                .expect("the priority is on the row")
                .style
        };

        let high = style_of("a @2026-08-14 !high", false);
        assert_eq!(high.fg, Some(colours.foreground));
        assert!(
            high.add_modifier.contains(ratatui::style::Modifier::BOLD),
            "the loud field is not loud: {high:?}"
        );

        for quiet in ["a @2026-08-14 !med", "a @2026-08-14 !low"] {
            let style = style_of(quiet, false);
            assert_eq!(style.fg, Some(colours.dim), "{quiet}");
            assert!(
                !style.add_modifier.contains(ratatui::style::Modifier::BOLD),
                "{quiet}"
            );
        }

        // Finished work is not urgent, however it was filed — the same reason a
        // ticked task stops saying how late it is.
        let ticked = style_of("a @2026-08-14 !high", true);
        assert_eq!(ticked.fg, Some(colours.dim));
        assert!(!ticked.add_modifier.contains(ratatui::style::Modifier::BOLD));
    }

    /// A row is built to a width and ratatui clips whatever overruns it, so an
    /// overspent budget is invisible on screen: the buffer looks the same
    /// whether the tags stopped or were cut off by the frame. The line itself
    /// is the only place the overrun exists, so it is where it gets measured.
    #[test]
    fn the_row_never_overruns_the_width_it_was_given() {
        let mut task = capture("a @2026-08-01 !high #alpha #bravo #charlie #delta", today());
        task.title = "a title long enough to leave the last tags nowhere to go".into();
        let rows = rows(&agenda(std::slice::from_ref(&task), today()));
        let render = render(crate::theme::MOCHA);

        for width in [COLUMNS_AT, 82, 92, 140] {
            let cols = Columns::of(&rows, width, render, Size::Wide);
            let drawn = columns(&task_line(&task, width, cols, render, Size::Wide).to_string());
            assert!(
                drawn <= width,
                "at {width} the row overran by {}",
                drawn - width
            );
        }
    }

    /// The column is measured over the whole list, not the rows that happen to
    /// be on screen — a column that resizes as you scroll is not a column.
    #[test]
    fn the_column_is_measured_over_the_whole_list_not_the_viewport() {
        let tasks = tasks(&[
            "short @2026-08-01",
            "a very much longer title down here @2026-08-01",
        ]);
        let rows = rows(&agenda(&tasks, today()));
        let wide = Columns::of(&rows, 86, render(crate::theme::MOCHA), Size::Wide);

        // Two rows of list is not enough to show the second task at all.
        let cramped = rendered(90, 5, &tasks);
        assert!(cramped[2].contains("short"), "{cramped:?}");
        // The frame, the cursor, the mark, then the title column and its gap.
        assert_eq!(
            at_column(&cramped[2], "9d ago"),
            1 + 2 + 2 + wide.title + GAP,
            "the visible row was measured on its own: {cramped:?}"
        );
    }

    /// `Glyphs::mark_width` is arithmetic done before there is a task to ask,
    /// so it has to agree with every mark `Glyphs::mark` can actually return.
    #[test]
    fn the_mark_width_matches_every_mark_in_its_set() {
        let mut done = capture("done", today());
        done.set_done(true);
        let cases = [
            capture("open", today()),
            capture("late @2026-08-01", today()),
            done,
        ];

        for glyphs in [Glyphs::Unicode, Glyphs::Ascii] {
            for task in &cases {
                assert_eq!(
                    columns(glyphs.mark(task, today())),
                    glyphs.mark_width(),
                    "{glyphs:?} disagrees with itself about {:?}",
                    task.title
                );
            }
        }
    }

    /// `[ ]` is two columns wider than `○`, and the budget is struck before
    /// there is a task to ask. Budget the Unicode figure under ASCII and every
    /// row overspends by two — which the tag budget absorbs by dropping tags,
    /// so it costs information rather than breaking the frame.
    #[test]
    fn the_column_budget_follows_the_glyph_set() {
        let mut long = capture("a @2026-08-01", today());
        long.title = "x".repeat(80);
        let rows = rows(&agenda(&[long], today()));
        let ascii = Render {
            glyphs: Glyphs::Ascii,
            ..render(crate::theme::MOCHA)
        };

        let unicode = Columns::of(&rows, 86, render(crate::theme::MOCHA), Size::Wide);
        let wider = Columns::of(&rows, 86, ascii, Size::Wide);
        assert_eq!(
            unicode.title,
            wider.title + 2,
            "the wider mark bought no width back"
        );

        // The group rule ends with the titles, so it has to follow the mark
        // too — under ASCII it would otherwise stop two columns short.
        let rule = |cols: Columns, render: Render<'_>| {
            columns(&header_line("Work", None, 86, cols, render).to_string())
        };
        assert_eq!(
            rule(wider, ascii),
            rule(unicode, render(crate::theme::MOCHA)),
            "the rule and the titles disagree about the mark"
        );
    }

    /// `str::find` answers in bytes, and a row starts with `│▌` — three bytes
    /// each. A layout assertion that counts those is measuring the encoding.
    fn at_column(row: &str, needle: &str) -> usize {
        let byte = row
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} is not in {row:?}"));
        columns(&row[..byte])
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
        // The bottom row is the notice line, outside the frame.
        let frame = screen.len() - 2;
        for row in &screen[1..frame] {
            assert!(row.ends_with('│'), "the right edge broke: {screen:?}");
        }
        assert!(screen[0].ends_with('┐') && screen[frame].ends_with('┘'));
        assert!(screen[2].ends_with("9d ago│"), "{screen:?}");
    }

    #[test]
    fn the_ascii_fallback_replaces_every_glyph() {
        let tasks = tasks(&["late @2026-08-01", "fine"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let counts = Counts::of(&tasks, today());
        let render = Render {
            glyphs: Glyphs::Ascii,
            ..render(crate::theme::MOCHA)
        };

        let mut terminal = Terminal::new(TestBackend::new(62, 8)).unwrap();
        terminal
            .draw(|f| draw(f, &mut screen, counts, render, &Notice::Hints, false, None))
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

    /// It is finished, so the lateness stopped being true — and the counts
    /// already agree: a completed task is never in `overdue`. The date it was
    /// for survives, because that much is still a fact.
    #[test]
    fn a_finished_task_is_not_late_however_far_past_its_date_it_is() {
        let mut done = capture("a @2026-08-08", today());
        done.set_done(true);
        assert_eq!(when(&done, today(), Size::Wide), "Aug 8");
        assert_eq!(when(&done, today(), Size::Narrow), "Aug 8");

        // Only lateness goes: a finished task due today still says so.
        let mut earlier = capture("a @2026-08-10 09:30", today());
        earlier.set_done(true);
        assert_eq!(when(&earlier, today(), Size::Wide), "09:30");
    }

    #[test]
    fn shortening_counts_columns_not_bytes() {
        assert_eq!(shorten("hello", 10, Glyphs::Unicode), "hello");
        assert_eq!(shorten("hello there", 8, Glyphs::Unicode), "hello t…");
        assert_eq!(shorten("şşşşş", 3, Glyphs::Unicode), "şş…");
        assert_eq!(shorten("🚀🚀🚀", 5, Glyphs::Unicode), "🚀🚀…");
        assert_eq!(shorten("anything", 0, Glyphs::Unicode), "");

        // Three columns rather than one, and it is held back rather than
        // assumed: `...` where `…` fitted cuts two more columns of title.
        assert_eq!(shorten("hello there", 8, Glyphs::Ascii), "hello...");
        assert_eq!(shorten("hello", 10, Glyphs::Ascii), "hello");
        // No room for the marker: the title is worth more than the dots.
        assert_eq!(shorten("hello", 2, Glyphs::Ascii), "he");
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
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    false,
                    None,
                )
            })
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
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render(colours),
                    &Notice::Hints,
                    false,
                    None,
                )
            })
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
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render(plain),
                    &Notice::Hints,
                    false,
                    None,
                )
            })
            .unwrap();

        for cell in terminal.backend().buffer().content() {
            assert_eq!(cell.fg, Color::Reset, "{:?}", cell.symbol());
            assert_eq!(cell.bg, Color::Reset, "{:?}", cell.symbol());
        }
    }

    /// The first of the side-pane rules: a task ticked done marks in place. If
    /// it jumped to the end of its group the row you just touched would fly off
    /// somewhere while you were looking at it.
    #[test]
    fn a_toggled_task_marks_in_place_and_moves_nothing() {
        let tasks = tasks(&["first @2026-08-01", "second @2026-08-01"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let before = screen.selected();

        let mut done = screen.task().unwrap().clone();
        done.set_done(true);
        screen.update_selected(done);

        assert_eq!(screen.selected(), before, "the cursor moved");
        assert!(screen.task().unwrap().done);
        assert_eq!(
            screen.task().unwrap().title,
            "first",
            "a different row was rewritten"
        );

        let text = rendered_with(62, 8, &tasks, |s| {
            let mut d = s.task().unwrap().clone();
            d.set_done(true);
            s.update_selected(d);
        });
        assert!(text[2].contains("✓ first"), "{text:?}");
        assert!(text[3].contains("! second"), "{text:?}");
    }

    /// The bottom line does four jobs and never changes the list's shape.
    #[test]
    fn the_bottom_line() {
        let colours = crate::theme::MOCHA;
        let shown = |notice: &Notice, size, height, glyphs| {
            notice
                .line(size, height, glyphs, colours)
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect::<String>()
        };

        assert!(
            shown(&Notice::Hints, Size::Wide, 20, Glyphs::Unicode).contains("spc done"),
            "the hints have to name the keys"
        );
        assert!(shown(&Notice::Hints, Size::Wide, 20, Glyphs::Unicode).contains("a add"));
        // Sixty columns is the narrowest pane that still counts as wide, and the
        // bar has to fit it — in both alphabets, `ret` being the longer of the
        // two. A hint bar that gets clipped is advertising half a key.
        for glyphs in [Glyphs::Unicode, Glyphs::Ascii] {
            let bar = shown(&Notice::Hints, Size::Wide, 20, glyphs);
            assert!(columns(&bar) <= 60, "{} columns: {bar}", columns(&bar));
        }
        assert_eq!(
            shown(&Notice::Hints, Size::Wide, 9, Glyphs::Unicode),
            " ?",
            "under ten rows the hint bar collapses"
        );
        assert!(!shown(&Notice::Hints, Size::Narrow, 20, Glyphs::Unicode).contains("move"));

        assert_eq!(
            shown(
                &Notice::Said("done: milk".into()),
                Size::Wide,
                20,
                Glyphs::Unicode
            ),
            " done: milk"
        );
        assert_eq!(
            shown(
                &Notice::Warned("nope".into()),
                Size::Wide,
                20,
                Glyphs::Unicode
            ),
            " ⚠ nope"
        );
        assert_eq!(
            shown(
                &Notice::Warned("nope".into()),
                Size::Wide,
                20,
                Glyphs::Ascii
            ),
            " ! nope",
            "the warning mark has an ASCII form too"
        );
    }

    /// The two keys that open the input, and the one that opens it already
    /// holding the task under the cursor.
    #[test]
    fn the_keys_that_open_the_input() {
        assert_eq!(action(press(KeyCode::Char('a'))), Action::Add);
        assert_eq!(
            action(press(KeyCode::Char('o'))),
            Action::Add,
            "a vim user reaches for `o` to open a new line"
        );
        assert_eq!(action(press(KeyCode::Enter)), Action::Change);
    }

    /// The one key that means two different things in the two modes: in the list
    /// it quits, in the input it cancels. Somebody half-way through typing a
    /// task who reaches for the universal "stop that" key should lose the
    /// sentence, not the session — docs/tui.md#two-modes.
    #[test]
    fn ctrl_c_quits_the_list_and_only_cancels_the_input() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(action(ctrl_c), Action::Quit);
        assert_eq!(typing(ctrl_c), Typed::Cancel);
        assert_eq!(typing(press(KeyCode::Esc)), Typed::Cancel);
    }

    /// Every list key is just a letter in here, which is what makes "nothing
    /// else can open it" true by construction rather than by discipline.
    #[test]
    fn the_input_takes_letters_and_leaves_everything_else_alone() {
        // `c` among them on purpose: it is `ctrl-c` that cancels, and a bare one
        // that cancelled would eat the sentence on a typo.
        for c in ['a', 'q', 'd', 'c', ' ', 'ş', '@'] {
            assert_eq!(typing(press(KeyCode::Char(c))), Typed::Char(c), "{c}");
        }
        assert_eq!(typing(press(KeyCode::Backspace)), Typed::Back);
        assert_eq!(typing(press(KeyCode::Enter)), Typed::Save);
        assert_eq!(typing(press(KeyCode::Left)), Typed::Left);
        assert_eq!(typing(press(KeyCode::Right)), Typed::Right);
        assert_eq!(typing(press(KeyCode::Home)), Typed::Home);
        assert_eq!(typing(press(KeyCode::End)), Typed::End);
        assert_eq!(typing(press(KeyCode::Delete)), Typed::Delete);

        // A modified key is nothing at all: `ctrl-v` and `alt-f` mean things in
        // a terminal that a one-line field has no business claiming, and a
        // control character in a title is a file the user cannot read back.
        for m in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
            assert_eq!(typing(KeyEvent::new(KeyCode::Char('d'), m)), Typed::Ignore);
        }
        let mut held = press(KeyCode::Char('a'));
        held.kind = KeyEventKind::Release;
        assert_eq!(
            typing(held),
            Typed::Ignore,
            "a key being let go is not a press"
        );
    }

    /// `⏎` starts from what the file actually says, not from our reading of it —
    /// which is also what lets the prefix survive an edit untouched.
    #[test]
    fn editing_is_pre_filled_from_the_line_as_the_file_has_it() {
        let doc = crate::parse::parse("  * [x] wash up @2026-08-12 #home\n");
        let task = doc.tasks().next().unwrap();

        let input = Input::editing(task);
        assert_eq!(input.text, "wash up @2026-08-12 #home");
        assert_eq!(
            input.editing.as_deref(),
            Some("  * [x] wash up @2026-08-12 #home")
        );
        assert_eq!(Input::adding().editing, None);
        // At the end, so a retype carries on from where the line stops.
        assert_eq!(input.at, input.text.len());
    }

    /// A field you can only append to is not a field: the fix for a typo four
    /// words back must not be retyping four words — docs/tui.md#adding.
    #[test]
    fn the_caret_moves_through_the_line_and_edits_where_it_stands() {
        let mut input = Input::new("wash şu".to_string(), None);

        // Multi-byte on purpose: every move steps by a whole char, or the next
        // slice panics.
        input.left();
        input.left();
        assert_eq!(input.at, "wash ".len());
        input.insert('b');
        assert_eq!(input.text, "wash bşu");

        input.home();
        input.right();
        input.back();
        assert_eq!((input.text.as_str(), input.at), ("ash bşu", 0));

        // Backspace at the start and delete at the end are both no-ops rather
        // than a wrap-around or a panic.
        input.back();
        input.end();
        input.right();
        input.delete();
        assert_eq!((input.text.as_str(), input.at), ("ash bşu", 8));

        input.home();
        input.delete();
        assert_eq!((input.text.as_str(), input.at), ("sh bşu", 0));
    }

    /// The field is coloured by what the parser **took**, not by the leading
    /// character: `@notaday` is a word in a title and has to look like one, or
    /// the field teaches a syntax the file does not have — docs/tui.md#adding.
    #[test]
    fn the_field_colours_what_the_parser_understood_and_nothing_else() {
        let colours = crate::theme::MOCHA;
        let input = Input::new("pay @thu 09:30 #home !high @notaday".to_string(), None);
        let (lines, _) = input_lines(&input, 60, render(colours));
        let spans: Vec<(&str, Style)> = lines[0]
            .spans
            .iter()
            .map(|s| (s.content.as_ref(), s.style))
            .collect();

        let style = |want: &str| {
            spans
                .iter()
                .find(|(text, _)| text.contains(want))
                .unwrap_or_else(|| panic!("{want} is not its own span: {spans:?}"))
                .1
        };

        assert_eq!(style("@thu").fg, Some(colours.accent));
        // The time belongs to the date that took it.
        assert_eq!(style("09:30").fg, Some(colours.accent));
        assert_eq!(style("#home").fg, Some(colours.tag));
        assert!(
            style("!high")
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD)
        );

        // The second `@` resolves to nothing, so it is title text — and it must
        // not be a span of its own at all.
        let plain = style("@notaday");
        assert_eq!(plain.fg, Some(colours.foreground));
        assert!(!plain.add_modifier.contains(ratatui::style::Modifier::BOLD));
    }

    /// The colouring is asked about the whole line and drawn over a window of
    /// it, so a word half off the left edge has to be cut, not dropped — and the
    /// byte arithmetic that does it must not walk off a multi-byte character.
    #[test]
    fn colouring_survives_a_line_wider_than_the_field() {
        let colours = crate::theme::MOCHA;
        let mut input = Input::new(
            "şşşş bir hayli uzun bir cümle @tomorrow #ev burada biter".to_string(),
            None,
        );
        // Every caret position the keys can actually produce — `at` is only ever
        // moved by whole characters.
        let stops: Vec<usize> = input
            .text
            .char_indices()
            .map(|(i, _)| i)
            .chain([input.text.len()])
            .collect();
        for width in [12, 20, 34, 60] {
            for at in stops.iter().copied() {
                input.at = at;
                let (lines, cursor) = input_lines(&input, width, render(colours));
                let drawn: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
                assert!(columns(&drawn) <= width, "{width}/{at}: {drawn:?}");
                assert!(cursor < width, "{width}/{at}: the caret left the field");
            }
        }
    }

    /// The window follows the caret. Scrolling only ever to the end of the line
    /// would hide the very character being typed.
    #[test]
    fn a_long_line_scrolls_to_wherever_the_caret_is() {
        let long = "a very long sentence that will not fit in a narrow pane at all";
        let mut input = Input::new(long.to_string(), None);
        let field = |input: &Input| {
            let (lines, at) = input_lines(input, 30, render(crate::theme::MOCHA));
            (lines[0].to_string(), at)
        };

        let (end, at) = field(&input);
        assert!(end.ends_with("at all"), "{end:?}");
        assert_eq!(at, 29, "the caret sits at the end of what is shown");

        input.home();
        let (start, at) = field(&input);
        assert!(start.starts_with(" add ▏a very long"), "{start:?}");
        assert!(
            !start.contains("at all"),
            "the caret scrolled off: {start:?}"
        );
        assert_eq!(at, columns(" add ▏"));
    }

    fn with_input(
        width: u16,
        height: u16,
        tasks: &[Task],
        input: &Input,
        glyphs: Glyphs,
    ) -> Vec<String> {
        let groups = agenda(tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let counts = Counts::of(tasks, today());
        let render = Render {
            glyphs,
            ..render(crate::theme::MOCHA)
        };
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render,
                    &Notice::Hints,
                    false,
                    Some(input),
                )
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .chunks(width as usize)
            .map(|row| row.iter().map(|c| c.symbol()).collect())
            .collect()
    }

    /// The field, the preview and the keys, pinned to the column. The preview is
    /// the point of the whole screen: `@thu` becomes a real date in front of the
    /// person typing it — docs/tui.md#adding.
    #[test]
    fn the_input_screen_exactly() {
        let tasks = tasks(&["pay the invoice @2026-08-10", "b", "c", "d"]);
        let input = Input::new("call the accountant @thu !high".to_string(), None);

        assert_eq!(
            with_input(70, 11, &tasks, &input, Glyphs::Unicode),
            [
                "┌ ratodo — 4 open · 0 overdue ───────────────────────────────────────┐",
                "│  TODAY ─────────────────────────────────────────────────────────── │",
                "│▌ ○ pay the invoice                                            today│",
                "│ ┌────────────────────────────────────────────────────────────────┐ │",
                "│ │ add ▏call the accountant @thu !high                            │ │",
                "│ │      due Thursday (2026-08-13)  ·  !high                       │ │",
                "│ └────────────────────────────────────────────────────────────────┘ │",
                "│                                                                    │",
                "│                                                                    │",
                "└────────────────────────────────────────────────────────────────────┘",
                " ⏎ save   esc cancel                                                  ",
            ]
        );
    }

    /// Nothing parseable leaves the preview empty rather than showing an error:
    /// plain text is a perfectly good task.
    #[test]
    fn a_sentence_with_no_syntax_in_it_previews_nothing() {
        let input = Input::new("just write it down".to_string(), None);
        let screen = with_input(70, 9, &tasks(&["a"]), &input, Glyphs::Unicode);
        let field = screen
            .iter()
            .position(|r| r.contains(" add ▏just write it down"));
        let field = field.unwrap_or_else(|| panic!("{screen:?}"));

        assert_eq!(
            screen[field + 1].replace(['│', ' '], ""),
            "",
            "an unparseable line is not an error: {screen:?}"
        );
        assert_eq!(
            screen[8].trim(),
            "⏎ save   esc cancel",
            "the way out is on the line under the box: {screen:?}"
        );
    }

    /// A capture box that hides what you are typing is not a capture box, so the
    /// field scrolls with the end of the line rather than truncating it.
    #[test]
    fn a_line_longer_than_the_pane_keeps_its_end_on_screen() {
        let input = Input::new(
            "a very long sentence that will not fit in a narrow pane at all".to_string(),
            Some("- [ ] x".to_string()),
        );
        let screen = with_input(30, 8, &tasks(&["x"]), &input, Glyphs::Unicode);
        let field = screen
            .iter()
            .find(|row| row.contains(" edit ▏"))
            .unwrap_or_else(|| panic!("{screen:?}"));

        // The border is the row's last column, so the end of the line is the
        // one before it.
        assert!(
            field
                .trim_end()
                .trim_end_matches(['│', ' '])
                .ends_with("at all"),
            "the end scrolled off: {screen:?}"
        );
    }

    /// The whole screen goes ASCII together, the input line included.
    #[test]
    fn the_input_line_has_an_ascii_form_too() {
        let input = Input::new("milk @tomorrow".to_string(), None);
        let screen = with_input(62, 7, &tasks(&["a"]), &input, Glyphs::Ascii);
        let text = screen.join("\n");

        assert!(text.contains(" add |milk @tomorrow"), "{text}");
        assert!(text.contains("ret save   esc cancel"), "{text}");
        assert!(
            text.is_ascii(),
            "something non-ASCII reached the screen: {text}"
        );
    }

    /// The input is the one thing that moves the list, and it moves it by
    /// exactly the row it borrowed — docs/decisions.md#reversed.
    #[test]
    fn the_input_covers_the_middle_and_moves_nothing() {
        let tasks = tasks(&["a @2026-08-10", "b", "c", "d"]);
        let quiet = rendered(40, 10, &tasks);
        let busy = with_input(40, 10, &tasks, &Input::adding(), Glyphs::Unicode);

        // The box covers four rows of the middle. Everything outside it is the
        // screen the reader was already looking at — the list does not scroll,
        // reflow or give up a row, which is what it did when the input lived on
        // the bottom line.
        assert_eq!(quiet[..2], busy[..2], "the list shifted under the reader");
        assert_eq!(quiet[6..9], busy[6..9], "the list shifted under the reader");
        assert!(
            busy[8].starts_with('└'),
            "the frame lost its foot: {busy:?}"
        );
        assert!(
            busy[2..6].iter().all(|r| r.contains('│')),
            "the box is not where it should be: {busy:?}"
        );
        assert!(busy[3].contains(" add ▏"), "{busy:?}");
        // The keys move off the hint bar for as long as the box is open: `a`
        // and `d` are letters in there, and naming them would be a lie.
        assert!(busy[9].contains("esc cancel"), "{busy:?}");
        assert!(!busy[9].contains("spc done"), "{busy:?}");
    }

    /// Where the cursor is, which is the only thing saying where the next
    /// character will land. It is the terminal's own — it blinks like every
    /// other text field — so nothing on the screen would show it missing.
    #[test]
    fn the_cursor_follows_the_end_of_what_has_been_typed() {
        let tasks = tasks(&["a @2026-08-10"]);
        let at = |text: &str, height: u16| {
            let groups = agenda(&tasks, today());
            let mut screen = Screen::new(rows(&groups));
            let counts = Counts::of(&tasks, today());
            let input = Input::new(text.to_string(), None);
            let mut terminal = Terminal::new(TestBackend::new(40, height)).unwrap();
            terminal
                .draw(|f| {
                    draw(
                        f,
                        &mut screen,
                        counts,
                        render(crate::theme::MOCHA),
                        &Notice::Hints,
                        false,
                        Some(&input),
                    )
                })
                .unwrap();
            let p = terminal.get_cursor_position().unwrap();
            (p.x, p.y)
        };

        // The box is 36 wide on a 40-column pane and starts two in, so the
        // field begins three columns further along: `│ add ▏`.
        assert_eq!(at("", 10), (9, 3));
        assert_eq!(at("milk", 10), (13, 3));
        // A pane with no room for the box draws no field, so the cursor is not
        // sent off to point at one that is not there.
        assert_eq!(at("milk", 1), (0, 0));
    }

    /// A pane dragged down to two rows keeps a row of list: the preview is the
    /// half of the input that can be given up, and the field is not.
    #[test]
    fn a_pane_too_short_for_the_preview_still_shows_the_field() {
        let tasks = tasks(&["a @2026-08-10"]);
        for height in [1u16, 2, 3, 4, 5] {
            let screen = with_input(40, height, &tasks, &Input::adding(), Glyphs::Unicode);
            assert_eq!(screen.len(), height as usize);
            // However short the pane gets, the two keys that end the input are
            // in the same place: the box is what gives way, not the way out.
            // One row is the exception the bottom line already made — a lone
            // hint bar helps less than a lone task does.
            assert!(
                height == 1 || screen[height as usize - 1].contains("esc cancel"),
                "{height}: {screen:?}"
            );
        }

        // Four rows: one for the bottom line leaves three, which is a border,
        // the field, and a border. The preview is the row that goes.
        let short = with_input(40, 4, &tasks, &Input::adding(), Glyphs::Unicode);
        assert!(short[1].contains(" add ▏"), "{short:?}");
        // Three rows leave two, and two rows are both border: nothing is drawn
        // rather than a box with nowhere to type in it.
        let shorter = with_input(40, 3, &tasks, &Input::adding(), Glyphs::Unicode);
        assert!(!shorter.iter().any(|r| r.contains("add")), "{shorter:?}");
    }

    /// One row is held back whatever happens, so a message never pushes the list
    /// up under the reader.
    #[test]
    fn the_list_is_the_same_height_whatever_the_bottom_line_says() {
        let tasks = tasks(&["a @2026-08-01", "b", "c", "d"]);
        let quiet = rendered_notice(40, 9, &tasks, &Notice::Hints);
        let noisy = rendered_notice(40, 9, &tasks, &Notice::Warned("careful".into()));

        assert_eq!(quiet[..8], noisy[..8], "the list moved to make room");
        assert!(noisy[8].contains("careful"), "{noisy:?}");
    }

    fn two_groups() -> Vec<Task> {
        in_section(&[("deploy", "Work"), ("invoice", "Work"), ("plumber", "Home")])
    }

    #[test]
    fn folding_hides_a_group_and_says_how_much_it_is_hiding() {
        let tasks = two_groups();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        assert_eq!(screen.fold(Fold::Close), None, "nothing was folded");
        assert_eq!(
            titles(&screen.rows),
            ["# ## Work", "", "# ## Home", "plumber"],
            "the tasks are gone but the heading stayed"
        );
        assert!(
            matches!(
                &screen.rows[0],
                Row::Header {
                    hidden: Some(2),
                    ..
                }
            ),
            "a collapsed group that does not say how much it hides is a dead end"
        );

        assert_eq!(screen.fold(Fold::Open), None);
        assert_eq!(
            titles(&screen.rows),
            ["# ## Work", "deploy", "invoice", "", "# ## Home", "plumber"]
        );
    }

    /// The collapsed header drawn exactly. Its rule has to stop short of the
    /// `l`, and with the key absent on an open header that is arithmetic no
    /// other test looks at — a mutant lived there.
    #[test]
    fn a_folded_header_leaves_room_for_the_key_that_opens_it() {
        let tasks = two_groups();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        screen.fold(Fold::Close);

        let counts = Counts::of(&tasks, today());
        let mut terminal = Terminal::new(TestBackend::new(44, 8)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    false,
                    None,
                )
            })
            .unwrap();

        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content()
            .chunks(44)
            .map(|r| r.iter().map(|c| c.symbol()).collect())
            .collect();

        assert_eq!(
            rows,
            [
                "┌ ratodo — 3 · 0! ─────────────────────────┐",
                "│▌ ## Work (2) ───────────────────────── l │",
                "│  ## Home ─────────────────────────────── │",
                "│  ○ plumber                               │",
                "│                                          │",
                "│                                          │",
                "└──────────────────────────────────────────┘",
                " ?                                          ",
            ]
        );
    }

    /// The same header past sixty columns, where the rule now stops at the
    /// title column. The `l` has to come with it — a key stranded at the right
    /// edge of a rule that ended thirty columns ago is not an instruction, and
    /// nothing else on the screen would have caught it.
    #[test]
    fn a_folded_header_keeps_its_key_beside_the_shortened_rule() {
        let tasks = two_groups();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        screen.fold(Fold::Close);

        let counts = Counts::of(&tasks, today());
        let mut terminal = Terminal::new(TestBackend::new(84, 6)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    false,
                    None,
                )
            })
            .unwrap();

        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content()
            .chunks(84)
            .map(|r| r.iter().map(|c| c.symbol()).collect())
            .collect();

        assert_eq!(
            rows,
            [
                "┌ ratodo — 3 open · 0 overdue ─────────────────────────────────────────────────────┐",
                "│▌ ## Work (2)  l                                                                  │",
                "│                                                                                  │",
                "│  ## Home ─────                                                                   │",
                "└──────────────────────────────────────────────────────────────────────────────────┘",
                " ?                                                                                  ",
            ]
        );

        // Both rules stop at the title column, and with only `plumber` left to
        // measure that column is the twelve-column floor, not seven.
        let visible = crate::ui::rows(&agenda(&tasks, today()));
        assert_eq!(
            Columns::of(&visible, 80, render(crate::theme::MOCHA), Size::Wide).title,
            12
        );
    }

    #[test]
    fn z_is_whichever_of_the_two_is_the_opposite_of_now() {
        let tasks = two_groups();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        screen.fold(Fold::Toggle);
        assert_eq!(titles(&screen.rows).len(), 4);
        screen.fold(Fold::Toggle);
        assert_eq!(titles(&screen.rows).len(), 6);
    }

    /// Asking twice is not an error, but it is not a change either — and the
    /// caller needs to know, or `l` on an open group looks like a broken key.
    #[test]
    fn folding_what_is_already_folded_reports_that_nothing_happened() {
        let tasks = two_groups();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        assert_eq!(screen.fold(Fold::Close), None);
        assert_eq!(screen.fold(Fold::Close), Some("already folded"));
        assert_eq!(screen.fold(Fold::Open), None);
        assert_eq!(
            screen.fold(Fold::Open),
            Some("nothing folded here"),
            "`l` on an open group has to say so, not sit there"
        );
    }

    /// The cursor was inside the group that just closed. Sending it to the top
    /// of the list is the one thing a side pane must not do.
    #[test]
    fn folding_under_the_cursor_leaves_it_nearby_not_at_the_top() {
        let tasks = two_groups();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        screen.move_by(1);
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("invoice"));

        screen.fold(Fold::Close);
        assert!(
            screen.task().is_none(),
            "the cursor should be on the collapsed group itself"
        );
        assert_eq!(screen.selected(), Some(0), "and not at the top by accident");
        assert!(matches!(
            screen.rows[screen.selected().unwrap()],
            Row::Header {
                hidden: Some(2),
                ..
            }
        ));

        // And from there `l` opens it again and steps back inside, which is the
        // only route back: there is nothing else on screen to put a cursor on.
        assert_eq!(screen.fold(Fold::Open), None);
        assert_eq!(screen.task().map(|t| t.title.as_str()), Some("deploy"));
    }

    /// A run of tasks above the file's first heading has no header to collapse
    /// into, and saying so beats a key that silently does nothing.
    #[test]
    fn a_group_with_no_heading_cannot_be_folded() {
        let tasks = tasks(&["a", "b"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));

        assert_eq!(screen.fold(Fold::Close), Some("no group to fold here"));
        assert_eq!(titles(&screen.rows), ["a", "b"]);
    }

    /// `ratodo add` in another pane fires the watcher, which reloads. A fold
    /// that came undone every time anything touched the file would be useless.
    #[test]
    fn a_fold_survives_a_reload() {
        let before = two_groups();
        let groups = agenda(&before, today());
        let mut screen = Screen::new(rows(&groups));
        screen.fold(Fold::Close);

        let after = in_section(&[
            ("deploy", "Work"),
            ("invoice", "Work"),
            ("arrived from outside", "Work"),
            ("plumber", "Home"),
        ]);
        let groups = agenda(&after, today());
        screen.replace(rows(&groups));

        assert!(
            matches!(
                &screen.rows[0],
                Row::Header {
                    hidden: Some(3),
                    ..
                }
            ),
            "the fold came undone, or the new task was not counted: {:?}",
            titles(&screen.rows)
        );
    }

    /// The first thing a new user sees. It has to teach — and it has to say
    /// where the file is, because the promise of this product is that the file
    /// is theirs.
    #[test]
    fn an_empty_list_teaches_instead_of_apologising() {
        let screen = rendered(60, 12, &[]);
        let text = screen.join("\n");

        assert!(text.contains("Nothing here yet"), "{text}");
        assert!(
            text.contains("~/.config/ratodo/todo.md"),
            "the path is missing: {text}"
        );
        assert!(
            text.contains("buy milk @tomorrow #home"),
            "the worked example is what actually teaches the syntax: {text}"
        );
        assert!(text.contains("$EDITOR"), "{text}");
    }

    /// The example sits in the box it will be typed into, and the line under it
    /// has already resolved the shorthand: `@tomorrow` is a date before anybody
    /// has pressed a key. That resolution is the whole reason the box is there.
    #[test]
    fn the_empty_screen_shows_the_example_in_a_real_input_box() {
        let screen = rendered(60, 16, &[]);
        let text = screen.join("\n");

        assert!(
            text.contains(&format!("add ▏{EXAMPLE}")),
            "the field is not the one `a` opens: {text}"
        );
        assert!(
            text.contains("due tomorrow (2026-08-11)"),
            "the shorthand was left unresolved: {text}"
        );
        assert!(
            screen.iter().filter(|r| r.contains('┌')).count() == 2,
            "the box did not draw inside the frame: {screen:?}"
        );

        // Six rows of text and four of box: below that the example goes back to
        // being a line, because losing it entirely is the one thing a short pane
        // must not do.
        let short = rendered(60, 12, &[]).join("\n");
        assert!(!short.contains("add ▏"), "the box did not fit: {short}");
        assert!(
            short.contains(&format!("Try:  a  then  {EXAMPLE}")),
            "{short}"
        );

        let mut empty = Screen::new(vec![]);
        let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut empty,
                    Counts::default(),
                    Render {
                        glyphs: Glyphs::Ascii,
                        ..render(crate::theme::MOCHA)
                    },
                    &Notice::Hints,
                    false,
                    None,
                )
            })
            .unwrap();
        let ascii: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(ascii.contains("add |"), "{ascii}");
        assert!(ascii.is_ascii(), "something non-ASCII reached it: {ascii}");
    }

    /// A `--file` path can be arbitrarily long. It gets shortened, because a
    /// broken right edge on the first screen somebody sees is a poor
    /// introduction.
    #[test]
    fn a_long_path_on_the_empty_screen_does_not_break_the_frame() {
        let long = "/home/somebody/very/deeply/nested/dotfiles/config/ratodo/todo.md";
        let counts = Counts::default();
        let mut screen = Screen::new(vec![]);
        let mut terminal = Terminal::new(TestBackend::new(50, 12)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    Render {
                        path: long,
                        ..render(crate::theme::MOCHA)
                    },
                    &Notice::Hints,
                    false,
                    None,
                )
            })
            .unwrap();

        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content()
            .chunks(50)
            .map(|r| r.iter().map(|c| c.symbol()).collect())
            .collect();

        let frame = rows.len() - 2;
        for row in &rows[1..frame] {
            assert!(row.ends_with('│'), "the frame broke: {rows:?}");
        }
        assert!(rows.iter().any(|r| r.contains('…')), "{rows:?}");
    }

    /// A file whose tasks are all filtered away is still a list, not an empty
    /// one — but a document of nothing but prose has nothing to show.
    #[test]
    fn a_document_with_no_tasks_at_all_gets_the_empty_screen() {
        let with_headers = Screen::new(vec![Row::header("Work"), Row::Spacer]);
        let counts = Counts::default();
        let mut screen = with_headers;
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    false,
                    None,
                )
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Nothing here yet"), "{text}");
    }

    /// A pane can be dragged down to one row. Holding a line back out of two is
    /// arithmetic that underflows if it is written the obvious way.
    #[test]
    fn a_pane_too_short_for_a_bottom_line_still_draws() {
        let tasks = tasks(&["a @2026-08-01"]);

        for height in [1u16, 2, 3] {
            let screen = rendered(40, height, &tasks);
            assert_eq!(screen.len(), height as usize);
            assert!(
                screen.iter().any(|r| !r.trim().is_empty()),
                "nothing was drawn at {height} rows: {screen:?}"
            );
        }

        // One row goes to the list, not to the hint bar: what fits there is the
        // frame and its counts, which is more use than " ?" on its own.
        assert!(
            rendered(40, 1, &tasks)[0].starts_with('┌'),
            "the notice took the only row: {:?}",
            rendered(40, 1, &tasks)
        );
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
