# Screens and interaction

[design.md](design.md) decides what things look like. This document decides what
is on the screen, in which state, and which key does what.

The question everything here answers:

> Someone is in the middle of real work. A task pops into their head. They glance
> at a pane in the corner of a tiling layout, get it out of their head, and go
> back. **Nothing they just did should require thinking about the tool.**

## Two modes, and that is the whole state machine

The temptation with a vim-flavoured TUI is a modal editor: normal mode, insert
mode, visual mode, a command line. That is the wrong shape here. Modes are a cost
you pay on *every* interaction, and this tool's entire premise is that an
interaction costs two seconds.

So there are exactly two modes:

| Mode | When | How you leave |
|---|---|---|
| **list** | Always, by default | `q` |
| **input** | Only while adding or editing a task | `⏎` saves, `esc` cancels, `ctrl-c` cancels |

You can never be in a mode you did not explicitly open, and `esc` always gets you
back. There is no command mode, no visual mode, no pending-operator state.

`ctrl-c` deserves its own line, because it means two different things in the two
modes: in list mode it quits, in input mode it **cancels the input and returns to
the list** — it does not quit. Someone half-way through typing a task who reaches
for the universal "stop that" key should lose the sentence, not the session.

**Vim keys, not vim modes.** That is the line — the same one lazygit, k9s, aerc
and ranger draw, and it is the right one for an audience that has vim muscle
memory but is not opening this thing to edit text.

## The bottom line

There is one reserved line under the frame. It is the only part of the screen
that changes shape, and it is doing the job vim's status line does:

| Shows | When |
|---|---|
| key hints | default |
| an input field | while adding or editing |
| a result message + undo | just after an action |
| a warning | on a write conflict |

One line, four jobs. Nothing pops over the list, nothing shifts the layout, and
**the list never moves under you** — which is the actual reason for this design,
not tidiness. A modal dialog that covers the tasks you were reading is exactly
the interruption this tool exists to avoid.

## Main screen

```
┌─ ratodo ────────────────────────────── 5 open · 1 overdue ─┐
│                                                            │
│ OVERDUE ─────────────────────────────────────────────────  │
│   ! rotate the backup keys                   2d ago  #ops  │
│                                                            │
│ TODAY ───────────────────────────────────────────────────  │
│ ▌ ○ pay the invoice                                 #home  │
│   ○ review the deploy PR                     16:00  #work  │
│                                                            │
│ THIS WEEK ───────────────────────────────────────────────  │
│   ○ book a dentist appointment         Thu 09:30  #health  │
│   ✓ migrate the server                                     │
│                                                            │
│ SOMEDAY ─────────────────────────────────────────────────  │
│   ○ finish chapter 13 of the Rust book               !low  │
│                                                            │
│ LATER (3) ──────────────────────────────────────────── l   │
│                                                            │
└────────────────────────────────────────────────────────────┘

 j k move   spc done   a add   ⏎ edit   d del   e $EDITOR   ? keys   q quit
```

Details that are decisions, not drawing:

