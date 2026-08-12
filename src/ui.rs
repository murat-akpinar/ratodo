//! ratatui drawing. See docs/tui.md.

use chrono::{Datelike, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph};

use crate::agenda::{Counts, Group, Kind, Period, Stats};
use crate::capture::Part;
use crate::model::{Priority, State, Task};
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
    /// `a` — the add form, wherever the pane has room for it, and the one-line
    /// box where it does not — docs/decisions.md.
    Add,
    /// `o` — the one-line box, always. A vim user reaches for `o` to open a new
    /// line, which is the fast path, so it keeps being the fast path rather than
    /// following `a` onto a screen.
    Quick,
    /// `⏎` — the same input, pre-filled with the selected task.
    Change,
    /// `y` — the same input again, pre-filled with a *copy* of the selected
    /// task, so a near-duplicate is an edit rather than a retype. What comes
    /// back is a new task: nothing is written until `⏎`, and the file is not
    /// touched at all if the box is cancelled.
    Duplicate,
    /// `h` `l` `z` — collapse or open the group under the cursor.
    Fold(Fold),
    /// `X` — immediately, with `u` to take it back.
    Delete,
    /// `d` — decided against rather than finished. `- [-]` in the file.
    Cancel,
    /// `p` — opens the input to ask how long for, and moves the date alone.
    Postpone,
    /// `u` — put the last change back.
    Undo,
    /// Hand the terminal to `$EDITOR`. The escape hatch for everything the
    /// tool cannot do — docs/product.md#product-decisions.
    Edit,
    Reload,
    /// Opens the key help, and closes it again — the only overlay in the
    /// product, and the only place a popup is the right answer.
    Help,
    /// `s` — opens the stats screen, and closes it again. A **screen** and not
    /// an overlay: it replaces the list rather than covering it, because
    /// nothing on it is glanced at mid-task — docs/tui.md#stats.
    Stats,
    /// `1` `2` `3` on the stats screen. They do nothing on the list, which is
    /// where the loop reads them and drops them.
    Over(Period),
    /// `esc`: closes the overlay, and does nothing at all otherwise. It must
    /// never quit — somebody pressing it out of habit keeps their pane.
    Close,
    /// A key that is bound to nothing on purpose but still owes an answer.
    /// Silence reads as a broken program — docs/tui.md#deliberately-unbound.
    Say(&'static str),
    Ignore,
}

/// Whether ctrl is down **as a chord**.
///
/// Windows reports AltGr as ctrl+alt, and AltGr is how `#`, `@` and `$` are
/// typed on the Turkish, German and Polish layouts — three characters this
/// program's own syntax is built out of. Both modifiers at once is a keyboard
/// layout, not a chord, and the character it produced is text.
fn chord(modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::CONTROL) && !modifiers.contains(KeyModifiers::ALT)
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
    let ctrl = chord(key.modifiers);

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
        KeyCode::Char('d') => Action::Cancel,
        KeyCode::Char('u') => Action::Undo,
        KeyCode::Char(' ') => Action::Toggle,
        KeyCode::Char('a') => Action::Add,
        KeyCode::Char('o') => Action::Quick,
        KeyCode::Enter => Action::Change,
        KeyCode::Char('h') | KeyCode::Left => Action::Fold(Fold::Close),
        KeyCode::Char('l') | KeyCode::Right => Action::Fold(Fold::Open),
        // `z` is the vim fold prefix, and here it is the whole of it.
        KeyCode::Char('z') => Action::Fold(Fold::Toggle),
        // Delete is the only irreversible-looking key on the list, so it is the
        // one that asks for shift. `d` is the reversible neighbour — docs/tui.md#keys.
        KeyCode::Char('X') => Action::Delete,
        KeyCode::Char('p') => Action::Postpone,
        KeyCode::Char('y') => Action::Duplicate,
        KeyCode::Char('e') => Action::Edit,
        KeyCode::Char('r') => Action::Reload,
        // The second screen. `s` opens it and `s` closes it again, the way `?`
        // works — one key, one place, and no way to be lost in it.
        KeyCode::Char('s') => Action::Stats,
        KeyCode::Char('1') => Action::Over(Period::Week),
        KeyCode::Char('2') => Action::Over(Period::Month),
        KeyCode::Char('3') => Action::Over(Period::Year),
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
    pub purpose: Purpose,
    /// The date field, while `tab` has it open. `None` is the whole of the box
    /// as it was: one line of text — docs/decisions.md#settled.
    pub field: Option<DateField>,
    /// The date `a` put in the box, for as long as it is still in the line and
    /// nobody has argued with it. `capture` gives the line to the **first** `@`,
    /// and the first one here is the one nobody typed — so a typed `@thu` takes
    /// this one's place rather than losing to it and leaving `@thu` sitting in
    /// the title. docs/tui.md#adding.
    opened_with: Option<String>,
}

/// `DD MM YYYY` with one part under the cursor.
///
/// Held as three numbers rather than as a `NaiveDate`, because a date being
/// edited passes through states a date cannot hold: the 31st on its way to a
/// month with thirty days. Every edit puts it back — `day` is clamped to the
/// month it is in — so what the field hands back is always a real day, which is
/// the entire reason it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateField {
    day: u32,
    month: u32,
    year: i32,
    part: DatePart,
    /// Digits typed into `part` since it took the cursor. Two fill a day or a
    /// month and four fill a year, and then the cursor moves on by itself —
    /// which is what makes `13082026` one gesture rather than seven.
    digits: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatePart {
    Day,
    Month,
    Year,
}

/// Why the field is open, and so what `⏎` will do with the line in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    /// `a` `o` — a new task, for the capture target.
    Add,
    /// `y` — also a new task, and the box says so in the accent: the line under
    /// the cursor is filled in but is **not** the line `⏎` will rewrite, and
    /// that is the one thing somebody who pressed `y` has to know.
    Copy,
    /// `⏎` — the raw line being rewritten.
    Edit(String),
    /// `p` — the raw line whose date is moving. What is being typed is a length
    /// of time, not a task, which is why it is a third purpose and not a flag on
    /// the second.
    Postpone(String),
}

impl Purpose {
    /// The line this is about, as the file has it. `None` for a new task.
    pub fn raw(&self) -> Option<&str> {
        match self {
            Purpose::Add | Purpose::Copy => None,
            Purpose::Edit(raw) | Purpose::Postpone(raw) => Some(raw),
        }
    }

    /// Upper case, the way the group headings on the list are: both are the
    /// tool's own word rather than the user's, and the screen says so the same
    /// way twice — docs/design.md#what-each-colour-means.
    fn label(&self) -> &'static str {
        match self {
            Purpose::Add => "ADD",
            Purpose::Copy => "COPY",
            Purpose::Edit(_) => "EDIT",
            Purpose::Postpone(_) => "PUT OFF",
        }
    }
}

/// How many days a month has, asked of the calendar rather than of a table:
/// the leap year comes free and there is nothing to get wrong.
fn days_in(year: i32, month: u32) -> u32 {
    (28..=31)
        .rev()
        .find(|day| NaiveDate::from_ymd_opt(year, month, *day).is_some())
        .unwrap_or(28)
}

impl DateField {
    fn new(date: NaiveDate) -> Self {
        DateField {
            day: date.day(),
            month: date.month(),
            year: date.year(),
            part: DatePart::Day,
            digits: 0,
        }
    }

    /// Always a real day: the parts are only ever set through here, and the day
    /// is clamped to the month it has landed in.
    fn date(self) -> NaiveDate {
        let mut settled = self;
        settled.settle();
        NaiveDate::from_ymd_opt(settled.year, settled.month, settled.day).unwrap_or_default()
    }

    /// Puts the three numbers back into a date.
    ///
    /// A part being typed passes through states a date cannot hold — `0` on the
    /// way to `05`, the 31st on the way into February — and this is where every
    /// one of them ends. Called on the way out of a part and again before the
    /// date is read, which is every path there is, which is what makes the
    /// fallback above unreachable rather than a guess.
    fn settle(&mut self) {
        self.month = self.month.clamp(1, 12);
        self.year = self.year.clamp(1970, 9999);
        self.day = self.day.clamp(1, days_in(self.year, self.month));
    }

    /// `↑` and `↓`. The day and the month wrap, because their ends are next to
    /// each other on a calendar; the year does not, because 9999 and 1970 are
    /// not neighbours and landing on one from the other is never what was meant.
    fn step(&mut self, by: i32) {
        self.settle();
        self.digits = 0;
        match self.part {
            DatePart::Day => {
                let last = days_in(self.year, self.month);
                self.day = wrap(self.day, by, 1, last);
            }
            DatePart::Month => {
                self.month = wrap(self.month, by, 1, 12);
                self.day = self.day.min(days_in(self.year, self.month));
            }
            DatePart::Year => {
                self.year = (self.year + by).clamp(1970, 9999);
                self.day = self.day.min(days_in(self.year, self.month));
            }
        }
    }

    /// `←` and `→`. The cursor stops at the ends rather than cycling: three
    /// fields are few enough to see, and a wrap would move it somewhere the eye
    /// was not.
    fn move_to(&mut self, forward: bool) {
        self.settle();
        self.digits = 0;
        self.part = match (self.part, forward) {
            (DatePart::Day, true) => DatePart::Month,
            (DatePart::Month, true) => DatePart::Year,
            (DatePart::Year, false) => DatePart::Month,
            (DatePart::Month, false) => DatePart::Day,
            (part, _) => part,
        };
    }

    /// One digit of `DDMMYYYY`.
    ///
    /// A part that is full, or that could not hold another digit, hands the
    /// cursor on by itself — so eight digits are eight keystrokes and no arrows.
    /// A digit that would make the part impossible closes it and starts the next
    /// one, which is how `4` `5` in the day reads as the 4th of May.
    fn digit(&mut self, d: u32) {
        let (max, width) = match self.part {
            DatePart::Day => (days_in(self.year, self.month), 2),
            DatePart::Month => (12, 2),
            DatePart::Year => (9999, 4),
        };
        let current = match self.part {
            DatePart::Day => self.day,
            DatePart::Month => self.month,
            DatePart::Year => self.year as u32,
        };

        let next = if self.digits == 0 {
            d
        } else {
            current * 10 + d
        };
        if next > max {
            // No room for it here. The part keeps what it has and the digit
            // starts the next one — unless there is no next one, and then it is
            // a keystroke with nowhere to go.
            if self.part == DatePart::Year {
                return;
            }
            self.move_to(true);
            self.digit(d);
            return;
        }

        // Stored raw, zero and all: `0` is how `05` starts, and a part that
        // clamped as it was typed could never reach it.
        self.digits += 1;
        match self.part {
            DatePart::Day => self.day = next,
            DatePart::Month => self.month = next,
            DatePart::Year => self.year = next as i32,
        }

        // Full, or one digit short of a number that cannot take another: `4` in
        // the day is finished, since `40` is not a day.
        if self.digits == width || next * 10 > max {
            self.move_to(true);
        }
    }
}

impl DateField {
    /// The three parts as they are drawn: `[13] 08  2026`.
    ///
    /// The brackets are always there and always around exactly one part, so the
    /// row is the same width wherever the cursor is. A marker that moved the
    /// digits sideways would be a row that twitches on every `←`.
    fn cells(self) -> [(DatePart, String); 3] {
        let cell = |part: DatePart, text: String| {
            let text = match part == self.part {
                true => format!("[{text}]"),
                false => format!(" {text} "),
            };
            (part, text)
        };
        [
            cell(DatePart::Day, format!("{:02}", self.day)),
            cell(DatePart::Month, format!("{:02}", self.month)),
            cell(DatePart::Year, format!("{:04}", self.year)),
        ]
    }
}

/// One step around a closed range, in either direction.
fn wrap(value: u32, by: i32, low: u32, high: u32) -> u32 {
    let span = (high - low + 1) as i32;
    let at = (value as i32 - low as i32 + by).rem_euclid(span);
    low + at as u32
}

impl Input {
    /// The only way to build one: the caret starts at the end of `text`, which
    /// is where a retype begins.
    pub fn new(text: String, purpose: Purpose) -> Self {
        Input {
            at: text.len(),
            text,
            purpose,
            field: None,
            opened_with: None,
        }
    }

    /// Pre-filled with today, because that is the date a new task has more often
    /// than every other date put together, and the box is where it is cheapest
    /// to change — docs/tui.md#adding.
    ///
    /// **Behind the caret, not in front of it.** The date is the field the tool
    /// guessed; the title is the one the user came to type, and it goes where
    /// the line puts it — first, the way the written line has it and the way the
    /// row on the screen reads.
    pub fn adding(today: NaiveDate) -> Self {
        let opening = format!(" @{today}");
        Input {
            at: 0,
            opened_with: Some(opening.clone()),
            ..Input::new(opening, Purpose::Add)
        }
    }

    /// Pre-filled with the task's text as it stands in the file, so an edit
    /// starts from what is actually written there rather than from our reading
    /// of it.
    pub fn editing(task: &Task) -> Self {
        Input::new(task.body().to_string(), Purpose::Edit(task.raw.clone()))
    }

    /// Empty, because the answer is short and pre-filling it with a guess would
    /// make the common case *delete* something before typing.
    pub fn postponing(task: &Task) -> Self {
        Input::new(String::new(), Purpose::Postpone(task.raw.clone()))
    }

    /// Pre-filled the same way as an edit, but as a new task: `Purpose::Add`,
    /// so `⏎` captures instead of rewriting the line it was copied from.
    ///
    /// The copy is re-opened first, which is what takes the completion stamp
    /// back off: `capture` has never heard of `✓2026-08-11` and would have left
    /// it sitting in the new task's title. Reusing `set_state` rather than
    /// stripping the word here keeps one definition of where the stamp lives.
    pub fn duplicating(task: &Task, today: NaiveDate) -> Self {
        let mut copy = task.clone();
        copy.set_state(State::Open, today);
        Input::new(copy.body().to_string(), Purpose::Copy)
    }

    pub fn insert(&mut self, c: char) {
        // While the field is open the digits are positional, and everything
        // else is a key with nowhere to go. That is what keeps `12` from having
        // to mean both December and twelve — docs/decisions.md#settled.
        if let Some(field) = self.field.as_mut() {
            if let Some(d) = c.to_digit(10) {
                field.digit(d);
            }
            return;
        }
        if c == '@' {
            self.drop_the_opening_date();
        }
        self.text.insert(self.at, c);
        self.at += c.len_utf8();
    }

    /// Takes the date `a` opened with back out, because the user is typing one.
    /// Once, and only while the line still holds it untouched: after that it is
    /// their date and not ours.
    fn drop_the_opening_date(&mut self) {
        let Some(opening) = self.opened_with.take() else {
            return;
        };
        let Some(from) = self.text.find(&opening) else {
            return;
        };
        let to = from + opening.len();
        self.text.replace_range(from..to, "");
        self.at = match self.at {
            at if at >= to => at - opening.len(),
            at => at.min(from),
        };
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
        if let Some(field) = self.field.as_mut() {
            field.move_to(false);
            return;
        }
        if let Some(c) = self.text[..self.at].chars().next_back() {
            self.at -= c.len_utf8();
        }
    }

    pub fn right(&mut self) {
        if let Some(field) = self.field.as_mut() {
            field.move_to(true);
            return;
        }
        if let Some(c) = self.text[self.at..].chars().next() {
            self.at += c.len_utf8();
        }
    }

    /// `↑` and `↓`, which mean nothing at all until the field is open.
    pub fn step(&mut self, by: i32) {
        if let Some(field) = self.field.as_mut() {
            field.step(by);
        }
    }

    pub fn home(&mut self) {
        self.at = 0;
    }

    pub fn end(&mut self) {
        self.at = self.text.len();
    }

    /// `tab`. Opens the date field on the date the line already has, and closes
    /// it the way `⏎` does — in and out with one key.
    ///
    /// The starting date is the one in the box if there is one, and today if
    /// there is not: a field that opens on the 1st of January makes you arrow
    /// back to where you already were.
    pub fn toggle_field(&mut self, today: NaiveDate) {
        if self.field.is_some() {
            self.apply_field();
            return;
        }
        // Through `capture` rather than by looking for an `@` here: the date the
        // field opens on has to be the date the line actually means, and there
        // is one function that decides that.
        let from = match &self.purpose {
            Purpose::Postpone(_) => crate::capture::later(&self.text, today),
            _ => crate::capture::capture(&self.text, today)
                .due
                .map(|d| d.date),
        };
        self.field = Some(DateField::new(from.unwrap_or(today)));
    }

    /// `⏎` with the field open: the date goes into the line and the keyboard
    /// goes back to the text. `true` when that is what happened, so that the
    /// same key still saves when the field is closed.
    pub fn apply_field(&mut self) -> bool {
        let Some(field) = self.field.take() else {
            return false;
        };
        let date = field.date();
        self.text = match &self.purpose {
            // `p` asks how long, and the one form it takes past its year
            // horizon is a date written out. There is nothing else in the box
            // to keep.
            Purpose::Postpone(_) => date.to_string(),
            _ => replace_date(&self.text, &format!("@{date}")),
        };
        self.at = self.text.len();
        true
    }

    /// `esc` with the field open: the field goes and the line is untouched.
    /// `true` when that is what happened — one `esc` per thing that is open.
    pub fn close_field(&mut self) -> bool {
        self.field.take().is_some()
    }
}

/// The line with its `@word` swapped for `with`, or `with` appended when there
/// is none.
///
/// The first `@` word, which is the one `capture` reads as the date. A word we
/// did not understand is still the user's, so this replaces exactly one word
/// and leaves the spacing of everything around it alone.
fn replace_date(text: &str, with: &str) -> String {
    let found = crate::capture::words(text)
        .into_iter()
        .find(|(_, word)| word.starts_with('@'));
    match found {
        Some((at, word)) => format!("{}{with}{}", &text[..at], &text[at + word.len()..]),
        None if text.trim().is_empty() => with.to_string(),
        None => format!("{} {with}", text.trim_end()),
    }
}

/// Replaces whatever `capture::parts` claims as one of `wanted` with `to`, and
/// removes it when `to` is `None`.
///
/// **The form never guesses where a field is in the line.** It asks the same
/// tokenizer the live preview reads, so the screen and the parse cannot disagree
/// about what is going to be written — which is the invariant the labelled-field
/// box was rejected for breaking, kept here rather than argued around:
/// docs/decisions.md.
///
/// A removal takes **one adjacent space** with it. Without that a line loses a
/// field and keeps a double space, and does it again on the next edit.
fn set_parts(text: &str, today: NaiveDate, wanted: &[Part], to: Option<&str>) -> String {
    let to = to.filter(|t| !t.is_empty());
    let claimed: Vec<std::ops::Range<usize>> = crate::capture::parts(text, today)
        .into_iter()
        .filter(|(_, part)| wanted.contains(part))
        .map(|(range, _)| range)
        .collect();

    let Some(first) = claimed.first().cloned() else {
        // Nothing to replace, so the only case left is adding one — and that is
        // the one place the tool chooses a position. The end of the line, which
        // is where `capture` puts a new task's fields anyway.
        return match (to, text.trim_end().is_empty()) {
            (None, _) => text.to_string(),
            (Some(to), true) => to.to_string(),
            (Some(to), false) => format!("{} {to}", text.trim_end()),
        };
    };

    let mut out = String::with_capacity(text.len() + 16);
    let mut at = 0;
    for range in &claimed {
        let keep = &text[at..range.start];
        match to.filter(|_| *range == first) {
            Some(word) => {
                out.push_str(keep);
                out.push_str(word);
                at = range.end;
            }
            // The word goes, and one space with it: the following one where
            // there is one, the preceding one otherwise.
            None => {
                let trailing = text[range.end..].starts_with(' ');
                out.push_str(match (trailing, keep.ends_with(' ')) {
                    (false, true) => &keep[..keep.len() - 1],
                    _ => keep,
                });
                at = range.end + usize::from(trailing);
            }
        }
    }
    out.push_str(&text[at.min(text.len())..]);
    out
}

/// A time goes **directly after the date**, because that is the only place
/// `capture` reads one: on its own it is words in a title, and `09:30 standup`
/// is a title somebody typed. `text` has no time in it — the caller took it out.
fn after_date(text: &str, today: NaiveDate, time: &str) -> String {
    if time.is_empty() {
        return text.to_string();
    }
    match crate::capture::parts(text, today)
        .into_iter()
        .find(|(_, part)| *part == Part::Date)
    {
        Some((range, _)) => format!(
            "{}{} {time}{}",
            &text[..range.start],
            &text[range.clone()],
            &text[range.end..]
        ),
        // Unreachable through the form — `Time` is not in the tab order without
        // a date — and appending is the only honest answer if it ever is.
        None => format!("{} {time}", text.trim_end()),
    }
}

/// The tags, which are a set rather than a token and so get their own function:
/// the whole set is cleared and written back as one word run.
fn set_tags(text: &str, today: NaiveDate, tags: &str) -> String {
    let written = tags
        .split_whitespace()
        .map(|word| match word.starts_with('#') {
            true => word.to_string(),
            false => format!("#{word}"),
        })
        .collect::<Vec<_>>()
        .join(" ");
    let cleared = set_parts(text, today, &[Part::Tag], None);
    match (written.is_empty(), cleared.trim_end().is_empty()) {
        (true, _) => cleared,
        (false, true) => written,
        // On the end rather than where the first one was: an emptied set has no
        // position of its own, and the end is the one place this tool chooses.
        (false, false) => format!("{} {written}", cleared.trim_end()),
    }
}

/// What the line currently says, read back through the same tokenizer. The form
/// stores none of this: every radio and every sub-field is a **view** of the one
/// string, which is what keeps one tokenizer and one truth — docs/decisions.md.
fn part_of(text: &str, today: NaiveDate, want: Part) -> Option<String> {
    crate::capture::parts(text, today)
        .into_iter()
        .find(|(_, part)| *part == want)
        .map(|(range, _)| text[range].to_string())
}

/// Every tag in the line, space separated and with their `#`.
fn tags_of(text: &str, today: NaiveDate) -> String {
    crate::capture::parts(text, today)
        .into_iter()
        .filter(|(_, part)| *part == Part::Tag)
        .map(|(range, _)| &text[range])
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which control has the keyboard.
///
/// Six fields, and they are exactly the six the format already carries — title,
/// date, time, tags, priority and which list. There is no seventh, because there
/// is nowhere in a one-line format to put one: no Description, no Project and no
/// Section picker — docs/redesign.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    Due,
    Time,
    Priority,
    Tags,
    List,
    Cancel,
    Create,
}

impl Field {
    fn label(self) -> &'static str {
        match self {
            Field::Title => "",
            Field::Due => "Due",
            Field::Time => "Time",
            Field::Priority => "Priority",
            Field::Tags => "Tags",
            Field::List => "List",
            Field::Cancel => "",
            Field::Create => "",
        }
    }

    /// Whether it is typed into rather than chosen from.
    fn typed(self) -> bool {
        matches!(self, Field::Time | Field::Tags)
    }
}

/// The add screen. **The line is the model**: the text box holds the whole line
/// exactly as the one-line box does, and every row under it is a view of that
/// one string — each reads `capture::parts` to know what is selected and writes
/// back by replacing the span the tokenizer claimed.
///
/// That is what lets the form exist at all. The labelled-field box was rejected
/// because five fields mean either joining them back into a line, which makes
/// the boundaries decoration, or a second parser, which eventually disagrees
/// with the first about what is going to be written. This is neither —
/// docs/decisions.md.
#[derive(Debug)]
pub struct Form {
    pub input: Input,
    pub focus: Field,
    today: NaiveDate,
    /// The open lists, so `List` knows its options. One list or none means the
    /// row is not drawn and `tab` steps over it, exactly as `$list` is only
    /// offered when there is more than one — docs/tui.md#which-list--work.
    lists: Vec<String>,
    /// What is being typed into `Time` or `Tags` right now, seeded from the line
    /// when the focus lands and written back into it on every keystroke.
    ///
    /// The one piece of state that is not the line, and it is only ever what is
    /// *mid-typing*: a trailing space is a second tag on its way and the line
    /// cannot hold one. Leaving the field throws it away, because by then the
    /// line has it.
    typing: String,
    /// The line **without** whatever the focused sub-field owns, taken when the
    /// focus lands on it. Every keystroke rebuilds from this rather than editing
    /// what the last one left behind.
    ///
    /// It has to work this way because half a time is not a time: `capture`
    /// claims `09:30` and has never heard of `0`, `09` or `09:`, so a sync that
    /// looked for the token it wrote a keystroke ago would find nothing and
    /// append a second one. Five keystrokes, five words in the line.
    base: String,
}

