# TUI redesign

> Status: **accepted 2026-08-12, building as v0.8.0.** Screens 0, 2, 3 and 5 are
> the work; Screen 4 is conditional on its own test. The list of steps is in
> [todo.md](../todo.md#what-is-left); the reversals each one owes to
> [decisions.md](decisions.md) are named under [What each one costs](#what-each-one-costs).

What the five mockups in `tui/` ask for, drawn against ratodo's real data, at
ratodo's real widths, so the choice could be made from pictures rather than from
adjectives.

This document was written as an argument and is kept as one: what was rejected,
and why, is the half worth reading. A decision that reverses something in
[tui.md](tui.md) or [design.md](design.md) is not settled by this file — it is
settled by an entry in [decisions.md](decisions.md), which is the order
[CLAUDE.md](../CLAUDE.md) asks for.

---

## What the mockups ask for

| File | Asks for | Status |
|---|---|---|
| `tui/tui-1.md` | a split layout: list on the left, form on the right | drawn below, **reject** |
| `tui/tui-crate-todo.md` | a labelled multi-field create form | drawn below, **take, as a second door** |
| `tui/tui-edit-todo.md` | the same form, prefilled, for editing | drawn below, **reject the edit half** |
| `tui/tui-fast-todo.md` | a one-field capture box | **already shipped** — this is `a` |
| `tui/tui-stats.md` | a statistics screen | drawn below, **take** |

Two of them contradict each other, and that contradiction is the useful part:
`tui-crate-todo.md` wants five labelled fields, `tui-fast-todo.md` wants one.
Both are right, at different moments. The proposal below is that they are two
keys and not one compromise.

But the thing all five have in common is not in the table, because none of them
is asking for it out loud: **they are drawn as if somebody is being welcomed.**
Titles centred, groups announced, keys presented. That is Screen 0, it is the
cheapest change in this document, and it is probably most of the gap.

## The principle underneath all of it

> **ratodo must not hide the file the way a database hides one.**

Everything below follows from that one line, and it is what separates this from
a nice-looking todo TUI:

```
                       ┌──────────────┐
                       │     FILE     │
                       │   todo.md    │
                       └──────┬───────┘
                              │
              ┌───────────────┼───────────────┐
              ↓               ↓               ↓
           CAPTURE          FORM            EDIT
          fast, typed    controlled     true to the source
              │               │               │
              └───────────────┼───────────────┘
                              ↓
                            TASK
                              │
                              ↓
                            STATS
                      derived, never stored
```

Three doors into the same file, one shape coming out of them, and a fifth screen
that is read-only arithmetic over what the file already says. **Nothing in this
document may add a place where the tool knows something the file does not.**
That is why there is no Description, no Project, and no split pane — not because
they are hard, but because each one would put state somewhere the user cannot
open in vim.

And the keys are the same picture:

```
  MAIN ─┬─ a ──→  NEW TASK          esc / ⏎
        ├─ ⏎ ──→  EDIT TASK         esc / ⏎
        ├─ s ──→  STATS ─┬─ 1 week
        │                ├─ 2 month     s / esc
        │                └─ 3 year
        └─ ? ──→  KEYS               esc / ?
```

Four doors, every one of them closed by `esc`, and no screen more than one key
from the list. That is the whole state machine, and it is still the two modes
`docs/tui.md` promised — a screen you opened on purpose, and the list.

## The three rules every mockup has to survive

1. **The pane is 34 columns wide.** Not always — but it is the case the tool was
   shaped for, and a screen that only works at 68 columns is a screen that stops
   working when the layout gets busy. Every mockup below is drawn twice.
2. **A task is one line in the user's file.** There is no Description field and
   no Project field, because there is nowhere to put them. The form's fields are
   exactly the six the format already carries: title, date, time, tags,
   priority, and which list.
3. **A line the tool did not change is written back byte for byte.** This is
   what kills form-based *editing*, and it is spelled out under Screen 4.

---

## Screen 0 — the skin  ·  NEW, and the cheapest thing here

The mockups' real advantage is not any one widget. It is that the screen
**greets you**: there is a band at the top that says where you are, the groups
announce themselves, and the keys at the bottom look like keys. Ours starts
mid-sentence. None of that needs a new widget, a new field or a new decision —
it is four changes to how the same rows are drawn:

```
╭─ ratodo ───────────────────────────────────────── Wednesday, 12 August 2026 ─╮
│                                                                              │
│    1            3            4              31/42            ▂▅▃█▆▂▁         │
│    OVERDUE      TODAY        THIS WEEK      DONE · 74%       MON — SUN       │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│  ╭─ OVERDUE · 1 ─────────────────────────┬──────────┬───────┬─────────────╮  │
│  │▌ !  rotate the backup keys            │ 2d ago   │       │ #ops        │  │
│  ╰───────────────────────────────────────┴──────────┴───────┴─────────────╯  │
│  ╭─ TODAY · 2 ───────────────────────────┬──────────┬───────┬─────────────╮  │
│  │  ○  pay the invoice                   │          │       │ #home       │  │
│  │  ○  review the deploy PR              │ 16:00    │ !high │ #work       │  │
│  ╰───────────────────────────────────────┴──────────┴───────┴─────────────╯  │
│  ╭─ THIS WEEK · 1 ───────────────────────┬──────────┬───────┬─────────────╮  │
│  │  ○  book a dentist appointment        │ Fri 09:30│       │ #health     │  │
│  ╰───────────────────────────────────────┴──────────┴───────┴─────────────╯  │
│  ╭─ ## tasks · 2 ────────────────────────┬──────────┬───────┬─────────────╮  │
│  │  ✓  migrate the server                │ done Tue │       │             │  │
│  │  ✗  rewrite the docs                  │          │       │ #docs       │  │
│  ╰───────────────────────────────────────┴──────────┴───────┴─────────────╯  │
│  ╭─ ## Someday · 1 ──────────────────────┬──────────┬───────┬─────────────╮  │
│  │  ○  finish chapter 13 of the Rust book│          │ !low  │             │  │
│  ╰───────────────────────────────────────┴──────────┴───────┴─────────────╯  │
│    LATER · 3 ────────────────────────────────────────────────────── l open   │
├──────────────────────────────────────────────────────────────────────────────┤
│  - [ ] rotate the backup keys @2026-08-10 #ops                               │
╰──────────────────────────────────────────────────────────────────────────────╯
  [j k] move  [spc] done  [a] add  [s] stats  [?] keys  [q] quit
```

### The grid, and the bug it fixes

This is the change that matters most and it is a **correction**, not a
decoration. Today's screen draws three line systems that never touch each other:

```
│  TODAY ─────────────────────────────                                         │
│  ○ pay the invoice                    │ today     │       │ #home            │
│  ○ review the deploy PR               │ 16:00     │ !high │ #work            │
│                                                                              │
```

The group rule stops in mid-air at column 39. The column separators begin at
column 40 out of nothing, attached to neither the rule above them nor anything
below. The group ends in a blank row that closes nothing. Three sets of strokes
sharing a screen without ever meeting is most of why it reads as unfinished.

**Every group becomes a nested box, and the column separators are its
junctions.** `┬` where a column meets the group's top edge, `┴` where it meets
the bottom. Nothing floats: every stroke starts at a corner and ends at one.

Four things follow:

- **It costs no rows.** A group is a heading, its tasks and a blank spacer
  today — one, *n*, one. As a box it is a top edge, its tasks and a bottom edge:
  one, *n*, one. The heading moves into the top edge and the spacer becomes the
  bottom edge. The arithmetic is identical.
- **It costs four columns**, two of indent and two of border. At 80 that is
  affordable; the narrow drawing below shows what it looks like when it is not.
- **The furniture stays furniture.** The box is drawn in `border`, the colour
  `docs/design.md` already reserves for frames and rules and forbids to content.
  The group's *name* keeps its `accent`. No new colour, no new role.
- **A folded group is a bare rule, not a box** — `LATER · 3 ─── l open` above.
  An empty box two rows tall to say a group is closed is exactly backwards, and
  the difference between a container and a line *is* the open/closed signal.

### The date column no longer repeats the header

Look at `pay the invoice`. Today that row says `today` in the date column, inside
a group whose heading says `TODAY`. The column is spending nine characters to
repeat the box it is already in.

**The rule: the date column says what the heading does not.**

| Group | What the heading says | What the column says |
|---|---|---|
| `OVERDUE` | that it is late | *how* late — `2d ago` |
| `TODAY` | the day | the **time**, or nothing — `16:00` |
| `THIS WEEK` | the week | which day — `Fri 09:30` |
| `## Someday` | nothing about dates | the date, if it has one |

`TODAY` is the group most people look at most often, and it is the one where the
column was pure noise. Emptied, the rows that *do* have a time stand out — which
is the only thing about a task due today that is still worth reading.

### The rest of the band

1. **Rounded corners** — `BorderType::Rounded`, one constant, the highest ratio
   of *looks modern* to *lines changed* here.
2. **Stat tiles, big number over small label.** `tui-stats.md`'s idea moved to
   where it is looked at every day. The numbers are the ones `ratodo status`
   already computes.
3. **The date, spelled out.** The first thing a todo list should say and the one
   thing this screen currently never says.
4. **A week sparkline** — seven cells of `▁▂▃▅▆▇█` off the completion stamps,
   the same data the stats screen draws large. Seven columns, and the only thing
   on the screen that says the tool has a memory.
5. **Counts on every heading** — `TODAY · 2`. `LATER (3)` already does this when
   folded; there is no reason the open ones stay silent.
6. **The keys look like keys** — `[a] add`, not `a add`. Brackets, so it survives
   `NO_COLOR`, and it reads as a keycap the way lazygit's and k9s's bars do.

And the footer, which is the one addition that is about this product rather than
about looking better: **the selected task's actual line from the file**, raw,
byte for byte. One row, and it says *this is a file and here is your line in it*
on the screen somebody stares at all day. It is also the honest answer to "did
the tool understand what I typed", with no form open and nothing to click.

The band costs five rows and the footer one. Below twenty rows the band drops to
a single line of counts, below sixteen it goes entirely, and the footer goes
with it.

### First run

The screen someone sees the first time, which is the "greets you" idea taken
literally. It only ever appears when the file has no tasks in it:

```
╭─ ratodo ────────────────────────────────────────────────────────────────────╮
│                                                                             │
│                                 ratodo                                      │
│                     a todo list that is still just a file                   │
│                                                                             │
│                       ~/.config/ratodo/todo.md                              │
│                                                                             │
│      ╭─────────────────────────────────────────────────────╮                │
│      │ ADD ▏buy milk @tomorrow #home                       │                │
│      ├─────────────────────────────────────────────────────┤                │
│      │      due tomorrow (2026-08-13)  ·  #home            │                │
│      ╰─────────────────────────────────────────────────────╯                │
│                                                                             │
│        [a] add your first task            [e] open it in $EDITOR            │
│                                                                             │
╰─────────────────────────────────────────────────────────────────────────────╯
```

The empty screen already does most of this — it already names the file and
already draws the box `a` opens, with `@tomorrow` resolved by the real parser.
What it does not have is the two centered lines at the top, and those two lines
are the whole of the welcome. **No ASCII-art logo**: this is a pane somebody
leaves open beside their work, and a banner is charming exactly once.

---

## Screen 1 — the list today, for comparison

This is not a mockup. It is what the binary drew at 80×24 on 2026-08-12:

```
┌ ratodo — 5 open · 1 overdue ─────────────────────────────────── ▰▱▱▱▱▱▱▱ 1/6 ┐
│  OVERDUE ───────────────────────────                                         │
│▌ ! rotate the backup keys             │ 2d ago    │       │ #ops             │
│                                                                              │
│  TODAY ─────────────────────────────                                         │
│  ○ pay the invoice                    │ today     │       │ #home            │
│  ○ review the deploy PR               │ 16:00     │ !high │ #work            │
│                                                                              │
│  THIS WEEK ─────────────────────────                                         │
│  ○ book a dentist appointment         │ Fri 09:30 │       │ #health          │
│                                                                              │
│  ## tasks ──────────────────────────                                         │
│  ✓ migrate the server                 │ Tue       │       │                  │
│  ✗ rewrite the docs                   │           │       │ #docs            │
│                                                                              │
│  ## Someday ────────────────────────                                         │
│  ○ finish chapter 13 of the Rust book │           │ !low  │                  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 j k move  spc done  a add  ⏎ edit  d cancel  p later  y copy  ? keys  q quit
```

It carries more information than the mockup's left pane — dates, tags,
priorities, progress, none of which `tui-1.md` has room for — and it still looks
plainer, which is the useful thing to notice. Density is not warmth. What it is
missing is the band at the top, the counts on the headers, the keycaps at the
bottom, and somewhere else to go: Screens 0, 3 and 5.

---

## Screen 2 — `a`, the add screen  ·  REPLACED

The mockups' form, with the two fields the file cannot hold removed, their
question, their radios and their buttons kept, and one line added at the bottom
that they do not have. **This is what `a` opens.**

```
╭────────────────────────────── NEW TASK ─────────────────────────────╮
│                                                                     │
│  What needs to be done?                                             │
│                                                                     │
│  ▌ ╭─────────────────────────────────────────────────────────────╮  │
│    │ call the accountant▏                                        │  │
│    ╰─────────────────────────────────────────────────────────────╯  │
│                                                                     │
│  Due         ◉ today    ○ tomorrow   ○ thu       ○ pick…            │
│  Time        ◉ none     ○ type…                                     │
│  Priority    ○ high     ○ med        ○ low       ◉ none             │
│                                                                     │
│  Tags        [ #home #work                                     ]    │
│  List        ◉ todo.md   ○ work.md                                  │
│                                                                     │
│  ─────────────────────────────────────────────────────────────────  │
│                                                                     │
│  PREVIEW                                                            │
│  - [ ] call the accountant @2026-08-12 #home #work                  │
│                                                                     │
│       [ esc cancel ]                          [ ⏎ create task ]     │
╰─ tab / shift-tab · navigate ────────────────────────────────────────╯
```

Seven things about it are decisions, not drawing:

- **`PREVIEW`, with its own label and its own rule above it.** This is the
  difference between a form that happens to show a line and a form whose
  *conclusion* is a line. The character of this product is not *I am filling in
  a form*, it is *I am writing a task into my file* — and the screen has to say
  which one it thinks it is. A Todoist form saves into a database and can tell
  you nothing; this one saves into your file, so the file is the last word on
  the screen. It is also the same live-parse code the fast box runs, read
  backwards: `capture` builds a task from text, this renders text from a task,
  and `Task::line()` is most of it already.
- **The question is theirs and it is right.** *What needs to be done?* is a
  better label than *Title*, and in a form there is room for it. It replaces the
  syntax-by-example hint, which had that job only because there was nowhere else
  to put a sentence.
- **Radio buttons, not highlighted brackets.** `◉` against `○` is a difference
  in *shape*, so it survives `NO_COLOR=1` and the ASCII fallback, where the pair
  becomes `(o)` and `( )`. `[ MED ]` with the selection carried by colour alone
  would break the rule in `docs/design.md`.
- **The buttons stay, and they carry their key.** `[ ⏎ create task ]` is both
  the button the mockup drew and the keybinding, so it is honest on a keyboard
  and still looks like a button. `tab` walks onto them like any other field.
- **`▌` sits beside the input box, not beside its label.** The marker points at
  the control that has the keyboard, and the label is not it. Same marker and
  same colour as the selected row on the list.
- **`Time` offers `none` and `type…`, and no clock times.** An `◉ 09:00
  ○ 16:00` pair invents two numbers out of nothing — they are not this user's
  hours, they are somebody's idea of a working day. `type…` opens four digits,
  which is faster than finding your hour in a list that does not have it.
- **`List` appears only when more than one list is open**, exactly as `$list`
  does today.

**No Section / Project field**, and this is worth being explicit about: a
capture currently lands below everything, outside every `##` section
(`model.rs:938`). Letting the form choose a section means teaching the writer to
*insert* into the middle of the file rather than append to the end. That is a
change to the write path, which is the one place fidelity is won or lost. It can
be done; it should not be smuggled in as a dropdown.

**Typing still works.** `@thu`, `#home` and `!high` in the question field are
parsed the way they always were and light the matching radio up as you type —
the form is a second way to say the same thing, not a replacement syntax. That
is what keeps the two paths from drifting: one tokenizer, one truth.

### What happens to the two-second capture

This is the real trade, and it is smaller than it looks: **the fast path was
never the TUI.** `docs/product.md` says so outright — `ratodo add 'pay the
invoice @tomorrow'` writes and exits without opening anything, and *"the second
one is the reason this product exists"*. The TUI's `a` is for when the pane is
already open and you are already looking at it. That moment can afford a form.

What the form must not do is become the only way in when there is no room for
it, which is Screen 3.

---

## Screen 3 — the one-line box, kept as the fallback

The box that exists today does not go away; it stops being the default. It is
what `a` opens when the pane is too small for the form — under **14 rows or 40
columns** — and it is still what `p` and `y` open, because those two ask one
question each and a form for one question is a form nobody wants:

```
╭─ ratodo ───────────────────────╮
│  TODAY · 2 ──────────────────  │
│  ○ pay the invoice     #home   │
│ ╭────────────────────────────╮ │
│ │ ADD ▏call the acc… @thu    │ │
│ ├────────────────────────────┤ │
│ │     due Thu (2026-08-13)   │ │
│ ╰────────────────────────────╯ │
│  ○ review the deploy…  16:00   │
╰────────────────────────────────╯
 ⏎ save   esc cancel
```

A form that half-fits is worse than a box that always fits, and the box is
already built and already tested. Keeping it costs nothing and it is the reason
the form is allowed to be as big as it is.

---

## Screen 4 — `⏎`, the edit form

`tui-edit-todo.md` is the same form, prefilled, and it should be exactly that —
the same code, a different title and a different button:

```
╭────────────────────────── EDIT TASK ───────────────────────────╮
│                                                                │
│▌  What needs to be done?                                       │
│   ╭──────────────────────────────────────────────────────────╮ │
│   │ rotate the backup keys▏                                  │ │
│   ╰──────────────────────────────────────────────────────────╯ │
│                                                                │
│   Due         ○ today    ○ tomorrow   ◉ 2026-08-10  ○ pick…    │
│   Time        ◉ none     ○ 09:00      ○ 16:00       ○ type…    │
│   Priority    ◉ high     ○ med        ○ low         ○ none     │
│                                                                │
│   Tags        [ #ops                                     ]     │
│                                                                │
│   ──────────────────────────────────────────────────────────   │
│   - [ ] #ops rotate the backup keys !high @2026-08-10          │
│                                                                │
│        [ esc  cancel ]                 [ ⏎  save changes ]     │
╰─ tab · shift-tab  move between fields ─────────────────────────╯
```

**Look at the preview line.** The tags are still first and the date is still
last, because that is how this user wrote it. That is not decoration — it is the
one hard problem in this screen, and it is why the naive version of it is
forbidden.

`⏎` today prefills with the task's raw text **byte for byte** and saving replaces
exactly that, so a line the user arranged their own way survives a retype. A
form that parses into six fields and re-serializes them cannot promise that: an
untouched form would turn the line above into

```
- [ ] rotate the backup keys @2026-08-10 #ops !high
```

Nothing was edited and the line moved. That is the guarantee in
`docs/architecture.md#round-trip-fidelity`, and it is exactly why `y` is allowed
to re-serialize — a copy is a *new* line — while `⏎` is not.

**The fix is small and it already has the data it needs.** `capture::parts`
hands back a `Range<usize>` for every token it recognises. So the edit does not
rebuild the line, it **splices** it: a field the form did not change leaves its
span alone, and a field that changed replaces its own span and nothing else.
Change the priority on the line above and `!high` becomes `!med` where it
stands; the tags stay first, the date stays last, and every byte between them is
the user's.

### The four cases the splice has to get right

This is the most technical thing in the document and the only thing in it that
can lose someone's data, so it is worth being precise before any of it is built:

| Case | What happens |
|---|---|
| **unchanged** | the span is not touched, and if *no* span changed the write is skipped entirely — the file is not opened, the undo is not spent. `docs/tui.md` already promises this for `⏎` |
| **changed** | the span's bytes are replaced in place. Position, order and the whitespace either side are the user's |
| **added** — a date where there was none | the only case where the tool chooses a position, so it gets the one rule: append at the end of the line, which is what `capture` does for a new task anyway |
| **removed** — the date cleared | the token **and one adjacent space** go, or a line loses a field and keeps a double space, and does it again on the next edit |

The title is the awkward one, because it is not a token: it is everything
`parts` did *not* claim. Editing it means replacing the unclaimed spans and
leaving the claimed ones where they sit — which is the same operation viewed
from the other side, but it is the case to write the test for first.

**The test is a property test, and it is not optional.** Open the form on a
fixture task, change nothing, save: the line must come back byte for byte, for
every fixture in `tests/fixtures/` including the deliberately awkward ones. Then
change one field at a time and assert every *other* byte survived. That is a
`tests/fidelity.rs` job, and per `CLAUDE.md` this module gets
`cargo mutants --timeout 90` before it is called done.

**And the fallback is real, which is why this is last in the build order.** If
the splice turns out hairier than it reads, `⏎` keeps the one-line box — already
built, already byte-perfect — and the form stays add-only. Nothing else in the
proposal depends on this screen. A beautiful TUI that silently reformats the
user's file has spent the only thing this product actually sells.

---

## Screen 5 — `s`, stats  ·  NEW

The one mockup that takes nothing away, and the answer to "there is only one
screen". Every number below already exists in the file: `✓2026-08-11` completion
stamps are what `done_on` reads.

```
╭─ ratodo / stats ─────────────────────────────────────────────────────────────╮
│                                                                              │
│  42 tasks          31 done          8 open          3 overdue                │
│  ████████████████████████████████████████████████░░░░░░░░░░░░░░░░  74%       │
│                                                                              │
│  DONE THIS WEEK                                                              │
│                                                                              │
│      MON     TUE     WED     THU     FRI     SAT     SUN                     │
│      ███     ████    ███     █████   ████    ██      ░                       │
│       4       6       4       8       6       3       0                      │
│                                                                              │
│  PRIORITY                              SECTIONS                              │
│                                                                              │
│  !high   ████████         8            ## tasks     ███████████   14         │
│  !med    █████████████   13            ## Someday   █████          6         │
│  !low    █████████████████ 21          (none)       ███            3         │
│                                                                              │
│  best day   Thursday        avg / day   4.4        streak   6 days           │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│  [1] week   [2] month   [3] year        [r] refresh        [esc] back        │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**No boxes and no rules between the blocks, deliberately.** The list screen is a
grid because its rows line up and are read across; this screen is five
paragraphs read one at a time, and a frame round each would be furniture with
nothing to hold. ratodo is a todo manager. A statistics screen is exactly where
a tool starts trying to look like Grafana, and the restraint has to be spent
here rather than argued about later.

- **`stats(&[Task], today) -> Stats` is a pure function**, `today` passed in, no
  clock inside — the same shape as `agenda`, and testable for the same reason.
- **No new dependency.** ratatui already ships `BarChart` and `Gauge`; the bars
  above are eight lines of `█` and `░` either way.
- **One honest caveat to put on the screen, not in a doc:** a task ticked before
  the completion stamp existed has no `done_on`, so it counts in `31 done` and
  in nothing that has a day attached. The week histogram and the streak start
  from the day stamping started. If that number is large the screen should say
  so rather than quietly under-report a streak.
- **Sections, not Projects.** `## Someday` is the file's own word for the same
  idea. With several lists open this block shows lists instead — a heading can
  repeat across files and merging them would be a lie.
- `s` opens it, `s` or `esc` closes it. It is a screen, not an overlay: it
  replaces the list rather than covering it, because nothing about it is glanced
  at mid-task.

---

## Screen 6 — the split layout, drawn honestly  ·  reject

`tui-1.md` at the width a terminal actually opens at. Left pane 40, right pane
40, which is what 80 columns divides into:

```
┌ ratodo — 5 open · 1 overdue ─────────┬─ NEW TASK ──────────────────────────┐
│  OVERDUE ─────────────────────────   │                                     │
│▌ ! rotate the backup k…       2d ago │▌ Title                              │
│                                      │  ┌───────────────────────────────┐  │
│  TODAY ───────────────────────────   │  │ call the accountant▏          │  │
│  ○ pay the invoice             today │  └───────────────────────────────┘  │
│  ○ review the deploy PR        16:00 │                                     │
│                                      │  Due    ◉ today  ○ tmw  ○ pick…     │
│  THIS WEEK ───────────────────────   │  Prio   ○ hi  ◉ med  ○ lo  ○ —      │
│  ○ book a dentist app…     Fri 09:30 │                                     │
│                                      │  Tags                               │
│  ## tasks ────────────────────────   │  ┌───────────────────────────────┐  │
│  ✓ migrate the server            Tue │  │ #home #work                   │  │
│  ✗ rewrite the docs                  │  └───────────────────────────────┘  │
│                                      │                                     │
│  ## Someday ──────────────────────   │  - [ ] call the accountant @2026…   │
│  ○ finish chapter 13 of…             │                                     │
└──────────────────────────────────────┴─────────────────────────────────────┘
 tab field   ⏎ save   esc cancel
```

Compare with Screen 1. Every title is now truncated, every tag is gone, the
priority column is gone, and the preview line in the form is cut at `@2026…`.
The mockup looked roomy because it was drawn at whatever width it needed; on a
real 80-column terminal both halves are worse than either was whole. Below 68
columns it cannot be drawn at all, and the tiling pane this tool was shaped for
is 40.

`docs/design.md:108` already says *one layout, no split panes*. This drawing is
why, rather than a claim that it is why.

The centered overlay in Screen 2 gets the same form, at 64 columns instead of
38, and gives the list back the moment it closes.

---

## All of it at 40 columns

The form stacks, the radios abbreviate, and the preview line is the last thing
to go:

```
╭─────────── NEW TASK ─────────────────╮
│                                      │
│  What needs to be done?              │
│ ▌╭─────────────────────────────────╮ │
│  │ call the accountant▏            │ │
│  ╰─────────────────────────────────╯ │
│                                      │
│  Due    ◉ today  ○ tmw   ○ pick…     │
│  Time   ◉ none   ○ type…             │
│  Prio   ○ hi   ◉ med   ○ lo   ○ —    │
│  Tags   [ #home #work            ]   │
│                                      │
│  PREVIEW                             │
│  - [ ] call the accountant @2026-0…  │
│                                      │
│  [ esc cancel ]      [ ⏎ create ]    │
╰─ tab · navigate ─────────────────────╯
```

That is 17 rows. **Below 15 rows or 40 columns, `a` opens the one-line box
instead** (Screen 3) — the form is the screen that can afford to be
unavailable, because the thing it falls back to is already built and always
fits.

The dashboard goes the same way. At 44 columns the tiles keep their shape and
lose their spacing, the boxes keep their edges and lose their interior columns —
which is today's rule anyway, since there are no columns below 80 — and the
indent drops from two to one:

```
╭─ ratodo ──────────────────── Wed 12 Aug ─╮
│                                          │
│   1       3       4       ▂▅▃█▆▂▁        │
│   LATE    TODAY   WEEK    31/42          │
│                                          │
├──────────────────────────────────────────┤
│ ╭─ OVERDUE · 1 ────────────────────────╮ │
│ │▌ !  rotate the backup k…      2d ago │ │
│ ╰──────────────────────────────────────╯ │
│ ╭─ TODAY · 2 ──────────────────────────╮ │
│ │  ○  pay the invoice            #home │ │
│ │  ○  review the deploy PR       16:00 │ │
│ ╰──────────────────────────────────────╯ │
│ ╭─ THIS WEEK · 1 ──────────────────────╮ │
│ │  ○  book a dentist ap…           Fri │ │
│ ╰──────────────────────────────────────╯ │
╰──────────────────────────────────────────╯
  [j k] [spc] [a] [s] [?] [q]
```

The boxes survive further down than expected because they cost rows they were
already spending. **Below 34 columns they go**, along with the outer frame, the
same way everything else does — bare rows, a heading and a blank line, which is
what that width has always been.

One open question at this width: three columns of the row are now furniture
(`│ `, ` │`) that were title before. The alternative is a **left spine only** —
`│` down the left edge of the group with rounded caps, no right border — which
marks the extent for one column instead of four. It is worth drawing if 44
columns turns out to be the width you actually run.

Stats at the same width drops the second column and the daily labels:

```
╭─ ratodo / stats ─────────────────────╮
│                                      │
│  42 tasks  31 done  8 open  3 late   │
│  ████████████████████░░░░░░░  74%    │
│                                      │
│  DONE THIS WEEK                      │
│                                      │
│    M    T    W    T    F    S    S   │
│    █    █    █    █    █    █    ░   │
│    4    6    4    8    6    3    0   │
│                                      │
│  !high   ████████          8         │
│  !med    █████████████    13         │
│  !low    ████████████████ 21         │
│                                      │
│  streak 6 days      avg 4.4 / day    │
│                                      │
├──────────────────────────────────────┤
│  [1] [2] [3]   [r]        [esc] back │
╰──────────────────────────────────────╯
```

---

## What each one costs

| Screen | Cost | Decision to record |
|---|---|---|
| **0 · the dashboard** | rounded borders, a five-row tile band, a sparkline, counts on group headers, keycaps on the hint bar, the raw-line footer. No new state, no new key, no new data — draw code and the width table | The band and the footer are new furniture on the main screen; note them, nothing is reversed |
| **0 · the group grid** | each group becomes a nested box and the column separators become its `┬`/`┴` junctions. Costs no rows, costs four columns | No — `docs/tui.md` already gives group headings a rule and columns their separators. This joins them up, which is what they were always drawn as |
| **0 · the date column** | drop the part the heading already says: `today` inside `TODAY` becomes the time, or nothing | Amends the date-column rule in `docs/tui.md`; nothing else changes |
| **2 · `a` opens the form** | a focus/field state machine, six fields, and `Task → line` for the preview, which `Task::line()` mostly already is | **Reverses `docs/tui.md`**: the box is no longer "one field, not five labelled ones". The reasoning still holds for the 40-column fallback, so the decision narrows rather than dies |
| **3 · the one-line box stays** | nothing — it exists and is tested | No |
| **4 · `⏎` opens the form too** | plus span-splicing through `capture::parts` so untouched fields keep their bytes and their order | **Only safe with the splice.** Re-serializing breaks `architecture.md#round-trip-fidelity` |
| **5 · `s` stats** | one pure function, one draw function, one key. No format change, no write path | New screen, so a `decisions.md` entry. Nothing reversed |
| Section / Project field | plus insert-into-section in the writer, which only appends today | Reverses how capture writes |
| Split panes | the layout, and the narrow case | Reverses `design.md:108` |
| Description field | a second line per task | Reverses `format.md`, and the promise under it |

## Build order

Four steps, each shippable on its own, each leaving the tool working. **Steps 1
to 3 were accepted on 2026-08-12 and step 4 was accepted conditionally** — it is
built only if the splice test in Screen 4 passes, and dropped without loss if it
does not. The steps as checkable work are in
[todo.md](../todo.md#what-is-left):

1. **The dashboard (Screen 0).** No decision to reverse, no new key, and it
   changes how every screen after it looks. Do it first for that reason alone.
2. **The form for `a` (Screens 2 and 3).** The `docs/tui.md` reversal is written
   first, narrowing "one field, not five" to the fallback case it still governs.
3. **Stats (`s`).** Independent of the other three; it can also swap places with
   step 2 if the form turns out to be the bigger job, which it probably is.
4. **The form for `⏎` (Screen 4)** — and only with the span splice. If that
   turns out harder than it reads, `⏎` keeps the one-line box and nothing is
   lost; the add form is the part that was actually asked for.

The three left out — split panes, a Description field, a Section picker — are
not "later", they are a different promise about whose file it is. If one of them
is wanted anyway, the reversal goes into `docs/decisions.md` first and the code
follows it, which is the order `CLAUDE.md` asks for.

## Still open

1. **The `p` and `y` boxes.** Both ask one question and both open the one-line
   box today. A form for one question is not a form, so the proposal leaves them
   alone — but if the box is no longer what `a` opens, they are the only two
   places it appears at full width, and it may want a different label.
2. **`o`.** It is a second key for `a` today, for vim hands. It can stay the
   fast one-line box while `a` becomes the form, which gives both behaviours a
   key and costs nothing — or that is too clever and it just follows `a`.
3. **The date `tab` field.** Inside the box, `tab` opens the three-part date
   picker. Inside the form, `tab` moves between fields and `Due · pick…` is the
   picker. Same key, two jobs, two screens — probably fine, worth a look when it
   is built.
