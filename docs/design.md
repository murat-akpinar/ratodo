# Visual design

## Palette — Catppuccin Mocha, accent = mauve

This is the **default**, not the only option. Colours are user-configurable
through `~/.config/ratodo/theme.conf`; the full spec is in
[theming.md](theming.md). What follows is what someone sees with zero
configuration, and it has to look right on its own.

Let the reasoning for the default be honest: the target audience (tiling WM
users) is already running Catppuccin, and the tool should not look foreign next
to the rest of their screen.

| Role | `theme.conf` key | Catppuccin Mocha | Hex |
|---|---|---|---|
| background | `background` | `base` | `#1e1e2e` |
| primary text | `foreground` | `text` | `#cdd6f4` |
| date, tags (dim) | `dim` | `subtext0` | `#a6adc8` |
| border | `border` | `overlay0` | `#6c7086` |
| selected row | `selection` | `surface0` | `#313244` |
| accent / selection marker | `accent` | `mauve` | `#cba6f7` |
| overdue | `overdue` | `red` | `#f38ba8` |
| today | `today` | `peach` | `#fab387` |
| done ✓ | `done` | `green` | `#a6e3a1` |
| completed task text | `done_text` | `overlay1` | `#7f849c` |
| tag `#tag` | `tag` | `blue` | `#89b4fa` |
| priority `!high` | `priority` | `yellow` | `#f9e2af` |

⚠️ Catppuccin is 24-bit RGB. `Color::Rgb` requires a truecolor terminal. The risk
is low for this audience — alacritty, kitty, wezterm and foot all support it —
but it degrades on a bare TTY and inside old `screen`. The answer is the built-in
`terminal` theme, which uses only ANSI 0–15: `ratodo --theme terminal`.

The middle column is the whole configuration surface — twelve keys, no more.
Everything below this line describes the *default* values; a user's theme can
change any of them.

## What each colour means

**One colour, one job.** A colour that answers two questions answers neither: if
mauve is the heading *and* the priority *and* the label *and* the border, then
mauve has stopped meaning anything and the screen reads as noise. This table is
the whole of it, and nothing on the screen is allowed a colour that is not one
of these rows.

| Colour | Its one job | Everywhere it appears |
|---|---|---|
| `foreground` | **full brightness: the user's own words, and the name of the mode they are in** | task titles, the line being typed, help descriptions, the `ADD` / `EDIT` / `PUT OFF` labels (bold) |
| `dim` | **a secondary fact** | dates that are not pressing, tags in a preview, the caret, the hint bar, the empty-box example |
| `border` | **furniture** — frames and rules, never content | the main frame, the column rules `│`, the preview separators, heading rules |
| `accent` | **the tool pointing at something** | group headings, the input box's own border, the focused cell of the date field, the key names in `?`, `→ work.md`, a resolved `@thu` and the day `p` lands on, the `COPY` label |
| `overdue` | **the negative outcome** | an overdue row and its date, a cancelled row, every warning and refusal in the bottom line and the preview |
| `today` | **due today** | a row due today and its date |
| `done` | **finished** | a ticked row, the `✓`, the progress bar |
| `tag` | **a tag** | `#home`, on the row and in the box |
| `priority` | **a priority** | `!high` bold, `!med` plain — `!low` is dim, because it asked to be |

Two things follow from the table that are easy to get wrong:

- **`accent` is the tool's voice, never the data's.** A group heading is ours; a
  task title is not. A resolved date under the input is us saying *this is what
  we understood*; the date on the row is the task's own and wears the row's
  colour. When something is in doubt, ask whether the user typed it.
- **A label is lit by weight, not by a colour of its own.** The four box labels
  are `foreground` and bold — full brightness against the dim caret beside them
  — and only `COPY` takes the accent, because only `COPY` has news. Giving them
  a hue would have meant a thirteenth role and a seventh meaning on the screen,
  which is the sprawl this table exists to stop. The palette is not the limit
  here: Catppuccin Mocha has fourteen accent colours and the other five built-in
  themes do not, so a role has to be fillable in `nord`, `gruvbox-dark` and a
  bare `terminal` too.
- **The tool's own words are upper case.** `OVERDUE`, `TODAY`, `LATER` on the
  list and `ADD`, `EDIT`, `COPY`, `PUT OFF` in the box. A user's heading keeps
  its `##` and its own casing; ours does not need one, because the case already
  says whose word it is.
- **A field never borrows the row's colour.** `!high` on a late row used to be
  drawn in `overdue`, which made the date and the priority the same red on the
  one row where they most need telling apart. Every field keeps its own job's
  colour whatever the row is doing.