impl Form {
    /// The pane the form needs. Under this `a` opens the one-line box instead —
    /// a form that half-fits is worse than a box that always fits, and the box
    /// is already built and already tested. docs/decisions.md.
    pub fn fits(area: Rect) -> bool {
        area.height >= 15 && area.width >= 40
    }

    pub fn adding(today: NaiveDate, lists: &[String]) -> Self {
        Form {
            input: Input::adding(today),
            focus: Field::Title,
            today,
            lists: lists.to_vec(),
            typing: String::new(),
            base: String::new(),
        }
    }

    /// The tab order, with the rows that have nothing to offer left out.
    ///
    /// `Time` goes when there is no date: the format cannot hold a time without
    /// one, so a row that accepted a time there would be a field the file cannot
    /// keep — docs/format.md.
    fn order(&self) -> Vec<Field> {
        let mut out = vec![Field::Title, Field::Due];
        if part_of(&self.input.text, self.today, Part::Date).is_some() {
            out.push(Field::Time);
        }
        out.extend([Field::Priority, Field::Tags]);
        if self.lists.len() > 1 {
            out.push(Field::List);
        }
        out.extend([Field::Cancel, Field::Create]);
        out
    }

    fn step_focus(&mut self, by: isize) {
        let order = self.order();
        let at = order.iter().position(|f| *f == self.focus).unwrap_or(0) as isize;
        let next = (at + by).rem_euclid(order.len() as isize) as usize;
        self.focus = order[next];
        // Seeded on arrival and thrown away on leaving: the line is where it
        // lives the rest of the time.
        let (text, today) = (self.input.text.clone(), self.today);
        let (typing, owns) = match self.focus {
            Field::Time => (
                part_of(&text, today, Part::Time).unwrap_or_default(),
                &[Part::Time][..],
            ),
            Field::Tags => (tags_of(&text, today), &[Part::Tag][..]),
            _ => (String::new(), &[][..]),
        };
        self.typing = typing;
        self.base = set_parts(&text, today, owns, None);
    }

    /// The options on the focused row, and which one is on. Built from the line
    /// every time rather than held: a radio that remembers what it was told is
    /// a second model of the same fact.
    fn choices(&self) -> Vec<(String, bool)> {
        self.choices_for(self.focus)
    }

    /// What is being typed into the focused sub-field right now.
    fn typed_text(&self) -> String {
        self.typing.clone()
    }

