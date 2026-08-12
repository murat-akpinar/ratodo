# Map — where everything is

The navigation file. [README.md](README.md) maps the *decisions*; this one maps
the *code*, because `src/ui.rs` is 8,314 lines and knowing which of eleven files
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
| [`model.rs`](../src/model.rs) | 1,002 | 25 | `Doc` / `Line` / `Item` / `Task` / `Due` / `Priority` / `State` |
| [`parse.rs`](../src/parse.rs) | 381 | 19 | file → `Doc`. **The raw line is always kept** |
| [`write.rs`](../src/write.rs) | 512 | 18 | `Doc` → file. Atomic, `.bak` first, mtime check |
| [`capture.rs`](../src/capture.rs) | 679 | 22 | free text → `Task`. Permissive on purpose, unlike `parse` |
| [`text.rs`](../src/text.rs) | 381 | 15 | every human-facing string that is not the TUI |
| [`agenda.rs`](../src/agenda.rs) | 1,028 | 33 | `(&[Task], today)` → `Vec<Group>`, plus `Counts`, `week` and `stats` |
| [`ics.rs`](../src/ics.rs) | 440 | 20 | VTODO, by hand, no crate |
| [`theme.rs`](../src/theme.rs) | 587 | 19 | 11 roles, 6 built-ins, `theme.conf` parser |
| [`ui.rs`](../src/ui.rs) | 8,314 | 144 | all ratatui drawing and the keymap |
| [`main.rs`](../src/main.rs) | 2,366 | 30 | clap, IO, the event loop |

