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

⚠️ Catppuccin is 24-bit RGB. `Color::Rgb` requires a truecolor terminal. The risk
is low for this audience — alacritty, kitty, wezterm and foot all support it —
but it degrades on a bare TTY and inside old `screen`. The answer is the built-in
`terminal` theme, which uses only ANSI 0–15: `ratodo --theme terminal`.

The middle column is the whole configuration surface — eleven keys, no more.
Everything below this line describes the *default* values; a user's theme can
change any of them.

## Rules

- **One accent colour (mauve) plus greys.** If everything is coloured, nothing is
  emphasised.
- **Red is only for overdue.** Nowhere else.
- **Green is only for completed.** Both are earned meanings; don't dilute them.
  The progress bar in the title rule is green for exactly this reason: it is the
  only other thing in the product that means *finished*, so it does not get a
  colour of its own.
- Two levels of hierarchy: task title bright, date/tags `dim`. There is no third.
- Generous whitespace. The blank lines between groups are half of the design.
- **One layout, no split panes.** No sidebar, no modal. There is one list.
- `○ ✓ !` symbols — never rely on colour alone (colour blindness, and so output
  survives being copy-pasted).
  ⚠️ These are **screen symbols only**, not file syntax. The file contains only
  `[ ]` and `[x]`; `!` means overdue and is derived from the date. `- [!]` is
  never written to the file.
- **Do not depend on a Nerd Font.** An ASCII fallback is mandatory: `[ ]` `[x]` `[!]`.
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