    fn choices_for(&self, field: Field) -> Vec<(String, bool)> {
        let text = &self.input.text;
        let today = self.today;
        match field {
            Field::Due => {
                let has = part_of(text, today, Part::Date);
                let named = |d: NaiveDate| format!("@{d}");
                let tomorrow = today.succ_opt().unwrap_or(today);
                let mut out = vec![
                    ("none".to_string(), has.is_none()),
                    ("today".to_string(), has.as_deref() == Some(&named(today))),
                    (
                        "tomorrow".to_string(),
                        has.as_deref() == Some(&named(tomorrow)),
                    ),
                ];
                // A date that is neither shows itself rather than nothing: the
                // form has to be able to say what the line already holds.
                if let Some(word) = has.filter(|w| *w != named(today) && *w != named(tomorrow)) {
                    out.push((word.trim_start_matches('@').to_string(), true));
                }
                // `pick` and not `pick…`: the ellipsis would be the one
                // character on this screen with no ASCII form, and plumbing the
                // glyph set into a model that is otherwise only the line is a
                // lot of wire for one dot.
                out.push(("pick".to_string(), false));
                out
            }
            Field::Priority => {
                let has = part_of(text, today, Part::Priority);
                let mut out = vec![("none".to_string(), has.is_none())];
                out.extend(["!high", "!med", "!low"].map(|p| {
                    (
                        p.trim_start_matches('!').to_string(),
                        has.as_deref() == Some(p),
                    )
                }));
                out
            }
            Field::List => {
                let has = crate::capture::list_of(text);
                self.lists
                    .iter()
                    .enumerate()
                    .map(|(n, name)| {
                        let on = match has {
                            Some(word) => crate::capture::names_list(word, name),
                            // No `$word` means the capture target, which is the
                            // first list — cli.md#several-lists rule 4.
                            None => n == 0,
                        };
                        (name.clone(), on)
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Turns the *n*th choice on, by writing it into the line.
    fn choose(&mut self, n: usize) {
        let text = self.input.text.clone();
        let today = self.today;
        let choices = self.choices();
        let Some((label, _)) = choices.get(n) else {
            return;
        };
        self.input.text = match self.focus {
            Field::Due => match label.as_str() {
                "none" => set_parts(&text, today, &[Part::Date, Part::Time], None),
                "today" => set_parts(&text, today, &[Part::Date], Some(&format!("@{today}"))),
                "tomorrow" => set_parts(
                    &text,
                    today,
                    &[Part::Date],
                    Some(&format!("@{}", today.succ_opt().unwrap_or(today))),
                ),
                // The date field the box already has, opened on the date the
                // line already means. `tab` is what opens it there; here the
                // radio is, which is the same key doing one job per screen.
                "pick" => {
                    self.input.toggle_field(today);
                    text
                }
                own => set_parts(&text, today, &[Part::Date], Some(&format!("@{own}"))),
            },
            Field::Priority => match label.as_str() {
                "none" => set_parts(&text, today, &[Part::Priority], None),
                name => set_parts(&text, today, &[Part::Priority], Some(&format!("!{name}"))),
            },
            Field::List => set_parts(
                &text,
                today,
                &[Part::List],
                Some(&format!("${}", label.trim_end_matches(".md"))),
            ),
            _ => text,
        };
        self.input.at = self.input.text.len();
        // The date row can appear or vanish under the cursor, so the focus is
        // re-seated on a row that still exists.
        if !self.order().contains(&self.focus) {
            self.focus = Field::Due;
        }
    }

    /// `←` and `→` on a chosen row: one step, applied at once. No `⏎` to
    /// confirm — the preview at the bottom is the confirmation, and it is
    /// already there.
    fn nudge(&mut self, by: isize) {
        let choices = self.choices();
        if choices.is_empty() {
            return;
        }
        let at = choices.iter().position(|(_, on)| *on).unwrap_or(0) as isize;
        let next = (at + by).rem_euclid(choices.len() as isize) as usize;
        self.choose(next);
    }

    /// Writes whatever is being typed in `Time` or `Tags` back into the line,
    /// rebuilding from `base` rather than editing the last keystroke's work.
    fn sync(&mut self) {
        let (base, today, typing) = (self.base.clone(), self.today, self.typing.clone());
        self.input.text = match self.focus {
            Field::Time => after_date(&base, today, typing.trim()),
            Field::Tags => set_tags(&base, today, &typing),
            _ => base,
        };
        self.input.at = self.input.text.len();
    }

    /// One keypress. Everything the form does to itself happens in here, so the
    /// event loop only ever hears the two answers it has to act on.
    pub fn press(&mut self, key: KeyEvent) -> Typed {
        let what = typing(key);
        // The date picker has the keyboard while it is open, whatever the focus
        // is — one `esc` per thing that is open, the same rule the box has.
        if self.input.field.is_some() {
            match what {
                Typed::Left => self.input.left(),
                Typed::Right => self.input.right(),
                Typed::Step(by) => self.input.step(by),
                Typed::Char(c) => self.input.insert(c),
                Typed::Field | Typed::Save => {
                    self.input.apply_field();
                }
                Typed::Cancel => {
                    self.input.close_field();
                }
                _ => {}
            }
            return Typed::Ignore;
        }

        match (what, self.focus) {
            (Typed::Cancel, _) => return Typed::Cancel,
            (Typed::Save, Field::Cancel) => return Typed::Cancel,
            (Typed::Save, _) => return Typed::Save,
            // `tab` is *next field* in here, and the date picker is reached
            // through `Due · pick…` instead. One key, one job per screen —
            // docs/tui.md#adding.
            (Typed::Field, _) => {
                let back = key.code == crossterm::event::KeyCode::BackTab
                    || key.modifiers.contains(KeyModifiers::SHIFT);
                self.step_focus(if back { -1 } else { 1 });
            }
            (Typed::Step(by), _) => self.step_focus(-by as isize),
            (Typed::Char(c), Field::Title) => self.input.insert(c),
            (Typed::Back, Field::Title) => self.input.back(),
            (Typed::Delete, Field::Title) => self.input.delete(),
            (Typed::Left, Field::Title) => self.input.left(),
            (Typed::Right, Field::Title) => self.input.right(),
            (Typed::Home, Field::Title) => self.input.home(),
            (Typed::End, Field::Title) => self.input.end(),
            (Typed::Char(c), field) if field.typed() => {
                self.typing.push(c);
                self.sync();
            }
            (Typed::Back, field) if field.typed() => {
                self.typing.pop();
                self.sync();
            }
            (Typed::Left, _) => self.nudge(-1),
            (Typed::Right, _) => self.nudge(1),
            _ => {}
        }
        Typed::Ignore
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
    /// `tab` — the date field, in and out.
    Field,
    /// `↑` `↓`, which mean nothing until the field is open.
    Step(i32),
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
    let ctrl = chord(key.modifiers);
    // The mirror of `chord`: alt on its own is `alt-f`, alt with ctrl is AltGr.
    let alt =
        key.modifiers.contains(KeyModifiers::ALT) && !key.modifiers.contains(KeyModifiers::CONTROL);

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
        // The one key that opens something from in here, and the two that only
        // do anything once it is open — docs/tui.md#the-date-field--tab.
        KeyCode::Tab | KeyCode::BackTab => Typed::Field,
        KeyCode::Up => Typed::Step(1),
        KeyCode::Down => Typed::Step(-1),
        // Every other modified key is left alone: `ctrl-v`, `alt-f` and the rest
        // mean things in a terminal that a one-line field has no business
        // claiming, and a stray control character in a task title is a file the
        // user cannot read back. AltGr is not one of them — see `chord`.
        KeyCode::Char(c) if !ctrl && !alt => Typed::Char(c),
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
        /// How many tasks are in the group. On every heading and not only on a
        /// folded one: `LATER (3)` already said it when it was closed, and
        /// there was never a reason the open ones stayed silent — docs/tui.md.
        ///
        /// One number rather than two. It used to be `hidden: Option<usize>`,
        /// counted a second time during the fold, and two counters of the same
        /// thing eventually disagree.
        count: usize,
        /// A collapsed group that does not say how much it is hiding, and which
        /// key opens it, is a dead end — docs/tui.md.
        folded: bool,
    },
    Task(Task),
    /// The row that closes a group. It was a blank spacer between two groups
    /// and it is now the bottom edge of the box the group is drawn in — the
    /// same row, spent on a border instead of on air. See `rows`.
    GroupEnd,
}

impl Row {
    fn header(title: &str, count: usize) -> Self {
        Row::Header {
            title: title.to_string(),
            count,
            folded: false,
        }
    }
}

/// Flattens the agenda into lines. A titled group is a heading, its tasks and
/// a closing row — one, *n*, one, which is what it has always been: the blank
/// row that used to sit between two groups is now the bottom edge of the box
/// the group is drawn in. Same arithmetic, one stroke instead of air —
/// docs/redesign.md.
///
/// An untitled group — the run of tasks above the file's first heading — keeps
/// its rows and gets an empty title, which draws as a box with no name on it.
pub fn rows(groups: &[Group<'_>]) -> Vec<Row> {
    let mut out = Vec::new();
    for group in groups {
        // `OVERDUE` is ours and `## Work` is the user's, and until now they were
        // the same bold word plus the same rule. The markdown marker the heading
        // already carries in the file is what tells them apart — it costs no
        // colour and no third level of hierarchy, and it says "this line is
        // yours" to anyone who has seen the file — docs/tui.md#main-screen.
        //
        // The file joins it when several lists are open, and it is the whole of
        // what says where a task lives: `## Work` in two files is two headings.
        // A file's run of tasks above its first heading gets a header of nothing
        // but the name, or it would look like more of the file before it.
        let header = match group.kind {
            // The marker goes on what the user wrote, and a heading that is
            // nothing but a file name is not something they wrote.
            Kind::Section { name: Some(_), .. } => {
                group.kind.heading().map(|shown| format!("## {shown}"))
            }
            _ => group.kind.heading(),
        };
        // A group with no name is still a group. It gets a nameless box rather
        // than a "(no section)" nobody wrote, and rather than rows left floating
        // beside the boxed ones.
        out.push(Row::header(&header.unwrap_or_default(), group.tasks.len()));
        out.extend(group.tasks.iter().map(|t| Row::Task((*t).clone())));
        out.push(Row::GroupEnd);
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
        let mut skipping = false;

        for row in &self.all {
            match row {
                Row::Header {
                    title,
                    count,
                    folded: _,
                } => {
                    skipping = self.folded.contains(title);
                    self.rows.push(Row::Header {
                        title: title.clone(),
                        count: *count,
                        folded: skipping,
                    });
                }
                Row::Task(t) => match skipping {
                    true => {}
                    false => self.rows.push(Row::Task(t.clone())),
                },
                // A folded group is a bare rule, not an empty two-row box: the
                // difference between a container and a line *is* the open
                // signal — docs/redesign.md. So the closing row goes with the
                // tasks it was closing over.
                Row::GroupEnd => match std::mem::take(&mut skipping) {
                    true => {}
                    false => self.rows.push(Row::GroupEnd),
                },
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
    /// folded. A run of tasks above the file's first `##` has a header row with
    /// nothing on it, and gets `None`: there is nothing to key a fold on and
    /// nothing to write on the header that would say it is folded.
    ///
    /// The **nearest** header and then a look at its name, rather than the
    /// nearest *named* one: skipping past an empty header would fold the group
    /// above it, which is a different group.
    fn group_at_cursor(&self) -> Option<String> {
        let at = self.state.selected()?;
        self.rows[..=at.min(self.rows.len().saturating_sub(1))]
            .iter()
            .rev()
            .find(|r| matches!(r, Row::Header { .. }))
            .and_then(|r| match r {
                Row::Header { title, .. } if !title.is_empty() => Some(title.clone()),
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
            if let Some(at) = self.rows.iter().position(
                |r| matches!(r, Row::Header { title: t, folded: true, .. } if *t == title),
            ) {
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
            Some(Row::Task(_)) | Some(Row::Header { folded: true, .. })
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
        match (self, task.state, task.is_overdue(today)) {
            (Glyphs::Unicode, State::Done, _) => "✓",
            (Glyphs::Unicode, State::Cancelled, _) => "✗",
            (Glyphs::Unicode, State::Open, true) => "!",
            (Glyphs::Unicode, State::Open, false) => "○",
            (Glyphs::Ascii, State::Done, _) => "[x]",
            (Glyphs::Ascii, State::Cancelled, _) => "[-]",
            (Glyphs::Ascii, State::Open, true) => "[!]",
            (Glyphs::Ascii, State::Open, false) => "[ ]",
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

    /// The rule between two columns. The row is a table past `COLUMNS_AT`, and
    /// this is what says so — docs/tui.md#width.
    fn divider(self) -> &'static str {
        match self {
            Glyphs::Unicode => "│",
            Glyphs::Ascii => "|",
        }
    }

    /// A group box's four corners — top left, top right, bottom left, bottom
    /// right. The same rounded set the outer frame uses, because they are the
    /// same kind of thing one inside the other.
    ///
    /// `+` in ASCII, like every other joint in the fallback. A box-drawing
    /// character left in a fallback is not a fallback.
    fn corners(self) -> [&'static str; 4] {
        match self {
            Glyphs::Unicode => ["╭", "╮", "╰", "╯"],
            Glyphs::Ascii => ["+", "+", "+", "+"],
        }
    }

    /// Where a column's divider meets the top and the bottom edge of its group
    /// box. The whole point of the box: a stroke that used to start out of
    /// nothing at column 40 now starts at a junction — docs/redesign.md.
    fn junctions(self) -> (&'static str, &'static str) {
        match self {
            Glyphs::Unicode => ("┬", "┴"),
            Glyphs::Ascii => ("+", "+"),
        }
    }

    /// The arrow the preview points a `$work` capture with.
    fn arrow(self) -> &'static str {
        match self {
            Glyphs::Unicode => "→",
            Glyphs::Ascii => "->",
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

    /// Filled and empty, for the bars on the stats screen. Solid blocks rather
    /// than the title bar's `▰▱`: that one is eight cells of *how far through*,
    /// these are a length somebody reads against the length beside it.
    fn block(self) -> (&'static str, &'static str) {
        match self {
            Glyphs::Unicode => ("█", "░"),
            Glyphs::Ascii => ("#", "."),
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
    ///
    /// **Rounded**, and so are the group boxes inside it: one constant, and the
    /// highest ratio of *looks finished* to *lines changed* in the redesign.
    /// The corner is the only thing on the frame that was ever going to say it,
    /// because everything else about a frame is decided by the pane.
    fn border(self) -> ratatui::symbols::border::Set<'static> {
        match self {
            Glyphs::Unicode => ratatui::symbols::border::ROUNDED,
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

    /// Where the rule inside the input box meets the frame. A rule that butts
    /// straight into the side border reads as a broken frame.
    fn tee(self) -> (&'static str, &'static str) {
        match self {
            Glyphs::Unicode => ("├", "┤"),
            Glyphs::Ascii => ("+", "+"),
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
    /// The open lists, by file name. What a `$work` in the input box is checked
    /// against — docs/tui.md#which-list--work.
    pub lists: &'a [String],
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
    // A finished task's due date is a fact about a deadline that no longer
    // applies; the day it was actually finished is the one thing still worth a
    // column. It only ever displaces the due date, so the column stays one date
    // wide — and a task ticked before the stamp existed still shows its old one.
    if let Some(on) = task.done_on.filter(|_| task.done()) {
        return match (today - on).num_days() {
            0 => "today".to_string(),
            1..=6 => on.format("%a").to_string(),
            _ => on.format("%b %-d").to_string(),
        };
    }
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
        (d, _) if d < 0 && task.open() => format!("{}d ago", -d),
        // The **time, or nothing**. A task due today sits under a heading that
        // says `TODAY`, so a column saying `today` spent nine characters
        // repeating the box it was already in. Emptied, the rows that do have a
        // time stand out — which is the only thing about a task due today still
        // worth reading. The rule is that the column says what the heading does
        // not: `2d ago` under `OVERDUE`, the day under `THIS WEEK`, the date
        // under a `##` section — docs/tui.md#main-screen.
        (0, _) => time.unwrap_or_default(),
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

/// What a column costs in front of its own text once the divider is drawn:
/// a space, the rule, a space. One column more than `GAP`, paid for out of the
/// title, which is the field with room to give at these widths.
const RULED: usize = 3;

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
    /// Whether anything on screen is tagged. Tags reserve no width — they are
    /// last and ragged — but the rule that opens their column is drawn on every
    /// row once one row has them, or the table would lose its last edge on
    /// exactly the rows with nothing in that cell.
    tags: bool,
}

/// The fourth breakpoint, in columns of **row** — the frame and the selection
/// marker are already off it, so it is four short of the terminal.
///
/// Columns have to be paid for: an empty priority column costs every row its
/// width whether or not anything on screen uses one. Below this there is not
/// enough row to buy alignment with, the old right-aligned block packs more
/// onto it, and packing wins when there is not much to pack into.
///
/// It was 76 until the group box took five columns off every row — a side
/// either end, the inset after the left one, and the two the box holds back so
/// it does not close flush against the frame. The number follows the row it
/// measures and not the terminal, and the terminal it answers to has not moved:
/// eighty columns, which is what a terminal opens at and what the drawings in
/// docs/redesign.md are made at, still gets its columns.
const COLUMNS_AT: usize = 71;

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
        let with_gap = |w: usize| if w == 0 { 0 } else { w + RULED };
        let (date, prio) = (with_gap(date), with_gap(prio));

        // The mark and its space — three columns wider under the ASCII
        // fallback, and budgeting the Unicode figure there spends width the row
        // does not have.
        let mark = render.glyphs.mark_width() + 1;
        // Tags get no reservation. They are last and ragged, so nothing lines
        // up after them, and reserving the widest row's worth would cut every
        // title to pay for tags most rows do not have — the exact inversion of
        // the drop order in docs/tui.md#width. task_line spends what is left.
        //
        // The rule that opens their column *is* reserved, and it has to be: it
        // is drawn on every row once any row is tagged, so a title allowed to
        // eat the last three columns would push it off the end of exactly the
        // rows that have nothing to show there.
        let tags = tasks().any(|t| !t.tags.is_empty());
        let room = width.saturating_sub(mark + date + prio + if tags { RULED } else { 0 });
        Self {
            title: title.min(room).max(12.min(width.saturating_sub(mark))),
            date,
            prio,
            tags,
        }
    }

    /// Where each column's divider sits in a task row, counted in display
    /// columns from the start of the row. What the group box needs in order to
    /// put a `┬` and a `┴` on the ends of a stroke that used to start out of
    /// nothing — docs/redesign.md.
    ///
    /// It walks the same additions `task_line` makes, in the same order, and a
    /// buffer test pins the two against each other: the junction landing one
    /// column off the divider is exactly the sort of thing that looks fine in
    /// the arithmetic and wrong on the screen.
    fn dividers(&self, mark: usize) -> Vec<usize> {
        if self.title == 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut at = mark + self.title;
        // Each entry opens with a space, then its divider, so the stroke is one
        // column past where the column before it ended.
        for width in [self.date, self.prio] {
            if width > 0 {
                out.push(at + 1);
                at += width;
            }
        }
        if self.tags && self.date + self.prio > 0 {
            out.push(at + 1);
        }
        out
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
    let rule = Style::default().fg(render.colours.border);
    let mut push = |text: String, column: usize, style: Style| {
        if column == 0 {
            if !text.is_empty() {
                right.push(Span::styled(format!("{}{text}", " ".repeat(GAP)), style));
            }
            return;
        }
        // The divider is drawn for an empty cell too, which is the whole of what
        // makes it a table: the rules run straight down the pane past the rows
        // that have no date and no priority.
        right.push(Span::styled(format!(" {} ", render.glyphs.divider()), rule));
        let pad = column.saturating_sub(columns(&text) + RULED);
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
        || (task.open() && task.due.is_some_and(|d| d.date == render.today));
    let date_style = if pressing {
        Style::default().fg(colour)
    } else {
        dim
    };
    push(date, cols.date, date_style);
    if size == Size::Wide {
        // The field the user typed to mean *how much this matters*, and dim
        // beside the tags was the screen saying it back in a whisper. It borrows
        // the row's colour from nobody: `!high` on a late row used to be the same
        // red as the date, which is the one row where the two need telling apart.
        // A ticked row keeps it too — the priority is a fact about the task and
        // not a claim about what is left to do, which the `✓` already answers.
        let style = priority_style(task.priority, dim, render);
        push(prio.unwrap_or_default(), cols.prio, style);
        // What is left of the row after the columns. A tag that does not fit is
        // dropped whole rather than cut: `#hea…` is not a filter, it is a
        // riddle. Tags go before the title — docs/tui.md#width.
        let mut room = match cols.title {
            0 => usize::MAX,
            title => width.saturating_sub(mark_width + title + cols.date + cols.prio),
        };
        // The rule that opens the tag column, drawn whether or not this row has
        // one to put in it — an empty cell keeps its place, which is the whole
        // difference between a table and three fields near each other.
        let lead = format!(" {} ", render.glyphs.divider());
        // The width check is the belt to the reservation's braces: a row may
        // never overrun, whatever the arithmetic above did.
        let ruled = cols.date + cols.prio > 0 && cols.tags && room >= columns(&lead);
        if ruled {
            room -= columns(&lead);
            right.push(Span::styled(lead, rule));
        }
        for (n, tag) in task.tags.iter().enumerate() {
            // Inside the column the tags are ragged: nothing lines up after
            // them, so they are spaced rather than ruled.
            let span = match (n, ruled) {
                (0, true) => format!("#{}", text::plain(tag)),
                _ => format!("  #{}", text::plain(tag)),
            };
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

/// A group's name and how many tasks are in it — `TODAY · 2`. On every heading,
/// open or folded: a collapsed group already had to say how much it was hiding,
/// and there was never a reason the open ones stayed silent — docs/tui.md.
///
/// A group with no name gets no count either. `· 2` on its own says nothing that
/// counting the rows under it does not.
fn heading(title: &str, count: usize, glyphs: Glyphs) -> String {
    let name = text::plain(title);
    if name.is_empty() {
        return name;
    }
    let (_, dot) = glyphs.punctuation();
    format!("{name} {dot} {count}")
}

/// A group heading with a rule after it — what a **folded** group is drawn as,
/// and what every group is drawn as below 34 columns. In a narrow pane the eye
/// needs a horizontal anchor to find where a group starts; a bare word does not
/// give it — docs/tui.md.
///
/// A line and not a box, on purpose: an empty two-row box to say a group is
/// closed is exactly backwards, and the difference between a container and a
/// line *is* the open/closed signal.
///
/// Where the rule **stops** is the title column once there is one: past
/// `COLUMNS_AT` a rule to the right edge is the heaviest thing on the screen
/// and says nothing, while one that ends with the titles draws the column
/// instead. Below it there is no column to end at, so it runs to the edge.
fn header_line(
    title: &str,
    count: usize,
    folded: bool,
    width: usize,
    cols: Columns,
    render: Render<'_>,
) -> Line<'static> {
    let name = heading(title, count, render.glyphs);
    // A collapsed group says which key opens it. One that does not is a dead
    // end — docs/tui.md.
    let tail = if folded { " l" } else { "" };

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

/// One column of air between a group box's left side and the mark. The drawings
/// in docs/redesign.md spend the selection marker's own column on it; here the
/// marker sits outside the box, in the gutter the `List` widget reserves for it,
/// so the space has to be asked for. `│! late` is a row nobody wants to read.
const INSET: usize = 1;

/// What the box holds back on the right, so it closes against air rather than
/// against the frame. The two columns on the left are the marker's gutter and
/// were already spent — which is why the box costs four columns of row and not
/// six, docs/redesign.md.
const BOX_MARGIN: usize = 2;

/// A group's top or bottom edge — the same stroke, the same junctions, and a
/// different pair of corners. `title` is drawn into the top edge; a group with
/// no name gets a bare one, which is the run of tasks above a file's first
/// heading. It is still a group, it just has nothing to be called: no
/// "(no section)" nobody wrote, and no orphan rows floating beside the boxes
/// either — docs/tui.md#main-screen.
///
/// This is the correction the redesign is mostly about. The screen used to draw
/// three sets of strokes that never met — a group rule stopping in mid-air, the
/// column dividers starting out of nothing below it, and a blank row closing a
/// group that was never a container. Here every stroke starts at a corner and
/// ends at one — docs/redesign.md.
fn group_edge(
    top: bool,
    title: &str,
    count: usize,
    width: usize,
    cols: Columns,
    render: Render<'_>,
) -> Line<'static> {
    let glyphs = render.glyphs;
    let border = Style::default().fg(render.colours.border);
    if width < 2 {
        return Line::from(Span::styled(
            glyphs.rule().to_string().repeat(width),
            border,
        ));
    }

    let [top_left, top_right, bottom_left, bottom_right] = glyphs.corners();
    let (down, up) = glyphs.junctions();
    let (open, close, junction) = match top {
        true => (top_left, top_right, down),
        false => (bottom_left, bottom_right, up),
    };

    // Built cell by cell rather than by arithmetic: every glyph in it is one
    // column wide, so an index into this is a column on the screen, and the
    // junctions can be dropped in wherever `dividers` says without four
    // separate slices having to agree about where they are.
    let mut cells = vec![glyphs.rule().to_string(); width];
    cells[0] = open.to_string();
    cells[width - 1] = close.to_string();
    for at in cols.dividers(glyphs.mark_width() + 1) {
        // Plus the left edge and the inset the task rows carry, which the
        // divider offsets are measured without.
        let at = at + 1 + INSET;
        if at < width - 1 {
            cells[at] = junction.to_string();
        }
    }

    let name = heading(title, count, glyphs);
    if name.is_empty() {
        return Line::from(Span::styled(cells.concat(), border));
    }

    // `╭─ NAME · n ` and then whatever is left of the edge. A name long enough
    // to reach the closing corner is cut rather than allowed to push one off,
    // and a junction under the name is simply covered by it.
    const LEAD: usize = 3;
    let name = shorten(&name, width.saturating_sub(LEAD + 2), glyphs);
    let after = LEAD + columns(&name) + 1;
    Line::from(vec![
        Span::styled(format!("{open}{} ", glyphs.rule()), border),
        Span::styled(name, Style::default().fg(render.colours.accent).bold()),
        Span::styled(format!(" {}", cells[after..].concat()), border),
    ])
}

/// A task row with the group box's sides on it. Padded to the full width first:
/// the tags are ragged and end wherever they end, and a right edge that follows
/// them is not an edge.
fn boxed(row: Line<'static>, width: usize, render: Render<'_>) -> Line<'static> {
    let side = render.glyphs.divider();
    let border = Style::default().fg(render.colours.border);
    let pad = width.saturating_sub(2 + INSET + row.width());
    let mut spans = vec![Span::styled(side, border), Span::raw(" ".repeat(INSET))];
    spans.extend(row.spans);
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(side, border));
    Line::from(spans)
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

/// The hint bar, filled to whatever the pane gives it.
///
/// `? keys` and `q quit` are pinned to the end: however little room there is,
/// the way to the rest of the keymap and the way out both stay. Everything
/// before them goes in until the next one would not fit, so the order is how
/// often a key is reached for — and the bar stops being a list that has to be
/// re-argued every time a key is added.
fn hints(width: usize, glyphs: Glyphs) -> String {
    // One space, not two. The two existed because nothing else told `a add`
    // from `spc done`; the brackets do that now, and at eighty columns the
    // second space is the difference between `p later  y copy` being on the bar
    // and not.
    const SEP: &str = " ";
    let tail = format!("{SEP}[?] keys{SEP}[q] quit");
    let keys: [(&str, &str); 8] = [
        ("j k", "move"),
        ("spc", "done"),
        ("a", "add"),
        (glyphs.enter(), "edit"),
        ("d", "cancel"),
        // `later` and not `put off`, which is what the input box calls it and
        // what the keymap calls it. Three columns, and they are the three that
        // decide whether `y copy` is on the bar at eighty — the width most
        // terminals open at, and the one width the newest key was invisible at.
        ("p", "later"),
        ("y", "copy"),
        // On the end, and the existing greedy fill decides the rest: this bar is
        // ordered by how often a key is reached for, and a second screen is not
        // reached for often. No new logic at all — docs/tui.md#the-bottom-line.
        ("s", "stats"),
    ];

    let mut out = String::new();
    for (key, what) in keys {
        // Brackets, so it reads as a keycap the way lazygit's and k9s's bars do
        // and survives `NO_COLOR` — the alternative is a colour, and a key that
        // is only a key because it is mauve is not one on half the terminals it
        // runs on. Two columns an entry, which is what decides how many of them
        // fit; the greedy fill below is what spends them.
        let entry = format!("{SEP}[{key}] {what}");
        // The leading space every message on this line carries.
        if 1 + columns(&out) + columns(&entry) + columns(&tail) > width {
            break;
        }
        out.push_str(&entry);
    }
    out.push_str(&tail);
    format!(" {}", out.trim_start())
}

impl Notice {
    fn line(
        &self,
        size: Size,
        width: usize,
        height: u16,
        glyphs: Glyphs,
        colours: Theme,
    ) -> Line<'static> {
        let (text, colour) = match self {
            // Only the keys that do something. A hint bar advertising a key that
            // is not implemented yet is a worse lie than no hint bar.
            Notice::Hints if height < 10 => (" ?".to_string(), colours.dim),
            Notice::Hints if size == Size::Wide => (hints(width, glyphs), colours.dim),
            // Keys only, no words. `X` and `e` gave up their slots here: delete
            // and `$EDITOR` are both reachable from `?`, and neither is what
            // somebody glancing at a narrow pane is about to press.
            Notice::Hints => (" [j k] [spc] [a] [d] [p] [?] [q]".to_string(), colours.dim),
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
/// What a priority is drawn in, on the row and in the input box alike.
///
/// Its own colour in two weights: `!high` bold, `!med` plain, and `!low` down
/// with the dim fields where it asked to be. Its own and not the accent, which
/// is the tool's voice and already the headings, the box border and the help
/// keys — one colour answering two questions answers neither
/// (docs/design.md#what-each-colour-means).
///
/// `None` and `!low` are the quiet ones. Every row keeps its priority, ticked or
/// not: it is a fact about the task rather than a claim about what is left to
/// do, and the `✓` beside it already answers that.
fn priority_style(priority: Option<Priority>, dim: Style, render: Render<'_>) -> Style {
    let loud = Style::default().fg(render.colours.priority);
    match priority {
        Some(Priority::High) => loud.bold(),
        Some(Priority::Med) => loud,
        _ => dim,
    }
}

/// What colour a parsed word gets, on the typed line and in the preview under it
/// alike — the same two weights the row gives it, so the box is not teaching a
/// colour the list then contradicts.
fn paint(
    part: crate::capture::Part,
    priority: Option<Priority>,
    plain: Style,
    render: Render<'_>,
) -> Style {
    match part {
        crate::capture::Part::Tag => Style::default().fg(render.colours.tag),
        crate::capture::Part::Priority => priority_style(priority, plain, render),
        _ => Style::default().fg(render.colours.accent),
    }
}

/// What the preview says about a `$work`: where it is going, or that it is
/// going nowhere.
///
/// `None` while the word could still become one of the open lists. The preview
/// redraws on every keystroke, and "no list w.md" on the way to `$work` is the
/// same nagging the date warning waits to avoid — wrong four times and right
/// once is how people learn to stop reading a line. `⏎` still refuses a half
/// typed one; being quiet is not the same as agreeing.
fn addressed(name: &str, render: Render<'_>) -> Option<Span<'static>> {
    if let Some(file) = render
        .lists
        .iter()
        .find(|file| crate::capture::names_list(name, file))
    {
        return Some(Span::styled(
            format!("{} {file}", render.glyphs.arrow()),
            Style::default().fg(render.colours.accent),
        ));
    }
    let becoming = render.lists.iter().any(|file| file.starts_with(name));
    (!becoming).then(|| {
        Span::styled(
            format!("no list {name}.md"),
            Style::default().fg(render.colours.overdue),
        )
    })
}

fn input_lines(input: &Input, width: usize, render: Render<'_>) -> (Vec<Line<'static>>, usize) {
    let dim = Style::default().fg(render.colours.dim);
    let head = format!(" {} {}", input.purpose.label(), render.glyphs.field());
    // Weight and full brightness, not a colour: the box's own border is already
    // the accent and is what says *you are in the box*, so a label in the same
    // colour an inch inside says it twice and leaves `COPY` — the one label with
    // news — nothing to be told apart by. `COPY` keeps the accent for that, and
    // is the only span on this screen where it lands on a word the tool did not
    // write as a heading. docs/design.md#what-each-colour-means.
    let label = match input.purpose {
        Purpose::Copy => Style::default().fg(render.colours.accent).bold(),
        _ => Style::default().fg(render.colours.foreground).bold(),
    };
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
    // A length of time is one word and not task syntax, so `parts` has nothing
    // to say about it: it is the accent once it resolves to a day and plain
    // until then, which is the same promise the sentence field makes.
    let moving = matches!(input.purpose, Purpose::Postpone(_));
    let lands = moving
        .then(|| crate::capture::later(&input.text, render.today))
        .flatten();
    // Read once and used by both halves of the box, so the word on the typed
    // line and the word under it are never coloured by two different readings.
    let parsed = crate::capture::capture(&input.text, render.today);

    // Split so the label can be lit without the caret glyph following it: the
    // two together are still `head`, so the cursor column above is unchanged.
    let caret = format!(" {}", render.glyphs.field());
    let named = head[..head.len() - caret.len()].to_string();
    let mut spans = vec![Span::styled(named, label), Span::styled(caret, dim)];
    let mut cut = from;
    for (word, part) in crate::capture::parts(&input.text, render.today) {
        if moving || part == crate::capture::Part::Text || word.end <= from || word.start >= to {
            continue;
        }
        let (start, end) = (word.start.max(from), word.end.min(to));
        if start > cut {
            spans.push(Span::styled(input.text[cut..start].to_string(), plain));
        }
        spans.push(Span::styled(
            input.text[start..end].to_string(),
            paint(part, parsed.priority, plain, render),
        ));
        cut = end;
    }
    if cut < to {
        let style = match (moving, lands) {
            (true, Some(_)) => Style::default().fg(render.colours.accent),
            _ => plain,
        };
        spans.push(Span::styled(input.text[cut..to].to_string(), style));
    }
    let field = Line::from(spans);

    // Field by field, in the same colours the typed line above it uses: the date
    // is the accent, a tag is `tag`, `!high` is bold, and the separators are
    // dim. One colour over the whole line said the parser had understood all of
    // it equally, which is the one thing the preview exists to be specific
    // about — and it made the date and the tag indistinguishable in the row
    // whose whole job is telling them apart. The title is not repeated: it is
    // already on the line above, in the same white.
    // The same rule the rows use, so the screen speaks one separator language.
    let dot = render.glyphs.divider();
    let rule = Style::default().fg(render.colours.border);
    let mut shown = vec![Span::styled("      ".to_string(), dim)];
    if let Some(open) = input.field {
        // The field takes the preview's row rather than a row of its own: the
        // box keeps the five it has, which is what lets it open on a pane too
        // short to grow. What the preview would have said is one keystroke away
        // and unchanged — docs/tui.md#the-date-field--tab.
        //
        // The part under the cursor is in brackets **and** in the accent, and
        // the brackets are what carry it under `NO_COLOR`: the one thing this
        // row has to say is which of the three the arrows are pointing at.
        for (part, text) in open.cells() {
            let focused = part == open.part;
            let style = match focused {
                true => Style::default().fg(render.colours.accent).bold(),
                false => plain,
            };
            shown.push(Span::styled(text, style));
        }
        shown.push(Span::styled(format!(" {}", render.glyphs.arrows()), dim));
    } else if moving {
        // The one thing worth previewing here is the day it lands on, because
        // `2w` is exactly the input whose answer nobody works out in their head.
        match lands {
            Some(date) => shown.push(Span::styled(
                crate::text::relative(crate::model::Due::new(date), render.today),
                Style::default().fg(render.colours.accent),
            )),
            None => shown.push(Span::styled(
                "how long?  2   3d   1w   fri".to_string(),
                dim,
            )),
        }
    } else if input.text.trim().is_empty() {
        // An empty box says what can go in it, the way the empty `p` box says
        // what a length of time looks like. By example and not by name: `@thu`
        // is the syntax and the hint in one, which is what the preview does for
        // every word already typed. This is the whole of what a box split into
        // labelled fields would have taught, at none of its cost —
        // docs/decisions.md#settled.
        //
        // `$` only where it means something. With one list there is nothing to
        // address, and a hint for a key that does nothing here is the lie the
        // hint bar has never told.
        let mut hint = String::from("@thu #home !high");
        if render.lists.len() > 1 {
            hint.push_str(" $list");
        }
        shown.push(Span::styled(hint, dim));
    } else {
        // The one row of the preview that has an opinion instead of a readout.
        // Everything else here reports what the parser took; this reports what
        // it refused, because the refusal is otherwise silent — the word stays
        // in the title and the box looks like it agreed. In `overdue`, which is
        // already the bottom line's warning colour, rather than a twelfth theme
        // role. The fields still follow it: one bad word does not hide the tag.
        if let Some(word) = crate::capture::unresolved_date(&input.text, render.today) {
            shown.push(Span::styled(
                format!("{word} is not a date"),
                Style::default().fg(render.colours.overdue),
            ));
        }
        // Where it is going, when the line says. First, because it is the one
        // field that is about the file rather than about the task — and because
        // the other opinion the preview has sits here too. An edit is not
        // addressed, so it does not answer one.
        if let Some(name) = crate::capture::list_of(&input.text)
            && !matches!(input.purpose, Purpose::Edit(_))
            && let Some(span) = addressed(name, render)
        {
            if shown.len() > 1 {
                shown.push(Span::styled(format!(" {dot} "), rule));
            }
            shown.push(span);
        }
        for (part, text) in crate::text::field_parts(&parsed, render.today) {
            if shown.len() > 1 {
                shown.push(Span::styled(format!(" {dot} "), rule));
            }
            shown.push(Span::styled(
                text,
                paint(part, parsed.priority, plain, render),
            ));
        }
    }
    let preview = Line::from(shown);
    // A rule between the two, because they are not the same thing: above it is
    // what you are typing and below it is what the file will get. Without it the
    // caret looks like it could be moved down into the preview, and people try.
    let rule = Line::from(Span::styled(
        render.glyphs.rule().to_string().repeat(width),
        Style::default().fg(render.colours.border),
    ));

    (vec![field, rule, preview], at.min(width.saturating_sub(1)))
}

/// What is on the screen instead of, or on top of, the list.
///
/// One parameter rather than a flag per screen: `draw` already takes seven
/// arguments, and the third and fourth screens would each have added a `bool`
/// that is only ever true when every other one is false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View<'a> {
    List,
    /// The one overlay in the product — it covers the list rather than
    /// replacing it, because it is read *about* what is underneath.
    Help,
    /// A screen and not an overlay: nothing on it is glanced at mid-task.
    Stats(&'a Stats, Period),
}

/// What is open over the list. One parameter rather than two `Option`s that
/// must never both be `Some`.
#[derive(Debug, Clone, Copy)]
pub enum Open<'a> {
    Nothing,
    /// The one-line box — still what `p` and `y` open at every width, and what
    /// `a` opens when the pane is too small for the form.
    Box(&'a Input),
    /// The form. `a`, wherever there is room for it — docs/decisions.md.
    Form(&'a Form),
}

pub fn draw(
    frame: &mut Frame,
    screen: &mut Screen,
    counts: Counts,
    render: Render<'_>,
    notice: &Notice,
    view: View<'_>,
    open: Open<'_>,
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

    // A screen rather than an overlay, so it takes the pane and the list is not
    // drawn at all — and the bottom line names its own keys, because none of the
    // list's do anything while it is up.
    if let View::Stats(stats, period) = view {
        stats_screen(frame, area, stats, period, render);
        if let Some(bottom) = bottom {
            let keys = match size {
                Size::Wide => " [1] week  [2] month  [3] year   [r] reload   [esc] back",
                _ => " [1] [2] [3]   [esc] back",
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    keys,
                    Style::default().fg(render.colours.dim),
                )))
                .style(Style::default().bg(render.colours.background)),
                bottom,
            );
        }
        return;
    }

    // The band owns the counts when it is drawn, so the title bar spends its
    // right-hand side on the date instead — the first thing a todo list should
    // say and the one thing this screen never said. When the band goes, the
    // counts and the progress bar come back to where they have always been.
    let banded = Band::of(whole.height, size);

    // Under 34 columns the frame is two of them, which is a tenth of the pane.
    let (dash, _) = render.glyphs.punctuation();
    let block = (size > Size::Bare).then(|| {
        let name = match banded {
            Band::None => format!(
                " ratodo {dash} {} ",
                title_counts(counts, size, render.glyphs)
            ),
            _ => " ratodo ".to_string(),
        };
        let right = match banded {
            Band::None => (size == Size::Wide)
                .then(|| progress(counts, area.width as usize, columns(&name), render))
                .flatten(),
            _ => Some(Line::from(Span::styled(
                format!(" {} ", render.today.format("%A, %-d %B %Y")),
                Style::default().fg(render.colours.dim),
            ))),
        };

        let block = Block::bordered()
            .border_set(render.glyphs.border())
            .border_style(Style::default().fg(render.colours.border))
            .title(name);
        match right {
            Some(right) => block.title(right.right_aligned()),
            None => block,
        }
    });
    let framed = block.as_ref().map_or(area, |b| b.inner(area));
    // What is left for the list once the band and the footer have been paid
    // for. They are drawn after it, over rows it was never given.
    let spent = banded.rows() + if banded == Band::None { 0 } else { FOOTER };
    let inner = Rect::new(
        framed.x,
        framed.y + banded.rows(),
        framed.width,
        framed.height.saturating_sub(spent),
    );

    // The selection marker is drawn into the row, so the width the layout gets
    // is what is left after it. That gutter doubles as the box's left indent,
    // which is why the box costs four columns of row and not six.
    let cursor = render.glyphs.cursor();
    let width = (inner.width as usize).saturating_sub(columns(cursor));
    // Groups are boxes wherever there is a frame to nest them in. Below 34
    // columns the frame goes and the boxes go with it — docs/redesign.md.
    let boxes = size > Size::Bare;
    let box_width = width.saturating_sub(BOX_MARGIN);
    let row_width = match boxes {
        true => box_width.saturating_sub(2 + INSET),
        false => width,
    };
    let cols = Columns::of(&screen.rows, row_width, render, size);

    if screen.rows.iter().all(|r| !matches!(r, Row::Task(_))) {
        empty(frame, area, block, render);
    } else {
        if let Some(block) = &block {
            frame.render_widget(block.clone(), area);
        }
        // Whether the row being drawn is inside a box, carried along rather than
        // stored on the row: a folded group is a bare rule and the run of tasks
        // above the file's first heading has no heading to open one, so "inside"
        // is a fact about where the cursor of this loop is and not about the task.
        let mut inside = false;
        // Without the box there is nothing for either of these to draw: the
        // closing row was a blank spacer at this width before it was an edge,
        // and a header with no name would be a rule with nothing on it.
        let furniture = |row: &Row| match row {
            Row::GroupEnd => true,
            Row::Header { title, .. } => title.is_empty(),
            _ => false,
        };
        let items: Vec<ListItem> = screen
            .rows
            .iter()
            .filter(|row| boxes || !furniture(row))
            .map(|row| match row {
                Row::Task(t) => {
                    let line = task_line(
                        t,
                        if inside { row_width } else { width },
                        cols,
                        render,
                        size,
                    );
                    ListItem::new(match inside {
                        true => boxed(line, box_width, render),
                        false => line,
                    })
                }
                Row::Header {
                    title,
                    count,
                    folded,
                } => {
                    inside = boxes && !folded;
                    ListItem::new(match inside {
                        true => group_edge(true, title, *count, box_width, cols, render),
                        false => header_line(title, *count, *folded, width, cols, render),
                    })
                }
                Row::GroupEnd => {
                    inside = false;
                    ListItem::new(group_edge(false, "", 0, box_width, cols, render))
                }
            })
            .collect();

        let list = List::new(items)
            .style(Style::default().bg(render.colours.background))
            .highlight_symbol(cursor)
            // Background only. Setting a foreground here would repaint the
            // selected row in the accent colour, and an overdue task would stop
            // being red the moment you moved the cursor onto it — which is the
            // one row you are most likely to be looking at. docs/design.md: red
            // only ever means late.
            .highlight_style(Style::default().bg(render.colours.selection));

        frame.render_stateful_widget(list, inner, &mut screen.state);

        if banded != Band::None {
            band(
                frame,
                Rect::new(area.x, framed.y, area.width, banded.rows()),
                banded,
                counts,
                crate::agenda::week(
                    screen.all.iter().filter_map(|r| match r {
                        Row::Task(t) => Some(t),
                        _ => None,
                    }),
                    render.today,
                ),
                render,
            );
            footer(
                frame,
                Rect::new(
                    area.x,
                    framed.y + framed.height - FOOTER,
                    area.width,
                    FOOTER,
                ),
                screen.task(),
                render,
            );
        }
    }

    if let View::Help = view {
        help(frame, area, render);
    }
    match open {
        Open::Nothing => {}
        Open::Box(input) => input_box(frame, area, input, render),
        Open::Form(form) => form_box(frame, area, form, render),
    }

    let Some(bottom) = bottom else { return };
    // While the input is open the line names the two keys that end it, and
    // nothing else: the list keys under it are letters until `esc`, so
    // advertising them there would be a lie.
    let line = match open {
        // The third key is only named while it does something. `tab` opens the
        // date field, and once it is open the line says what ends *it* — the
        // two keys that end the box are the same two either way, and the row
        // has to be read at a glance rather than decoded.
        Open::Box(open) => Line::from(Span::styled(
            match (open.field.is_some(), size == Size::Wide) {
                (true, _) => format!(" {} date   esc back", render.glyphs.enter()),
                (false, true) => format!(" {} save   esc cancel   tab date", render.glyphs.enter()),
                (false, false) => format!(" {} save   esc cancel", render.glyphs.enter()),
            },
            Style::default().fg(render.colours.dim),
        )),
        // The form names `tab` on its own bottom border, where the eye already
        // is, so this line says the two keys that end it and nothing else.
        Open::Form(form) => Line::from(Span::styled(
            match form.input.field.is_some() {
                true => format!(" {} date   esc back", render.glyphs.enter()),
                false => format!(" {} create   esc cancel", render.glyphs.enter()),
            },
            Style::default().fg(render.colours.dim),
        )),
        Open::Nothing => notice.line(
            size,
            whole.width as usize,
            whole.height,
            render.glyphs,
            render.colours,
        ),
    };

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(render.colours.background)),
        bottom,
    );
}

/// The rule above the footer and the footer itself. Two rows, and the rule is
/// not optional: without it the file's own line reads as one more task row,
/// which is the one thing it is not.
const FOOTER: u16 = 2;

/// How much of the band at the top the pane can pay for.
///
/// It is the first thing to go, and it goes in two steps rather than one: the
/// tiles are worth five rows on a pane somebody leaves open and worth nothing on
/// a pane with six tasks in it — docs/tui.md#width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Band {
    /// The date, the tiles and the week. Five rows.
    Full,
    /// The same numbers on one line, and the rule under them. Two rows.
    Counts,
    /// Nothing, and the footer goes with it.
    None,
}

impl Band {
    fn of(height: u16, size: Size) -> Self {
        match height {
            // No frame to hang it in, and no width to lay tiles across.
            _ if size < Size::Wide => Band::None,
            20.. => Band::Full,
            16..=19 => Band::Counts,
            _ => Band::None,
        }
    }

    fn rows(self) -> u16 {
        match self {
            Band::Full => 5,
            Band::Counts => 2,
            Band::None => 0,
        }
    }
}

/// The seven cells of the week, as blocks. `None` under the ASCII fallback, and
/// that is a decision rather than an omission: a seven-cell bar chart made of
/// `#` and `-` is not a bar chart, it is a row of punctuation the reader has to
/// be told is a chart. It goes the way the columns go below eighty — the screen
/// is allowed to say less when it cannot say it well.
fn sparkline(week: [usize; 7], glyphs: Glyphs) -> Option<String> {
    if glyphs == Glyphs::Ascii {
        return None;
    }
    const CELLS: [&str; 8] = ["▁", "▁", "▂", "▃", "▅", "▆", "▇", "█"];
    let peak = *week.iter().max().unwrap_or(&0);
    if peak == 0 {
        return None;
    }
    Some(
        week.iter()
            .map(|&n| CELLS[(n * 7).div_ceil(peak).min(7)])
            .collect(),
    )
}

/// The tiles: a big number over a small label. The numbers are the ones
/// `ratodo status` already computes, so the band adds no state and no new data —
/// docs/redesign.md.
fn tiles(counts: Counts, week: [usize; 7], glyphs: Glyphs) -> Vec<(String, String)> {
    let (_, dot) = glyphs.punctuation();
    let total = counts.open + counts.done;
    let mut out = vec![
        (counts.overdue.to_string(), "OVERDUE".to_string()),
        (counts.today.to_string(), "TODAY".to_string()),
        (counts.open.to_string(), "OPEN".to_string()),
    ];
    if total > 0 {
        out.push((
            format!("{}/{total}", counts.done),
            format!("DONE {dot} {}%", counts.done * 100 / total),
        ));
    }
    if let Some(bars) = sparkline(week, glyphs) {
        let (dash, _) = glyphs.punctuation();
        out.push((bars, format!("MON {dash} SUN")));
    }
    out
}

/// Lays the tiles across the width, and gives back the number row and the label
/// row. A tile that will not fit whole is not drawn at all: half a label is
/// worse than one tile fewer.
fn tile_rows(tiles: &[(String, String)], width: usize) -> (String, String) {
    const INDENT: usize = 4;
    const GUTTER: usize = 4;
    let mut numbers = " ".repeat(INDENT);
    let mut labels = " ".repeat(INDENT);
    for (number, label) in tiles {
        let cell = columns(number).max(columns(label)) + GUTTER;
        if columns(&labels) + cell > width {
            break;
        }
        // Padded by measured columns rather than by `{:<w$}`, which counts
        // `char`s: `▅██▁▁▁▁` is seven columns and seven chars but twenty-one
        // bytes, and `·` is one column and two — a format width gets one of the
        // two rows right and slides the other one sideways.
        let pad = |text: &str| format!("{text}{}", " ".repeat(cell - columns(text)));
        numbers.push_str(&pad(number));
        labels.push_str(&pad(label));
    }
    (numbers, labels)
}

/// The band, drawn into the rows `draw` reserved for it. `area` is the frame's
/// interior plus the two border columns, so the rule at the bottom can meet the
/// frame at a `├` and a `┤` rather than stopping one column short of each.
fn band(
    frame: &mut Frame,
    area: Rect,
    kind: Band,
    counts: Counts,
    week: [usize; 7],
    render: Render<'_>,
) {
    let dim = Style::default().fg(render.colours.dim);
    let accent = Style::default().fg(render.colours.accent);
    let inner = area.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();
    match kind {
        Band::Full => {
            let (numbers, labels) = tile_rows(&tiles(counts, week, render.glyphs), inner);
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(numbers, accent)));
            lines.push(Line::from(Span::styled(labels, dim)));
            lines.push(Line::default());
        }
        // One line, and the labels stay rather than the numbers: `3 TODAY` still
        // reads without its second row, and a row of bare numbers does not.
        //
        // A week of nothing rather than the real one, which is how the sparkline
        // is dropped: it has no label that survives being read inline, and seven
        // cells wedged between two counts is a smudge. The labels lose their
        // second word for the same reason — `DONE · 45%` beside a `·` separator
        // is two dots doing different jobs on one line.
        Band::Counts => {
            let (_, dot) = render.glyphs.punctuation();
            let one = tiles(counts, [0; 7], render.glyphs)
                .iter()
                .map(|(n, l)| format!("{n} {}", l.split(' ').next().unwrap_or(l)))
                .collect::<Vec<_>>()
                .join(&format!("  {dot}  "));
            lines.push(Line::from(Span::styled(
                format!(
                    "  {}",
                    shorten(&one, inner.saturating_sub(2), render.glyphs)
                ),
                dim,
            )));
        }
        Band::None => return,
    }
    // The content goes inside the frame and the rule goes across it, so they are
    // two rectangles: a paragraph wide enough to hold the `├` would paint over
    // the `│` on every row above it.
    let rows = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(render.colours.background)),
        Rect::new(area.x + 1, area.y, inner as u16, rows),
    );
    frame.render_widget(
        Paragraph::new(rule_across(area.width as usize, render)),
        Rect::new(area.x, area.y + rows, area.width, 1),
    );
}

/// A rule the full width of the frame, meeting it at both ends. The same joint
/// the input box uses, and for the same reason: a rule that butts straight into
/// the side border reads as a broken frame.
fn rule_across(width: usize, render: Render<'_>) -> Line<'static> {
    let (left, right) = render.glyphs.tee();
    let middle = render
        .glyphs
        .rule()
        .to_string()
        .repeat(width.saturating_sub(2));
    Line::from(Span::styled(
        format!("{left}{middle}{right}"),
        Style::default().fg(render.colours.border),
    ))
}

/// The selected task's line, from the file, byte for byte.
///
/// One row, and it is the row that says *this is a file and this is your line in
/// it* on the screen somebody stares at all day. It is also the honest answer to
/// "did the tool understand what I typed", with no box open and nothing to press
/// — docs/redesign.md.
///
/// A task the tool has edited this session shows what **will** be written rather
/// than what was read: `raw` is only authoritative while `dirty` is false, and a
/// footer that lied about a line the user just ticked would be worse than none.
fn footer(frame: &mut Frame, area: Rect, task: Option<&Task>, render: Render<'_>) {
    let width = area.width.saturating_sub(2) as usize;
    let raw = task.map(|t| match t.dirty {
        true => t.line(),
        false => t.raw.clone(),
    });
    let line = Line::from(Span::styled(
        format!(
            "  {}",
            shorten(
                &text::plain(raw.as_deref().unwrap_or("")),
                width.saturating_sub(2),
                render.glyphs
            )
        ),
        Style::default().fg(render.colours.dim),
    ));
    frame.render_widget(
        Paragraph::new(rule_across(area.width as usize, render)),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(render.colours.background)),
        Rect::new(area.x + 1, area.y + 1, width as u16, 1),
    );
}

/// A bar as long as `n` is against `peak`, and **no trough behind it**: the
/// empty cells are drawn only where the whole length is the point, which is the
/// one bar at the top of the screen. Everywhere else a row of `░` behind every
/// bar turns a set of lengths the eye compares into a grid it has to read.
///
/// A count of nothing still gets one cell, so a day with no work on it is a
/// mark on the row rather than a hole in it — and a bar of one is never rounded
/// away to nothing, which would make `1` and `0` look the same.
fn bar_of(n: usize, peak: usize, width: usize, glyphs: Glyphs) -> String {
    let (full, empty) = glyphs.block();
    if peak == 0 || width == 0 {
        return String::new();
    }
    match n {
        0 => empty.to_string(),
        n => full.repeat((n * width).div_ceil(peak).clamp(1, width)),
    }
}

/// The one bar that keeps its trough: how far through the whole list you are is
/// a fraction, and a fraction needs its denominator drawn.
fn gauge(n: usize, of: usize, width: usize, glyphs: Glyphs) -> String {
    let (full, empty) = glyphs.block();
    if of == 0 || width == 0 {
        return String::new();
    }
    let filled = (n * width / of).min(width);
    format!("{}{}", full.repeat(filled), empty.repeat(width - filled))
}

/// `s` — the stats screen. Read-only arithmetic over what the file already
/// says, and the answer to "there is only one screen" — docs/tui.md#stats.
///
/// **No boxes and no rules between the blocks, deliberately.** The list is a
/// grid because its rows line up and are read across; this is five paragraphs
/// read one at a time, and a frame round each would be furniture with nothing to
/// hold. A statistics screen is exactly where a tool starts trying to look like
/// Grafana, and the restraint is spent here rather than argued about later.
fn stats_screen(frame: &mut Frame, area: Rect, stats: &Stats, period: Period, render: Render<'_>) {
    let (dash, dot) = render.glyphs.punctuation();
    let block = (Size::of(area.width) > Size::Bare).then(|| {
        Block::bordered()
            .border_set(render.glyphs.border())
            .border_style(Style::default().fg(render.colours.border))
            .title(format!(" ratodo / stats {dash} {} ", period.name()))
    });
    let inner = block.as_ref().map_or(area, |b| b.inner(area));
    if let Some(block) = block {
        frame.render_widget(block, area);
    }

    let width = inner.width as usize;
    let dim = Style::default().fg(render.colours.dim);
    let accent = Style::default().fg(render.colours.accent);
    let done = Style::default().fg(render.colours.done);

    // Each block is drawn whole or not at all, and they go in the order
    // docs/tui.md#stats sets out: the two-column block first, then the daily
    // labels, then the histogram. Never a scrollbar — this screen is glanceable
    // or it is nothing.
    let mut blocks: Vec<Vec<Line<'static>>> = Vec::new();

    // The header: four numbers and the bar under them. Laid out as entries with
    // a gap rather than a format string with counted spaces, so the row narrows
    // by losing a whole word instead of by being cut mid-number.
    let total = stats.total.max(1);
    let mut header = "  ".to_string();
    for (n, label) in [
        (stats.total, "tasks"),
        (stats.done, "done"),
        (stats.open, "open"),
        (stats.overdue, "overdue"),
    ] {
        let entry = format!("{n} {label}");
        if columns(&header) + columns(&entry) + 2 > width {
            break;
        }
        header.push_str(&entry);
        header.push_str("      ");
    }
    blocks.push(vec![
        Line::default(),
        Line::from(Span::styled(header, accent)),
        Line::from(vec![
            Span::styled(
                format!(
                    "  {}",
                    gauge(stats.done, total, width.saturating_sub(10), render.glyphs)
                ),
                done,
            ),
            Span::styled(format!("  {}%", stats.done * 100 / total), dim),
        ]),
    ]);

    // The histogram: labels, bars, counts.
    let cells = stats.buckets.len().max(1);
    let cell = (width.saturating_sub(4) / cells).max(1);
    let peak = stats.buckets.iter().map(|(_, n)| *n).max().unwrap_or(0);
    let across = |pick: &dyn Fn(&(String, usize)) -> String| {
        let mut out = "    ".to_string();
        for bucket in &stats.buckets {
            let text = pick(bucket);
            out.push_str(&format!(
                "{text}{}",
                " ".repeat(cell.saturating_sub(columns(&text)))
            ));
        }
        out
    };
    let labels = across(&|(label, _)| label.clone());
    let bars = across(&|(_, n)| bar_of(*n, peak, cell.saturating_sub(2), render.glyphs));
    let numbers = across(&|(_, n)| n.to_string());

    blocks.push(vec![
        Line::default(),
        Line::from(Span::styled(
            format!("  DONE THIS {}", period.name()),
            Style::default().fg(render.colours.foreground).bold(),
        )),
        Line::default(),
        Line::from(Span::styled(labels, dim)),
        Line::from(Span::styled(bars, done)),
        Line::from(Span::styled(numbers, dim)),
    ]);

    // Priority on the left, sections on the right. Two columns of the same
    // block, so they drop together — half of a pair reads as a rendering fault.
    let half = width / 2;
    let rows = stats.priority.len().max(stats.sections.len());
    let prio_peak = stats.priority.iter().map(|(_, n)| *n).max().unwrap_or(0);
    let section_peak = stats.sections.iter().map(|(_, n)| *n).max().unwrap_or(0);
    let mut two = vec![
        Line::default(),
        Line::from(vec![
            Span::styled(
                format!("  PRIORITY{}", " ".repeat(half.saturating_sub(10))),
                Style::default().fg(render.colours.foreground).bold(),
            ),
            Span::styled(
                "SECTIONS",
                Style::default().fg(render.colours.foreground).bold(),
            ),
        ]),
        Line::default(),
    ];
    for n in 0..rows {
        let side = |entry: Option<(&str, usize)>, peak: usize, label: usize| match entry {
            None => " ".repeat(half),
            Some((name, count)) => {
                let name = shorten(name, label, render.glyphs);
                // Capped, and this is the one place a bar is allowed to be:
                // the eye reads these against each other, not against the pane.
                let bar = bar_of(
                    count,
                    peak,
                    half.saturating_sub(label + 8).min(16),
                    render.glyphs,
                );
                let text = format!(
                    "  {name}{} {bar} {count}",
                    " ".repeat(label.saturating_sub(columns(&name)))
                );
                format!("{text}{}", " ".repeat(half.saturating_sub(columns(&text))))
            }
        };
        two.push(Line::from(vec![
            Span::styled(
                side(stats.priority.get(n).map(|(p, c)| (*p, *c)), prio_peak, 6),
                dim,
            ),
            Span::styled(
                side(
                    stats.sections.get(n).map(|(s, c)| (s.as_str(), *c)),
                    section_peak,
                    // A third of the column, up to twelve. A fixed twelve at
                    // forty-four leaves two columns for the bar, which is not a
                    // bar — the name has to give some back at that width.
                    (half / 3).min(12),
                ),
                dim,
            ),
        ]));
    }
    blocks.push(two);

    // The three summary numbers, and the one caveat that belongs on a screen
    // rather than in a document.
    let mut tail = vec![
        Line::default(),
        Line::from(Span::styled(
            format!(
                "  best {}   {}      avg / day   {}.{}      streak   {}",
                match period {
                    Period::Week => "day",
                    Period::Month => "week",
                    Period::Year => "month",
                },
                stats
                    .best
                    .as_ref()
                    .map(|(label, _)| label.clone())
                    .unwrap_or_else(|| dash.to_string()),
                stats.per_day_x10 / 10,
                stats.per_day_x10 % 10,
                match stats.streak {
                    1 => "1 day".to_string(),
                    n => format!("{n} days"),
                }
            ),
            dim,
        )),
    ];
    if stats.unstamped > 0 {
        tail.push(Line::default());
        tail.push(Line::from(Span::styled(
            format!(
                "  {} finished before ratodo stamped the day {} in the totals, in no bar",
                stats.unstamped, dot
            ),
            Style::default().fg(render.colours.overdue),
        )));
    }
    blocks.push(tail);

    // What the pane can pay for, dropped in the documented order rather than
    // scrolled — docs/tui.md#stats. `blocks` is header, histogram, two-column,
    // tail. The two-column block goes first, then the histogram gives up its day
    // labels, then the histogram itself. Every other screen in this product has
    // a documented answer to "what happens in ten rows"; this is that answer,
    // and it is not a scrollbar.
    const LABELS: usize = 3;
    let room = inner.height as usize;
    let mut keep = [true, true, true, true];
    let mut labels = true;
    let spent = |keep: [bool; 4], labels: bool, blocks: &[Vec<Line<'static>>]| -> usize {
        blocks
            .iter()
            .zip(keep)
            .filter(|(_, k)| *k)
            .map(|(b, _)| b.len())
            .sum::<usize>()
            - usize::from(!labels && keep[1])
    };
    for step in 0..3 {
        if spent(keep, labels, &blocks) <= room {
            break;
        }
        match step {
            0 => keep[2] = false,
            1 => labels = false,
            _ => keep[1] = false,
        }
    }
    if !labels {
        blocks[1].remove(LABELS);
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (block, k) in blocks.into_iter().zip(keep) {
        if k {
            lines.extend(block);
        }
    }
    lines.truncate(room);

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(render.colours.background)),
        inner,
    );
}

/// The line as it is being typed, coloured by what the tokenizer **took** — a
/// `@notaday` stays plain here exactly as it will in the file. The window is
/// anchored on the caret, so a line longer than the box scrolls under it.
///
/// Returns the caret's column as well, because it is the same arithmetic.
fn typed_line(input: &Input, width: usize, render: Render<'_>) -> (Line<'static>, usize) {
    let plain = Style::default().fg(render.colours.foreground);
    let before = tail(&input.text[..input.at], width);
    let after = lead(
        &input.text[input.at..],
        width.saturating_sub(columns(&before)),
    );
    let (from, to) = (input.at - before.len(), input.at + after.len());
    let parsed = crate::capture::capture(&input.text, render.today);

    let mut spans = Vec::new();
    let mut cut = from;
    for (word, part) in crate::capture::parts(&input.text, render.today) {
        if part == Part::Text || word.end <= from || word.start >= to {
            continue;
        }
        let (start, end) = (word.start.max(from), word.end.min(to));
        if start > cut {
            spans.push(Span::styled(input.text[cut..start].to_string(), plain));
        }
        spans.push(Span::styled(
            input.text[start..end].to_string(),
            paint(part, parsed.priority, plain, render),
        ));
        cut = end;
    }
    spans.push(Span::styled(input.text[cut..to].to_string(), plain));
    (Line::from(spans), columns(&before))
}

/// `◉` against `○`, and `(o)` against `( )` in ASCII. A difference in **shape**,
/// so the selection survives `NO_COLOR=1` and the fallback: `[ MED ]` with the
/// choice carried by colour alone breaks the rule in docs/design.md.
fn radios(choices: &[(String, bool)], glyphs: Glyphs) -> String {
    let (on, off) = match glyphs {
        Glyphs::Unicode => ("◉", "○"),
        Glyphs::Ascii => ("(o)", "( )"),
    };
    choices
        .iter()
        .map(|(label, chosen)| format!("{} {label}", if *chosen { on } else { off }))
        .collect::<Vec<_>>()
        .join("  ")
}

/// `a`, the add screen — the form. Screens 2 and 3 of docs/redesign.md, and the
/// reversal it rests on is docs/decisions.md.
///
/// A centred overlay, and the pane comes back the moment it closes. Under
/// `Form::fits` it is not drawn at all and `a` opens the one-line box instead:
/// a form that half-fits is worse than a box that always fits.
fn form_box(frame: &mut Frame, area: Rect, form: &Form, render: Render<'_>) {
    let width = 64.min(area.width.saturating_sub(4));
    // Two of block border, and two of margin either side of the content so that
    // nothing inside closes flush against the frame — the same two the group
    // boxes on the list hold back.
    let inner = (width as usize).saturating_sub(6);
    let dim = Style::default().fg(render.colours.dim);
    let plain = Style::default().fg(render.colours.foreground);
    let accent = Style::default().fg(render.colours.accent);
    let border = Style::default().fg(render.colours.border);

    // `▌` sits beside the **control** that has the keyboard, not beside its
    // label: the marker points at what the keys are going to reach, and the
    // label is not it. Same marker and same colour as the selected row on the
    // list — docs/redesign.md.
    let mark = |field: Field| -> Span<'static> {
        match form.focus == field {
            true => Span::styled(render.glyphs.cursor().trim_end().to_string(), accent),
            false => Span::raw(" "),
        }
    };
    let row = |field: Field, body: Vec<Span<'static>>| -> Line<'static> {
        let mut spans = vec![mark(field), Span::raw(" ")];
        if !field.label().is_empty() {
            spans.push(Span::styled(format!("{:<10}", field.label()), dim));
        }
        spans.extend(body);
        Line::from(spans)
    };

    // The text box is drawn by hand rather than with a `Block`, so the focus
    // marker can sit in the column to its left: a block owns its whole
    // rectangle and there is nowhere outside it to put one.
    let (typed, caret) = typed_line(&form.input, inner.saturating_sub(4), render);
    let [top_left, top_right, bottom_left, bottom_right] = render.glyphs.corners();
    let side = render.glyphs.divider();
    let edge = match form.focus == Field::Title {
        true => accent,
        false => border,
    };
    let rule = render
        .glyphs
        .rule()
        .to_string()
        .repeat(inner.saturating_sub(2));
    let mut lines: Vec<Line<'static>> = vec![
        // Their question, and it is better than `Title`: in a form there is room
        // for a sentence, and it replaces the syntax-by-example hint that only
        // ever had that job because there was nowhere else to put one.
        Line::from(vec![
            Span::raw("  "),
            Span::styled("What needs to be done?", plain),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{top_left}{rule}{top_right}"), edge),
        ]),
        Line::from({
            // The marker takes the gutter column and the box starts where its
            // own corners do, or the row slides one column out from under them.
            let mut spans = vec![
                mark(Field::Title),
                Span::raw(" "),
                Span::styled(format!("{side} "), edge),
            ];
            let pad = inner.saturating_sub(4 + typed.width());
            spans.extend(typed.spans);
            spans.push(Span::raw(" ".repeat(pad + 1)));
            spans.push(Span::styled(side.to_string(), edge));
            spans
        }),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{bottom_left}{rule}{bottom_right}"), edge),
        ]),
    ];

    let fields_at = lines.len();
    for field in form.order() {
        let body: Vec<Span<'static>> = match field {
            Field::Title | Field::Cancel | Field::Create => continue,
            Field::Time | Field::Tags => {
                let shown = match (field, form.focus == field) {
                    (_, true) => form.typed_text(),
                    (Field::Time, false) => {
                        part_of(&form.input.text, render.today, Part::Time).unwrap_or_default()
                    }
                    _ => tags_of(&form.input.text, render.today),
                };
                let room = inner.saturating_sub(14); // label, brackets, marker
                vec![Span::styled(
                    format!("[ {:<room$} ]", shorten(&shown, room, render.glyphs)),
                    plain,
                )]
            }
            // `Due · pick…` opens the three-part date field the box already
            // has, and it takes the row over while it is up: the radios and the
            // picker are two answers to one question and only one of them can
            // be the live one — docs/tui.md#the-date-field--tab.
            Field::Due if form.input.field.is_some() => {
                let open = form.input.field.as_ref().expect("just checked");
                let mut spans = Vec::new();
                for (part, text) in open.cells() {
                    spans.push(Span::styled(
                        text,
                        match part == open.part {
                            true => accent.bold(),
                            false => plain,
                        },
                    ));
                }
                spans.push(Span::styled(
                    format!(" {}", render.glyphs.arrows()),
                    Style::default().fg(render.colours.dim),
                ));
                spans
            }
            _ => vec![Span::styled(
                shorten(
                    &radios(&form.choices_for(field), render.glyphs),
                    inner.saturating_sub(10),
                    render.glyphs,
                ),
                plain,
            )],
        };
        lines.push(row(field, body));
    }

    // `PREVIEW`, with its own label and its own rule above it. The difference
    // between a form that happens to show a line and a form whose *conclusion*
    // is a line: this one saves into your file, so the file is the last word on
    // the screen — docs/redesign.md.
    let preview_at = lines.len();
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(render.glyphs.rule().to_string().repeat(inner), border),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "PREVIEW",
            Style::default().fg(render.colours.foreground).bold(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            shorten(
                &crate::capture::capture(&form.input.text, render.today).line(),
                inner,
                render.glyphs,
            ),
            dim,
        ),
    ]));

    // The buttons stay, and they carry their key: `[ ⏎ create task ]` is both
    // the button the mockups drew and the keybinding, so it is honest on a
    // keyboard and still looks like a button.
    let create = format!("[ {} create task ]", render.glyphs.enter());
    let cancel = "[ esc cancel ]";
    let gap = inner.saturating_sub(columns(cancel) + columns(&create) + 2);
    let buttons_at = lines.len();
    lines.push(Line::from(vec![
        mark(Field::Cancel),
        Span::raw(" "),
        Span::styled(cancel.to_string(), plain),
        Span::raw(" ".repeat(gap)),
        mark(Field::Create),
        Span::raw(" "),
        Span::styled(create, plain),
    ]));

    // Blank rows are the give. They go in between the four blocks — question,
    // fields, preview, buttons — one at a time from the bottom up, and the form
    // is not drawn at all below `Form::fits`.
    let mut spare = (area.height as usize).saturating_sub(lines.len() + 2);
    for at in [buttons_at, preview_at, fields_at].into_iter() {
        if spare == 0 {
            break;
        }
        spare -= 1;
        lines.insert(at, Line::default());
    }

    let height = (lines.len() as u16 + 2).min(area.height);
    let box_area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );

    frame.render_widget(Clear, box_area);
    frame.render_widget(
        Paragraph::new(lines.clone()).block(
            Block::bordered()
                .border_set(render.glyphs.border())
                .border_style(Style::default().fg(render.colours.accent))
                .style(Style::default().bg(render.colours.background))
                .title(Line::from(Span::styled(" NEW TASK ", accent.bold())).centered())
                .title_bottom(Span::styled(
                    format!(" tab {} next field ", render.glyphs.punctuation().1),
                    dim,
                )),
        ),
        box_area,
    );

    // The terminal's own cursor, and only where there is text to type: on a
    // radio row it would blink at a control that does not take characters.
    if form.focus == Field::Title {
        let at = lines
            .iter()
            .position(|line| line.spans.len() > 1 && line.spans[1].content.starts_with(side))
            .unwrap_or(2);
        frame.set_cursor_position((box_area.x + 3 + caret as u16, box_area.y + 1 + at as u16));
    }
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
    // Border, field, rule, preview — and the preview is what goes first, exactly
    // as it did on the bottom line. Under three rows there is nothing to draw at
    // all: two of them are border and the field would have nowhere to sit, so the
    // pane keeps its tasks and the bottom line still names the keys.
    let height = 5.min(area.height);
    if height < 3 {
        return;
    }
    let box_area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );

    let (mut lines, at) = input_lines(input, (width as usize).saturating_sub(2), render);
    // A pane with one row to spare gives it to the preview: the rule is there to
    // separate two things, and with only one of them on screen it separates
    // nothing while costing the more useful line.
    if height < 5 {
        lines.remove(1);
    }

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
    if height >= 5 {
        tee(frame, box_area, render.glyphs, render.colours.accent);
    }
    // The terminal's own cursor, not a drawn block: it blinks the way every
    // other text field the user has ever typed into does, and it costs a line.
    frame.set_cursor_position((box_area.x + 1 + at as u16, box_area.y + 1));
}

/// Joins the rule under the field to the two side borders. The rule is drawn as
/// text inside the block, which knows nothing about it, so the two cells where
/// they meet are set afterwards — a rule butting into `│` reads as a frame that
/// broke rather than a divider.
fn tee(frame: &mut Frame, box_area: Rect, glyphs: Glyphs, colour: Color) {
    let (left, right) = glyphs.tee();
    let y = box_area.y + 2;
    let buffer = frame.buffer_mut();
    buffer[(box_area.x, y)].set_symbol(left).set_fg(colour);
    buffer[(box_area.x + box_area.width - 1, y)]
        .set_symbol(right)
        .set_fg(colour);
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
    // `:` and `/` were here and are not any more. The rule above is that this
    // lists keys that do something, and those two do not — pressing either
    // answers in the status line, which is the moment it teaches anything. The
    // row they cost is what keeps the box inside a fourteen-row pane.
    let keys: [(String, &str); 11] = [
        (format!("j k  {}", render.glyphs.arrows()), "move"),
        ("g G".to_string(), "top / bottom"),
        ("ctrl-d ctrl-u".to_string(), "half page"),
        ("spc".to_string(), "toggle done"),
        // Three to this row rather than an eleventh: at ten keys plus a border
        // the box still fits a fourteen-row pane, and `y` is a third way into
        // the same input box, so it belongs beside the other two anyway.
        (
            format!("a o  {}  y", render.glyphs.enter()),
            "add / edit / copy",
        ),
        ("X  u".to_string(), "delete / undo"),
        ("d  p".to_string(), "cancel / put off"),
        ("h l  z".to_string(), "fold this group"),
        // The eleventh, and it gets a row of its own rather than doubling up:
        // the comment two rows down gives the ceiling as twelve keys plus the
        // border on a fourteen-row pane, so there is one row still spare.
        ("s".to_string(), "stats"),
        // Two keys to a row, so that the box still fits a fourteen-row pane.
        // At twelve rows of keys the border takes `q  ctrl-c` off the bottom,
        // and a help screen that cuts off at quit is worse than none.
        ("e  r".to_string(), "$EDITOR / re-read"),
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
    // The welcome, and it is two lines. **No ASCII-art logo**: this is a pane
    // somebody leaves open beside their work, and a banner is charming exactly
    // once — docs/redesign.md#first-run. They go only where there is height to
    // spare, because on a short pane the thing that teaches is the box below.
    let mut lines = Vec::new();
    if inner.height >= 14 {
        lines.push(Line::raw(""));
        lines.push(
            Line::styled("ratodo", Style::default().fg(render.colours.accent).bold()).centered(),
        );
        lines.push(Line::styled("a todo list that is still just a file", dim).centered());
    }
    lines.extend([
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
    ]);
    // Five rows for the box under the rows above it. Where they do not fit, the
    // example stays a line of text: it is the part that teaches, so it is the
    // last thing a short pane is allowed to lose.
    let room = inner.height >= 11 && inner.width >= 34;
    if !room {
        lines.push(Line::styled(
            format!("  Try:  a  then  {EXAMPLE}"),
            Style::default().fg(render.colours.accent),
        ));
    }

    let below = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(render.colours.background)),
        inner,
    );

    if room {
        // Directly under the rows just written, whether or not the welcome is
        // one of them: a box positioned by a constant slides behind the text the
        // moment anything above it changes length.
        example(frame, inner, below, render);
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
fn example(frame: &mut Frame, inner: Rect, below: u16, render: Render<'_>) {
    let width = 48.min(inner.width.saturating_sub(4));
    let area = Rect::new(inner.x + 2, inner.y + below, width, 5);
    let (lines, _) = input_lines(
        &Input::new(EXAMPLE.to_string(), Purpose::Add),
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
    tee(frame, area, render.glyphs, render.colours.border);
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

/// Green is for done and red for the negative outcome — docs/design.md#rules —
/// so this is the whole of the colour logic and there is nowhere else to add to
/// it.
///
/// Green was spent on the progress bar alone and the row that earned it was
/// grey, which made ticking a task the one action on this screen that said
/// nothing back. Red widened from "only for overdue" to cover cancelled as
/// well, a deliberate reversal recorded in docs/decisions.md: `✗` against `!`
/// is what tells those two apart, and the rule that nothing is carried by
/// colour alone is why a shared colour is enough.
fn task_colour(task: &Task, today: NaiveDate, colours: Theme) -> Color {
    match task.state {
        State::Done => colours.done,
        State::Cancelled => colours.overdue,
        State::Open if task.is_overdue(today) => colours.overdue,
        State::Open if task.due.is_some_and(|d| d.date == today) => colours.today,
        State::Open => colours.foreground,
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
            lists: &[],
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
                Row::GroupEnd => String::new(),
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
            ["# OVERDUE", "late", "", "# ## Work", "write the plan", ""]
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

    /// One key opens the second screen and the same key closes it, the way `?`
    /// works. The three period keys are answered here whatever is on screen —
    /// the loop is where they stop meaning anything on the list, because that is
    /// where it knows which screen is up.
    #[test]
    fn the_keys_of_the_second_screen() {
        assert_eq!(action(press(KeyCode::Char('s'))), Action::Stats);
        assert_eq!(
            action(press(KeyCode::Char('1'))),
            Action::Over(Period::Week)
        );
        assert_eq!(
            action(press(KeyCode::Char('2'))),
            Action::Over(Period::Month)
        );
        assert_eq!(
            action(press(KeyCode::Char('3'))),
            Action::Over(Period::Year)
        );
        // And `esc` closes it, which is the same key that closes the overlay.
        assert_eq!(action(press(KeyCode::Esc)), Action::Close);
        // Not a fourth period, and `S` is not a second door.
        assert_eq!(action(press(KeyCode::Char('4'))), Action::Ignore);
        assert_eq!(action(press(KeyCode::Char('S'))), Action::Ignore);
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
        for code in [KeyCode::Char('x'), KeyCode::Char('w'), KeyCode::Char('P')] {
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
                        match helping {
                            true => View::Help,
                            false => View::List,
                        },
                        Open::Nothing,
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
                    View::Help,
                    Open::Nothing,
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
                "╭ ra╭ keys ────────────────────────────────╮───╮",
                "│  ╭│  j k  ↓ ↑       move                 │╮  │",
                "│▌ ││  g G            top / bottom         ││  │",
                "│  ╰│  ctrl-d ctrl-u  half page            │╯  │",
                "│   │  spc            toggle done          │   │",
                "│   │  a o  ⏎  y      add / edit / copy    │   │",
                "│   │  X  u           delete / undo        │   │",
                "│   │  d  p           cancel / put off     │   │",
                "│   │  h l  z         fold this group      │   │",
                "│   │  s              stats                │   │",
                "│   │  e  r           $EDITOR / re-read    │   │",
                "│   │  q  ctrl-c      quit                 │   │",
                "╰───╰───────── esc or ? to close ──────────╯───╯",
                " [j k] [spc] [a] [d] [p] [?] [q]                ",
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

        // Eleven keys plus two of border is thirteen rows, so a fifteen-row
        // pane puts it at the top with nothing to spare.
        for (height, top) in [(15u16, 0usize), (17, 1), (21, 3)] {
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
                        View::Help,
                        Open::Nothing,
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
                    View::Help,
                    Open::Nothing,
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
        // Without the modifier they are cancel and undo.
        assert_eq!(action(press(KeyCode::Char('d'))), Action::Cancel);
        assert_eq!(action(press(KeyCode::Char('u'))), Action::Undo);
        // Delete is the shifted one, and the bare letter must never reach it:
        // `x` is unbound and `d` is the reversible neighbour.
        assert_eq!(action(press(KeyCode::Char('X'))), Action::Delete);
        assert_eq!(action(press(KeyCode::Char('x'))), Action::Ignore);
        // `y` is the vim yank the hand goes for; `p` is not free to be the
        // paste, because it has put a date off since v0.2.0.
        assert_eq!(action(press(KeyCode::Char('y'))), Action::Duplicate);
        assert_eq!(action(press(KeyCode::Char('p'))), Action::Postpone);
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

    /// A group is a heading, its tasks and a closing row — one, *n*, one. The
    /// row that closes it used to be the blank spacer *between* two groups and
    /// is now the bottom edge of the box the group is drawn in, so the last
    /// group has one where it used to end in mid-air.
    #[test]
    fn a_group_is_a_heading_its_tasks_and_a_closing_row() {
        let tasks = tasks(&["late @2026-08-01", "now @2026-08-10", "also @2026-08-10"]);
        let groups = agenda(&tasks, today());
        assert_eq!(
            titles(&rows(&groups)),
            ["# OVERDUE", "late", "", "# TODAY", "now", "also", ""]
        );
    }

    /// No leading blank row: a pane that opens with an empty first line looks
    /// like it failed to draw.
    #[test]
    fn the_first_group_gets_no_spacer_above_it() {
        let tasks = tasks(&["a @2026-08-10"]);
        assert_eq!(rows(&agenda(&tasks, today()))[0], Row::header("TODAY", 1));
    }

    /// An untitled group is the run of tasks above the file's first heading. It
    /// gets a header row with nothing on it — a box with no name, rather than a
    /// "(no section)" nobody wrote or two rows left floating beside the boxes.
    #[test]
    fn an_untitled_group_gets_a_box_with_no_name_on_it() {
        let tasks = tasks(&["a", "b"]);
        assert_eq!(
            titles(&rows(&agenda(&tasks, today()))),
            ["# ", "a", "b", ""]
        );
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
        ticked.set_state(State::Done, today());
        let mut after = in_section(&[("inserted", "S"), ("one", "S")]);
        after.push(ticked);
        after.extend(in_section(&[("three", "S")]));

        screen.replace(rows(&agenda(&after, today())));
        assert_eq!(
            screen.task().map(|t| t.title.as_str()),
            Some("two"),
            "the cursor followed the raw line instead of the task"
        );
        assert!(screen.task().unwrap().done());
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
                    View::List,
                    Open::Nothing,
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

    /// A list with something finished on three days of the week, so the band
    /// has a sparkline to draw and a footer has a line to show.
    fn a_week_of_work() -> Vec<Task> {
        let mut out = tasks(&["late @2026-08-08 #ops", "now @2026-08-10 16:00"]);
        for (title, day) in [("mon", 10), ("wed", 12), ("wed too", 12)] {
            let mut done = capture(title, today());
            done.set_state(State::Done, NaiveDate::from_ymd_opt(2026, 8, day).unwrap());
            out.push(done);
        }
        out
    }

    /// The band, exactly, at the width the drawings are made at. Five rows: the
    /// blank, the numbers, the labels, the blank, and the rule that meets the
    /// frame at both ends — docs/redesign.md.
    #[test]
    fn the_band_exactly() {
        let screen = rendered(80, 24, &a_week_of_work());
        assert_eq!(
            &screen[..6],
            [
                "╭ ratodo ────────────────────────────────────────────── Monday, 10 August 2026 ╮",
                "│                                                                              │",
                "│    1          1        2       3/5           ▅▁█▁▁▁▁                         │",
                "│    OVERDUE    TODAY    OPEN    DONE · 60%    MON — SUN                       │",
                "│                                                                              │",
                "├──────────────────────────────────────────────────────────────────────────────┤",
            ]
        );
    }

    /// The band owns the counts while it is drawn, so the title bar spends its
    /// right-hand side on the date instead — and takes them back the moment the
    /// band goes, rather than leaving the pane with no counts at all.
    #[test]
    fn the_title_bar_says_the_date_only_while_the_band_is_carrying_the_counts() {
        let tasks = a_week_of_work();

        let tall = rendered(80, 24, &tasks);
        assert!(tall[0].contains("Monday, 10 August 2026"), "{tall:?}");
        assert!(
            !tall[0].contains("open"),
            "the counts are said twice: {tall:?}"
        );

        let short = rendered(80, 14, &tasks);
        assert!(short[0].contains("2 open · 1 overdue"), "{short:?}");
        assert!(!short[0].contains("August"), "{short:?}");
    }

    /// The first thing to go, in two steps rather than one, and the footer goes
    /// with the last of them — docs/tui.md#width.
    #[test]
    fn the_band_gives_way_a_step_at_a_time_as_the_pane_shortens() {
        let tasks = a_week_of_work();
        let band_of = |height: u16| {
            let screen = rendered(80, height, &tasks);
            let sparkline = screen.iter().any(|r| r.contains('█'));
            let labels = screen.iter().any(|r| r.contains("OVERDUE    TODAY"));
            let inline = screen.iter().any(|r| r.contains("1 OVERDUE  ·"));
            let footer = screen.iter().any(|r| r.contains("- [ ] late"));
            (sparkline, labels, inline, footer)
        };

        assert_eq!(band_of(24), (true, true, false, true), "the whole band");
        assert_eq!(band_of(20), (true, true, false, true), "still whole at 20");
        assert_eq!(
            band_of(19),
            (false, false, true, true),
            "one line of counts"
        );
        assert_eq!(
            band_of(16),
            (false, false, true, true),
            "still one line at 16"
        );
        assert_eq!(band_of(15), (false, false, false, false), "band and footer");

        // And a pane too narrow for tiles never gets one however tall it is.
        assert!(
            !rendered(50, 30, &tasks)
                .iter()
                .any(|r| r.contains("OVERDUE    TODAY"))
        );
    }

    /// The row that says *this is a file and this is your line in it*. Byte for
    /// byte, and it follows the cursor rather than the top of the list.
    #[test]
    fn the_footer_is_the_selected_tasks_own_line_from_the_file() {
        let tasks = a_week_of_work();
        let on_first = rendered(80, 24, &tasks);
        assert!(
            on_first
                .iter()
                .any(|r| r.contains("- [ ] late @2026-08-08 #ops")),
            "{on_first:?}"
        );

        let moved = rendered_with(80, 24, &tasks, |s| s.move_by(1));
        assert!(
            moved
                .iter()
                .any(|r| r.contains("- [ ] now @2026-08-10 16:00")),
            "the footer did not follow the cursor: {moved:?}"
        );
        assert!(
            !moved.iter().any(|r| r.contains("- [ ] late @2026-08-08")),
            "{moved:?}"
        );
    }

    /// A seven-cell bar chart made of `#` and `-` is not a bar chart. It goes
    /// the way the columns go below eighty, and the rest of the band stays —
    /// todo.md, decided before it was drawn rather than at the assertion.
    #[test]
    fn the_sparkline_has_no_ascii_form_and_says_so_by_not_being_there() {
        let week = [1, 0, 3, 0, 0, 0, 0];
        assert_eq!(sparkline(week, Glyphs::Unicode).as_deref(), Some("▃▁█▁▁▁▁"));
        assert_eq!(sparkline(week, Glyphs::Ascii), None);
        // Nothing finished this week is not a chart either.
        assert_eq!(sparkline([0; 7], Glyphs::Unicode), None);

        // The tallest day is always full and a day with nothing in it is always
        // the floor, so the shape is a shape and not a rounding artefact.
        let tall = sparkline([9, 1, 0, 0, 0, 0, 0], Glyphs::Unicode).unwrap();
        assert!(tall.starts_with('█'), "{tall}");
        assert!(tall.ends_with("▁▁▁▁▁"), "{tall}");
    }

    /// The band is furniture like everything else, and furniture is where the
    /// ASCII fallback has escaped twice before.
    #[test]
    fn the_band_and_the_footer_are_ascii_under_a_c_locale() {
        let tasks = a_week_of_work();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let counts = Counts::of(&tasks, today());
        let render = Render {
            glyphs: Glyphs::Ascii,
            ..render(crate::theme::MOCHA)
        };

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render,
                    &Notice::Hints,
                    View::List,
                    Open::Nothing,
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

        assert!(text.contains("OVERDUE"), "the band did not draw: {text}");
        assert!(text.contains("- [ ] late @2026-08-08 #ops"), "{text}");
        assert!(
            text.is_ascii(),
            "something non-ASCII reached the screen: {text}"
        );
    }

    fn stats_of(width: u16, height: u16, tasks: &[Task], period: Period) -> Vec<String> {
        let groups = agenda(tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let stats = crate::agenda::stats(tasks, today(), period);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::of(tasks, today()),
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    View::Stats(&stats, period),
                    Open::Nothing,
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

    /// The whole screen, exactly. A **screen and not an overlay**: the list is
    /// not under it, because nothing on it is glanced at mid-task.
    #[test]
    fn the_stats_screen_exactly() {
        let screen = stats_of(66, 22, &a_week_of_work(), Period::Week);
        assert_eq!(
            screen,
            [
                "╭ ratodo / stats — WEEK ─────────────────────────────────────────╮",
                "│                                                                │",
                "│  5 tasks      3 done      2 open      1 overdue                │",
                "│  ████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░  60%   │",
                "│                                                                │",
                "│  DONE THIS WEEK                                                │",
                "│                                                                │",
                "│    MON     TUE     WED     THU     FRI     SAT     SUN         │",
                "│    ███     ░       ██████  ░       ░       ░       ░           │",
                "│    1       0       2       0       0       0       0           │",
                "│                                                                │",
                "│  PRIORITY                      SECTIONS                        │",
                "│                                                                │",
                "│  !high   0                       (none)     ██████████████ 5   │",
                "│  !med    0                                                     │",
                "│  !low    0                                                     │",
                "│                                                                │",
                "│  best day   WED      avg / day   1.0      streak   1 day       │",
                "│                                                                │",
                "│                                                                │",
                "╰────────────────────────────────────────────────────────────────╯",
                " [1] week  [2] month  [3] year   [r] reload   [esc] back          ",
            ]
        );
    }

    /// `1` `2` `3` change what is being counted, and the heading says which — a
    /// screen of numbers that does not say what they are over is a screen of
    /// numbers.
    #[test]
    fn the_three_keys_change_the_period_and_the_screen_says_which() {
        for (period, name, first) in [
            (Period::Week, "WEEK", "MON"),
            (Period::Month, "MONTH", "W1"),
            (Period::Year, "YEAR", "JAN"),
        ] {
            let screen = stats_of(80, 22, &a_week_of_work(), period);
            assert!(screen[0].contains(name), "{period:?}: {screen:?}");
            assert!(
                screen
                    .iter()
                    .any(|r| r.contains(&format!("DONE THIS {name}"))),
                "{period:?}: {screen:?}"
            );
            assert!(
                screen
                    .iter()
                    .any(|r| r.trim_matches(['│', ' ']).starts_with(first)),
                "{period:?}: {screen:?}"
            );
        }
    }

    /// What the screen does in a short pane, which is the question every other
    /// screen in this product already had an answer to. The order is the one in
    /// docs/tui.md#stats: the two-column block, then the day labels, then the
    /// histogram — and never a scrollbar.
    #[test]
    fn the_stats_screen_drops_its_blocks_in_the_documented_order() {
        let tasks = a_week_of_work();
        let has = |height: u16, needle: &str| {
            stats_of(80, height, &tasks, Period::Week)
                .iter()
                .any(|r| r.contains(needle))
        };

        assert!(has(22, "PRIORITY") && has(22, "MON") && has(22, "DONE THIS"));
        assert!(!has(14, "PRIORITY"), "the two-column block goes first");
        assert!(has(14, "MON") && has(14, "DONE THIS"));
        assert!(!has(13, "MON"), "then the day labels");
        assert!(has(13, "DONE THIS"));
        assert!(!has(12, "DONE THIS"), "then the histogram");
        // And the header and the summary line are what is left standing.
        assert!(has(12, "tasks") && has(12, "streak"));
    }

    /// The caveat that belongs on the screen rather than in a document: a task
    /// ticked before the stamp existed is in the totals and in no bar, and a
    /// screen that quietly under-reports a streak is worse than one that admits
    /// what it cannot see.
    #[test]
    fn a_completion_with_no_stamp_is_named_on_the_screen() {
        let mut tasks = a_week_of_work();
        assert!(
            !stats_of(80, 22, &tasks, Period::Week)
                .iter()
                .any(|r| r.contains("before ratodo stamped")),
            "nothing to admit, so nothing is said"
        );

        let mut old = capture("finished long ago", today());
        old.set_state(State::Done, today());
        old.done_on = None;
        tasks.push(old);
        assert!(
            stats_of(80, 24, &tasks, Period::Week)
                .iter()
                .any(|r| r.contains("1 finished before ratodo stamped the day")),
            "{:?}",
            stats_of(80, 24, &tasks, Period::Week)
        );
    }

    /// Bars are a length the eye compares against the length beside it, so the
    /// only one with a trough drawn behind it is the one that is a fraction.
    #[test]
    fn a_bar_is_a_length_and_only_the_gauge_has_a_trough() {
        assert_eq!(bar_of(4, 4, 8, Glyphs::Unicode), "████████");
        assert_eq!(bar_of(2, 4, 8, Glyphs::Unicode), "████");
        // Never rounded away: one is not none.
        assert_eq!(bar_of(1, 100, 8, Glyphs::Unicode), "█");
        // And none is a mark on the row rather than a hole in it.
        assert_eq!(bar_of(0, 4, 8, Glyphs::Unicode), "░");
        assert_eq!(bar_of(0, 0, 8, Glyphs::Unicode), "");

        assert_eq!(gauge(1, 4, 8, Glyphs::Unicode), "██░░░░░░");
        assert_eq!(gauge(0, 4, 8, Glyphs::Unicode), "░░░░░░░░");
        assert_eq!(gauge(4, 4, 8, Glyphs::Unicode), "████████");
        assert_eq!(gauge(1, 0, 8, Glyphs::Unicode), "");

        assert_eq!(bar_of(2, 4, 8, Glyphs::Ascii), "####");
        assert_eq!(gauge(2, 4, 8, Glyphs::Ascii), "####....");
    }

    /// The second screen is furniture too, and furniture is where the fallback
    /// has escaped before.
    #[test]
    fn the_stats_screen_is_ascii_under_a_c_locale() {
        let tasks = a_week_of_work();
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let stats = crate::agenda::stats(&tasks, today(), Period::Week);
        let render = Render {
            glyphs: Glyphs::Ascii,
            ..render(crate::theme::MOCHA)
        };

        let mut terminal = Terminal::new(TestBackend::new(80, 22)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::of(&tasks, today()),
                    render,
                    &Notice::Hints,
                    View::Stats(&stats, Period::Week),
                    Open::Nothing,
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

        assert!(text.contains("DONE THIS WEEK"), "{text}");
        assert!(
            text.is_ascii(),
            "something non-ASCII reached the screen: {text}"
        );
    }

    fn form(text: &str, lists: &[&str]) -> Form {
        let names: Vec<String> = lists.iter().map(|s| s.to_string()).collect();
        let mut form = Form::adding(today(), &names);
        form.input = Input::new(text.to_string(), Purpose::Add);
        form
    }

    fn tab(form: &mut Form, times: usize) {
        for _ in 0..times {
            form.press(press(KeyCode::Tab));
        }
    }

    /// **The line is the model.** Every row is a view of one string, so a form
    /// that opened on a line already carrying fields shows them without being
    /// told — and this is the whole reason the labelled-field box could be
    /// reversed without bringing back the second parser that killed it:
    /// docs/decisions.md.
    #[test]
    fn every_row_is_a_view_of_the_one_line() {
        let form = form(
            "call the accountant @2026-08-14 09:30 #home #work !high",
            &[],
        );
        let on = |field: Field| -> Vec<String> {
            form.choices_for(field)
                .into_iter()
                .filter(|(_, chosen)| *chosen)
                .map(|(label, _)| label)
                .collect()
        };

        assert_eq!(on(Field::Due), ["2026-08-14"]);
        assert_eq!(on(Field::Priority), ["high"]);
        assert_eq!(tags_of(&form.input.text, today()), "#home #work");
        assert_eq!(
            part_of(&form.input.text, today(), Part::Time).as_deref(),
            Some("09:30")
        );
    }

    /// A radio writes into the line and nothing else. What is around the token
    /// it replaced is the user's and stays where they put it.
    #[test]
    fn choosing_a_radio_replaces_one_word_and_leaves_the_rest_alone() {
        let mut form = form("#ops rotate the keys !high @2026-08-14", &[]);
        tab(&mut form, 3); // Title -> Due -> Time -> Priority
        assert_eq!(form.focus, Field::Priority);

        // none, high, med, low — so one step right of `high` is `med`.
        form.press(press(KeyCode::Right));
        assert_eq!(form.input.text, "#ops rotate the keys !med @2026-08-14");
        form.press(press(KeyCode::Right));
        assert_eq!(form.input.text, "#ops rotate the keys !low @2026-08-14");
        // And round to `none`, which takes the word out with one adjacent space.
        form.press(press(KeyCode::Right));
        assert_eq!(
            form.input.text, "#ops rotate the keys @2026-08-14",
            "clearing takes one adjacent space with it"
        );
    }

    /// The four cases a span rewrite has to get right, and the space either
    /// side of a removed word is the one that is easy to get wrong twice.
    #[test]
    fn a_span_is_replaced_added_or_removed_and_never_smeared() {
        let day = today();
        let line = "rotate #ops the keys !high @2026-08-14";

        // Changed in place: position, order and the whitespace are the user's.
        assert_eq!(
            set_parts(line, day, &[Part::Priority], Some("!low")),
            "rotate #ops the keys !low @2026-08-14"
        );
        // Removed, with one space and not two.
        assert_eq!(
            set_parts(line, day, &[Part::Priority], None),
            "rotate #ops the keys @2026-08-14"
        );
        // Removed from the end: the space in front of it goes instead.
        assert_eq!(
            set_parts(line, day, &[Part::Date], None),
            "rotate #ops the keys !high"
        );
        // Added where there was none — the end, which is the one position this
        // tool ever chooses.
        assert_eq!(
            set_parts("rotate the keys", day, &[Part::Priority], Some("!med")),
            "rotate the keys !med"
        );
        // And nothing at all is still nothing at all.
        assert_eq!(set_parts("", day, &[Part::Date], None), "");
        assert_eq!(
            set_parts("  ", day, &[Part::Priority], Some("!low")),
            "!low"
        );
    }

    /// Tags are a set, so they are cleared and written back together — and a
    /// word typed without its `#` gets one, because the field is a place to
    /// name tags rather than a place to remember punctuation.
    #[test]
    fn the_tag_field_is_a_set_and_puts_the_hash_back() {
        let day = today();
        assert_eq!(
            set_tags("buy milk #home #work @2026-08-14", day, "kitchen"),
            "buy milk @2026-08-14 #kitchen"
        );
        assert_eq!(set_tags("buy milk #home", day, "#a #b"), "buy milk #a #b");
        assert_eq!(set_tags("buy milk #home", day, ""), "buy milk");
    }

    /// Half a time is not a time — `capture` has never heard of `09:` — so the
    /// sync rebuilds from the line as it was when the field was focused rather
    /// than looking for a token it wrote a keystroke ago. Five keystrokes used
    /// to leave five words in the line.
    #[test]
    fn typing_a_time_writes_one_word_and_not_one_per_keystroke() {
        let mut form = form("standup @2026-08-12", &[]);
        tab(&mut form, 2);
        assert_eq!(form.focus, Field::Time);

        for c in "09:30".chars() {
            form.press(press(KeyCode::Char(c)));
        }
        assert_eq!(form.input.text, "standup @2026-08-12 09:30");

        // And back out again, one character at a time.
        for _ in 0..5 {
            form.press(press(KeyCode::Backspace));
        }
        assert_eq!(form.input.text, "standup @2026-08-12");
    }

    /// The format cannot hold a time without a date, so the row is not in the
    /// tab order without one: a field the file cannot keep is worse than a
    /// field that is not there.
    #[test]
    fn the_time_row_is_not_offered_without_a_date() {
        let mut undated = form("buy milk", &[]);
        tab(&mut undated, 2);
        assert_eq!(undated.focus, Field::Priority);

        let mut dated = form("buy milk @2026-08-12", &[]);
        tab(&mut dated, 2);
        assert_eq!(dated.focus, Field::Time);
    }

    /// `List` appears only when there is more than one list to address, exactly
    /// as `$list` does — docs/tui.md#which-list--work.
    #[test]
    fn the_list_row_appears_only_when_there_is_a_choice() {
        let one = form("buy milk", &["todo.md"]);
        assert!(!one.order().contains(&Field::List));

        let two = form("buy milk", &["todo.md", "work.md"]);
        assert!(two.order().contains(&Field::List));

        // Title, Due, Priority, Tags, List — no `Time` row, because the line
        // has no date for one to hang off.
        let mut two = two;
        tab(&mut two, 4);
        assert_eq!(two.focus, Field::List);
        two.press(press(KeyCode::Right));
        assert_eq!(two.input.text, "buy milk $work");
    }

    /// Typing still works. `@thu`, `#home` and `!high` in the question field
    /// parse as they always did and light the matching radio as they are typed
    /// — one tokenizer, one truth, and the day there are two the form and the
    /// box disagree about what gets written.
    #[test]
    fn typing_the_syntax_lights_the_radio() {
        let mut form = form("", &[]);
        for c in "pay the invoice !high".chars() {
            form.press(press(KeyCode::Char(c)));
        }
        assert_eq!(form.focus, Field::Title, "the keys went to the text");
        assert_eq!(
            form.choices_for(Field::Priority)
                .into_iter()
                .find(|(_, on)| *on)
                .map(|(label, _)| label),
            Some("high".to_string())
        );
    }

    /// `esc` cancels from anywhere, `⏎` creates from anywhere except the button
    /// that says otherwise, and the buttons carry their own key.
    #[test]
    fn the_two_keys_that_end_the_form() {
        let mut form = form("buy milk", &[]);
        assert_eq!(form.press(press(KeyCode::Enter)), Typed::Save);
        assert_eq!(form.press(press(KeyCode::Esc)), Typed::Cancel);

        // On the cancel button `⏎` means what the button says.
        while form.focus != Field::Cancel {
            tab(&mut form, 1);
        }
        assert_eq!(form.press(press(KeyCode::Enter)), Typed::Cancel);
        tab(&mut form, 1);
        assert_eq!(form.focus, Field::Create);
        assert_eq!(form.press(press(KeyCode::Enter)), Typed::Save);
    }

    /// `tab` is *next field* in here and the date picker is reached through
    /// `Due · pick…` instead: one key, one job per screen. And it wraps, so
    /// there is no end of the form to fall off.
    #[test]
    fn tab_walks_the_fields_and_comes_back_round() {
        let mut form = form("buy milk @2026-08-12", &[]);
        let order = form.order();
        for want in order.iter().skip(1).chain(order.iter().take(1)) {
            tab(&mut form, 1);
            assert_eq!(form.focus, *want);
        }
    }

    /// A form that half-fits is worse than a box that always fits, and the box
    /// is already built and already tested — docs/decisions.md.
    #[test]
    fn the_form_gives_way_to_the_one_line_box_in_a_small_pane() {
        assert!(Form::fits(Rect::new(0, 0, 40, 15)));
        assert!(!Form::fits(Rect::new(0, 0, 39, 15)));
        assert!(!Form::fits(Rect::new(0, 0, 40, 14)));
    }

    /// The whole screen, exactly, and the preview is its conclusion: a form
    /// that saves into your file puts the file last.
    #[test]
    fn the_form_screen_exactly() {
        let tasks = tasks(&["late @2026-08-01"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let mut form = Form::adding(today(), &[]);
        form.input = Input::new("call the accountant @2026-08-12 #home".into(), Purpose::Add);

        let mut terminal = Terminal::new(TestBackend::new(56, 20)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::of(&tasks, today()),
                    render(crate::theme::MOCHA),
                    &Notice::Hints,
                    View::List,
                    Open::Form(&form),
                )
            })
            .unwrap();
        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content()
            .chunks(56)
            .map(|r| r.iter().map(|c| c.symbol()).collect())
            .collect();

        assert_eq!(
            rows,
            [
                "╭ ratodo — 1 · 1! ─────────────────────────────────────╮",
                "│ ╭──────────────────── NEW TASK ────────────────────╮ │",
                "│▌│  What needs to be done?                          │ │",
                "│ │  ╭────────────────────────────────────────────╮  │ │",
                "│ │▌ │ call the accountant @2026-08-12 #home      │  │ │",
                "│ │  ╰────────────────────────────────────────────╯  │ │",
                "│ │                                                  │ │",
                "│ │  Due       ○ none  ○ today  ○ tomorrow  ◉ 2026…  │ │",
                "│ │  Time      [                                  ]  │ │",
                "│ │  Priority  ◉ none  ○ high  ○ med  ○ low          │ │",
                "│ │  Tags      [ #home                            ]  │ │",
                "│ │                                                  │ │",
                "│ │  ──────────────────────────────────────────────  │ │",
                "│ │  PREVIEW                                         │ │",
                "│ │  - [ ] call the accountant @2026-08-12 #home     │ │",
                "│ │                                                  │ │",
                "│ │  [ esc cancel ]               [ ⏎ create task ]  │ │",
                "│ ╰ tab · next field ────────────────────────────────╯ │",
                "╰──────────────────────────────────────────────────────╯",
                " ⏎ create   esc cancel                                  ",
            ]
        );
    }

    /// A form full of new furniture, under a C locale. The radios are the one
    /// that matters: `◉` against `○` is a difference in **shape**, so the
    /// choice survives both the fallback and `NO_COLOR`.
    #[test]
    fn the_form_is_ascii_under_a_c_locale() {
        assert_eq!(
            radios(&[("a".into(), true), ("b".into(), false)], Glyphs::Ascii),
            "(o) a  ( ) b"
        );
        assert_eq!(
            radios(&[("a".into(), true), ("b".into(), false)], Glyphs::Unicode),
            "◉ a  ○ b"
        );

        let tasks = tasks(&["late @2026-08-01"]);
        let groups = agenda(&tasks, today());
        let mut screen = Screen::new(rows(&groups));
        let mut form = Form::adding(today(), &[]);
        form.input = Input::new("call the accountant #home".into(), Purpose::Add);
        let render = Render {
            glyphs: Glyphs::Ascii,
            ..render(crate::theme::MOCHA)
        };

        let mut terminal = Terminal::new(TestBackend::new(64, 20)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::of(&tasks, today()),
                    render,
                    &Notice::Hints,
                    View::List,
                    Open::Form(&form),
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

        assert!(text.contains("NEW TASK"), "{text}");
        assert!(text.contains("PREVIEW"), "{text}");
        assert!(
            text.is_ascii(),
            "something non-ASCII reached the screen: {text}"
        );
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
                "╭ ratodo — 2 open · 1 overdue ───────────────────────────────╮",
                "│  ╭─ OVERDUE · 1 ────────────────────────────────────────╮  │",
                "│▌ │ ! late                                   1d ago  #ops│  │",
                "│  ╰──────────────────────────────────────────────────────╯  │",
                "│  ╭─ ## Work · 1 ────────────────────────────────────────╮  │",
                "│  │ ○ write the plan                                     │  │",
                "│  ╰──────────────────────────────────────────────────────╯  │",
                "│                                                            │",
                "╰────────────────────────────────────────────────────────────╯",
                " [j k] move [spc] done [a] add [⏎] edit [?] keys [q] quit     ",
            ]
        );
    }

    /// The progress bar, drawn exactly: green for what is finished, the rule
    /// between the two titles, and the count flush to the corner.
    #[test]
    fn the_title_bar_shows_what_is_finished() {
        let mut done = capture("migrate the server", today());
        done.set_state(State::Done, today());
        let tasks = [capture("late @2026-08-09 #ops", today()), done];

        assert_eq!(
            rendered(62, 5, &tasks),
            [
                "╭ ratodo — 1 open · 1 overdue ───────────────── ▰▰▰▰▱▱▱▱ 1/2 ╮",
                "│  ╭─ OVERDUE · 1 ────────────────────────────────────────╮  │",
                "│▌ │ ! late                                   1d ago  #ops│  │",
                "╰────────────────────────────────────────────────────────────╯",
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
        done.set_state(State::Done, today());
        let tasks = [capture("late @2026-08-09", today()), done];

        let narrow = rendered(46, 5, &tasks);
        assert!(
            narrow[0].starts_with("╭ ratodo — 1 · 1! · 1✓"),
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
        done.set_state(State::Done, today());
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
                    View::List,
                    Open::Nothing,
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
        let input = Input::new("buy milk @thu #home".to_string(), Purpose::Add);

        let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    Counts::of(&tasks, today()),
                    render,
                    &Notice::Hints,
                    View::Help,
                    Open::Box(&input),
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
                "╭ ratodo — 1 · 1! ───────────────────────────────╮",
                "│  ╭─ OVERDUE · 1 ────────────────────────────╮  │",
                "│▌ │ ! an extremely long task title t…  1d ago│  │",
                "╰────────────────────────────────────────────────╯",
                " ?                                                ",
            ]
        );
    }

    /// The same list one breakpoint down: short counts and no tags. The box
    /// survives here — it costs the row four columns and it costs the pane one
    /// row per group, which is the one place it is not free: the blank spacer
    /// this width dropped is a row it never had to give back.
    #[test]
    fn the_narrow_screen_exactly() {
        let mut work = capture("write the plan", today());
        work.section = Some("Work".into());
        let tasks = [capture("late @2026-08-09 #ops", today()), work];

        assert_eq!(
            rendered(46, 9, &tasks),
            [
                "╭ ratodo — 2 · 1! ───────────────────────────╮",
                "│  ╭─ OVERDUE · 1 ────────────────────────╮  │",
                "│▌ │ ! late                         1d ago│  │",
                "│  ╰──────────────────────────────────────╯  │",
                "│  ╭─ ## Work · 1 ────────────────────────╮  │",
                "│  │ ○ write the plan                     │  │",
                "│  ╰──────────────────────────────────────╯  │",
                "╰────────────────────────────────────────────╯",
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
        let screen = rendered(62, 12, &tasks);

        assert!(
            screen[0].contains("ratodo — 3 open · 1 overdue"),
            "{screen:?}"
        );
        assert!(
            screen[1].starts_with("│  ╭─ OVERDUE · 1 ────"),
            "{screen:?}"
        );
        assert!(screen[2].contains("▌ │ ! late"), "{screen:?}");
        assert!(screen[2].contains("9d ago"), "{screen:?}");
        assert!(screen[4].contains("TODAY"), "{screen:?}");
        assert!(screen[5].contains("○ now"), "{screen:?}");
        assert!(screen[5].contains("16:00"), "{screen:?}");
        assert!(!screen[5].contains('▌'), "two rows drawn as selected");
        assert!(screen[8].contains("○ soon"), "{screen:?}");
        assert!(
            screen[8].contains("!low") && screen[8].contains("#ops"),
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

        // Nine columns sit between the terminal and the row that COLUMNS_AT
        // measures: two of frame, two of selection marker, and the five the
        // group box takes off every row it draws.
        let packed = COLUMNS_AT as u16 + 8;
        assert!(
            at(packed) > COLUMNS_AT - 10,
            "columns one column too early: {:?}",
            rendered(packed, 8, &tasks)
        );
        assert_eq!(
            at(packed + 1),
            1 + 2 + 1 + INSET + 2 + columns("a much longer title here") + RULED,
            "columns one column too late: {:?}",
            rendered(packed + 1, 8, &tasks)
        );
    }

    /// **The column says what the heading does not.** `today` inside a group
    /// headed `TODAY` spent nine characters saying where it already was; the
    /// other three groups each say something their heading cannot —
    /// docs/tui.md#main-screen.
    #[test]
    fn the_date_column_never_repeats_the_heading_above_it() {
        let column = |spec: &str| when(&capture(spec, today()), today(), Size::Wide);

        // OVERDUE says it is late; the column says how late.
        assert_eq!(column("a @2026-08-08"), "2d ago");
        // TODAY says the day; the column says the time, or nothing at all.
        assert_eq!(column("a @2026-08-10"), "");
        assert_eq!(column("a @2026-08-10 16:00"), "16:00");
        // THIS WEEK says the week; the column says which day of it.
        assert_eq!(column("a @2026-08-14"), "Fri");
        assert_eq!(column("a @2026-08-14 09:30"), "Fri 09:30");
        // A `##` section says nothing about dates, so the column says the date.
        assert_eq!(column("a @2026-09-20"), "Sep 20");

        // And a group where every task is due today unstyled spends no width on
        // the column at all, rather than a column of blanks.
        let rows = [
            Row::Task(capture("a @2026-08-10", today())),
            Row::Task(capture("b @2026-08-10", today())),
        ];
        assert_eq!(
            Columns::of(&rows, 86, render(crate::theme::MOCHA), Size::Wide).date,
            0
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
        let screen = rendered(90, 14, &tasks);

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

    /// The rules run straight down the pane. A row with no date and no priority
    /// draws them in the same places as a row with both — an empty cell keeps
    /// its column, which is the whole difference between a table and three
    /// fields that happen to be near each other.
    #[test]
    fn the_column_rules_line_up_down_the_pane() {
        let tasks = tasks(&[
            "late one @2026-08-01 !high #ops",
            "no priority @2026-08-14 #home",
            "nothing at all",
            "priority only !low",
        ]);
        let screen = rendered(90, 12, &tasks);
        let bars = |row: &str| -> Vec<usize> {
            let last = row.chars().count() - 1;
            row.char_indices()
                .filter(|(_, c)| *c == '│')
                .map(|(i, _)| row[..i].chars().count())
                // The frame's own two sides are not column rules.
                .filter(|at| *at > 0 && *at < last)
                .collect()
        };

        let rows: Vec<Vec<usize>> = screen
            .iter()
            .filter(|r| {
                ["late one", "no priority", "nothing at all", "priority only"]
                    .iter()
                    .any(|title| r.contains(title))
            })
            .map(|r| bars(r))
            .collect();
        assert_eq!(rows.len(), 4, "not every task is on screen: {screen:?}");
        for row in &rows {
            // Three column rules and the group box's own two sides. The sides
            // count here on purpose: they are the same stroke, and the whole
            // point of the box is that the column rules now end on it.
            assert_eq!(row.len(), 5, "a row is missing a rule: {screen:?}");
            assert_eq!(row, &rows[0], "the rules do not line up: {screen:?}");
        }

        // And below the breakpoint there is nothing to line up, so there are no
        // column rules to draw: three characters of noise per row is what a
        // table costs when it has no columns. The box keeps its two sides.
        let narrow = rendered(70, 12, &tasks);
        for row in narrow.iter().filter(|r| r.contains("late one")) {
            assert_eq!(bars(row).len(), 2, "{narrow:?}");
        }
    }

    /// The whole claim of the group box, in one assertion: nothing floats. The
    /// `┬` on the top edge, the `│` in the row and the `┴` on the bottom edge
    /// are one stroke, so they are one column — docs/redesign.md.
    ///
    /// This is the arithmetic that looks right in `Columns::dividers` and lands
    /// a column out on the screen, which is exactly the class of bug the group
    /// rule stopping in mid-air was.
    #[test]
    fn every_junction_lands_on_the_rule_it_is_the_end_of() {
        let tasks = tasks(&[
            "late one @2026-08-01 !high #ops",
            "no priority @2026-08-14 #home",
        ]);
        let screen = rendered(90, 10, &tasks);
        let at = |row: &str, wanted: char| -> Vec<usize> {
            row.chars()
                .enumerate()
                .filter(|(_, c)| *c == wanted)
                .map(|(i, _)| i)
                .collect()
        };

        let top = at(&screen[1], '┬');
        let bottom = at(&screen[3], '┴');
        // Four of the row's verticals are furniture — the frame either side and
        // the box either side — and what is between them is the columns.
        let sides = at(&screen[2], '│');
        let inner = &sides[2..sides.len() - 2];

        assert_eq!(top.len(), 3, "{screen:?}");
        assert_eq!(top, bottom, "the box does not close on its own junctions");
        assert_eq!(top, inner, "a junction is not on its rule: {screen:?}");
    }

    /// The four widths the redesign was drawn at, swept: every row exactly the
    /// width it was given, every frame closing, and nothing non-ASCII on the
    /// screen under a C locale. The box-drawing set is new furniture and the
    /// fallback has escaped through new furniture twice before — todo.md.
    #[test]
    fn every_row_closes_at_every_width_the_redesign_was_drawn_at() {
        let tasks = a_week_of_work();
        for width in [80u16, 60, 44, 34] {
            for height in [24u16, 18, 14, 10] {
                let screen = rendered(width, height, &tasks);
                assert_eq!(screen.len(), height as usize, "{width}x{height}");
                for row in &screen {
                    assert_eq!(
                        columns(row),
                        width as usize,
                        "{width}x{height} row is not the width it was given: {row:?}"
                    );
                }

                // The frame either closes on both sides of every row it draws,
                // or it is not drawn at all — never half of one.
                let framed = screen[0].starts_with('╭');
                if framed {
                    assert!(screen[0].ends_with('╮'), "{width}x{height}: {screen:?}");
                    let foot = screen.len() - 2;
                    assert!(
                        screen[foot].starts_with('╰') && screen[foot].ends_with('╯'),
                        "{width}x{height}: {screen:?}"
                    );
                    for row in &screen[1..foot] {
                        assert!(
                            row.starts_with('│') || row.starts_with('├'),
                            "{width}x{height}: {row:?}"
                        );
                    }
                }
                assert_eq!(framed, width >= 34, "{width}x{height}");
            }
        }
    }

    /// Below 34 columns the frame goes, and the box goes with it: two columns of
    /// border out of thirty-three is a tenth of the pane spent on furniture, and
    /// this width has always been bare rows — docs/tui.md#width.
    #[test]
    fn the_box_goes_with_the_frame_below_thirty_four_columns() {
        let tasks = tasks(&["late one @2026-08-01", "no priority @2026-08-14"]);

        let bare = rendered(33, 8, &tasks);
        for row in &bare {
            for glyph in ['╭', '╮', '╰', '╯', '│', '┬', '┴'] {
                assert!(!row.contains(glyph), "{glyph} survived at 33: {bare:?}");
            }
        }
        // And one column wider it is all back.
        let boxed = rendered(34, 8, &tasks);
        assert!(boxed[1].contains('╭'), "{boxed:?}");
        assert!(boxed[3].contains('╰'), "{boxed:?}");
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
        assert_eq!(cols.date, 6 + RULED, "the date column");
        assert_eq!(cols.prio, 5 + RULED, "the priority column");
        // 86 less the mark and its space, less both of those. The title asked
        // for 80 and the row has this much to give it.
        assert_eq!(
            cols.title,
            86 - 2 - (6 + RULED) - (5 + RULED),
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
            5 + RULED,
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

        // Where the budget runs out, to the column. The pane leaves 92 columns
        // of row and the group box takes five of them — a side either end, the
        // inset after the left one, and the two it holds back off the frame.
        // The mark, the 56-column title, the date column and the priority
        // column spend most of what is left, and `#alpha` is what the rest buys.
        let title = columns("a title long enough to leave the last tags nowhere to go");
        let budget = 92 - 5 - (2 + title + (6 + RULED) + (5 + RULED));
        // The rule in front of the first tag is part of what the budget buys,
        // and these two spend every column of it.
        assert_eq!(budget, 12, "the tag budget moved");
        // Nine of the twelve go on the rule and `#alpha`. `  #bravo` wants
        // eight more, so it goes whole rather than half — three columns are
        // left unspent and that is the point of the rule.
        assert_eq!(columns(" │ #alpha"), 9);
        assert!(budget < columns(" │ #alpha  #bravo"));
        assert!(screen[2].contains("#alpha"), "{screen:?}");
        for missing in ["#bravo", "#charlie", "#delta"] {
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
            if done {
                task.set_state(State::Done, today());
            }
            let rows = [Row::Task(task.clone())];
            let cols = Columns::of(&rows, 86, render(colours), Size::Wide);
            let line = task_line(&task, 86, cols, render(colours), Size::Wide);
            // The date is the first styled entry after the mark and the title,
            // and past the rule that opens its column — which has a colour of
            // its own and is not the thing being asked about here.
            line.spans[3..]
                .iter()
                .find(|s| {
                    let text = s.content.trim();
                    !text.is_empty() && text != render(colours).glyphs.divider()
                })
                .expect("the date is on the row")
                .style
        };

        assert_eq!(style_of("a @2026-08-08", false).fg, Some(colours.overdue));
        // Due today and untimed, the column is **empty** — the heading above it
        // already says `TODAY`. So the only thing left in it to colour is a
        // time, and that is the one most worth seeing.
        assert_eq!(
            when(&capture("a @2026-08-10", today()), today(), Size::Wide),
            "",
            "the column repeated the heading"
        );
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

    /// The priority has a colour of its own, in two weights: `!high` bold,
    /// `!med` plain, `!low` down with the dim fields. Its own and not the
    /// accent, which is already the headings, the box border and the help keys
    /// — docs/design.md#what-each-colour-means.
    #[test]
    fn the_priority_is_its_own_colour_in_two_weights() {
        let colours = crate::theme::MOCHA;
        let style_of = |spec: &str, done: bool| {
            let mut task = capture(spec, today());
            if done {
                task.set_state(State::Done, today());
            }
            let rows = [Row::Task(task.clone())];
            let cols = Columns::of(&rows, 86, render(colours), Size::Wide);
            let line = task_line(&task, 86, cols, render(colours), Size::Wide);
            // By the word and not by the leading `!`: on a late row the mark is
            // an `!` too, and it is the first one in the line.
            let want = task.priority.expect("the spec has a priority").as_str();
            line.spans
                .iter()
                .find(|s| s.content == want)
                .expect("the priority is on the row")
                .style
        };

        let high = style_of("a @2026-08-14 !high", false);
        assert_eq!(high.fg, Some(colours.priority));
        assert!(
            high.add_modifier.contains(ratatui::style::Modifier::BOLD),
            "the loud field is not loud: {high:?}"
        );

        // The middle one is the same colour and not the same weight, which is
        // the whole of what separates them.
        let med = style_of("a @2026-08-14 !med", false);
        assert_eq!(med.fg, Some(colours.priority));
        assert!(!med.add_modifier.contains(ratatui::style::Modifier::BOLD));

        let low = style_of("a @2026-08-14 !low", false);
        assert_eq!(low.fg, Some(colours.dim));
        assert!(!low.add_modifier.contains(ratatui::style::Modifier::BOLD));

        // It borrows the row's colour from nobody, and it is not the accent the
        // heading above it wears either. On a late row the date is `overdue` red
        // and the priority is still its own — that is the one row where the two
        // most need telling apart.
        let late = style_of("a @2026-08-08 !high", false);
        assert_eq!(late.fg, Some(colours.priority));
        assert_ne!(colours.priority, colours.accent);

        // A ticked row keeps it, and keeps its weight. The priority is a fact
        // about the task, not a claim about what is left to do — the `✓` is what
        // answers that, and a finished `!med` going grey beside an open `!high`
        // read as the colour having failed rather than as the task being done.
        let ticked = style_of("a @2026-08-14 !high", true);
        assert_eq!(ticked.fg, Some(colours.priority));
        assert!(ticked.add_modifier.contains(ratatui::style::Modifier::BOLD));
        assert_eq!(
            style_of("a @2026-08-14 !med", true).fg,
            Some(colours.priority)
        );
        assert_eq!(style_of("a @2026-08-14 !low", true).fg, Some(colours.dim));
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
        // Ninety columns of terminal is eighty-one of row: two of frame, two of
        // cursor gutter, and five the group box takes.
        let wide = Columns::of(&rows, 81, render(crate::theme::MOCHA), Size::Wide);

        // Two rows of list is not enough to show the second task at all.
        let cramped = rendered(90, 5, &tasks);
        assert!(cramped[2].contains("short"), "{cramped:?}");
        // The frame, the cursor, the box side and its inset, the mark, then the
        // title column and its gap.
        assert_eq!(
            at_column(&cramped[2], "9d ago"),
            1 + 2 + 1 + INSET + 2 + wide.title + RULED,
            "the visible row was measured on its own: {cramped:?}"
        );
    }

    /// `Glyphs::mark_width` is arithmetic done before there is a task to ask,
    /// so it has to agree with every mark `Glyphs::mark` can actually return.
    #[test]
    fn the_mark_width_matches_every_mark_in_its_set() {
        let mut done = capture("done", today());
        done.set_state(State::Done, today());
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
            columns(&header_line("Work", 2, false, 86, cols, render).to_string())
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
        assert!(!bare[0].contains('╭'), "the frame survived: {bare:?}");
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
        assert!(screen[0].ends_with('╮') && screen[frame].ends_with('╯'));
        // The box's own right side, the margin it holds back, and then the
        // frame — three edges that all have to land, not one.
        assert!(screen[2].ends_with("9d ago│  │"), "{screen:?}");
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
            .draw(|f| {
                draw(
                    f,
                    &mut screen,
                    counts,
                    render,
                    &Notice::Hints,
                    View::List,
                    Open::Nothing,
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

        assert!(text.contains("> | [!] late"), "{text}");
        assert!(text.contains("| [ ] fine"), "{text}");
        assert!(text.contains("+- OVERDUE / 1 --"), "{text}");
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
    /// already agree: a completed task is never in `overdue`. What the column
    /// says instead is the day it was finished, which is the one date about a
    /// done task that is still worth the width.
    #[test]
    fn a_finished_task_shows_when_it_was_done_not_how_late_it_was() {
        let mut done = capture("a @2026-08-08", today());
        done.set_state(State::Done, today());
        assert_eq!(when(&done, today(), Size::Wide), "today");
        assert_eq!(when(&done, today(), Size::Narrow), "today");

        // Not just today's: the stamp is read back off the line like any field.
        let older = crate::parse::parse("- [x] a @2026-08-08 ✓2026-07-30\n");
        let older = older.tasks().next().expect("a task");
        assert_eq!(when(older, today(), Size::Wide), "Jul 30");

        // A task ticked before the stamp existed still has a date to show.
        let unstamped = crate::parse::parse("- [x] a @2026-08-08\n");
        let unstamped = unstamped.tasks().next().expect("a task");
        assert_eq!(when(unstamped, today(), Size::Wide), "Aug 8");

        // The stamp only displaces the due date on a task that is *done*.
        let cancelled = crate::parse::parse("- [-] a @2026-08-08\n");
        let cancelled = cancelled.tasks().next().expect("a task");
        assert_eq!(when(cancelled, today(), Size::Wide), "Aug 8");
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
                    View::List,
                    Open::Nothing,
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
                    View::List,
                    Open::Nothing,
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
        done.set_state(State::Done, today());
        let tasks = [
            capture("late @2026-08-01", today()),
            capture("now @2026-08-10", today()),
            done,
        ];

        for (_, colours) in crate::theme::BUILT_IN {
            assert_eq!(colour_of(40, 12, &tasks, "late", colours), colours.overdue);
            assert_eq!(colour_of(40, 12, &tasks, "now", colours), colours.today);
            // Green, and only now: it was the one action on this screen that
            // said nothing back — docs/design.md#rules reserved green for
            // exactly this and had spent it on the progress bar alone.
            assert_eq!(colour_of(40, 12, &tasks, "finished", colours), colours.done);
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
                    View::List,
                    Open::Nothing,
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
        done.set_state(State::Done, today());
        screen.update_selected(done);

        assert_eq!(screen.selected(), before, "the cursor moved");
        assert!(screen.task().unwrap().done());
        assert_eq!(
            screen.task().unwrap().title,
            "first",
            "a different row was rewritten"
        );

        let text = rendered_with(62, 8, &tasks, |s| {
            let mut d = s.task().unwrap().clone();
            d.set_state(State::Done, today());
            s.update_selected(d);
        });
        assert!(text[2].contains("✓ first"), "{text:?}");
        assert!(text[3].contains("! second"), "{text:?}");
    }

    /// The bottom line does four jobs and never changes the list's shape.
    #[test]
    fn the_bottom_line() {
        let colours = crate::theme::MOCHA;
        let shown = |notice: &Notice, size, width, height, glyphs| {
            notice
                .line(size, width, height, glyphs, colours)
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect::<String>()
        };

        assert!(
            shown(&Notice::Hints, Size::Wide, 60, 20, Glyphs::Unicode).contains("[spc] done"),
            "the hints have to name the keys"
        );
        assert!(shown(&Notice::Hints, Size::Wide, 60, 20, Glyphs::Unicode).contains("[a] add"));
        // Sixty columns is the narrowest pane that still counts as wide, and the
        // bar has to fit it — in both alphabets, `ret` being the longer of the
        // two. A hint bar that gets clipped is advertising half a key.
        for glyphs in [Glyphs::Unicode, Glyphs::Ascii] {
            let bar = shown(&Notice::Hints, Size::Wide, 60, 20, glyphs);
            assert!(columns(&bar) <= 60, "{} columns: {bar}", columns(&bar));
        }
        assert_eq!(
            shown(&Notice::Hints, Size::Wide, 60, 9, Glyphs::Unicode),
            " ?",
            "under ten rows the hint bar collapses"
        );
        assert!(!shown(&Notice::Hints, Size::Narrow, 40, 20, Glyphs::Unicode).contains("move"));

        // The bar fills what it is given, and never spills: it is one line, and
        // a hint that wrapped would take a row off the list.
        for width in 60..=200usize {
            for glyphs in [Glyphs::Unicode, Glyphs::Ascii] {
                let bar = shown(&Notice::Hints, Size::Wide, width, 20, glyphs);
                assert!(columns(&bar) <= width, "{width}: {bar}");
                // Whatever else goes, these two stay — the way to the rest of
                // the keymap, and the way out.
                assert!(bar.contains("[?] keys"), "{width}: {bar}");
                assert!(bar.contains("[q] quit"), "{width}: {bar}");
            }
        }

        // What eighty columns buys — the width a terminal opens at unless
        // somebody moved it. A user who never opens `?` finds a key here or not
        // at all, so what is on the bar at eighty is pinned rather than left to
        // a rendering detail.
        //
        // **`[y] copy` is what the keycaps cost.** Brackets are two columns an
        // entry and the bar is a greedy fill, so the last one no longer fits at
        // eighty; it is back at eighty-eight. That was measured rather than
        // assumed, and it is why the separator went from two spaces to one —
        // the brackets already tell one entry from the next, and the space they
        // gave back is `[p] later` still being here.
        let wide = shown(&Notice::Hints, Size::Wide, 80, 20, Glyphs::Unicode);
        for named in ["[spc] done", "[d] cancel", "[p] later"] {
            assert!(wide.contains(named), "{named} is not on the bar: {wide}");
        }
        assert!(!wide.contains("[y] copy"), "{wide}");
        assert!(
            shown(&Notice::Hints, Size::Wide, 88, 20, Glyphs::Unicode).contains("[y] copy"),
            "the copy key never comes back"
        );

        // And they go in that order as the pane narrows, rather than the bar
        // being clipped mid-word.
        let sixty = shown(&Notice::Hints, Size::Wide, 60, 20, Glyphs::Unicode);
        assert!(sixty.contains("[spc] done"), "{sixty}");
        assert!(!sixty.contains("[d] cancel"), "{sixty}");

        assert_eq!(
            shown(
                &Notice::Said("done: milk".into()),
                Size::Wide,
                60,
                20,
                Glyphs::Unicode
            ),
            " done: milk"
        );
        assert_eq!(
            shown(
                &Notice::Warned("nope".into()),
                Size::Wide,
                60,
                20,
                Glyphs::Unicode
            ),
            " ⚠ nope"
        );
        assert_eq!(
            shown(
                &Notice::Warned("nope".into()),
                Size::Wide,
                60,
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
        // The two doors part company here. `a` opens the form wherever there is
        // room for it; `o` is the vim hand reaching to open a new line, which
        // is the fast path, so it keeps being the fast path — the one-line box,
        // at every width. docs/decisions.md.
        assert_eq!(
            action(press(KeyCode::Char('o'))),
            Action::Quick,
            "`o` is the fast path and stays the box"
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
        // But ctrl *and* alt together is AltGr, not a chord — see below.
        let mut held = press(KeyCode::Char('a'));
        held.kind = KeyEventKind::Release;
        assert_eq!(
            typing(held),
            Typed::Ignore,
            "a key being let go is not a press"
        );
    }

    /// Windows reports AltGr as ctrl+alt. On the Turkish, German and Polish
    /// layouts `#`, `@` and `$` are AltGr keys, so reading that as a chord
    /// leaves the three characters the syntax is *made of* untypeable — and
    /// AltGr-c, which is a letter on several layouts, quitting the program.
    #[test]
    fn altgr_is_a_layout_and_not_a_chord() {
        let altgr = KeyModifiers::CONTROL | KeyModifiers::ALT;
        for c in ['#', '@', '$', 'c'] {
            assert_eq!(
                typing(KeyEvent::new(KeyCode::Char(c), altgr)),
                Typed::Char(c),
                "{c}"
            );
        }
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('c'), altgr)),
            Action::Ignore,
            "altgr-c is not ctrl-c"
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
            input.purpose.raw(),
            Some("  * [x] wash up @2026-08-12 #home")
        );
        assert_eq!(Input::adding(today()).purpose, Purpose::Add);
        // At the end, so a retype carries on from where the line stops.
        assert_eq!(input.at, input.text.len());
    }

    /// `a` opens on today, because that is the date a new task has more often
    /// than every other date put together and the box is where it is cheapest to
    /// change. The trailing space is what makes it a prefix rather than an edit:
    /// the title is typed straight on, and the caret is already there.
    #[test]
    fn adding_opens_with_todays_date_in_the_box() {
        let input = Input::adding(today());
        // Behind the caret: the title is typed where the written line has it,
        // and the date the tool guessed sits after it.
        assert_eq!(input.text, " @2026-08-10");
        assert_eq!(input.at, 0);

        // And it is a real date to everything downstream, not decoration: the
        // preview reads it, and `capture` takes it out of the title.
        let (lines, _) = input_lines(&input, 40, render(crate::theme::MOCHA));
        let preview = lines[2]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect::<String>();
        assert!(preview.contains("2026-08-10"), "{preview:?}");
    }

    /// The date the box opened with loses to the date the user types. `capture`
    /// gives the line to the first `@`, so without this the shorthand every
    /// screenshot in the docs types would be the one that lost — and would sit
    /// in the title afterwards, which is the silent half of the bug.
    #[test]
    fn a_typed_date_takes_the_opening_ones_place() {
        let mut input = Input::adding(today());
        for c in "milk @thu #home".chars() {
            input.insert(c);
        }
        assert_eq!(input.text, "milk @thu #home");
        assert_eq!(input.at, input.text.len());
        assert_eq!(
            crate::capture::capture(&input.text, today()).title,
            "milk",
            "the opening date was left in the title"
        );

        // Once. A second `@` is the user's own business — an address in a title
        // is not a date and takes nothing with it.
        let mut twice = Input::adding(today());
        for c in "mail @thu bob@work".chars() {
            twice.insert(c);
        }
        assert_eq!(twice.text, "mail @thu bob@work");

        // And it is the *typed* `@` that does it, not the mere presence of a
        // date: a title typed on its own keeps what the box opened with.
        let mut plain = Input::adding(today());
        for c in "milk".chars() {
            plain.insert(c);
        }
        assert_eq!(plain.text, "milk @2026-08-10");
    }

    /// `y` fills the box the same way `⏎` does and then means something else by
    /// it: what comes back is a new task, so the line it was copied from is not
    /// the thing `⏎` rewrites.
    #[test]
    fn a_copy_starts_from_the_task_and_saves_as_a_new_one() {
        let doc = crate::parse::parse("  * [ ] wash up @2026-08-12 #home !high\n");
        let task = doc.tasks().next().unwrap();

        let input = Input::duplicating(task, today());
        assert_eq!(input.text, "wash up @2026-08-12 #home !high");
        assert_eq!(input.purpose, Purpose::Copy);
        assert_eq!(input.purpose.raw(), None, "a copy rewrites no line");
        assert_eq!(input.at, input.text.len());

        // And the box says so, in the accent: "this is not the line you were
        // looking at" is the one thing `y` has to get across. The other three
        // are full brightness and bold but take no colour — the box's own border
        // is already the accent and is what says *you are in the box*, so a
        // label repeating it an inch inside leaves this one nothing to be told
        // apart by.
        let (lines, _) = input_lines(&input, 60, render(crate::theme::MOCHA));
        let head = &lines[0].spans[0];
        assert_eq!(head.content, " COPY");
        assert_eq!(head.style.fg, Some(crate::theme::MOCHA.accent));

        for purpose in [
            Purpose::Add,
            Purpose::Edit(String::new()),
            Purpose::Postpone(String::new()),
        ] {
            let box_ = Input::new(String::new(), purpose);
            let (lines, _) = input_lines(&box_, 60, render(crate::theme::MOCHA));
            let head = &lines[0].spans[0];
            assert_eq!(
                head.style.fg,
                Some(crate::theme::MOCHA.foreground),
                "{:?}",
                box_.purpose
            );
            assert!(
                head.style
                    .add_modifier
                    .contains(ratatui::style::Modifier::BOLD),
                "{:?}",
                box_.purpose
            );
            // Upper case, the way the group headings on the list are: the tool's
            // own word, said the same way twice.
            assert_eq!(head.content, head.content.to_uppercase());
        }
    }

    /// The completion stamp is the field `capture` has never heard of, so a copy
    /// of a finished task would have carried `✓2026-08-11` into the new one's
    /// *title*. Copying something to do it again is the whole point of `y` on a
    /// ticked row.
    #[test]
    fn a_copy_of_a_finished_task_does_not_carry_the_stamp() {
        let doc = crate::parse::parse("- [x] ship the release @2026-08-08 ✓2026-08-10 #ops\n");
        let task = doc.tasks().next().unwrap();

        let input = Input::duplicating(task, today());
        assert_eq!(input.text, "ship the release @2026-08-08 #ops");

        // And it comes back open, not ticked: the copy is work to do.
        let fresh = crate::capture::capture(&input.text, today());
        assert_eq!(fresh.state, State::Open);
        assert_eq!(fresh.done_on, None);
    }

    /// A cancelled task copies too, and the `[-]` does not come with it — the
    /// state lives in the checkbox, and the copy is a fresh open one.
    #[test]
    fn a_copy_of_a_cancelled_task_comes_back_open() {
        let doc = crate::parse::parse("- [-] learn the flute #someday\n");
        let task = doc.tasks().next().unwrap();

        let input = Input::duplicating(task, today());
        assert_eq!(input.text, "learn the flute #someday");
        assert_eq!(
            crate::capture::capture(&input.text, today()).state,
            State::Open
        );
    }

    /// A field you can only append to is not a field: the fix for a typo four
    /// words back must not be retyping four words — docs/tui.md#adding.
    #[test]
    fn the_caret_moves_through_the_line_and_edits_where_it_stands() {
        let mut input = Input::new("wash şu".to_string(), Purpose::Add);

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
        let input = Input::new(
            "pay @thu 09:30 #home !high @notaday".to_string(),
            Purpose::Add,
        );
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
            Purpose::Add,
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
        let mut input = Input::new(long.to_string(), Purpose::Add);
        let field = |input: &Input| {
            let (lines, at) = input_lines(input, 30, render(crate::theme::MOCHA));
            (lines[0].to_string(), at)
        };

        let (end, at) = field(&input);
        assert!(end.ends_with("at all"), "{end:?}");
        assert_eq!(at, 29, "the caret sits at the end of what is shown");

        input.home();
        let (start, at) = field(&input);
        assert!(start.starts_with(" ADD ▏a very long"), "{start:?}");
        assert!(
            !start.contains("at all"),
            "the caret scrolled off: {start:?}"
        );
        assert_eq!(at, columns(" ADD ▏"));
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
                    View::List,
                    Open::Box(input),
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
        let input = Input::new("call the accountant @thu !high".to_string(), Purpose::Add);

        assert_eq!(
            with_input(70, 11, &tasks, &input, Glyphs::Unicode),
            [
                "╭ ratodo — 4 open · 0 overdue ───────────────────────────────────────╮",
                "│  ╭─ TODAY · 1 ──────────────────────────────────────────────────╮  │",
                "│▌╭────────────────────────────────────────────────────────────────╮ │",
                "│ │ ADD ▏call the accountant @thu !high                            │ │",
                "│ ├────────────────────────────────────────────────────────────────┤ │",
                "│ │      due Thursday (2026-08-13) │ !high                         │ │",
                "│ ╰────────────────────────────────────────────────────────────────╯ │",
                "│  │ ○ d                                                          │  │",
                "│  ╰──────────────────────────────────────────────────────────────╯  │",
                "╰────────────────────────────────────────────────────────────────────╯",
                " ⏎ save   esc cancel   tab date                                       ",
            ]
        );
    }

    /// The preview says *what* the parser understood, so it has to say which
    /// part is which. It used to be one accent-coloured string, which told the
    /// reader the parser had understood all of it equally — the date and the tag
    /// were the same colour in the one row whose job is telling them apart.
    #[test]
    fn the_preview_is_coloured_field_by_field() {
        let render = render(crate::theme::MOCHA);
        let input = Input::new(
            "call the accountant @thu #home !high".to_string(),
            Purpose::Add,
        );
        let (lines, _) = input_lines(&input, 60, render);

        let shown: Vec<(String, Style)> = lines[2]
            .spans
            .iter()
            .map(|s| (s.content.to_string(), s.style))
            .collect();

        let styled = |want: &str| {
            shown
                .iter()
                .find(|(text, _)| text.contains(want))
                .unwrap_or_else(|| panic!("{want} is not in the preview: {shown:?}"))
                .1
        };

        assert_eq!(styled("due Thursday").fg, Some(render.colours.accent));
        assert_eq!(styled("#home").fg, Some(render.colours.tag));
        // The same two weights the row gives it, or the box teaches a colour the
        // list then contradicts.
        assert_eq!(styled("!high").fg, Some(render.colours.priority));
        assert!(
            styled("!high")
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD),
            "the priority lost its weight: {shown:?}"
        );
        // The separators are the quiet part, and the row is not one colour.
        // They are the same rule the columns in the list are drawn with, in the
        // same border colour, so the screen has one separator and not two.
        assert_eq!(styled("│").fg, Some(render.colours.border));
        assert!(
            shown
                .iter()
                .map(|(_, style)| style.fg)
                .collect::<std::collections::HashSet<_>>()
                .len()
                >= 3,
            "the preview came out in one colour again: {shown:?}"
        );
    }

    /// Nothing parseable leaves the preview empty rather than showing an error:
    /// plain text is a perfectly good task.
    #[test]
    fn a_sentence_with_no_syntax_in_it_previews_nothing() {
        let input = Input::new("just write it down".to_string(), Purpose::Add);
        let screen = with_input(70, 9, &tasks(&["a"]), &input, Glyphs::Unicode);
        let field = screen
            .iter()
            .position(|r| r.contains(" ADD ▏just write it down"));
        let field = field.unwrap_or_else(|| panic!("{screen:?}"));

        // Two rows down, because the rule sits between the field and what it
        // will become.
        assert_eq!(
            screen[field + 2].replace(['│', ' '], ""),
            "",
            "an unparseable line is not an error: {screen:?}"
        );
        assert_eq!(
            screen[8].trim(),
            "⏎ save   esc cancel   tab date",
            "the way out is on the line under the box: {screen:?}"
        );
    }

    /// The one thing the preview is allowed to have an opinion about. A word we
    /// did not understand stays in the title, which is right, and silent — so
    /// `@2026-13-45` looked accepted until the file had it. The tag beside it
    /// still previews: one bad word does not take the row over.
    #[test]
    fn a_date_that_does_not_exist_is_said_out_loud() {
        let render = render(crate::theme::MOCHA);
        let input = Input::new(
            "call the plumber @2026-13-45 #home".to_string(),
            Purpose::Add,
        );
        let (lines, _) = input_lines(&input, 60, render);
        let shown: Vec<(String, Style)> = lines[2]
            .spans
            .iter()
            .map(|s| (s.content.to_string(), s.style))
            .collect();

        let warned = shown
            .iter()
            .find(|(text, _)| text.contains("is not a date"))
            .unwrap_or_else(|| panic!("the preview stayed quiet about it: {shown:?}"));
        assert!(warned.0.contains("@2026-13-45"), "{shown:?}");
        assert_eq!(
            warned.1.fg,
            Some(render.colours.overdue),
            "the warning is not in the colour the bottom line warns in: {shown:?}"
        );
        assert!(
            shown.iter().any(|(text, _)| text.contains("#home")),
            "one bad word swallowed the rest of the preview: {shown:?}"
        );
    }

    /// The other half of it: the words that are *not* failed dates stay silent.
    /// A preview that cries about every `@` is one people stop reading.
    #[test]
    fn the_words_that_are_not_failed_dates_stay_quiet() {
        let render = render(crate::theme::MOCHA);
        for text in [
            "email a@b about it",
            "pay the invoice @thu",
            "read the @ sign chapter",
            "just write it down",
            "the @2026-08-20 one",
        ] {
            let input = Input::new(text.to_string(), Purpose::Add);
            let (lines, _) = input_lines(&input, 60, render);
            let shown = lines[2]
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect::<String>();
            assert!(!shown.contains("is not a date"), "{text:?} → {shown:?}");
        }
    }

    /// The whole claim of the date field in one test: there is no sequence of
    /// keys that puts a day the calendar does not have into the line. The
    /// complaint it answers is `@2026-13-45`, which the text box takes and the
    /// preview can only *say* is wrong.
    #[test]
    fn the_date_field_cannot_produce_a_day_that_does_not_exist() {
        let jan = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();

        // February, from the 31st of January: the day comes with it, clamped.
        let mut field = DateField::new(jan);
        field.move_to(true);
        field.step(1);
        assert_eq!(field.date(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());

        // And a leap February takes the 29th.
        let mut field = DateField::new(NaiveDate::from_ymd_opt(2028, 1, 31).unwrap());
        field.move_to(true);
        field.step(1);
        assert_eq!(field.date(), NaiveDate::from_ymd_opt(2028, 2, 29).unwrap());

        // Every arrow from every part, a hundred times over, still a real day.
        let mut field = DateField::new(jan);
        for i in 0..100 {
            field.step(if i % 3 == 0 { -1 } else { 1 });
            field.move_to(i % 5 == 0);
            let date = field.date();
            assert_eq!(
                NaiveDate::from_ymd_opt(date.year(), date.month(), date.day()),
                Some(date)
            );
        }

        // The day wraps inside its own month rather than spilling into the next.
        let mut field = DateField::new(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());
        field.step(1);
        assert_eq!(field.date(), NaiveDate::from_ymd_opt(2026, 4, 1).unwrap());
        field.step(-1);
        assert_eq!(field.date(), NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());

        // The month wraps; the year does not, because 9999 and 1970 are not
        // neighbours on any calendar.
        let mut field = DateField::new(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        field.move_to(true);
        field.step(1);
        assert_eq!(field.date(), NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        field.move_to(true);
        field.step(-100_000);
        assert_eq!(field.date().year(), 1970);
    }

    /// The row it draws: three parts, one of them in brackets, and the same
    /// width wherever the cursor is — a strip that shifted sideways on every
    /// `←` would be unreadable. It fits the narrowest pane the box opens in,
    /// under a locale that cannot draw an arrow.
    #[test]
    fn the_date_field_row_is_steady_and_fits() {
        let mut input = Input::new("renew the passport".to_string(), Purpose::Add);
        input.toggle_field(NaiveDate::from_ymd_opt(2026, 8, 11).unwrap());

        let row = |input: &Input, glyphs: Glyphs| {
            let render = Render {
                glyphs,
                ..render(crate::theme::MOCHA)
            };
            // 28 columns is the box inside a 34-column pane.
            let (lines, _) = input_lines(input, 28, render);
            lines[2]
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect::<String>()
        };

        let day = row(&input, Glyphs::Unicode);
        assert!(day.contains("[11] 08  2026"), "{day:?}");

        input.right();
        let month = row(&input, Glyphs::Unicode);
        assert!(month.contains(" 11 [08] 2026"), "{month:?}");
        assert_eq!(
            columns(&day),
            columns(&month),
            "the row moved sideways with the cursor"
        );

        for glyphs in [Glyphs::Unicode, Glyphs::Ascii] {
            let row = row(&input, glyphs);
            assert!(columns(&row) <= 28, "{glyphs:?}: {row:?} is too wide");
            if glyphs == Glyphs::Ascii {
                assert!(row.is_ascii(), "{row:?}");
            }
        }
    }

    /// Eight digits, in order, and no arrows: `13082026` is the 13th of August.
    /// A part that cannot take another digit hands the cursor on by itself,
    /// which is what makes it one gesture.
    #[test]
    fn eight_digits_fill_the_date_in_order() {
        let type_in = |keys: &str| {
            let mut field = DateField::new(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
            for c in keys.chars() {
                field.digit(c.to_digit(10).expect("a digit"));
            }
            field.date()
        };

        assert_eq!(
            type_in("13082026"),
            NaiveDate::from_ymd_opt(2026, 8, 13).unwrap()
        );
        assert_eq!(
            type_in("01012030"),
            NaiveDate::from_ymd_opt(2030, 1, 1).unwrap()
        );

        // A digit that cannot fit closes its part and starts the next: `4` is
        // the whole day, because there is no 40th, and `5` is then the month.
        assert_eq!(
            type_in("452026"),
            NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()
        );

        // A month of 13 is unreachable rather than refused: the `1` is the
        // month, the `3` cannot join it, so the `3` is the year's first digit.
        let mut field = DateField::new(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        for c in "0113".chars() {
            field.digit(c.to_digit(10).unwrap());
        }
        assert_eq!(field.month, 1, "a 13th month was reachable");
        assert_eq!(field.part, DatePart::Year);
    }

    /// What `tab` and `⏎` do to the line: one `@` word replaced or one
    /// appended, and the rest of the sentence left exactly as it was.
    #[test]
    fn the_field_writes_one_word_into_the_line() {
        let today = today();
        let apply = |text: &str, purpose: Purpose| {
            let mut input = Input::new(text.to_string(), purpose);
            input.toggle_field(today);
            input.field.as_mut().expect("the field is open").day = 20;
            assert!(input.apply_field());
            input.text
        };

        // It opens on the date the line already has, and puts it back in place.
        assert_eq!(
            apply("call the accountant @2026-08-13 #work", Purpose::Add),
            "call the accountant @2026-08-20 #work"
        );
        // Shorthand is a date too, and it is resolved before it is replaced.
        assert_eq!(
            apply("call the accountant @thu !high", Purpose::Add),
            "call the accountant @2026-08-20 !high"
        );
        // No date in the line: one word, appended, with one space.
        assert_eq!(
            apply("call the accountant", Purpose::Add),
            "call the accountant @2026-08-20"
        );
        assert_eq!(apply("", Purpose::Add), "@2026-08-20");
        // `p` asks how long, and takes the bare date — the one form it accepts
        // past its year horizon.
        assert_eq!(apply("", Purpose::Postpone(String::new())), "2026-08-20");

        // An `@` word that is not a date is still the date's place in the line,
        // and replacing it is the point: it is the typo being fixed.
        assert_eq!(
            apply("pay it @2026-13-45 #home", Purpose::Add),
            "pay it @2026-08-20 #home"
        );
    }

    /// The field is a mode you open on purpose and leave with `esc`, and the
    /// box underneath is untouched while it is open.
    #[test]
    fn esc_closes_the_field_before_the_box() {
        let mut input = Input::new("water the plants".to_string(), Purpose::Add);
        assert!(!input.close_field(), "there was nothing open to close");

        input.toggle_field(today());
        assert!(input.field.is_some());

        // Typing while it is open moves digits, not the line.
        input.insert('1');
        input.insert('5');
        assert_eq!(input.text, "water the plants");

        assert!(input.close_field(), "esc did not take the field");
        assert_eq!(input.text, "water the plants", "esc changed the line");
        assert!(!input.close_field(), "the second esc is the box's");

        // And with no field open, the keys are the keys they always were.
        input.insert('!');
        assert_eq!(input.text, "water the plants!");
    }

    /// An empty box says what goes in it, and it fits the narrowest pane the
    /// design promises — this is the whole of what a box split into labelled
    /// fields would have taught, and the width is why it is not one.
    #[test]
    fn an_empty_box_names_the_four_sigils() {
        let lists = ["todo.md".to_string(), "work.md".to_string()];
        let several = Render {
            lists: &lists,
            ..render(crate::theme::MOCHA)
        };
        let line = |render: Render<'_>| {
            // Cleared rather than `adding`, which opens with today's date in it:
            // the hint is what an *empty* box says, and emptying it is one `^u`.
            let box_ = Input::new(String::new(), Purpose::Add);
            let (lines, _) = input_lines(&box_, 28, render);
            lines[2]
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect::<String>()
        };

        // 28 columns is a 34-column pane, the narrowest the box is drawn in.
        let hint = line(several);
        assert_eq!(hint.trim(), "@thu #home !high $list", "{hint:?}");
        assert!(columns(&hint) <= 28, "{hint:?} does not fit the pane");

        // One list, and `$` addresses nothing: the hint does not teach a key
        // that would only be refused.
        let hint = line(render(crate::theme::MOCHA));
        assert_eq!(hint.trim(), "@thu #home !high", "{hint:?}");

        // The moment there is a task in the box the preview goes back to
        // reporting it, and plain text still gets no lecture.
        for (typed, expected) in [("buy milk @thu", "due"), ("buy milk", "")] {
            let input = Input::new(typed.to_string(), Purpose::Add);
            let (lines, _) = input_lines(&input, 60, several);
            let shown: String = lines[2]
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect();
            assert!(!shown.contains('@'), "{typed:?} → {shown:?}");
            assert!(shown.contains(expected), "{typed:?} → {shown:?}");
        }
    }

    /// The preview answers a `$` before `⏎` does: where the capture is going
    /// when the list is open, and that it is going nowhere when it is not. The
    /// fields still follow either answer.
    #[test]
    fn the_preview_says_which_list_a_capture_is_addressed_to() {
        let lists = ["todo.md".to_string(), "work.md".to_string()];
        let render = Render {
            lists: &lists,
            ..render(crate::theme::MOCHA)
        };
        let shown = |text: &str| {
            let input = Input::new(text.to_string(), Purpose::Add);
            let (lines, _) = input_lines(&input, 60, render);
            lines[2]
                .spans
                .iter()
                .map(|s| (s.content.to_string(), s.style))
                .collect::<Vec<_>>()
        };

        let open = shown("call the accountant $work @thu");
        let named = open
            .iter()
            .find(|(text, _)| text.contains("work.md"))
            .unwrap_or_else(|| panic!("the preview did not say where it goes: {open:?}"));
        assert_eq!(named.0, "→ work.md", "{open:?}");
        assert_eq!(named.1.fg, Some(render.colours.accent), "{open:?}");
        assert!(
            open.iter().any(|(text, _)| text.contains("Thursday")),
            "the address swallowed the fields: {open:?}"
        );

        // The arrow falls back with every other glyph: `LC_ALL=C` puts nothing
        // on this screen the terminal cannot draw.
        let ascii = Render {
            glyphs: Glyphs::Ascii,
            ..render
        };
        let input = Input::new("call the accountant $work".to_string(), Purpose::Add);
        let (lines, _) = input_lines(&input, 60, ascii);
        let line: String = lines[2]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(line.contains("-> work.md"), "{line:?}");
        assert!(line.is_ascii(), "{line:?}");

        // A list nobody has: the second opinion, in the colour of the first.
        let missing = shown("call the accountant $wrok @thu");
        let warned = missing
            .iter()
            .find(|(text, _)| text.contains("wrok.md"))
            .unwrap_or_else(|| panic!("the preview stayed quiet about it: {missing:?}"));
        assert_eq!(warned.0, "no list wrok.md", "{missing:?}");
        assert_eq!(warned.1.fg, Some(render.colours.overdue), "{missing:?}");
        assert!(
            missing.iter().any(|(text, _)| text.contains("Thursday")),
            "one bad word took the row over: {missing:?}"
        );

        // Every prefix on the way to `$work` stays quiet, the way every prefix
        // on the way to a date does. Four wrong warnings and one right one is
        // how a line stops being read.
        for half in ["$w", "$wo", "$wor", "$work", "$work.", "$work.m"] {
            let typed = format!("call the accountant {half}");
            let line: String = shown(&typed).into_iter().map(|(text, _)| text).collect();
            assert!(!line.contains("no list"), "{half} → {line:?}");
        }

        // Nothing addressed, nothing said — and an edit is never addressed, so
        // it does not answer a question it will refuse anyway.
        for (text, purpose) in [
            ("call the accountant @thu", Purpose::Add),
            (
                "call the accountant $work",
                Purpose::Edit("- [ ] call the accountant".to_string()),
            ),
        ] {
            let input = Input::new(text.to_string(), purpose);
            let (lines, _) = input_lines(&input, 60, render);
            let line = lines[2]
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect::<String>();
            assert!(!line.contains(".md"), "{text:?} → {line:?}");
        }
    }

    /// `p` reuses the box but not its reading of what is in it: a length of
    /// time is not a sentence, and the preview answers the question the box
    /// just asked — which day does this land on.
    #[test]
    fn the_postpone_box_previews_the_day_it_lands_on() {
        let task = capture("pay the invoice @2026-08-12", today());
        let mut input = Input::postponing(&task);
        let screen = |input: &Input| with_input(70, 9, &tasks(&["a"]), input, Glyphs::Unicode);

        // Empty: the question, not an error, and not a task's worth of fields.
        let rows = screen(&input);
        let at = rows
            .iter()
            .position(|r| r.contains(" PUT OFF ▏"))
            .unwrap_or_else(|| panic!("{rows:?}"));
        assert!(
            rows[at + 2].contains("how long?"),
            "the empty box says nothing useful: {rows:?}"
        );

        // A bare number is days — the answer to "1 gün mü 2 gün mü".
        for c in "2".chars() {
            input.insert(c);
        }
        let rows = screen(&input);
        assert!(
            rows[at + 2].contains("Wednesday (2026-08-12)"),
            "two days from Monday is Wednesday: {rows:?}"
        );

        // And what `@` takes, `p` takes.
        input.back();
        for c in "1w".chars() {
            input.insert(c);
        }
        let rows = screen(&input);
        assert!(
            rows[at + 2].contains("2026-08-17"),
            "a week from Monday: {rows:?}"
        );

        // Nonsense goes back to the question rather than previewing a date.
        input.back();
        input.back();
        for c in "3x".chars() {
            input.insert(c);
        }
        let rows = screen(&input);
        assert!(
            rows[at + 2].contains("how long?"),
            "'3x' is not a length of time: {rows:?}"
        );
    }

    /// The three states are three symbols, in both glyph sets, and the ASCII
    /// ones stay the width the column arithmetic was told about.
    #[test]
    fn every_state_has_its_own_mark() {
        let mut task = capture("a", today());
        let marks = |task: &Task| {
            (
                Glyphs::Unicode.mark(task, today()),
                Glyphs::Ascii.mark(task, today()),
            )
        };

        assert_eq!(marks(&task), ("○", "[ ]"));
        task.set_state(State::Done, today());
        assert_eq!(marks(&task), ("✓", "[x]"));
        task.set_state(State::Cancelled, today());
        assert_eq!(marks(&task), ("✗", "[-]"));

        // A cancelled task is not late, however long ago it was due — it is off
        // the list, so there is nothing left to be late for.
        let mut late = capture("a @2026-08-01", today());
        assert_eq!(marks(&late), ("!", "[!]"));
        late.set_state(State::Cancelled, today());
        assert_eq!(marks(&late), ("✗", "[-]"));
        assert!(!late.is_overdue(today()));
    }

    /// Green is the tick saying something back; red is the other outcome. The
    /// three states are three colours, and a cancelled row shares `overdue`
    /// with a late one — `✗` against `!` is what separates those two, which is
    /// the rule that nothing is carried by colour alone doing its job.
    #[test]
    fn each_state_gets_its_own_colour() {
        let colours = crate::theme::MOCHA;
        let mut task = capture("a @2026-08-01", today());

        assert_eq!(task_colour(&task, today(), colours), colours.overdue);

        task.set_state(State::Done, today());
        assert_eq!(task_colour(&task, today(), colours), colours.done);

        task.set_state(State::Cancelled, today());
        assert_eq!(task_colour(&task, today(), colours), colours.overdue);
        assert_ne!(
            colours.done, colours.overdue,
            "finished and cancelled would be indistinguishable"
        );

        // A cancelled task that was never late is red all the same: it is the
        // state that is being said, not the date.
        let mut fine = capture("a @2099-01-01", today());
        fine.set_state(State::Cancelled, today());
        assert_eq!(task_colour(&fine, today(), colours), colours.overdue);
        assert!(!fine.is_overdue(today()));
    }

    /// A capture box that hides what you are typing is not a capture box, so the
    /// field scrolls with the end of the line rather than truncating it.
    #[test]
    fn a_line_longer_than_the_pane_keeps_its_end_on_screen() {
        let input = Input::new(
            "a very long sentence that will not fit in a narrow pane at all".to_string(),
            Purpose::Edit("- [ ] x".to_string()),
        );
        let screen = with_input(30, 8, &tasks(&["x"]), &input, Glyphs::Unicode);
        let field = screen
            .iter()
            .find(|row| row.contains(" EDIT ▏"))
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
        let input = Input::new("milk @tomorrow".to_string(), Purpose::Add);
        let screen = with_input(62, 7, &tasks(&["a"]), &input, Glyphs::Ascii);
        let text = screen.join("\n");

        assert!(text.contains(" ADD |milk @tomorrow"), "{text}");
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
        let busy = with_input(40, 10, &tasks, &Input::adding(today()), Glyphs::Unicode);

        // The box covers five rows of the middle. Everything outside it is the
        // screen the reader was already looking at — the list does not scroll,
        // reflow or give up a row, which is what it did when the input lived on
        // the bottom line.
        assert_eq!(quiet[..2], busy[..2], "the list shifted under the reader");
        assert_eq!(quiet[7..9], busy[7..9], "the list shifted under the reader");
        assert!(
            busy[8].starts_with('╰'),
            "the frame lost its foot: {busy:?}"
        );
        assert!(
            busy[2..7].iter().all(|r| r.contains('│')),
            "the box is not where it should be: {busy:?}"
        );
        assert!(busy[3].contains(" ADD ▏"), "{busy:?}");
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
            let input = Input::new(text.to_string(), Purpose::Add);
            let mut terminal = Terminal::new(TestBackend::new(40, height)).unwrap();
            terminal
                .draw(|f| {
                    draw(
                        f,
                        &mut screen,
                        counts,
                        render(crate::theme::MOCHA),
                        &Notice::Hints,
                        View::List,
                        Open::Box(&input),
                    )
                })
                .unwrap();
            let p = terminal.get_cursor_position().unwrap();
            (p.x, p.y)
        };

        // The box is 36 wide on a 40-column pane and starts two in, so the
        // field begins three columns further along: `│ ADD ▏`.
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
            let screen = with_input(40, height, &tasks, &Input::adding(today()), Glyphs::Unicode);
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
        let short = with_input(40, 4, &tasks, &Input::adding(today()), Glyphs::Unicode);
        assert!(short[1].contains(" ADD ▏"), "{short:?}");
        // Three rows leave two, and two rows are both border: nothing is drawn
        // rather than a box with nowhere to type in it.
        let shorter = with_input(40, 3, &tasks, &Input::adding(today()), Glyphs::Unicode);
        assert!(!shorter.iter().any(|r| r.contains("ADD")), "{shorter:?}");
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
            ["# ## Work", "# ## Home", "plumber", ""],
            "the tasks are gone but the heading stayed"
        );
        assert!(
            matches!(
                &screen.rows[0],
                Row::Header {
                    count: 2,
                    folded: true,
                    ..
                }
            ),
            "a collapsed group that does not say how much it hides is a dead end"
        );

        assert_eq!(screen.fold(Fold::Open), None);
        assert_eq!(
            titles(&screen.rows),
            [
                "# ## Work",
                "deploy",
                "invoice",
                "",
                "# ## Home",
                "plumber",
                ""
            ]
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
                    View::List,
                    Open::Nothing,
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
                "╭ ratodo — 3 · 0! ─────────────────────────╮",
                "│▌ ## Work · 2 ───────────────────────── l │",
                "│  ╭─ ## Home · 1 ──────────────────────╮  │",
                "│  │ ○ plumber                          │  │",
                "│  ╰────────────────────────────────────╯  │",
                "│                                          │",
                "╰──────────────────────────────────────────╯",
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
                    View::List,
                    Open::Nothing,
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
                "╭ ratodo — 3 open · 0 overdue ─────────────────────────────────────────────────────╮",
                "│▌ ## Work · 2  l                                                                  │",
                "│  ╭─ ## Home · 1 ──────────────────────────────────────────────────────────────╮  │",
                "│  │ ○ plumber                                                                  │  │",
                "╰──────────────────────────────────────────────────────────────────────────────────╯",
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
        assert_eq!(titles(&screen.rows).len(), 7);
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
                count: 2,
                folded: true,
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
        assert_eq!(titles(&screen.rows), ["# ", "a", "b", ""]);
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
                    count: 3,
                    folded: true,
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

    /// The two lines that are the whole of the welcome, and where they stop
    /// being worth their rows. **No ASCII-art logo** — this is a pane somebody
    /// leaves open beside their work, and a banner is charming exactly once.
    #[test]
    fn the_first_run_screen_says_hello_in_two_lines_and_no_more() {
        let tall = rendered(66, 18, &[]).join("\n");
        assert!(
            tall.contains("a todo list that is still just a file"),
            "{tall}"
        );
        // Centred, both of them, and nothing above them but one blank row.
        let rows = rendered(66, 18, &[]);
        assert_eq!(rows[2].trim_matches(['│', ' ']), "ratodo");
        assert!(rows[2].starts_with("│    "), "not centred: {rows:?}");

        // On a short pane the greeting is the first thing to go: the box below
        // it is the part that teaches.
        let short = rendered(66, 13, &[]).join("\n");
        assert!(!short.contains("still just a file"), "{short}");
        assert!(short.contains("Nothing here yet"), "{short}");
    }

    /// The example sits in the box it will be typed into, and the line under it
    /// has already resolved the shorthand: `@tomorrow` is a date before anybody
    /// has pressed a key. That resolution is the whole reason the box is there.
    #[test]
    fn the_empty_screen_shows_the_example_in_a_real_input_box() {
        let screen = rendered(60, 16, &[]);
        let text = screen.join("\n");

        assert!(
            text.contains(&format!("ADD ▏{EXAMPLE}")),
            "the field is not the one `a` opens: {text}"
        );
        assert!(
            text.contains("due tomorrow (2026-08-11)"),
            "the shorthand was left unresolved: {text}"
        );
        assert!(
            screen.iter().filter(|r| r.contains('╭')).count() == 2,
            "the box did not draw inside the frame: {screen:?}"
        );

        // Six rows of text and four of box: below that the example goes back to
        // being a line, because losing it entirely is the one thing a short pane
        // must not do.
        let short = rendered(60, 12, &[]).join("\n");
        assert!(!short.contains("ADD ▏"), "the box did not fit: {short}");
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
                    View::List,
                    Open::Nothing,
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
        assert!(ascii.contains("ADD |"), "{ascii}");
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
                    View::List,
                    Open::Nothing,
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
        let with_headers = Screen::new(vec![Row::header("Work", 0), Row::GroupEnd]);
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
                    View::List,
                    Open::Nothing,
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
            rendered(40, 1, &tasks)[0].starts_with('╭'),
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