Why `capture` is not `parse`, why `text.rs` exists at all, and why paths are
resolved once at the top: [architecture.md](architecture.md#module-layout).

---

## `src/ui.rs` — the one that needs a map

Code is lines 1–3994. **Tests are 3995–8314** — that is 52% of the file, and
the helpers at the top of the test module (`today`, `render`, `tasks`,
`in_section`, `titles`, `press`, `rendered`, `rendered_with`, `paint`,
`with_input`, `at_column`, `stats_of`, `form`, `a_week_of_work`) are what every
buffer test is built from.

**It stays one file.** The form was the natural place to split, and the answer
at step 2 was no: it shares the tokenizer, the glyph set, the width arithmetic
and `Input` with the box it falls back to, and a file importing eight private
items from its neighbour is one file wearing two names. The length is in the
tests, which live with their code by Rust convention.

### Keys

| ~Line | Item | Note |
|---|---|---|
| 20 | `enum Action` | what a keypress *means*, separate from reading it |
| 86 | `fn action(KeyEvent)` | the keymap. Testable without a terminal — that is the point |
| 1087 | `enum Typed` · 1106 `fn typing` | the same, for the input box and the form |

A new key is **two** edits: `Action` + `action()` here, and the match arm in
`main::run`. Plus a row in `ui::help` and a slot in `ui::hints`.

### The input box

| ~Line | Item |
|---|---|
| 154 | `struct Input` — text, caret, purpose, optional date field |
| 179 | `struct DateField` + `DatePart` — the `tab` three-part picker |
| 199 | `enum Purpose` — Add / Change / Duplicate / Postpone, and their labels |
| 395 | `impl Input` — typing, caret movement, `tab`, `esc` |
| 2390 | `fn input_lines` — the field and the live parse under it |
| 3662 | `fn input_box` — the box drawn over the middle of the list |

### The form — `a` and `⏎`

**The line is the model.** The text box holds the whole line and every row under
it is a view of that one string: each reads `capture::parts` to know what is
selected and writes back by replacing the span that tokenizer claimed. There is
no second parser and nothing to re-serialize, which is what makes the form safe
for `⏎` — [decisions.md](decisions.md).

| ~Line | Item |
|---|---|
| 613 | `fn set_parts` — replace / remove / append one claimed span, one adjacent space |
| 661 | `fn after_date` — a time goes directly after the date, the only place `capture` reads one |
| 683 | `fn set_tags` — the tag set, cleared and written back as one run |
| 705 | `part_of` · 713 `tags_of` — reading the line back through the same tokenizer |
| 729 | `enum Field` — the six the format carries, plus the two buttons |
| 771 | `struct Form` · 798 `impl Form` — `adding`, `editing`, `fits`, `order`, `choices_for`, `press` |
| 3364 | `fn typed_line` — the line as typed, coloured by what the parser took |
| 3397 | `fn radios` — `◉`/`○`, `(o)`/`( )` |
| 3415 | `fn form_box` — the whole screen |

### The stats screen — `s`

| ~Line | Item |
|---|---|
| `agenda.rs` 107 | `enum Period` — week / month / year |
| `agenda.rs` 138 | `struct Stats` · 172 `fn stats` — pure, `today` a parameter |
| `agenda.rs` 89 | `fn week` — the seven cells the main screen's sparkline reads too |
| 3095 | `bar_of` · 3108 `gauge` — a length, and the one bar that is a fraction |
| 3125 | `fn stats_screen` — blocks, and the order they drop in |

### The list

| ~Line | Item | Note |
|---|---|---|
| 1147 | `enum Row` | `Header` · `Task` · `GroupEnd`. The blank row became the group box's bottom edge |
| 1187 | `fn rows(&[Group]) -> Vec<Row>` | flattens the agenda. Where `## ` markers are added |
| 1219 | `struct Screen` | `all` / `folded` / `rows` / `state` |
| 1239 | `impl Screen` | `refresh` keeps the cursor **by identity**; `is_selectable` is the selection invariant |

### Drawing

| ~Line | Item | Note |
|---|---|---|
| 1464 | `enum Glyphs` | **every** Unicode glyph and its ASCII fallback. New furniture goes here first |
| 1667 | `enum Size` | `Bare` <34 · `Narrow` 34–59 · `Wide` ≥60 |
| 1688 | `struct Render` | colours, glyphs, today, path, lists — everything not the list |
| 1703 | `fn columns` | display columns, not bytes. `ş` is 1, `🚀` is 2 |
| 1713 | `shorten` / `tail` / `lead` | cutting text to a width |
| 1769 | `fn when` | the date column's text. **Says what the heading does not** |
| 1855 | `COLUMNS_AT = 71` · 1857 `impl Columns` | the fourth breakpoint, in columns of **row** |
| 1950 | `fn task_line` | one row: mark, title, date, priority, tags |
| 2074 | `heading` · 2096 `header_line` | the name plus its count; the folded form is a bare rule |
| 2131 | `INSET` · 2137 `BOX_MARGIN` · 2151 `group_edge` · 2212 `boxed` | the group box |
| 2226 | `enum Notice` · 2242 `fn hints` | the bottom line; the hint bar is a greedy fill over 8 keys |
| 2575 | `enum View` · 2587 `enum Open` | which screen, and what is over it |
| 2596 | **`fn draw`** | the entry point. Frame, band, list, footer, overlays, bottom line |
| 2866 | `const FOOTER` · 2883 `impl Band` | what the top and bottom rows cost, and when they go |
| 2908 | `sparkline` · 2927 `tiles` · 2951 `tile_rows` · 2975 `band` | the five rows at the top |
| 3037 | `rule_across` · 3060 `footer` | the rule that meets the frame, and the file's own line |
| 3728 | `fn help` | the `?` overlay — 11 keys, ceiling of 12 |
| 3808 | `fn empty` · 3885 `fn example` | the first-run screen |
| 3910 | `title_counts` · 3930 `filled` · 3948 `progress` | the title bar and its `3/8` |
| 3984 | `fn task_colour` | red / green / grey, one place |

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
| [`fidelity.rs`](../tests/fidelity.rs) | 8 | round-trip, byte-for-byte, over every fixture — including opening the form on a task and closing it again. **The most important file here** |
| [`property.rs`](../tests/property.rs) | 4 | 4,000 generated documents, the generator its own oracle |
| [`cli.rs`](../tests/cli.rs) | 48 | the real binary, `$XDG_*` pointed at a scratch directory |
| `fixtures/` | — | `simple.md` `gnarly.md` `crlf.md` `no-final-newline.md` `empty.md` |

Unit tests live in the file they test — 144 of them inside `src/ui.rs` alone.
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

All four steps are **built** (2026-08-12) and the tag is not cut: the maintainer
looks at it in their own terminal first, per [CLAUDE.md](../CLAUDE.md).

| Step | Where it landed |
|---|---|
| **1 · dashboard** | `ui.rs`: `Row`/`rows`, `group_edge`, `boxed`, `heading`, `when`, `Band`, `band`, `footer`, `hints`, `draw`. `COLUMNS_AT` 76 → 71, because the box takes five columns off every row |
| **2 · `a` opens a form** | `ui.rs`: `set_parts`, `set_tags`, `Field`, `Form`, `form_box`. **No new file** — it shares the tokenizer, glyphs, widths and `Input` with the box it falls back to |
| **3 · `s` stats** | `agenda.rs`: `Period`, `Stats`, `stats`, `week`. `ui.rs`: `stats_screen`, `bar_of`, `gauge` |
| **4 · `⏎` opens the form** | `model.rs`: `Task::retype` writes the bytes it was given instead of a rendering of them. No `splice_at` — the form already edits the line in place |

Four hard invariants sit directly under this work: round-trip fidelity, never
reordering the user's file, `agenda`'s `today` parameter, and the ASCII
fallback. They are listed in full in [CLAUDE.md](../CLAUDE.md).
