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
| `⏎ save   esc cancel` | while the input box is open |
| a result message + undo | just after an action |
| a warning | on a write conflict |

One line, four jobs, and **it never changes size**. The list does not move under
you: not when a message arrives, not when a warning does, and not when the input
opens — the input is a box over the middle of the list now, and takes no row from
this line to do it ([decisions.md](decisions.md#reversed)).

While the box is open this line names the two keys that end it and nothing else.
The list keys under it are letters until `esc`, so advertising them would be a
lie.

**The hint bar fills whatever the pane gives it.** `? keys` and `q quit` are
pinned to the end — however little room there is, the way to the rest of the
keymap and the way out both stay — and everything before them goes in until the
next one would not fit. The order is how often a key is reached for:

```
move · done · add · edit · cancel · later · copy
```

So sixty columns, the narrowest pane that still counts as wide, gets through
`⏎ edit`; a little wider brings `d cancel`, and eighty — the width a terminal
opens at unless somebody moved it — brings both `p later` and `y copy`. The
date key is `later` here and `put off` everywhere else, which is the one place
the bar does not use the keymap's own word: at eighty those three columns are
the difference between the newest key being on the bar and being findable only
in `?`. `X`
and `e` are not on it at any width — delete and `$EDITOR` are both a keystroke
away in `?`, and neither is what somebody glancing at a side pane is about to
press. Below the wide threshold the bar drops to bare keys, `j k  spc  a  d  p
?  q`.

This replaced a fixed list of six, which had to be re-argued every time a key
was added and was wrong at both ends: clipped on a narrow pane, and wasting
twenty columns on a wide one.

## Main screen

```
┌─ ratodo ─ 5 open · 1 overdue ───────────── ▰▰▰▱▱▱▱▱ 3/8 ─┐
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
│   ✓ migrate the server                          Mon        │
│   ✗ rewrite the docs                                 #docs │
│                                                            │
│ ## Someday ──────────────────────────────────────────────  │
│   ○ finish chapter 13 of the Rust book               !low  │
│                                                            │
│ LATER (3) ──────────────────────────────────────────── l   │
│                                                            │
└────────────────────────────────────────────────────────────┘

 j k move  spc done  a add  ⏎ edit  d cancel  ? keys  q quit
```

Details that are decisions, not drawing:

- **`▌` is the selection**, in `accent`, with the row on `selection` background.
  A colour alone is not enough — see [design.md](design.md#rules).
- **A finished row is green and a cancelled one is red.** Green is `done`, the
  colour [design.md](design.md#rules) reserved for exactly this and had spent
  only on the progress bar — ticking a task was the one action on this screen
  that said nothing back. Red is `overdue`, shared with a late task: the rule
  widened from "red is only for overdue" to "red is the negative outcome", and
  `✗` against `!` is what still separates them. Three states, three colours,
  and none of them carried by colour alone.
- **A finished row's date is the day it was finished**, not the day it was due —
  that deadline stopped applying the moment it was ticked, and the completion
  date is the one date about it still worth the width. It only ever displaces
  the due date, so the column stays one date wide, and a task ticked before the
  stamp existed still shows its old one. The stamp itself is in the file:
  [format.md](format.md#the-completion-stamp).
- **Group headers get a rule.** In a narrow pane the eye needs a horizontal
  anchor to find where a group starts; a bare word does not give it. Here it
  runs to the right edge; past eighty columns it stops at the title column
  instead — [below](#width).
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
- **`## Someday` is a heading from the user's file**, not one of ours, and it
  **keeps its `##`** to say so. Both are a bold word plus a rule, and without the
  marker nothing on the screen told you whose word it was. The marker is already
  in the file, it costs no second colour and no third level of hierarchy
  ([design.md](design.md#rules)), and it survives the ASCII fallback unchanged.
  Dated groups come first, then the user's own sections in file order.
- **With several lists open the heading says which one**: `## Sprint (work.md)`.
  The dated groups stay mixed — an overdue task is overdue whichever file it is
  in, and that is the whole point of one screen — but two files can hold a
  `## Work`, and merging them would pull one file's tasks up under the other's
  heading. Nothing else on the row changes, so a single list looks exactly as it
  did. `e` opens the file the cursor is in. See [cli.md](cli.md#several-lists).
- The date is relative where that reads better (`2d ago`, `Thu 09:30`) and
  absolute where it does not (`Aug 20`). At this width it is right-aligned, so
  the eye reads down the right edge; past eighty columns it becomes a real
  left-aligned column — [below](#width).
- **The date goes loud only when it presses.** A late task's `3d ago` is in
  `overdue` and a `16:00` due today is in `today` — the same two colours the
  title already uses, so no twelfth theme role — while `Fri 09:30` and `Aug 20`
  stay dim. The date column is where the lateness actually is, and it was the one
  field saying so in grey while the title beside it went red. A finished task's
  date is dim whatever it says: it is neither late nor due.
- **A finished task is never late.** `2d ago` on a ticked line states something
  that stopped being true when the box was ticked, and it contradicts the counts,
  which already leave finished work out of `overdue`. It shows the plain date
  instead (`Aug 8`) — still a fact, and still worth seeing. The task stays in
  `OVERDUE` all the same: membership there is positional, and the list does not
  move under you.
- **`!high` is bold**, in the row's own colour rather than the grey the rest of
  the right-hand fields sit in. It is the one field the user typed to mean
  *urgent*, and saying it back in the same whisper as the date wastes it. Weight,
  not a twelfth theme colour: it reads the same on a bare TTY with `NO_COLOR=1`,
  where a colour would have said nothing at all. `!med` and `!low` stay quiet —
  three loud rows teach nothing about which of them is which — and a ticked task
  is not urgent however it was filed.
- Counts in the title bar are the same numbers a waybar module will show in v4
  ([roadmap.md](roadmap.md)). Same wording, one source.
- **What is finished sits on the right of the title rule**, as eight cells and a
  `3/8`. It appears when the first task is ticked and not before: an empty bar is
  not information, because `5 open` on the left already says you are at the
  start. Below sixty columns the bar gives way and the count stays — `5 · 1! · 3✓`
  — and below thirty-four there is no frame to put it in at all. The two ends are
  reserved: any progress fills a cell, and anything short of finished leaves one
  empty.

## Adding

`a` opens the input in a box over the middle of the list. Nothing scrolls,
nothing is given up, and the box lands where the eye already is:

```
┌─ ratodo ────────────────────────────── 5 open · 1 overdue ─┐
│                                                            │
│ TODAY ───────────────────────────────────────────────────  │
│ ▌ ○ pay the invoice                                 #home  │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ add ▏call the accountant @thu !high                   │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      due Thursday (2026-08-13)  ·  !high              │ │
│  └───────────────────────────────────────────────────────┘ │
│   ○ book a dentist appointment         Thu 09:30  #health  │
│                                                            │
└────────────────────────────────────────────────────────────┘
 ⏎ save   esc cancel
```

**The box, and not the bottom line, because of where the bottom line is.** In a
pane in the corner of a tiling layout that line sits at the bottom edge of the
screen, and glancing down there to type is the head movement this tool exists to
avoid. The box costs the rows it covers for as long as it is open, and gives
them straight back — which is a different thing from the screen changing shape.
See [decisions.md](decisions.md#reversed).

**A rule separates the two halves of the box.** Above it is what you are typing;
below it is what the file will get. Without it the caret looks like something
that could be moved down into the preview, and people try — the box is one
field, not two. The rule goes when the pane is too short to have both, since it
then separates nothing while costing the more useful line.

Its second line is a **live parse preview**, and it is the most valuable ten
lines of code in the TUI. As you type `@thu`, it resolves to a real date in front
of you. That does three things at once: it teaches the syntax without anyone
reading [format.md](format.md), it catches a typo before it reaches the file, and
it proves the shorthand actually did what you meant.

If nothing parses, the preview line stays empty rather than showing an error —
plain text is a perfectly good task.

**One exception, and it is the only place the preview has an opinion instead of a
readout.** `@2026-13-45` is not a date, so the word falls back to being part of
the title — which is right, a word we did not understand belongs to the user —
and it used to do so in silence, right up until the file had it. So an `@` that
was meant as a date and can never be one gets said out loud, in the same colour
the bottom line warns in:

```
│  │ add ▏call the plumber @2026-13-45 #home                │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      @2026-13-45 is not a date  ·  #home              │ │
│  └───────────────────────────────────────────────────────┘ │
```

*Can never be* is the load-bearing half. The preview redraws on every keystroke,
so a warning that fires on anything unresolved fires through all of `@2`, `@20`,
`@202` on the way to a perfectly good `@2026-08-20` — wrong ten times and right
once, which is how people learn to stop reading it. It speaks the moment the word
stops being able to become a date and not before: `@2026-0` is on its way
somewhere, `@2026-13` is not. The fields still follow it — one bad word does not
take the row over.

**The field colours itself as you type**, and it colours by what the parser
*took*: `@thu` and the `09:30` the date took with it go `accent`, `#home` goes
`tag`, `!high` goes bold. A `@notaday` stays plain text, because that is what it
will be in the file — a colour that promises more than the parser delivers
teaches a syntax the format does not have. The preview says *what* was
understood; the colour says *where*, on the words themselves, which is where the
typo is.

Both readings come from one tokenizer: `capture::parts` hands out every word with
what it means, and `capture` builds the task out of the same list. Two readings
of the same text would drift, and the day they did the field would be lying.

`⏎` saves and closes. `esc` cancels and the text is discarded. `ctrl-c` does the
same — in here it is not the quit key. `e` is still the way out to `$EDITOR` for
anything more involved.

`←` and `→` move the caret, `home` and `end` jump to the ends of the line,
`backspace` takes the character before it and `del` the one under it. A field you
can only append to is not a field: fixing a typo four words back should not mean
retyping four words.

The typed line scrolls rather than truncating, and it scrolls with the **caret**
rather than with the end of the line: what you are typing is always on screen,
and a capture box that hides that is not a capture box. An empty line saves
nothing.

While the input is open the keyboard belongs to it. `a`, `d` and `q` are letters
in there, which is how "you can never be in a mode you did not open" is made
true by construction rather than by discipline.

### Putting a date off — `p`

`p` opens the same box, and it is the same box for a reason: the caret, the
scrolling, the rule and the way out are all already right, and a second kind of
prompt would be a second thing to get wrong. What changes is the question.

```
│  ┌───────────────────────────────────────────────────────┐ │
│  │ put off ▏2                                            │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      Wednesday (2026-08-12)                           │ │
│  └───────────────────────────────────────────────────────┘ │
```

What goes in is a length of time, not a sentence, so the field does not colour
it as task syntax — it is `accent` once it resolves to a day and plain until
then, which is the same promise the sentence field makes. The preview answers
the question the box asked: **which day does this land on**. `2w` is exactly the
input nobody works out in their head.

It takes everything `@` takes — `3d`, `1w`, `fri`, `tomorrow`, `2026-09-01` —
plus a **bare number meaning days**, because a box that has just asked *how long*
has no other reading of `2`. That reading is `p`'s alone: `@2` in a sentence is
somebody typing about the number two. An empty box says `how long?  2  3d  1w
fri` rather than nothing, and anything unparseable is refused before the file is
opened.

It moves `@` and nothing else. The time stays — putting "Friday at 09:30" off by
a week is still half past nine — and a task with no date at all gets one, which
is the only sense `p` can make of it. Before this, moving a date meant reopening
the whole line with `⏎` and retyping it, which is a lot of keys for "not today".

### Copying — `y`

The third key into the same box, and for the same reason the second one was: a
task that is nearly a task you already have should be an edit, not a retype.

```
│  ┌───────────────────────────────────────────────────────┐ │
│  │ add ▏water the plants @2026-08-12 #home                │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      due Wednesday (2026-08-12)  ·  #home             │ │
│  └───────────────────────────────────────────────────────┘ │
```

It says `add`, and that is the whole design. `y` fills the box the way `⏎` does
and then means something else by it: what comes back is a **new** task, so the
line it was copied from is not the line `⏎` rewrites. Nothing is written until
`⏎`, and a cancelled box leaves the file exactly as it was — which is the
difference between this and a copy that lands first and is edited afterwards.

Two things do not come with the copy. The **completion stamp** goes, because
`capture` has never heard of `✓2026-08-11` and would have left it sitting in the
new task's title — and copying a finished task to do it again is most of the
point of `y` on a ticked row. The **state** goes with it: a copy of a `[x]` or a
`[-]` comes back as `[ ]`, since a copy is work to do.

The copy is written as a fresh capture, so its fields come out in the canonical
order rather than where they sat in the original line. Round-trip fidelity is a
promise about lines the tool did not touch; this is a line the tool is writing.

**There is no `p` to pair it with, and no register.** `p` has put a date off
since v0.2.0, and the paste half would have had nowhere useful to go: a capture
lands in the capture target regardless of where the cursor is, so "paste here"
and "paste there" would have been the same key doing the same thing. One key
that copies the task under the cursor is the whole of what the two would have
bought. See [decisions.md](decisions.md#settled).

## Editing

`⏎` on a selected task opens the same input, pre-filled with the task's text as
it appears in the file — everything after the checkbox, byte for byte. Same
preview line, same keys.

Saving replaces exactly that: the **prefix survives untouched**, so the
indentation, the bullet the user chose (`-`, `*` or `+`) and whether the box is
ticked all come through a retype unharmed. Nothing else in the file is written,
which is the invariant in
[architecture.md](architecture.md#round-trip-fidelity). Retyping a line without
changing it writes nothing at all and does not spend the undo.

## Deleting — no confirmation dialog

```
 deleted “rotate the backup keys”                        u undo
```

A confirmation prompt is the wrong trade here. It stops the flow on **every**
delete to protect against the rare mistaken one. Undo inverts that: deleting
costs one key, and the mistake costs one more. The shift on `X` does the rest —
it is not a dialog, but it is not a key you land on by accident either.

`u` undoes the last change in this session — a delete, a toggle, or an edit. The
`.bak` file is the backstop underneath it. See
[architecture.md](architecture.md#concurrent-editing).

It is **one level, and it keeps the whole document** rather than inverting the
change that was made. An undo built from an inverse operation is an undo that
can be subtly wrong about what it is putting back, and a few kilobytes is not a
reason to accept that. A write that gets refused does not spend it either: a
refusal changes nothing, and that has to include the undo slot.

## Write conflict

The one case where the tool must interrupt, because the alternative is losing
someone's work:

```
 ⚠ changed on disk - nothing was written.  r reload
```

Ordinary external changes never reach this screen — inotify re-reads the file and
the list updates silently. This appears only when *we* were about to write on top
of a change we had not seen.

A refusal that arrives while the input is open re-reads the file itself and hands
the sentence back to the field, so the next `⏎` goes against the list as it now
is. Nothing is merged and nothing is overwritten; the typed text is kept, which
is the promise. `r` is not needed in there, and could not be typed anyway.

**The selection survives a reload.** It is tracked by task identity, not row
index — if a `git pull` adds four tasks above the one you were looking at, your
cursor does not jump. A tool that loses your place while you are reading it is
not usable as a side pane.

## Empty

The first thing a new user sees, so it has to teach rather than apologise:

```
┌ ratodo — 0 open · 0 overdue ───────────────────────────────┐
│                                                            │
│  Nothing here yet.                                         │
│                                                            │
│  a          add your first task                            │
│  e          open ~/.config/ratodo/todo.md in $EDITOR       │
│                                                            │
│  ┌──────────────────────────────────────────────┐          │
│  │ add ▏buy milk @tomorrow #home                │          │
│  ├──────────────────────────────────────────────┤          │
│  │      due tomorrow (2026-08-11)  ·  #home     │          │
│  └──────────────────────────────────────────────┘          │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

The example is doing the real work: it shows `@` and `#` in use, which is faster
than any syntax table. It sits in the box `a` actually opens, drawn by the same
code — so the line under it has already resolved `@tomorrow` into a date before
anything has been typed, which is the part of the syntax worth teaching. The
border is the frame's colour, not the accent: the accent border marks the box
that has the keyboard, and this one is a picture of it.

Below eleven rows the box does not fit and the example goes back to being a line —
`Try:  a  then  buy milk @tomorrow #home`. It is the part that teaches, so it is
the last thing a short pane loses.

The screen names the file path too, because the promise of this product is that
the file is yours — you should be told where it is on day one.

## Help — `?`

```
┌ keys ──────────────────────────────────┐
│  j k  ↓ ↑        move                  │
│  g G             top / bottom          │
│  ctrl-d ctrl-u   half page             │
│  spc             toggle done           │
│  a o  ⏎          add / edit            │
│  X  u            delete / undo         │
│  d  p            cancel / put off      │
│  h l  z          fold this group       │
│  e  r            $EDITOR / re-read     │
│  q  ctrl-c       quit                  │
└────────── esc or ? to close ───────────┘
```

This is the one overlay in the product, and it is the only place a popup is the
right answer — you asked for it, and it covers nothing you were mid-way through
reading.

The way out is on the bottom border, where it costs no row. Ten keys plus two of
border is twelve, and twelve is what fits a fourteen-row pane: the last line of a
help screen must never be the one that falls off, least of all when it is quit.
Grouping the keys into blocks with blank lines between them would cost four rows
and exactly that.

Only keys that are built are listed — which is why `:` and `/` are **not** here
any more. They do nothing, pressing either answers in the status line, and that
is where they teach anything at all; the row they were costing is what keeps
`d  p` inside the same twelve.

## Keys

| Key | Action | Note |
|---|---|---|
| `j` `k` / `↓` `↑` | move | Both, always. Arrows cost nothing and not everyone has vim hands |
| `g` / `G` | top / bottom | A vim user typing `gg` gets the top on the first `g` and a harmless no-op on the second — so no pending-key state machine is needed |
| `ctrl-d` / `ctrl-u` | half page | |
| `spc` | toggle done | |
| `a` / `o` | add | `o` because a vim user will reach for it to open a new line |
| `⏎` | edit the selected task | |
| `d` | cancel — decided against | `- [-]` in the file; `d` again takes it back, the same way `spc` does. Out of the counts, never overdue, not exported. See [format.md](format.md#the-three-states) |
| `u` | undo the last change | |
| `X` | delete | Immediate, with `u` to undo. **Capital**: the one key that takes a line out of the file is the one that asks for shift, and see the note on `x` under [Deliberately unbound](#deliberately-unbound) |
| `p` | put the date off | Opens the input box to ask how long — `2`, `3d`, `1w`, `fri` — and moves `@` alone. Retyping the whole line through `⏎` was the only way to move a date, which is a lot of keys for "not today" |
| `y` | copy the selected task | Opens the input box pre-filled with it, as a **new** task — edit it and `⏎` saves a second one. The vim yank the hand reaches for, and there is no `p` to pair it with. See [Copying — `y`](#copying--y) |
| `h` / `l` | fold / unfold the group under the cursor | Not "fold LATER". In `lf`, `ranger` and `yazi` — which this audience uses daily — `h` and `l` collapse and expand *what is under the cursor*, and that muscle memory arrives with them |
| `z` | the same, as one toggle | `z` is the vim fold prefix |
| `e` | open `$EDITOR` | The escape hatch — a settled decision, see [product.md](product.md#product-decisions) |
| `r` | re-read the file | Rarely needed; inotify does it |
| `?` | key help | |
| `q` / `ctrl-c` | quit | |

### Deliberately unbound

- **`x`** — in vim it deletes a character, in a checklist it means "tick the box".
  Two strong and opposite intuitions on one key, so it gets neither. `X` takes
  the vim half a shift away: the shift is the point, since delete is the only
  key that takes a line out of the file, and a bare letter next to `j` and `k`
  is too cheap for that.
- **`esc` in list mode** — does nothing. It must never quit. Someone hitting
  `esc` out of habit should not lose the pane.
- **`:`** — there is no command mode. Pressing it prints `no command mode — ? for
  keys` on the bottom line, which is more useful than silence.
- **`/`** — search arrives in v2. Until then it says so, rather than doing
  nothing: `search comes in v2`. A key that appears broken is worse than one that
  explains itself.
- **`dd`** — one `d` is enough for what `d` does, and a pending-operator state is
  exactly the vim-ness we decided not to import.

## Width

This tool lives in a column of a tiling layout, so narrow is the normal case,
not the edge case. Four breakpoints:

**Roomy (≥ 80 columns)** — the right-hand fields become **columns**. The date,
the priority and the tags each start in the same place on every row, and the
title column is as wide as the widest title in the list:

```
┌ ratodo — 5 open · 1 overdue ─────────────────────────────────── ▰▱▱▱▱▱▱▱ 1/6 ┐
│  OVERDUE ───────────────────────────                                         │
│  ! rotate the backup keys              2d ago           #ops                 │
│                                                                              │
│  TODAY ─────────────────────────────                                         │
│  ○ pay the invoice                     today            #home                │
│  ○ review the deploy PR                16:00            #work                │
│                                                                              │
│  THIS WEEK ─────────────────────────                                         │
│  ○ book a dentist appointment          Thu 09:30        #health              │
│  ✓ migrate the server                  Thu                                   │
│                                                                              │
│  ## Someday ────────────────────────                                         │
│▌ ○ finish chapter 13 of the Rust book             !low                       │
└──────────────────────────────────────────────────────────────────────────────┘
```

Why this is a breakpoint and not simply the layout: **a column costs every row
its width, whether or not that row uses it.** One `!low` in the whole list buys
a priority column that every other row then carries as blank space. Past eighty
columns there is room to spend; below it the packed right-aligned block fits
more onto the row, and fitting more on wins when there is not much row.

Three things follow from the columns, and they are decisions:

- **The group rule stops at the title column** instead of running to the right
  edge. At this width a rule to the edge is the heaviest thing on the screen and
  says nothing; one that ends where the titles end draws the column instead.
- **The title column is measured over the whole list, not the visible rows.** A
  column that resizes as you scroll past a long title is not a column.
- **Tags get no column of their own.** They are last and ragged, so nothing
  lines up after them, and reserving the widest row's worth would cut every
  title to pay for tags most rows do not have. They spend what is left of the
  row, and a tag that does not fit is dropped whole — `#hea…` is not a filter,
  it is a riddle.

**Wide (60–79 columns)** — the main screen above: no columns, the right-hand
fields packed against the right edge.

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

1. the columns — the right-hand fields pack against the right edge again
2. blank spacer rows between groups
3. tags
4. priority
5. the date shortens (`Thu 09:30` → `Thu` → `2d`)
6. the title is truncated with `…` — **last, and never below 12 characters**

Inside the columns the same order applies to a single row that runs out of
width: its tags go before its title is cut.

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

**The whole screen includes the help overlay**, which is where it used to stop:
`↓ ↑` and `⏎` were written into the key list as literals, and the buffer test
never caught it because it does not open the overlay. It is the one screen
somebody opens *because* they are lost, so a key it cannot draw is the worst
place to put one. The arrows become `down up`, `⏎` becomes `ret` as it does
everywhere else, and the test now opens the overlay, the input and its preview
together. The other two that escaped: the `…` on a cut title, now `...` — three
columns, held back rather than assumed — and the `·` between the fields of the
input preview, which becomes `/` like the one in the title bar. The messages that
carried an `—` were reworded rather than made switchable; the bottom line is the
one place a warning has to be readable however the terminal is set up.

## Rules that keep it comfortable in a side pane

These are the ones easy to lose while implementing, and they are the difference
between a tool you leave open and one you close:

1. **The list does not move under you.** Toggling a task done marks it in place;
   it does not jump to the end of its group until the next reload. Watching a row
   you just touched fly somewhere else is disorienting.
2. **Nothing covers the list that you did not open**: the help overlay and the
   input box, both of which `esc` closes, and neither of which moves a row.
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
