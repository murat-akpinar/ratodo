# Map — where everything is

The navigation file. [README.md](README.md) maps the *decisions*; this one maps
the *code*, because `src/ui.rs` is 5,654 lines and knowing which of eleven files
to open is most of the work.

**Line numbers are a hint, not an address.** They drift with every commit. The
names do not — grep for the name, the number is only there to say *roughly how
far down*.

---

## Start here: what am I changing?

| I want to change… | Open | Also |
|---|---|---|
| How a line of `todo.md` is read | [`src/parse.rs`](../src/parse.rs) | [format.md](format.md) · `cargo mutants` per [CLAUDE.md](../CLAUDE.md) |
| How a line is written back | [`src/write.rs`](../src/write.rs), [`src/model.rs`](../src/model.rs) | [architecture.md](architecture.md) · `tests/fidelity.rs` |
| What `@thu` / `#tag` / `!high` / `$list` mean when **typed** | [`src/capture.rs`](../src/capture.rs) | one tokenizer only — `parts` is it |
| Which group a task lands in | [`src/agenda.rs`](../src/agenda.rs) | `agenda(&[Task], today)`, `today` is a parameter |
| Anything drawn on the TUI | [`src/ui.rs`](../src/ui.rs) | [tui.md](tui.md) · [design.md](design.md) |
| A keybinding | `ui::action` **and** `main::run`'s match | [tui.md](tui.md) keymap · the `?` overlay in `ui::help` |
| Terminal-free text (`list`, `status`, `--json`) | [`src/text.rs`](../src/text.rs) | never `println!` inside `main.rs` if there is a decision in it |
| A colour | [`src/theme.rs`](../src/theme.rs) | [theming.md](theming.md) — a new role means every built-in grows a line |
| `.ics` output | [`src/ics.rs`](../src/ics.rs) | [calendar.md](calendar.md) |
| A subcommand or flag | [`src/main.rs`](../src/main.rs) `Cli` / `Command` | `completions/` — a test asks the binary and they cannot rot |
| Where files are found on disk | `main.rs` — `dirs`, `default_path`, `lists`, `Derived` | **the only file that reads the environment** |

---

## `src/` — eleven files, flat

Sizes and test counts as of 2026-08-12.

| File | Lines | Tests | What it is |
|---|---|---|---|
| [`lib.rs`](../src/lib.rs) | 14 | — | the module list. Exists so `tests/` can reach the core |
| [`model.rs`](../src/model.rs) | 963 | 24 | `Doc` / `Line` / `Item` / `Task` / `Due` / `Priority` / `State` |
| [`parse.rs`](../src/parse.rs) | 381 | 19 | file → `Doc`. **The raw line is always kept** |
| [`write.rs`](../src/write.rs) | 512 | 18 | `Doc` → file. Atomic, `.bak` first, mtime check |
| [`capture.rs`](../src/capture.rs) | 679 | 22 | free text → `Task`. Permissive on purpose, unlike `parse` |
| [`text.rs`](../src/text.rs) | 381 | 15 | every human-facing string that is not the TUI |
| [`agenda.rs`](../src/agenda.rs) | 583 | 23 | `(&[Task], today)` → `Vec<Group>`, plus `Counts` |
| [`ics.rs`](../src/ics.rs) | 440 | 20 | VTODO, by hand, no crate |
| [`theme.rs`](../src/theme.rs) | 587 | 19 | 11 roles, 6 built-ins, `theme.conf` parser |
| [`ui.rs`](../src/ui.rs) | 5,654 | 113 | all ratatui drawing and the keymap |
| [`main.rs`](../src/main.rs) | 2,250 | 30 | clap, IO, the event loop |