## Rules

- **One accent colour (mauve) plus greys, and the six meanings above.** If
  everything is coloured, nothing is emphasised.
- **Red is the negative outcome**: overdue, and cancelled. *(Was "only for
  overdue" — widened 2026-08-11, see [decisions.md](decisions.md#reversed).)*
  Nowhere else. `!` and `✗` are what tell the two apart, which is the symbol
  rule below doing the job it exists for.
- **Green is only for completed.** Both are earned meanings; don't dilute them.
  The progress bar in the title rule is green for exactly this reason: it is the
  only other thing in the product that means *finished*, so it does not get a
  colour of its own — and the finished **row** wears it too, since the row is
  the thing that earned it.
- Two levels of hierarchy: task title bright, date/tags `dim`. There is no third.
- Generous whitespace. The blank lines between groups are half of the design.
- **A rule between two columns, and nowhere else.** Past
  [`COLUMNS_AT`](tui.md#width) the date, priority and tags start in the same
  place on every row, and a dim `│` says where each one begins — the row is a
  table there, so it is drawn as one. Below that breakpoint the fields are
  ragged and there is nothing to separate, so there are no rules: three
  characters of noise per row is what a table costs when it has no columns. The
  same `│` separates the fields in the input box's preview, so the screen has
  one separator and not two. Rules are `border`, the colour the frame is
  already drawn in — a grid is scenery, and scenery does not get the accent.
- **One layout, no split panes.** No sidebar, no modal. There is one list.
- `○ ✓ ✗ !` symbols — never rely on colour alone (colour blindness, and so output
  survives being copy-pasted).
  ⚠️ `!` is a **screen symbol only**, derived from the date, and `- [!]` is
  never written to the file. `○ ✓ ✗` do have file forms — `[ ]` `[x]` `[-]` —
  and those three are the whole of it. See [format.md](format.md#the-three-states).
- **Do not depend on a Nerd Font.** An ASCII fallback is mandatory: `[ ]` `[x]` `[-]` `[!]`.
- **Do not use strikethrough.** crossterm supports it, but terminal support is
  inconsistent and for half the users a completed task becomes unreadable. Dim
  colour plus `✓` is enough.

## Screens

Every screen, every interaction state and the full keymap live in
[tui.md](tui.md) — one canonical set of sketches, so there is never a second
drawing to disagree with the first.

Quick capture, without opening the TUI at all:

```
$ ratodo add 'pay the invoice @tomorrow #home'
added: pay the invoice  ·  due tomorrow (2026-08-11)  ·  #home
$
```

One line of output, then you're back. Nothing fancy — the user is in the middle
of something else.

## Agenda grouping rules (v1)

The agenda is a pure function: `agenda(&[Task], today) -> Vec<Group>`. Today's
date is a **parameter**; there is no `Local::now()` call inside the function,
otherwise it cannot be tested. The real logic of the product is here, and all of
it is testable without a terminal.

| # | Group | Condition | How it looks |
|---|---|---|---|
| 1 | OVERDUE | `due < today` | `!` · red · "2 days ago" |
| 2 | TODAY | `due == today` | `○` · peach · `16:00` if a time is set |
| 3 | THIS WEEK | `today < due ≤ +7d` | `○` · dim · `Aug 20` |
| 4 | LATER | `due > +7d` | `○` · dim · collapsed, `l` expands |
| 5 | *(the file's `##` heading)* | no date | `○` · in file order |
| 6 | — | `[x]` completed | `✓` · dim · at the end of its own **dated** group |

Undated tasks stay under the `##` sections of the file, **in the order they
appear in the file**. We do not reorder the user's own arrangement — and that
beats row 6, so a completed undated task keeps its place rather than sinking.
The two rules can only collide there, because rows 1–4 are already an ordering
the user did not write.

Inside a dated group: open before completed, then by date, then by time. A task
with no time heads its own day, the way a calendar puts all-day events above the
timetable. Ties keep file order — the sort is stable, deliberately.

Group 1 is about *where a task sits*, which is not the same question as whether
it still needs attention: a completed task keeps the date it had, so it appears
under OVERDUE with a `✓`, while `Task::is_overdue` — the one that drives the `!`
symbol, the counts and `status`'s exit code — says no. Both readings of the word
are right; they are answering different questions, and the code says so where
they meet.

There is exactly one view mode: **agenda**. There is no second "file view /
agenda view" mode — two modes means state management, key conflicts and two
separate drawing paths. v1 does not need it.
