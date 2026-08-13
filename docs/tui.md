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

**Two screens is not a third mode.** `?` covers the list and [`s`](#stats--s)
replaces it, and both are closed by the key that opened them or by `esc`. The
keymap does not change under either — the list's keys simply do not act while a
screen you opened on purpose is up, which is the same promise as "you can never
be in a mode you did not open" read from the other side:

```
  MAIN ─┬─ a ──→  NEW TASK    esc / ⏎
        ├─ ⏎ ──→  EDIT TASK   esc / ⏎
        ├─ s ──→  STATS ──┬─ 1 week
        │                 ├─ 2 month   s / esc
        │                 └─ 3 year
        └─ ? ──→  KEYS       esc / ?
```

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
press. Below the wide threshold the bar drops to bare keys, `[j k] [spc] [a] [d] [p]
[?] [q]`.

**The keys look like keys.** `[a] add`, not `a add` — brackets, so it reads as a
keycap the way lazygit's and k9s's bars do and survives `NO_COLOR`, where a key
that is only a key because it is mauve is not one. They cost two columns an
entry against a bar that is a greedy fill, so the separator went from two spaces
to one — the brackets already tell one entry from the next. What that buys and
what it costs was measured rather than assumed: at eighty columns `[p] later`
still fits and `[y] copy` does not, and `[y] copy` comes back at eighty-eight.

This replaced a fixed list of six, which had to be re-argued every time a key
was added and was wrong at both ends: clipped on a narrow pane, and wasting
twenty columns on a wide one.

## Main screen

```
╭ ratodo ─────────────────────── Wednesday, 12 August 2026 ╮
│                                                            │
│    1          3        8       3/11         ▂▅▃█▆▂▁        │
│    OVERDUE    TODAY    OPEN    DONE · 27%   MON — SUN      │
│                                                            │
├────────────────────────────────────────────────────────────┤
│  ╭─ OVERDUE · 1 ────────────────────────────────────────╮  │
│▌ │ ! rotate the backup keys                 2d ago  #ops│  │
│  ╰──────────────────────────────────────────────────────╯  │
│  ╭─ TODAY · 2 ──────────────────────────────────────────╮  │
│  │ ○ pay the invoice                               #home│  │
│  │ ○ review the deploy PR                   16:00  #work│  │
│  ╰──────────────────────────────────────────────────────╯  │
│  ╭─ THIS WEEK · 3 ──────────────────────────────────────╮  │
│  │ ○ book a dentist appointment       Thu 09:30  #health│  │
│  │ ✓ migrate the server                           Mon   │  │
│  │ ✗ rewrite the docs                              #docs│  │
│  ╰──────────────────────────────────────────────────────╯  │
│  ╭─ ## Someday · 1 ─────────────────────────────────────╮  │
│  │ ○ finish chapter 13 of the Rust book             !low│  │
│  ╰──────────────────────────────────────────────────────╯  │
│  LATER · 3 ───────────────────────────────────────── l     │
├────────────────────────────────────────────────────────────┤
│  - [ ] rotate the backup keys @2026-08-10 #ops             │
╰────────────────────────────────────────────────────────────╯

 [j k] move [spc] done [a] add [⏎] edit [d] cancel [?] keys [q] quit
```

Details that are decisions, not drawing:

- **The band at the top**, five rows, and the only thing on the screen that says
  the tool has a memory. The date spelled out — the first thing a todo list
  should say and the one thing this screen never said — then stat tiles as a big
  number over a small label, and a seven-cell sparkline off the `✓` completion
  stamps for the current week, Monday first.

  Every number in it is one `ratodo status` already computes, so the band adds
  no state and no new data. **The band owns the counts while it is drawn**, which
  is why the title bar spends its right-hand side on the date instead; when the
  band goes, the counts and the progress bar come back to the title rather than
  disappearing.

  The sparkline has **no ASCII form**, and that is a decision rather than an
  omission: seven cells of `#` and `-` is not a bar chart, it is a row of
  punctuation the reader has to be told is a chart. It goes the way the columns
  go below eighty. A week with nothing finished in it draws none either.
- **The footer: the selected task's line, from the file, raw.** One row, and it
  is the row that says *this is a file and this is your line in it* on the screen
  somebody stares at all day. It is also the honest answer to "did the tool
  understand what I typed", with no box open and nothing to press. A task the
  tool has changed this session shows the line that **will** be written — a
  footer that showed the old bytes after a tick would be worse than none.

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
- **A group is a box, and the group header is its top edge.** In a narrow pane
  the eye needs a horizontal anchor to find where a group starts; a bare word
  does not give it. The rule used to stop in mid-air and the blank row after the
  group closed nothing, so the box is where both of those go: the heading is the
  top edge, the blank row is the bottom edge, and past `COLUMNS_AT` the column
  dividers meet them at `┬` and `┴`. Nothing floats — every stroke starts at a
  corner and ends at one. Same row count at 60 columns and up, one row per group
  more between 34 and 59, five columns of row, and nothing at all below 34 where
  the frame goes too. It is drawn in `border` and the group's name keeps its
  accent: the box is scenery, and scenery does not get the accent
  ([decisions.md](decisions.md#the-blank-row-between-groups-becomes-the-groups-bottom-edge-2026-08-12)).

  **A group with no name still gets one**, with nothing written on its top edge.
  That is the run of tasks above a file's first heading — no "(no section)"
  nobody wrote, and no rows left floating beside the boxed ones either.
- **A folded group is a bare rule, not a box.** An empty two-row box to say a
  group is closed is exactly backwards: the difference between a container and
  a line *is* the open/closed signal.
- **Every heading carries its count** — `TODAY · 2`. `LATER (3)` already did
  this when it was folded, and there was never a reason the open ones stayed
  silent; a group with no *name* gets no count either, because `· 2` on its own
  says nothing that counting the rows under it does not.
- **`LATER · 3` stays collapsed** and shows its count and its key. A collapsed
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
- **The column says what the heading does not**, amended 2026-08-12. `today`
  inside a group headed `TODAY` was spending nine characters saying where it
  already was:

  | Group | The heading says | The column says |
  |---|---|---|
  | `OVERDUE` | that it is late | *how* late — `2d ago` |
  | `TODAY` | the day | the **time, or nothing** — `16:00` |
  | `THIS WEEK` | the week | which day — `Fri 09:30` |
  | `## Someday` | nothing about dates | the date — `Sep 20` |

  `TODAY` is the group most people look at most often and it was the one where
  the column was pure noise. Emptied, the rows that *do* have a time stand out,
  which is the only thing about a task due today still worth reading — and a
  group where nothing is timed now spends no width on the column at all.

  A **finished** row is unaffected: its column is the day it was finished, which
  no heading says.
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
- **The priority has a colour of its own, in two weights.** `!high` is
  `priority` and bold, `!med` is `priority`, `!low` stays in the grey the rest of
  the right-hand fields sit in. It is the field the user typed to mean *how much
  this matters*, and saying all three back in the same whisper wastes it. Its
  **own** role and not a borrowed one, because every other colour is already
  answering a question — red the negative outcome, green finished, orange today,
  blue a tag, mauve the tool's own voice
  ([design.md](design.md#what-each-colour-means)). It borrows the row's colour
  from nobody either: on a late row the red date and the priority beside it stay
  two different things, which is the one row where they most need telling apart.
  The weight is what survives `NO_COLOR=1`, where a colour says nothing at all.
  **A ticked or cancelled row keeps it**, because the priority is a fact about
  the task and not a claim about what is left to do — the `✓` and the `✗` are
  what answer that, and a finished `!med` going grey beside an open `!high` read
  as the colour having failed.
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

**`a` opens a form. `o` opens the one-line box.** Two doors into the same file,
and both were already bound to the same one, so giving each a behaviour costs a
key nobody has to learn: the vim hand that reaches for `o` to open a new line is
the one that wanted the fast path anyway.

The form is [below](#the-form--a); the box is the rest of this section, and it is
still what `o`, `p` and `y` open at every width — and what `a` opens when the
pane is under **15 rows or 40 columns**. A form that half-fits is worse than a
box that always fits.

### The one-line box — `o`

It opens over the middle of the list. Nothing scrolls, nothing is given up, and
the box lands where the eye already is:

```
┌─ ratodo ────────────────────────────── 5 open · 1 overdue ─┐
│                                                            │
│ TODAY ───────────────────────────────────────────────────  │
│ ▌ ○ pay the invoice                                 #home  │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ ADD ▏call the accountant @thu !high                   │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      due Thursday (2026-08-13)  ·  !high              │ │
│  └───────────────────────────────────────────────────────┘ │
│   ○ book a dentist appointment         Thu 09:30  #health  │
│                                                            │
└────────────────────────────────────────────────────────────┘
 ⏎ save   esc cancel
```

**The box opens with today's date already in it, behind the caret:**

```
│  │ ADD ▏▏ @2026-08-12                                    │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      due today (2026-08-12)                           │ │
```

Today is the date a new task has more often than every other date put together,
and the box is the cheapest place in the tool to change one — `tab` and the
arrows, or four keystrokes over the digits. **Behind** the caret because the date
is the field the tool guessed and the title is the one you came to type: it goes
first, where the written line has it and where the row on the screen reads it.

**A date you type takes that one's place.** `capture` gives the line to the
first `@`, and the first one here is the one nobody typed — so the shorthand
every example above uses still wins, and the opening date does not end up
stranded in the title. It goes on the `@` keystroke and it goes once: a second
`@` is a word in a title, and `bob@work` takes nothing with it.

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

**A box you have emptied says what can go in it**, by example rather than by
name — backspace over the date it opened with, and the hint is there:

```
│  ┌───────────────────────────────────────────────────────┐ │
│  │ ADD ▏                                                 │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      @thu #home !high $list                           │ │
│  └───────────────────────────────────────────────────────┘ │
```

It goes the moment there is anything to report, and `$list` appears only when
there is more than one list to address. Twenty-two columns, so it survives the
34-column pane — which is also the reason the box is one field and not five
labelled ones. That arithmetic is in
[decisions.md](decisions.md#settled).

**One exception, and it is the only place the preview has an opinion instead of a
readout.** `@2026-13-45` is not a date, so the word falls back to being part of
the title — which is right, a word we did not understand belongs to the user —
and it used to do so in silence, right up until the file had it. So an `@` that
was meant as a date and can never be one gets said out loud, in the same colour
the bottom line warns in:

```
│  │ ADD ▏call the plumber @2026-13-45 #home                │ │
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
`tag`, and `!high` and `!med` take `priority` in the same two weights the row
gives them. A `@notaday` stays plain text, because that is what it
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

### Which list — `$work`

With several lists open, `a` still means `todo.md` — but a `$work` in the
sentence sends that one capture to `work.md`, and the preview says so before
`⏎` does anything:

```
│  ┌───────────────────────────────────────────────────────┐ │
│  │ ADD ▏call the accountant @thu $work                   │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      → work.md  ·  due Thursday (2026-08-13)          │ │
│  └───────────────────────────────────────────────────────┘ │
```

The word never reaches the file — it is the address on the envelope, not part of
the task — so the line saved is `- [ ] call the accountant @2026-08-13`. It is
the fourth sigil and it comes out of the same `capture::parts` as the other
three.

A `$` naming no open list is the preview's **second** opinion, in the same colour
as the first, and `⏎` refuses rather than creating the file:

```
│  │      no list wrok.md  ·  due Thursday (2026-08-13)     │ │
```

It waits, for the same reason the date warning waits. `$w`, `$wo` and `$wor` are
all on their way to `$work`, and a line that says *no list w.md* four times
before saying the right thing once is a line people learn to skip. It speaks
when what has been typed is no longer the start of any open list. Staying quiet
is not agreeing: `⏎` on a half-typed `$w` is refused all the same, because the
refusal costs a keystroke and a wrong file costs a task.

Unlike `@notaday`, a well-formed `$wrok` keeps the accent on the typed line while
the preview says it goes nowhere. The colour is a promise about the *word* — the
tokenizer did take this one as a list — and whether that list exists is a
question about the directory, which is what the line below answers. Rejecting it
in the tokenizer instead would mean handing `capture::parts` the contents of a
directory, and it is the one function in the program with no world in it.

`$` addresses a **capture**. A `$` on `⏎` is refused with a sentence saying so:
what an edit writes is the file its task came from, and moving a line between
two lists is two writes against two mtimes. The rest — why the default is fixed
at all, why `$50` is money, and why the first `$` wins — is in
[decisions.md](decisions.md#a-capture-always-goes-to-todomd--work-picks-the-list-2026-08-11).

### The date field — `tab`

`@thu` and `@3d` are how a date gets typed, and they are why the box has no
picker in the way. But `@2026-13-45` is a date the text box takes and the
preview can only *say* is wrong, and a keyboard that stutters is how it gets
typed. `tab` opens a field where that date does not exist:

```
│  ┌───────────────────────────────────────────────────────┐ │
│  │ ADD ▏renew the passport                               │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      [11] 08  2026  ← → ↓ ↑                           │ │
│  └───────────────────────────────────────────────────────┘ │
 ⏎ date   esc back
```

`↑` `↓` change the part in brackets and `←` `→` move between the three. **The
row names both pairs**, and drops `← →` first when it runs out of room: the
brackets say which part has the cursor and nothing in them says how to move it,
so a key named nowhere is a key found by accident.

Digits fill the parts in order — `13082026` is the 13th of August, eight keystrokes and no
arrows, because a part that cannot take another digit hands the cursor on by
itself. `⏎` writes the day into the line as one `@YYYY-MM-DD` word and gives the
keyboard back to the text; `esc` closes the field and leaves the line alone.

**The brackets, not just the colour.** Which of the three the arrows are
pointing at is the one thing this row has to say, and `NO_COLOR` must not take
it. They are also always there and always around exactly one part, so the row is
the same width wherever the cursor is — a strip that shifted sideways on every
`←` would be a row nobody can read.

**It cannot produce a day the calendar does not have.** The day is clamped to
the month it is in, so arrowing the 31st of January into February gives the 28th
— or the 29th, in a leap year, because the length of a month is asked of the
calendar rather than of a table. A month of `13` is unreachable: the `1` is the
month, the `3` cannot join it, so the `3` starts the year. That is the whole
point of the field, and it is the half of the `@2026-13-45` complaint the
warning line under the box cannot do.

**It opens on the date the line already has** — through `capture`, so `@thu`
resolves first — and on today when there is none. A field that opened on the 1st
of January would make you arrow back to where you already were.

It takes the same `tab` in the `p` box, where it writes the bare date rather
than an `@` word: that is the one form `p` accepts past
[its year horizon](#putting-a-date-off--p).

### Putting a date off — `p`

`p` opens the same box, and it is the same box for a reason: the caret, the
scrolling, the rule and the way out are all already right, and a second kind of
prompt would be a second thing to get wrong. What changes is the question.

```
│  ┌───────────────────────────────────────────────────────┐ │
│  │ PUT OFF ▏2                                            │ │
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

**A length of time stops at a year.** A keyboard that stutters turns `22` into
`2222`, and the difference between those two is twenty-two days and six years —
both perfectly good arithmetic, which is why the file used to take the second one
without a word. Past 365 days the answer is refused, in every form that can carry
a doubled digit: `2222`, `2222d`, `222w`. The way to move a task past a year is
to **write the date out** — `2032-09-10` is a day somebody meant and is not
capped, which is also why the refusal names it:

```
 ⚠ try 2, 3d, 1w, fri - a year at most, or write the date
```

The horizon is on `p` alone. `@` names a day, and a day you name is yours.

It moves `@` and nothing else. The time stays — putting "Friday at 09:30" off by
a week is still half past nine — and a task with no date at all gets one, which
is the only sense `p` can make of it. Before this, moving a date meant reopening
the whole line with `⏎` and retyping it, which is a lot of keys for "not today".

### Copying — `y`

The third key into the same box, and for the same reason the second one was: a
task that is nearly a task you already have should be an edit, not a retype.

```
│  ┌───────────────────────────────────────────────────────┐ │
│  │ COPY ▏water the plants @2026-08-12 #home              │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │      due Wednesday (2026-08-12) │ #home               │ │
│  └───────────────────────────────────────────────────────┘ │
```

**It says `COPY`, in the accent**, and that label is the whole design. `y` fills
the box the way `⏎` does and then means something else by it: what comes back is
a **new** task, so the line it was copied from is not the line `⏎` rewrites.
The label is the only thing on the screen that says so, which is why it is the
one of the four that takes a colour — the other three are bold and full
brightness, and this one is bold and the accent. It said `add` until 2026-08-11,
and a box filled from the row under the cursor that says `ADD` like every other
box is a box nobody reads the first word of. Nothing is written until `⏎`, and a
cancelled box leaves the file exactly as it was — which is the difference
between this and a copy that lands first and is edited afterwards.

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

### The form — `a`

```
╭───────────────── NEW TASK ─────────────────╮
│  What needs to be done?                    │
│  ╭────────────────────────────────────╮    │
│▌ │ call the accountant▏                │    │
│  ╰────────────────────────────────────╯    │
│                                            │
│  Date / Time [ 2026-08-12▏]  [ 09:30  ]    │
│  Priority    ◉ none  ○ high  ○ med  ○ low  │
│  Tags        [ #home #work            ]    │
│  List        ◉ todo.md  ○ work.md          │
│                                            │
│  ──────────────────────────────────────    │
│  PREVIEW                                   │
│  - [ ] call the accountant @2026-08-12 #home │
│                                            │
│  [ esc cancel ]        [ ⏎ create task ]   │
╰─ tab · next field · shift-tab · back ──────╯
```

**The line is the model, and that is the whole design.** There is one string —
the line the file will get — and every one of the six controls is a *view* of it,
the question field included. Each reads `capture::parts` to know what it is
holding and writes back by replacing the span that tokenizer claimed. One string,
one tokenizer, one truth — which is what lets the form exist at all, since the
labelled-field box was rejected for needing either a join back into a line or a
second parser ([decisions.md](decisions.md#reversed)).

**The question field holds the sentence and nothing else.** `@fri`, `14:00`,
`!med` and `#home` have boxes of their own, so the one place the whole line
appears is the `PREVIEW` — which is what makes the preview the form's conclusion
rather than a second copy of the field above it.

- **Six fields and no seventh:** title, date, time, tags, priority and which
  list — exactly the six a one-line format carries. No Description, no Project,
  no Section picker.
- **`PREVIEW`, with its own label and its own rule above it.** The difference
  between a form that happens to show a line and a form whose *conclusion* is a
  line. A form that saves into a database can tell you nothing; this one saves
  into your file, so the file is the last word on the screen.
- **Typing still works, and it is the same words moving.** `@thu`, `#home` and
  `!high` typed into the question field go into the line, `parts` claims them,
  and they are in their own boxes before the keystroke is over — they leave the
  sentence as soon as the field gives up the keyboard. The day there are two
  tokenizers is the day the form and the box disagree about what gets written.
- **`a` opens on today's date, in the date box.** That is where the one-line box
  had to hide it behind the caret, and it is a better place for it: it can be
  seen and changed without deleting anything. A date typed into the sentence
  still takes its place, once and only while the line still holds ours untouched.
- **Editing the sentence rewrites the title run and nothing else.** A line whose
  title words are interleaved with tokens — `rotate #ops the keys` — gets its
  title put back where the first of those words stood, with the tokens keeping
  their own order. Everything the edit did not reach still keeps its bytes.
- **The date and its time share a row**, because a date and its time are one
  thought. Below about fifty columns there is not room for both boxes and they
  take a row each, which is the same fallback everything else on this screen
  has.
- **The first character typed into the date or time replaces what is in it.**
  A box with a caret in it invites typing, and typing used to *append*: `thu`
  onto `2026-08-13` came out as `2026-08-13thu`, which is not a day, so the only
  way to change the date by hand was to empty it first. Backspace, delete or an
  arrow says *edit this one* instead, and what is there stays. Tags are the
  exception and keep what they hold — a set is something you add to.
- **The three rows carry the caret the sentence field does.** `←` `→` `home`
  `end` and `delete` work in the date, the time and the tags, because a field
  you can only backspace out of is not a field.
- **A row writes a token or it writes nothing.** `0930` is a time on its way to
  being one and `2026-08-1thu` is not a day at all; both used to go into the
  line as words, which put the first in the title and the second in the file.
  The rows keep what is being typed and the line waits for it to mean
  something — the same tokenizer answers both, which is the whole design. What
  the `PREVIEW` shows is what the file gets.
- **The date is typed, not picked off a row of `today / tomorrow / thu`.** A
  smaller screen and a bigger vocabulary: the field takes anything `capture`
  resolves, so `thu`, `3d` and `2026-08-14` all work and the form invents no
  fixed set of days. The `PREVIEW` is what says which day `thu` came out as,
  which is the same live parse the one-line box has always had. Emptying it
  takes the time with it — a time with no date is not a field the file can keep.
- **`↑` `↓` on the date open the three-part picker** the box already has, and it
  takes the row over while it is up. `tab` is next-field in here, so the arrows
  are its door — and **the first press only opens it.** A key that edits the date
  on its way to showing you the date is one you have to notice and undo, and a
  picker is the wrong place to be surprised; the second press steps it.
- **Radios are `◉` against `○`**, and `(o)` against `( )` in ASCII: a difference
  in *shape*, so the choice survives `NO_COLOR=1` and the fallback. `←` and `→`
  move one, and it applies at once — the preview is the confirmation and it is
  already on the screen.
- **`▌` sits beside the row that has the keyboard, and the caret `▏` beside the
  control.** Two marks because one row can hold two controls; both are shapes,
  so both survive `NO_COLOR`. Same marker and same colour as the selected row on
  the list.
- **The buttons carry their key.** `[ ⏎ create task ]` is both the button and the
  keybinding, so it is honest on a keyboard and still looks like a button. In the
  narrowest pane the form is drawn in it gives up its noun — `[ ⏎ create ]` —
  because the row has nothing else to truncate against, and a button wide enough
  to reach the frame loses its own `]`.
- **`tab` is *next field* here and `shift-tab` is the way back**, and the border
  names both — one key, one job per screen. The back key is the one nobody finds
  by pressing keys, so a form that only advertises `tab` gets walked forwards
  eight times to go back one; below about forty columns the border has no room
  for it and names `tab` alone. Inside the one-line box `tab` is still the date
  picker, because there are no fields there to walk.
- **`Time` is not in the tab order without a date.** The format cannot hold a
  time without one, so a row that accepted one would be a field the file cannot
  keep.
- **`List` appears only when more than one list is open**, exactly as `$list`
  does.
- **No Section picker**, and this is worth being explicit about: a capture lands
  below everything, outside every `##`. Letting the form choose a section means
  teaching the writer to *insert* into the middle of the file rather than append
  to the end — a change to the write path, which is the one place fidelity is won
  or lost. It can be done; it must not be smuggled in as a dropdown.

**What happens to the two-second capture:** nothing. The fast path was never the
TUI — [product.md](product.md) says `ratodo add 'pay the invoice @tomorrow'`
writes and exits, and that *"the second one is the reason this product exists"*.
`a` is for when the pane is already open and you are already looking at it, and
that moment can afford a form. `o` is there for when it cannot.

## Editing

`⏎` on a selected task opens the [form](#the-form--a), pre-filled with the task's
text as it appears in the file — everything after the checkbox, byte for byte.
The same form, a different title and a different button, which is all that
separates the two screens because it is all that separates the two jobs. Under
15 rows or 40 columns it opens the one-line box instead, exactly as `a` does.

**Saving writes back the bytes the field was left holding**, and this is worth
being precise about, because until 2026-08-12 it did not:

- The **prefix survives untouched** — the indentation, the bullet the user chose
  (`-`, `*` or `+`), whether the box is ticked, and the gap between the checkbox
  and the first word, however many spaces that is.
- **So does everything the edit did not reach.** The form replaces the one span
  `capture::parts` claimed and leaves every other byte alone, so a line arranged
  `#ops rotate the keys !high @2026-08-10` comes back with its tags still first
  and its date still last after a priority change. It used to come back in the
  tool's canonical order, having been re-rendered through `capture` on the way —
  which meant *editing one word reformatted the line*. A reader could not tell
  that from the sentence this section used to end with, so it is spelled out
  here rather than left as an implication of the invariant in
  [architecture.md](architecture.md#round-trip-fidelity).
- **An edit that changed nothing writes nothing at all** and does not spend the
  undo. Compared against the untrimmed field, so a line with trailing spaces is
  not "changed" by being looked at.
- `$list` is refused rather than swallowed: it addresses a *capture*, and moving
  a line between two files is two writes against two mtimes, which is not this
  key.

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
│  │ ADD ▏buy milk @tomorrow #home                │          │
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
╭ keys ──────────────────────────────────╮
│  j k  ↓ ↑        move                  │
│  g G             top / bottom          │
│  ctrl-d ctrl-u   half page             │
│  spc             toggle done           │
│  a o  ⏎  y       add / edit / copy     │
│  X  u            delete / undo         │
│  d  p            cancel / put off      │
│  h l  z          fold this group       │
│  s               stats                 │
│  e  r            $EDITOR / re-read     │
│  q  ctrl-c       quit                  │
╰────────── esc or ? to close ───────────╯
```

This is the one overlay in the product, and it is the only place a popup is the
right answer — you asked for it, and it covers nothing you were mid-way through
reading.

The way out is on the bottom border, where it costs no row. Eleven keys plus two
of border is thirteen, and a fourteen-row pane is what has to hold it: the last
line of a help screen must never be the one that falls off, least of all when it
is quit. That leaves **one row spare**, which is why `s` got a line of its own
rather than doubling up with `e  r`. Grouping the keys into blocks with blank
lines between them would cost four rows and exactly that.

Only keys that are built are listed — which is why `:` and `/` are **not** here
any more. They do nothing, pressing either answers in the status line, and that
is where they teach anything at all; the row they were costing is what keeps
`d  p` inside the same twelve.

## Stats — `s`

The second screen, and the answer to "there is only one screen". Every number on
it is already in the file: `✓2026-08-11` completion stamps are what it reads.

```
╭ ratodo / stats — WEEK ───────────────────────────────────────────────────────╮
│                                                                              │
│  ╭─ TOTALS ───────────────────────────────────────────────────────────────╮  │
│  │ 42 tasks      31 done      8 open      3 overdue                       │  │
│  │ ████████████████████████████████████░░░░░░░░░░░░  74%                  │  │
│  ╰────────────────────────────────────────────────────────────────────────╯  │
│  ╭─ DONE THIS WEEK ───────────────────────────────────────────────────────╮  │
│  │   MON       TUE       WED       THU       FRI       SAT       SUN      │  │
│  │   ████      ██████    ████      ████████  ██████    ███       ░        │  │
│  │   4         6         4         8         6         3         0        │  │
│  ╰────────────────────────────────────────────────────────────────────────╯  │
│  ╭─ PRIORITY ─────────────────────────────────────────────────────────────╮  │
│  │ !high  ████████ 8                                                      │  │
│  │ !med   █████████████ 13                                                │  │
│  │ !low   ████████████████████████ 21                                     │  │
│  ╰────────────────────────────────────────────────────────────────────────╯  │
│  ╭─ SECTIONS ─────────────────────────────────────────────────────────────╮  │
│  │ ## tasks     ████████████████████████ 14                               │  │
│  │ ## Someday   ██████████ 6                                              │  │
│  │ (none)       █████ 3                                                   │  │
│  ╰────────────────────────────────────────────────────────────────────────╯  │
│  ╭─ PACE ─────────────────────────────────────────────────────────────────╮  │
│  │ best day   THU      avg / day   4.4      streak   6 days               │  │
│  ╰────────────────────────────────────────────────────────────────────────╯  │
│                                                                              │
╰──────────────────────────────────────────────────────────────────────────────╯
 [1] week  [2] month  [3] year   [r] reload   [esc] back
```

- **A screen, not an overlay.** `s` opens it, `s` or `esc` closes it, and it
  replaces the list rather than covering it — nothing on it is glanced at
  mid-task. While it is up the list's keys do not act: `spc` ticking a task
  nobody can see is the failure that rule exists to stop.
- **`1` `2` `3` are week, month and year**, and the heading says which. A week is
  seven days, a month is its weeks and a year is its twelve months — never more
  than twelve bars, because a histogram that needs a scrollbar is not one. It is
  always *this* week, month or year, never "the last thirty days": that is a
  different question and not one a calendar can be asked at a glance.
- **Every block is a box, the same box the agenda draws a group in, and they
  touch.** This was five paragraphs with nothing round them, argued for as
  restraint against looking like Grafana; on a real screen it read as one loose
  column of text in a product where everything else is a container, and a
  heading with nothing holding what is under it is not a category. The boxes
  are the categorisation — [decisions.md](decisions.md#reversed). A blank row
  between two closed containers is a row spent saying what the edges said, so
  there is none, exactly as on the list. Below 34 columns the frame goes and the
  boxes go with it, also exactly as on the list.
- **`TOTALS` and `PACE` are named too.** A row of numbers with no word over it
  is the thing the reader has to work out, and the two summary blocks were the
  ones with nothing to be called.
- **The block is `SECTIONS` or `LISTS`, and the word follows what is
  under it.** With one list open those rows are the file's own `## ` headings;
  with several they are the files, because `## Work` in two files is two places
  and adding them together would be a lie about a heading nobody shares. A
  column of file names under the word `SECTIONS` is the heading disagreeing with
  its own block.
- **Only the top bar has a trough.** How far through the list you are is a
  fraction and a fraction needs its denominator drawn; everywhere else a bar is a
  length read against the length beside it, and a row of `░` behind each one
  turns that into a grid. A count of nothing still gets one cell, and a count of
  one is never rounded away to none.
- **One caveat, on the screen and not in a document:** a task ticked before the
  completion stamp existed has no `done_on`, so it counts in `31 done` and in
  nothing with a day attached. When there are any, the screen says how many
  rather than quietly under-reporting the streak.
- **Sections, not projects.** `## Someday` is the file's own word for the same
  idea. With several lists open the box shows lists instead, and its heading
  says `LISTS` — a heading can repeat across files and merging them would be a
  lie.
- **A streak survives a morning.** Today having nothing on it yet does not break
  one; a streak that resets every morning and comes back after lunch is a clock.
- **What it does in a short pane**, in this order and never a scrollbar:
  `SECTIONS` goes first, then `PRIORITY`, then the histogram's day labels, then
  the histogram itself. `TOTALS` and `PACE` are what is left standing. Blocks go
  whole — a box cut off at the bottom of the pane would lose its own bottom edge,
  which is the rendering fault these boxes exist to stop.
- **It gets no file of its own.** `stats(&[Task], today, period) -> Stats` has
  `agenda`'s exact signature and `agenda`'s exact purity, so it lives in
  `agenda.rs` beside it and
  [architecture.md](architecture.md#module-layout)'s eleven files do not move.

## Keys

| Key | Action | Note |
|---|---|---|
| `j` `k` / `↓` `↑` | move | Both, always. Arrows cost nothing and not everyone has vim hands |
| `g` / `G` | top / bottom | A vim user typing `gg` gets the top on the first `g` and a harmless no-op on the second — so no pending-key state machine is needed |
| `ctrl-d` / `ctrl-u` | half page | |
| `spc` | toggle done | |
| `a` | add, in the [form](#the-form--a) | Falls back to the one-line box under 15 rows or 40 columns |
| `o` | add, in the [one-line box](#the-one-line-box--o) | Always the box. A vim user reaches for `o` to open a new line, which is the fast path, so it stays the fast path |
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
| `s` | stats | Opens the [stats screen](#stats--s) and closes it again. `1` `2` `3` change the period while it is up and do nothing on the list |
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
│  ! rotate the backup keys            │ 2d ago    │      │ #ops               │
│                                                                              │
│  TODAY ─────────────────────────────                                         │
│  ○ pay the invoice                   │ today     │      │ #home              │
│  ○ review the deploy PR              │ 16:00     │      │ #work              │
│                                                                              │
│  THIS WEEK ─────────────────────────                                         │
│  ○ book a dentist appointment        │ Thu 09:30 │      │ #health            │
│  ✓ migrate the server                │ Thu       │      │                    │
│                                                                              │
│  ## Someday ────────────────────────                                         │
│▌ ○ finish chapter 13 of the Rust book│           │ !low │                    │
└──────────────────────────────────────────────────────────────────────────────┘
```

Why this is a breakpoint and not simply the layout: **a column costs every row
its width, whether or not that row uses it.** One `!low` in the whole list buys
a priority column that every other row then carries as blank space. Past eighty
columns there is room to spend; below it the packed right-aligned block fits
more onto the row, and fitting more on wins when there is not much row.

Four things follow from the columns, and they are decisions:

- **A dim `│` between them.** Once the fields line up the row *is* a table, and
  a table without rules was read as one run-on line. **An empty cell keeps its
  rules** — the row with no priority draws them in the same places as the row
  with one — because rules that appear and disappear per row are worse than none.
  They are `border`, the colour the frame is already drawn in: a grid is scenery
  and scenery does not get the accent. The same `│` separates the fields in the
  input box's preview, so the screen has one separator and not two. Below this
  breakpoint there are no rules, because there is nothing lined up to separate.
- **The dividers end on the group box.** A `┬` where a column meets the top
  edge, a `┴` where it meets the bottom. Before the box the group rule stopped
  at the title column and the dividers began one column later out of nothing;
  the box is what joined them up, and it is why a rule to the right edge is no
  longer the heaviest thing on the screen — it is an edge, and it closes.
- **The title column is measured over the whole list, not the visible rows.** A
  column that resizes as you scroll past a long title is not a column.
- **Tags get no column of their own.** They are last and ragged, so nothing
  lines up after them, and reserving the widest row's worth would cut every
  title to pay for tags most rows do not have. They spend what is left of the
  row, and a tag that does not fit is dropped whole — `#hea…` is not a filter,
  it is a riddle. The **rule** that opens their column is reserved, though, and
  it has to be: it is drawn on every row once any row is tagged, so a title
  allowed to eat the last three columns would push it off exactly the rows with
  nothing to show there. A list where nobody tagged anything gets no rule, the
  same way it gets no priority column.

**Wide (60–79 columns)** — the main screen above: no columns, the right-hand
fields packed against the right edge.

**Narrow (34–59 columns)** — the tags and the hint bar shrink first. The box
survives here, and this is the one width where it is not free: the blank spacer
row was already gone at this width, so the bottom edge is a row per group that
the pane has to find.

```
╭ ratodo — 5 · 1! ─────────────────╮
│  ╭─ OVERDUE · 1 ──────────────╮  │
│▌ │ ! rotate the backup k…  2d │  │
│  ╰────────────────────────────╯  │
│  ╭─ TODAY · 2 ────────────────╮  │
│  │ ○ pay the invoice     today│  │
│  │ ○ review the deploy…  16:00│  │
│  ╰────────────────────────────╯  │
╰──────────────────────────────────╯
 [j k] [spc] [a] [d] [?]
```

**Short panes** drop the band before anything else, in two steps rather than
one: under **20 rows** it becomes a single line of counts, and under **16** it
goes entirely and the footer goes with it. The tiles are worth five rows on a
pane somebody leaves open beside their work and worth nothing on a pane with six
tasks in it. Under **10 rows** the hint bar collapses to `?`.

**Very narrow (< 34 columns)** — the frame is dropped entirely, and the boxes
with it; just rows. The band needs 60 columns to lay tiles across and is not
drawn below that at any height.

What is given up, in order, as the **height** shrinks: the band's tiles, then
the band, then the footer with it, then the hint bar. The list is the last thing
standing, which is the rule the whole table is derived from.

What is given up, in order, as the width shrinks:

1. the columns — the right-hand fields pack against the right edge again
2. blank spacer rows between groups *(spent on the group box since 2026-08-12,
   so what goes here is the box's interior spacing, not a row)*
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
becomes `+ - |`, the group boxes become `+ - |` with `+` for their corners and
their junctions, the group rules become `-`, and the `—` and `·` in the title
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