Why `capture` is not `parse`, why `text.rs` exists at all, and why paths are
resolved once at the top: [architecture.md](architecture.md#module-layout).

---

## `src/ui.rs` — the one that needs a map

Code is lines 1–2300. **Tests are 2301–5654** — that is 60% of the file, and
the helpers at the top of the test module (`today`, `render`, `tasks`,
`in_section`, `titles`, `press`, `rendered`, `rendered_with`, `paint`,
`with_input`, `at_column`) are what every buffer test is built from.

### Keys

| ~Line | Item | Note |
|---|---|---|
| 19 | `enum Action` | what a keypress *means*, separate from reading it |
| 74 | `fn action(KeyEvent)` | the keymap. Testable without a terminal — that is the point |
| 587 | `enum Typed` · 606 `fn typing` | the same, for the input box |

A new key is **two** edits: `Action` + `action()` here, and the match arm in
`main::run`. Plus a row in `ui::help` and a slot in `ui::hints`.

### The input box

| ~Line | Item |
|---|---|
| 135 | `struct Input` — text, caret, purpose, optional date field |
| 160 | `struct DateField` + `DatePart` — the `tab` three-part picker |
| 180 | `enum Purpose` — Add / Change / Duplicate / Postpone, and their labels |
| 376 | `impl Input` — typing, caret movement, `tab`, `esc` |
| 1666 | `fn input_lines` — the field and the live parse under it |
| 1988 | `fn input_box` — the box drawn over the middle of the list |

### The list

| ~Line | Item | Note |
|---|---|---|
| 647 | `enum Row` | `Header` · `Task` · `Spacer`. **The blank row is a row, not a margin** |
| 670 | `fn rows(&[Group]) -> Vec<Row>` | flattens the agenda. Where `## ` markers are added |
| 703 | `struct Screen` | `all` / `folded` / `rows` / `state` |
| 747 | `Screen::refresh` | rebuilds visible rows; keeps the cursor **by identity** |
| 894 | `Screen::is_selectable` | tasks and folded headers only — the selection invariant |
| 908 | `Screen::move_by` | steps over headers and blanks, stops at the ends |

### Drawing

| ~Line | Item | Note |
|---|---|---|
| 948 | `enum Glyphs` | **every** Unicode glyph and its ASCII fallback. New furniture goes here first |
| 1113 | `enum Size` | `Bare` <34 · `Narrow` 34–59 · `Wide` ≥60 |
| 1134 | `struct Render` | colours, glyphs, today, path, lists — everything not the list |
| 1149 | `fn columns` | display columns, not bytes. `ş` is 1, `🚀` is 2 |
| 1159 | `shorten` / `tail` / `lead` | cutting text to a width |
| 1215 | `fn when` | the date column's text |
| 1268 | `struct Columns` · 1287 `COLUMNS_AT = 76` | the fourth breakpoint. **76, not 80** |
| 1353 | `fn task_line` | one row: mark, title, date, priority, tags |
| 1479 | `fn header_line` | the group heading and its rule |
| 1515 | `enum Notice` · 1531 `fn hints` | the bottom line; the hint bar is a greedy fill over 7 keys |
| 1845 | **`fn draw`** | the entry point. Frame, list, overlays, bottom line |
| 2054 | `fn help` | the `?` overlay — 10 keys, ceiling of 12 |
| 2130 | `fn empty` · 2191 `fn example` | the first-run screen |
| 2216 | `title_counts` · 2236 `filled` · 2254 `progress` | the title bar and its `3/8` |
| 2290 | `fn task_colour` | red / green / grey, one place |

---

## `src/main.rs`

| ~Line | Item |
|---|---|
| 19 | `struct Cli` · 33 `enum Command` — every subcommand and flag |
| 107 | `fn dispatch` — the one place the environment is read |
| 131–213 | `dirs` `env_path` `default_path` `lists` `capture_target` `addressed_list` `capture_into` |
| 282 | `struct Derived` — the `.bak` and `.ics` paths, resolved once and carried |
| 444 | `fn watch` — inotify, on the **directory** |
| 475 | `fn tui` — terminal setup and teardown |
| 510 | `struct Live` — files, undo, screen, counts |
| 526–947 | `Live`'s methods: `reload` `edit` `write_back` `toggle` `cancel` `delete` `save_typed` `undo` |
| 963 | **`fn run`** — the event loop and the `Action` match |
| 1127+ | `done` · `list` · `status` — the scriptable surface |

---

## `tests/`

| File | Tests | What it pins |
|---|---|---|
| [`fidelity.rs`](../tests/fidelity.rs) | 6 | round-trip, byte-for-byte, over every fixture. **The most important file here** |
| [`property.rs`](../tests/property.rs) | 4 | 4,000 generated documents, the generator its own oracle |
| [`cli.rs`](../tests/cli.rs) | 48 | the real binary, `$XDG_*` pointed at a scratch directory |
| `fixtures/` | — | `simple.md` `gnarly.md` `crlf.md` `no-final-newline.md` `empty.md` |

Unit tests live in the file they test — 158 `fn`s inside `src/ui.rs` alone.
Strategy and the two tests that matter most: [testing.md](testing.md).

---

## Everything else

| Path | What it is |
|---|---|
| `docs/` | the decision record — [README.md](README.md) is its index |
| `../notes.md` | raw thinking, loose ends, the idea graveyard |
| `../todo.md` | the task list. Work top to bottom |
| `../CLAUDE.md` | working rules for agents: workflow, invariants, commit format |
| `../README.md` | user-facing. First sentence says "built **with** ratatui" |
| `../tui/` | the five mockups the v0.8.0 redesign answers |
| `completions/` | hand-written bash/zsh/fish. A test asks the binary what it answers to |
| `packaging/PKGBUILD` | built and verified against the tag. `.SRCINFO` beside it |
| `../flake.nix` | written, **never evaluated** — no `nix` on this machine |
| `scripts/check-docs.py` | every relative link and anchor in every `.md` must resolve |
| `scripts/demo.py` | records `assets/demo.gif`. Needs kitty, menyoki, ffmpeg, X11 |
| `../cliff.toml` | git-cliff. `filter_unconventional = true` — a bad message vanishes |
| `../CHANGELOG.md` | generated. Never edited by hand |

---

## The v0.8.0 redesign, by file

The plan is [redesign.md](redesign.md); the checkable steps are in
[todo.md](../todo.md). What each step actually touches:

| Step | Files |
|---|---|
| **1 · dashboard** | `ui.rs`: `Row`, `rows`, `Screen::is_selectable`, `header_line`, `task_line`, `Columns`, `Glyphs`, `hints`, `draw`. New glyphs (`╭╮╰╯`, `┬┴`) need an ASCII form **before** the `is_ascii()` assertions see them |
| **2 · `a` opens a form** | `ui.rs` or a new flat file beside it — decide at this step, and [architecture.md](architecture.md#module-layout)'s file list moves either way. `decisions.md` entry **first** |
| **3 · `s` stats** | `agenda.rs` — `stats` has `agenda`'s signature, so it goes beside it and the module list does not move. Plus `ui.rs` draw, `hints`, `help` |
| **4 · `⏎` opens the form** | `model.rs` — `Task::splice` (~224) exists; the step adds `splice_at(range, to)` beside it. `capture::parts` supplies the ranges. `tests/fidelity.rs` |

Four hard invariants sit directly under this work: round-trip fidelity, never
reordering the user's file, `agenda`'s `today` parameter, and the ASCII
fallback. They are listed in full in [CLAUDE.md](../CLAUDE.md).