- **`▌` is the selection**, in `accent`, with the row on `selection` background.
  A colour alone is not enough — see [design.md](design.md#rules).
- **Group headers get a rule to the right edge.** In a narrow pane the eye needs
  a horizontal anchor to find where a group starts; a bare word does not give it.
- **`LATER (3)` stays collapsed** and shows its count and its key. A collapsed
  group that does not say how to open it is a dead end.

  A collapsed group is also **selectable** — the cursor lands on it, the way it
  lands on a closed directory in `lf` or `ranger`. That is not decoration: the
  tasks inside are gone from the screen, so the header is the only thing left to
  put a cursor on, and without one `l` would have nothing to open. Folding would
  be a one-way trip.

  Folds are remembered by heading, so they survive a reload — `ratodo add` in
  another pane must not quietly undo them. A file with the same heading twice
  therefore folds both at once, which is the price of that and is the user's own
  arrangement.
- **`SOMEDAY` is a `##` heading from the user's file**, not one of ours. Dated
  groups come first, then the user's own sections in file order.
- The date column is right-aligned and relative where that reads better
  (`2d ago`, `Thu 09:30`), absolute where it does not (`Aug 20`).
- Counts in the title bar are the same numbers a waybar module will show in v4
  ([roadmap.md](roadmap.md)). Same wording, one source.

## Adding

`a` opens the input on the bottom line. The list stays exactly where it was:

```
┌─ ratodo ────────────────────────────── 5 open · 1 overdue ─┐
│                                                            │
│ TODAY ───────────────────────────────────────────────────  │
│ ▌ ○ pay the invoice                                 #home  │
│   ○ review the deploy PR                     16:00  #work  │
│                                                            │
│ THIS WEEK ───────────────────────────────────────────────  │
│   ○ book a dentist appointment         Thu 09:30  #health  │
│                                                            │
└────────────────────────────────────────────────────────────┘
 add ▏call the accountant @thu !high█
     due Thu 2026-08-13  ·  !high              ⏎ save   esc cancel
```

The second line is a **live parse preview**, and it is the most valuable ten
lines of code in the TUI. As you type `@thu`, it resolves to a real date in front
of you. That does three things at once: it teaches the syntax without anyone
reading [format.md](format.md), it catches a typo before it reaches the file, and
it proves the shorthand actually did what you meant.

If nothing parses, the preview line stays empty rather than showing an error —
plain text is a perfectly good task.

`⏎` saves and closes. `esc` cancels and the text is discarded. `e` is still the
way out to `$EDITOR` for anything more involved.

## Editing

`⏎` on a selected task opens the same input, pre-filled with the task's text as
it appears in the file. Same preview line, same keys.

Editing writes back **only** the fields that changed; the rest of the line stays
byte-for-byte identical, which is the invariant in
[architecture.md](architecture.md#round-trip-fidelity).

## Deleting — no confirmation dialog

```
 deleted “rotate the backup keys”                        u undo
```

A confirmation prompt is the wrong trade here. It stops the flow on **every**
delete to protect against the rare mistaken one. Undo inverts that: deleting
costs one key, and the mistake costs one more.

`u` undoes the last change in this session — a delete, a toggle, or an edit. The
`.bak` file is the backstop underneath it. See
[architecture.md](architecture.md#concurrent-editing).

## Write conflict

The one case where the tool must interrupt, because the alternative is losing
someone's work:

```
 ⚠ todo.md changed on disk since it was read — not saved.
   r reload (your edit is kept in the input line)   e open $EDITOR
```

Ordinary external changes never reach this screen — inotify re-reads the file and
the list updates silently. This appears only when *we* were about to write on top
of a change we had not seen.

**The selection survives a reload.** It is tracked by task identity, not row
index — if a `git pull` adds four tasks above the one you were looking at, your
cursor does not jump. A tool that loses your place while you are reading it is
not usable as a side pane.

## Empty

The first thing a new user sees, so it has to teach rather than apologise:

```
┌─ ratodo ────────────────────────────────────────── 0 open ─┐
│                                                            │
│   Nothing here yet.                                        │
│                                                            │
│   a          add your first task                           │
│   e          open ~/.config/ratodo/todo.md in $EDITOR      │
│                                                            │
│   Try:  a  then  buy milk @tomorrow #home                  │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

The example line is doing the real work: it shows `@` and `#` in use, which is
faster than any syntax table. It names the file path too, because the promise of
this product is that the file is yours — you should be told where it is on day
one.

## Help — `?`

```
┌─ keys ───────────────────────────────────────┐
│                                              │
│  move     j k   ↑ ↓        g G   top / bottom│
│           ctrl-d ctrl-u    half page         │
│                                              │
│  do       spc   toggle done                  │
│           a o   add        ⏎   edit          │
│           d     delete     u   undo          │
│                                              │
│  view     l     LATER fold / unfold          │
│           z     fold this group              │
│                                              │
│  file     e     $EDITOR    r   reload        │
│                                              │
│  quit     q     ctrl-c                       │
│                                              │
├──────────────────────────────────────────────┤
│            esc or ? to close                 │
└──────────────────────────────────────────────┘
```

This is the one overlay in the product, and it is the only place a popup is the
right answer — you asked for it, and it covers nothing you were mid-way through
reading.

## Keys

| Key | Action | Note |
|---|---|---|
| `j` `k` / `↓` `↑` | move | Both, always. Arrows cost nothing and not everyone has vim hands |
| `g` / `G` | top / bottom | A vim user typing `gg` gets the top on the first `g` and a harmless no-op on the second — so no pending-key state machine is needed |
| `ctrl-d` / `ctrl-u` | half page | |
| `spc` | toggle done | |
| `a` / `o` | add | `o` because a vim user will reach for it to open a new line |
| `⏎` | edit the selected task | |
| `d` | delete | Immediate, with `u` to undo |
| `u` | undo the last change | |
| `h` / `l` | fold / unfold the group under the cursor | Not "fold LATER". In `lf`, `ranger` and `yazi` — which this audience uses daily — `h` and `l` collapse and expand *what is under the cursor*, and that muscle memory arrives with them |
| `z` | the same, as one toggle | `z` is the vim fold prefix |
| `e` | open `$EDITOR` | The escape hatch — a settled decision, see [product.md](product.md#product-decisions) |
| `r` | re-read the file | Rarely needed; inotify does it |
| `?` | key help | |
| `q` / `ctrl-c` | quit | |

### Deliberately unbound

- **`x`** — in vim it deletes a character, in a checklist it means "tick the box".
  Two strong and opposite intuitions on one key, so it gets neither.
- **`esc` in list mode** — does nothing. It must never quit. Someone hitting
  `esc` out of habit should not lose the pane.
- **`:`** — there is no command mode. Pressing it prints `no command mode — ? for
  keys` on the bottom line, which is more useful than silence.
- **`/`** — search arrives in v2. Until then it says so, rather than doing
  nothing: `search comes in v2`. A key that appears broken is worse than one that
  explains itself.
- **`dd`** — `d` is enough, and a pending-operator state is exactly the vim-ness
  we decided not to import.

## Width

This tool lives in a column of a tiling layout, so narrow is the normal case,
not the edge case. Three breakpoints:

**Wide (≥ 60 columns)** — the main screen above.

**Narrow (34–59 columns)** — blank spacer rows and the hint bar shrink first:

```
┌─ ratodo ──────────────── 5 · 1! ─┐
│ OVERDUE ───────────────────────  │
│   ! rotate the backup k…     2d  │
│ TODAY ─────────────────────────  │
│ ▌ ○ pay the invoice       #home  │
│   ○ review the deploy…    16:00  │
│ THIS WEEK ─────────────────────  │
│   ○ book a dentist ap…      Thu  │
└──────────────────────────────────┘
 j k  spc  a  d  ?
```

**Very narrow (< 34 columns)** — the frame is dropped entirely; just rows.

What is given up, in order, as the width shrinks:

1. blank spacer rows between groups
2. tags
3. priority
4. the date shortens (`Thu 09:30` → `Thu` → `2d`)
5. the title is truncated with `…` — **last, and never below 12 characters**

Tags go before dates because a date is actionable and a tag is a filter you do
not have in v1 anyway. The title is sacred: a row you cannot identify is not a
row, it is noise.

Short terminals matter too: under 10 rows, the hint bar collapses to ` ? ` and
group spacing goes to zero.

## No colour, no Nerd Font

Every symbol has an ASCII form and no meaning is carried by colour alone
([design.md](design.md#rules)), so `NO_COLOR=1` on a bare TTY still reads
correctly:

```
┌─ ratodo ────────────────────────────── 5 open · 1 overdue ─┐
│                                                            │
│ OVERDUE ─────────────────────────────────────────────────  │
│   [!] rotate the backup keys                 2d ago  #ops  │
│                                                            │
│ TODAY ───────────────────────────────────────────────────  │
│ > [ ] pay the invoice                               #home  │
│   [ ] review the deploy PR                   16:00  #work  │
│   [x] migrate the server                                   │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

Selection becomes `>`, the symbols become `[ ] [x] [!]`. Note that these are the
*file's* own syntax, which is a pleasant accident: the fallback rendering looks
like the file it is showing.

**The two are separate switches, and they read separate signals.** Colour comes
off `NO_COLOR` and the theme; glyphs come off the **locale** — `$LC_ALL`,
`$LC_CTYPE`, `$LANG`, in that order, and anything that is not UTF-8 gets the
ASCII form. Whether a terminal can draw `○` and whether its user wants colour
are different questions, however often the same terminal answers no to both.

When the ASCII form is chosen it takes **the whole screen** with it: the frame
becomes `+ - |`, the group rules become `-`, and the `—` and `·` in the title
become `-` and `/`. A fallback that leaves box-drawing characters in the border
is not a fallback; it is the same broken screen with tidier checkboxes. The test
for it asserts the entire buffer `is_ascii()` rather than checking three symbols.

## Rules that keep it comfortable in a side pane

These are the ones easy to lose while implementing, and they are the difference
between a tool you leave open and one you close:

1. **The list does not move under you.** Toggling a task done marks it in place;
   it does not jump to the end of its group until the next reload. Watching a row
   you just touched fly somewhere else is disorienting.
2. **No dialog ever covers the list**, except the help overlay you asked for.
3. **Every action is one key.** No prefixes, no confirmations, no pending state.
4. **The selection survives reloads**, tracked by identity rather than index.
5. **0% CPU when idle** ([architecture.md](architecture.md#the-event-loop)).
   A pane that sits open all day must cost nothing.
6. **`ratodo add` still exists.** The fastest capture never opens this UI at all,
   and the TUI is for when it happens to already be open.

## Not in v1

- Mouse support. Not the audience, and it would be the only interaction not
  reachable from the keyboard.
- Sticky group headers while scrolling.
- Reordering tasks from the TUI — that would reorder the user's file, which
  [product.md](product.md) forbids.
- Multi-select and bulk actions.
- A second pane, a preview pane, or a sidebar
  ([design.md](design.md#rules): one layout).
