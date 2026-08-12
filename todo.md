# todo

The build list. Decisions behind any of these live in [docs/](docs/README.md);
loose ends live in [notes.md](notes.md).

**v1 shipped as `v0.1.0`, the next batch as `v0.2.0`, the crates.io release as
`v0.3.0`, the date field as `v0.4.0` and the ruled columns as `v0.5.0` — all on
2026-08-11. The input box opening on today's date is `v0.6.0` and the colour
scheme — one colour, one job — is `v0.7.0`, tidied into `v0.7.1` and `v0.7.2`,
all 2026-08-12. The screen redesign is `v0.8.0` and is what is being built now.**
Steps 0–8 below are the record of how that was built and are kept for the
reasoning in them, not because there is anything left to do in them. The work
that is actually open is the short list directly under this line.

## What is left

Three open. Two are packaging and block nothing; the third is the screen, and it
is the release. Order inside the redesign is by dependency — every step after
the first is drawn on top of it — and everything else here is ordered by reach.
The ticked ones are kept here rather than moved down, because the reasoning in
them is about things that were asked for and are **not** being built — a key,
and a box split into fields. The second of those is the one the redesign
reverses, and it is reversed on purpose and in writing rather than quietly.

- [ ] **Improve the UI — the screen is correct and reads as unfinished.** Every
      field on it is right and there is nowhere on it to go: one screen, opening
      mid-sentence, drawn in three line systems that never touch each other — a
      group rule that stops in mid-air at column 39, column separators that start
      at column 40 out of nothing, and a blank row closing a group that was never
      a container. Density is not warmth: the screen already carries more than
      any of the mockups and still looks plainer than all of them.
      Five mockups were drawn against it — they are in `tui/` — and each is
      answered one at a time in [docs/redesign.md](docs/redesign.md), redrawn
      with ratodo's real data at ratodo's real widths. That is what makes the
      rejections worth anything: the split pane and the Description field are
      turned down from a **picture** of what they do to an 80-column terminal,
      not from a rule quoted at them.
      **This is `v0.8.0`**, not a patch: it reverses a decision that is published
      in [docs/tui.md](docs/tui.md) and it adds a screen.

      The principle the whole redesign hangs off, and the test for anything added
      to it later:

      > **ratodo must not hide the file the way a database hides one.**

      Nothing below may create a place where the tool knows something the file
      does not. That single line is why there is no Description field, no Project
      field and no split pane — not because they are hard, but because each one
      puts state somewhere the user cannot open in vim. It is also why the form's
      last word is a `PREVIEW` of the line the file will get, and why the main
      screen's last row is the selected task's raw line, byte for byte.

      Four steps, in this order, each shippable on its own and each leaving the
      tool working. **All four are the work.** Step 4 was written up as
      conditional and that was **corrected on 2026-08-12 by reading the code**:
      the machinery it was waiting on has been in `model.rs` since v0.2.0, and
      the danger it was hedging against is already in the product — see the step
      itself. The order is not preference: step 1 changes how every screen after
      it looks, so doing it second means drawing everything twice. The costs and
      the rejected alternatives for each are the table in
      [docs/redesign.md](docs/redesign.md#what-each-one-costs).

      - [x] **1 · The dashboard.** Screen 0, and the cheapest thing in the
            document: no new state, no new key, no new data — the numbers are the
            ones `ratodo status` already computes. Six changes to how the same
            rows are drawn. **Done 2026-08-12**, and one thing it cost that was
            not on the list: `COLUMNS_AT` came down five, because the box takes
            five columns off every row and eighty columns of terminal had to keep
            the columns it already had.
            - [x] **Every group becomes a nested box, and the column separators
                  become its `┬`/`┴` junctions.** **Done.** This is the one that matters and
                  it is a **correction, not a decoration**: nothing floats, every
                  stroke starts at a corner and ends at one. It costs **no rows**
                  — a heading, *n* tasks and a spacer is a top edge, *n* tasks and
                  a bottom edge — and four columns. A **folded** group stays a
                  bare rule rather than an empty two-row box: the difference
                  between a container and a line *is* the open/closed signal
            - [x] **What the box actually costs, which is not draw code.**
                  **Done** — `Row::Spacer` became `Row::GroupEnd`, every group
                  now emits one including the last and including the unnamed
                  one, a folded group drops it again, and the four selection
                  tests were updated rather than worked around.
                  `ui::rows` (`src/ui.rs:668`) flattens the agenda into a flat
                  `Vec<Row>` of `Header`, `Task` and `Spacer`, and its own doc
                  comment says *"the blank row between groups is half of the
                  design … so it is a row, not a margin"* — the rule is in the
                  **code**, not only in `design.md`. A box means `Header` and
                  `Spacer` become a top and a bottom edge, and the cursor has to
                  step over edges the way it steps over headers and blanks today
                  (`moving_steps_over_the_headers_and_the_blanks`). That is the
                  selection logic, which is the one thing on this screen with an
                  invariant on it — four tests are pinned to the current shape and
                  they are the specification, not an obstacle
            - [x] **`COLUMNS_AT` is 76, not 80.** The redesign says eighty
                  throughout — the width a terminal opens at — but the constant
                  the code breaks on is a row width, chosen so the breakpoint
                  sits above the row it measures. **Done, and it moved to 71**:
                  the box takes five columns off every row, so the constant had
                  to come down by five for eighty columns of terminal to keep
                  the columns it has today
            - [x] **The band at the top** — the date spelled out, stat tiles as a
                  big number over a small label, and a seven-cell week sparkline
                  off the `✓` stamps. Five rows, and the only thing on the screen
                  that says the tool has a memory. **Done.** The week comes from
                  `agenda::week`, which takes an iterator and `today` as a
                  parameter, so step 3's histogram is the same function. The band
                  **owns the counts while it is drawn** and the title bar spends
                  its right-hand side on the date instead; when the band goes the
                  counts and the progress bar come back
            - [x] **The date column stops repeating the heading.** **Done** —
                  one arm of `when`, and the width comes back with it: a group
                  where nothing is timed now spends no columns on the date at
                  all rather than a column of blanks. `today` inside
                  a group headed `TODAY` spends nine characters saying where it
                  already is. The rule: the column says what the heading does not
                  — `2d ago` under `OVERDUE`, the **time or nothing** under
                  `TODAY`, the day under `THIS WEEK`, the date under a `##`
                  section. Amends the date rule in [docs/tui.md](docs/tui.md)
            - [x] **Counts on every heading**, `BorderType::Rounded`, and keycaps
                  on the hint bar — `[a] add`, in brackets so it survives
                  `NO_COLOR` and reads as a keycap the way lazygit's bar does —
                  **done.** The count is one number now, not two:
                  `hidden: Option<usize>` became `count` plus `folded`, so the
                  fold stopped re-counting what `rows` already knew. A group
                  with no name gets no count. The keycaps cost `[y] copy` at
                  eighty columns and the separator went from two spaces to one
                  to pay for `[p] later` — measured, not assumed
            - [x] **The footer: the selected task's line from the file, raw.** One
                  row, and it is the row that says *this is a file and this is
                  your line in it* on the screen somebody stares at all day. It is
                  also the honest answer to "did the tool understand what I
                  typed", with no box open and nothing to press. **Done** — two
                  rows including the rule above it, because without the rule the
                  file's own line reads as one more task row. A task edited this
                  session shows the line that *will* be written, since `raw` is
                  only authoritative while `dirty` is false
            - [x] **First run** — two centred lines over the box `a` already
                  draws. **No ASCII-art logo**: this is a pane left open beside
                  the work, and a banner is charming exactly once. **Done**, and
                  the greeting is the first thing a short pane gives up: under
                  fourteen rows the box below it is what teaches. The box moved
                  from a hard-coded row to *under whatever was written above it*,
                  which is what a constant there was always going to cost
            - [x] **The widths, which are the part that gets skipped.** Band down
                  to one line of counts under 20 rows and gone under 16, footer
                  with it; boxes and frame gone under 34 columns. **Done and
                  pinned**, band drop order included; the band also needs 60
                  columns to lay tiles across and is not drawn below that at any
                  height. Drawn at 44 in
                  [docs/redesign.md](docs/redesign.md#all-of-it-at-40-columns),
                  with one question left open there: at that width three of the
                  row's columns are furniture, and a **left spine only** marks the
                  same extent for one. Worth drawing if 44 is the width actually
                  run
            - [x] **The ASCII forms, decided before anything is drawn.** Three of
                  the four new glyphs have none: `╭╮╰╯` (rounded corners),
                  `▁▂▃▅▆▇█` (the sparkline) and the `┬`/`┴` junctions.
                  **Done.** Corners and junctions are `+`. **The sparkline has
                  no ASCII form and is not drawn under one** — the honest answer
                  the item itself argued for, decided here rather than at the
                  assertion, and pinned by a test that renders the whole band
                  under `Glyphs::Ascii` and asserts the buffer `is_ascii()`. `src/ui.rs`
                  asserts the **whole buffer** `is_ascii()` in five places, so
                  this is not a polish item — it is the first thing that goes red.
                  The frame already falls back to `+ - |` and the junctions can be
                  `+`; the sparkline is the open one, and the honest answer is
                  that a seven-cell bar chart made of ASCII is not a bar chart —
                  **it goes, like the columns go below 80.** Decide it here rather
                  than at the assertion
            - [x] Buffer tests at 80 / 60 / 44 / 34, and `LC_ALL=C` still putting
                  nothing non-ASCII on the screen — the box-drawing set is new
                  furniture and the ASCII fallback has escaped through new
                  furniture twice before. **Done** as a sweep over those four
                  widths crossed with four heights, asserting every row is
                  exactly the width it was given and every frame closes on both
                  sides or is not drawn at all

      - [ ] **2 · `a` opens a form.** Screens 2 and 3. Six fields, which are
            exactly the six the format already carries — title, date, time, tags,
            priority and which list — and no seventh, because there is nowhere in
            a one-line format to put one.
            - [ ] **The reversal goes into
                  [docs/decisions.md](docs/decisions.md) first**, before a line of
                  code, the way `$work` and the date field did. It **narrows**
                  rather than dies: "one field, not five labelled ones" still
                  governs the 34-column pane, which is the case its arithmetic was
                  always about
            - [ ] **`PREVIEW`, with its own label and its own rule above it.** The
                  difference between a form that happens to show a line and a form
                  whose *conclusion* is a line. It is the same tokenizer read
                  backwards — `capture` builds a task from text, this renders text
                  from a task, and `Task::line()` is most of it already
            - [ ] **Typing still works.** `@thu`, `#home` and `!high` in the
                  question field parse as they always did and light the matching
                  radio as you type. One tokenizer, one truth — the day there are
                  two, the form and the box disagree about what gets written
            - [ ] **Radios `◉`/`○`, ASCII `(o)`/`( )`** — a difference in *shape*,
                  so the selection survives `NO_COLOR=1`. `▌` sits beside the
                  **control** that has the keyboard, not beside its label. Buttons
                  carry their own key: `[ ⏎ create task ]`
            - [ ] **The one-line box stays**, and this is what lets the form be as
                  big as it is. Under **15 rows or 40 columns** `a` opens the box
                  instead — a form that half-fits is worse than a box that always
                  fits, and the box is already built and already tested. `p` and
                  `y` keep it at every width: a form for one question is a form
                  nobody wants
            - [ ] **Three loose ends to settle while building it**, from
                  [docs/redesign.md](docs/redesign.md#still-open): whether `o`
                  stays the fast box while `a` becomes the form, whether the `p`
                  and `y` boxes want a different label now that the box is no
                  longer what `a` opens, and `tab` meaning *next field* in the
                  form and *date picker* in the box

      - [ ] **3 · `s` — the stats screen.** The one mockup that takes nothing
            away, and the answer to "there is only one screen". Every number is
            already in the file; `✓2026-08-11` is what `done_on` reads.
            - [ ] **`stats(&[Task], today) -> Stats`, pure, `today` a parameter**
                  — the same shape as `agenda` and testable for the same reason.
                  No clock inside it, ever
            - [ ] **No new dependency and no new format.** The bars are `█` and
                  `░`. `s` opens and `s` or `esc` closes; `1` `2` `3` are week,
                  month and year. A **screen, not an overlay** — nothing on it is
                  glanced at mid-task
            - [ ] **No boxes and no rules between the blocks, deliberately.** The
                  list is a grid because its rows are read across; this is five
                  paragraphs read one at a time. A statistics screen is exactly
                  where a tool starts trying to look like Grafana, and the
                  restraint gets spent here rather than argued about later
            - [ ] **One caveat on the screen, not in a doc:** a task ticked before
                  the completion stamp existed has no `done_on`, so it counts in
                  `31 done` and in nothing with a day attached. If that number is
                  large the screen says so rather than quietly under-reporting the
                  streak
            - [ ] **It does not get a file of its own.**
                  [docs/architecture.md](docs/architecture.md#module-layout) says
                  eleven files, flat, and means it. `stats` has `agenda`'s exact
                  signature and `agenda`'s exact purity, so it goes in
                  `agenda.rs` beside it and the module list does not move. A
                  twelfth file would be the first `mod.rs` pyramid brick
            - [ ] **`s` has room in both places it has to go, and the code says
                  how much.** The `?` overlay is ten keys plus two of border
                  (`src/ui.rs:2062`), and its own comment gives the ceiling:
                  twelve keys plus the border is fourteen, which is the pane it
                  must fit. So `s` gets **its own row at eleven**, with one row
                  still spare — no doubling up, no reshuffle. The hint bar is a
                  greedy fill over a fixed array of seven (`src/ui.rs:1534`)
                  ordered by how often a key is reached for, so `("s", "stats")`
                  goes on the **end** and the existing fill decides the rest:
                  no new logic at all
            - [x] **Keycaps are not free on that bar.** **Measured on
                  2026-08-12, before `s` was added to it.** With two-space
                  separators eighty columns lost `[p] later` *and* `[y] copy`;
                  with one space it keeps `[p] later`, and `[y] copy` returns at
                  eighty-eight. Sixty columns gets through `[⏎] edit` and loses
                  `[d] cancel`. Pinned in `the_bottom_line`, so adding
                  `("s", "stats")` to the array will move a number a test is
                  watching rather than something nobody notices
            - [ ] **What the screen does in a short pane.** Drawn at 80 and at 44
                  in [docs/redesign.md](docs/redesign.md#all-of-it-at-40-columns)
                  — both about twenty rows tall, and **neither says what happens
                  in ten**. Every other screen in this product has a documented
                  answer to that; this one needs the same, in the same drop order
                  (the two-column block first, then the daily labels, then the
                  histogram) rather than a scrollbar
            - [ ] A [docs/decisions.md](docs/decisions.md) entry — a new screen,
                  nothing reversed — a [docs/tui.md](docs/tui.md) section, since
                  that document owns every screen and the keymap, and unit tests
                  over `stats` including the empty list and the unstamped case

      - [ ] **4 · `⏎` opens the form too — last, and smaller than it was
            written up as.** The redesign hedged this step on the risk that a
            form which parses six fields and re-serialises them would turn
            `- [ ] #ops rotate the keys !high @2026-08-10` into the canonical
            order having edited nothing. Three things read out of the code on
            2026-08-12 change that arithmetic, and they are worth stating because
            each one was assumed the other way round:
            - **`Task::splice` already exists** — `model.rs:224`, private, taking
              a predicate and an `Option<&str>` — and it *is* the four cases:
              replaced in place, removed **with one adjacent space** (one, both
              ways, deliberately), appended at the end of the line, or nothing.
              `postpone` has used it for `p` since v0.2.0, so moving one field
              and leaving the rest of the line where the user put it is not a
              thing to build. It even carries its `cargo mutants` equivalent
              mutant written down in the doc comment.
            - **`parts` claims every word**, title words included, as
              `Part::Text`. The write-up called the title "everything `parts` did
              not claim"; its spans are known like any other token's.
            - **The danger is already shipped.** `main.rs:882` writes nothing
              when the typed text is unchanged, and calls `retype` when it is
              not — and `retype` rebuilds the body canonically. `model.rs:474`
              is the test that says so: `* [x] wash up @2026-08-12 #home`,
              edited, comes back as `* [x] wash up tonight #home !high`. So
              today's `⏎` is byte-perfect only in the case where it writes
              nothing at all. **This step does not add that risk, it takes it
              away** — which is the opposite of the reason it was going to be
              dropped.
            - [ ] **Splice per field — but by range, not by predicate, and this
                  is the trap.** `splice` takes a predicate and returns the
                  **first** word that matches it. There is no `is_time`, and
                  `16:00` in a *title* would be found before the real time and
                  spliced instead. `is_due` has the mirror of it: it insists on
                  `@YYYY-MM-DD`, so a hand-written `@friday` is not found and a
                  Due edit **appends a second date** to the line. Both go away by
                  taking the range from `parts` — which knows a time only counts
                  directly after a date, and knows which `@` won — so step 4 adds
                  a `splice_at(range, to)` beside the predicate version rather
                  than three more predicates. The predicate form stays for
                  `postpone` and the done stamp, which have no line position to
                  work from
            - [ ] A field the form did not touch never reaches either, so its
                  bytes, its position and the whitespace either side stay the
                  user's — which is more than `⏎` promises now
            - [ ] **The two that `splice` cannot do as written, and they are the
                  real work.** It finds **one** word: tags are a set, so adding
                  one and removing another is two calls and an order to decide;
                  and the title is a **run** of `Text` words that the user may
                  have interleaved with tokens — `rotate #ops the keys`. Rule:
                  replace the run when the `Text` words are contiguous, and when
                  they are not, that one edit falls back to today's `retype`.
                  Nothing regresses, because `retype` is what happens now
            - [ ] **The test, which is now a regression test and not a gate.**
                  `tests/fidelity.rs`: open the form on every fixture including
                  the gnarly ones, change nothing, save — byte for byte. Then one
                  field at a time, asserting every *other* byte survived. Plus
                  `cargo mutants --timeout 90` per [CLAUDE.md](CLAUDE.md), since
                  this touches `model` and `capture`
            - [ ] **[docs/tui.md](docs/tui.md) says what an edit does to the
                  field order, because right now it does not.** *"Saving replaces
                  exactly that"* is true of the body and reads as true of the
                  bytes, one sentence before it invokes
                  [round-trip fidelity](docs/architecture.md#round-trip-fidelity).
                  A reader cannot tell from it that editing a word normalises the
                  line. Fixed by this step, and worth writing down either way

      - [ ] **The docs owe more than the one reversal, and the extra ones were
            missed on the first read.** [docs/redesign.md](docs/redesign.md)
            names `design.md:108` — one layout, no split panes — and stops there.
            [docs/design.md](docs/design.md#rules) has **three more rules the
            redesign walks into**, and each is either amended in writing or the
            drawing changes:
            - [ ] **"Generous whitespace. The blank lines between groups are half
                  of the design."** The group box *eats* that blank line — it
                  becomes the bottom edge. The row arithmetic is identical, which
                  is the redesign's argument, but "identical arithmetic" is not
                  the same claim as "the whitespace was half the design and we
                  are spending it on a border". This is the single largest thing
                  to look at on a real screen before step 1 is called done
            - [ ] **"A rule between two columns, and nowhere else."** The box's
                  top and bottom edges are rules that are not between two
                  columns. The rule as written forbids exactly what the grid
                  correction does, and it was written to stop three characters of
                  noise per row — so the amendment has to say why an edge is not
                  that
            - [ ] **"One layout, no split panes. No sidebar, no modal."** The
                  form in step 2 is a centred overlay, which is a modal, and
                  [docs/tui.md](docs/tui.md) currently calls the help overlay
                  *the one overlay in the product*. Two documents will disagree
                  the moment the form is drawn
            - [ ] **No new theme role**, and this one is a constraint rather than
                  a reversal: the band, the boxes and the bars are `border` and
                  `accent`, the bars in `done`. If any of them wants a colour of
                  its own then [docs/theming.md](docs/theming.md) grows a key and
                  every built-in theme grows a line, which is a much bigger
                  change than it looks from the screen
      - [ ] **`src/ui.rs` is 5,654 lines and this adds three screens to it.**
            [docs/architecture.md](docs/architecture.md#module-layout) says
            eleven files, flat, and `ui.rs` is already three times the next
            largest. Flat is the rule and a `ui/` directory is the pyramid that
            document forbids — but **one more flat file is not a pyramid**, and
            the form is the natural seam: it has its own state machine, its own
            fields and its own keymap, and it is the one part of this work that
            can be read without the list. Either way `architecture.md`'s file
            list changes, so decide it at step 2 rather than discovering it at
            seven thousand lines
      - [ ] **`assets/demo.gif` shows the old screen**, and it is the first thing
            on the README and on crates.io. `scripts/demo.py` re-records it but
            needs kitty, menyoki, ffmpeg and X11, so it is the maintainer's
            machine and not a CI step. Last thing before the tag, once the screen
            has stopped moving

      **Not in this, and not "later" either:** split panes, a Description field
      and a Section/Project picker. Each is a different promise about whose file
      it is — the first reverses [docs/design.md](docs/design.md), the second
      reverses [docs/format.md](docs/format.md), and the third means teaching the
      writer to *insert* into the middle of a file it only appends to today,
      which is the one place fidelity is won or lost. If one is wanted anyway,
      the reversal goes into [docs/decisions.md](docs/decisions.md) first and the
      code follows it.

      **The release is the one in [CLAUDE.md](CLAUDE.md), not a green suite.**
      `cargo install --force --path .`, then **stop** and let the maintainer look
      at it in their own terminal. Driving it on a pty is evidence that it
      *works*; this whole item is about whether it *reads*, which is the half a
      publish cannot take back.

- [x] **Copying a task means retyping it.** A task that is nearly one you already
      have — same tag, same shape, different day — had no way in but `a` and the
      whole line again. `y` opens the input box pre-filled with the selected
      task, as a new one, and `⏎` saves it. Asked for as `y`/`p` with a register;
      `p` is taken and the register bought nothing, because a capture lands in
      the capture target wherever the cursor is. See
      [docs/decisions.md](docs/decisions.md#settled)

- [x] **`$list` in the input, and the input as four fields.** Asked for on
      2026-08-11, and it is two pieces that arrived in one sentence. The first
      shipped. The second was measured and **rejected**, and what it was really
      after shipped instead as one dim row.
      - [x] **`$work` routes the capture.** `a` wrote to `todo.md` and nothing
            else — [cli.md](docs/cli.md#several-lists) rule 4 — so capturing into
            `work.md` meant leaving the TUI for
            `ratodo --file ~/.config/ratodo/work.md add '...'`. `$` puts it in
            the box: a fourth sigil beside `@` `#` `!`, read by the one
            tokenizer `capture::parts` already is, previewed as `→ work.md` the
            way `@thu` is previewed as a date. Rule 4 said a fixed target on
            purpose — "`a` must not mean a different file depending on what the
            cursor happens to be over" — and `$` does not break that: the target
            is a **word the user typed**, not the cursor position. That
            distinction is the whole reversal and it went in
            [docs/decisions.md](docs/decisions.md#a-capture-always-goes-to-todomd--work-picks-the-list-2026-08-11)
            before the code did —
            **done**, and it works on `ratodo add` as well as in the box. The
            word never reaches the file, `$work` and `$work.md` are one list,
            the first `$` wins, `$50` is money, and a `$` on `⏎` is refused
            rather than swallowed, because `capture` drops the word and silence
            would eat the title. Two of the open questions below went with it.
            One thing the suite would not have caught and running it did: the
            preview nagged `no list w.md` through every keystroke of `$work`, so
            it now waits until the word can no longer become one of the lists —
            the same rule the date warning already had
      - [x] **The box becomes four fields with `tab` between them** — **no.**
            The reversal was drafted and then measured, and the measurement
            killed it. It is **five** fields once `!high` is counted, the box is
            `min(70, pane − 4)` wide, and a 34-column pane leaves **28 columns**
            — five fields and four separators is three characters a field. At 60
            it is forty-two columns and `2026-08-13` alone is ten. Drawable at
            eighty and nowhere else, which buys a second input mode for the
            narrow pane the product was designed around.
            The invariant argument is the one that settles it: keeping one
            tokenizer means joining the fields back into a line for
            `capture::parts`, at which point the boundaries are decoration over
            the same string, paid for with a focus state, `tab`/`shift-tab`,
            five carets and five scroll windows. Not joining them is a second
            parser, and two parsers of the same box eventually disagree about
            what it will write. A tag *field* also cannot hold `#home #work`,
            which one line already does. Written up in
            [docs/decisions.md](docs/decisions.md#settled)
      - [x] **What it was actually after — discoverability — shipped instead.**
            An empty box now reads `@thu #home !high $list` in the dim, exactly
            as the empty `p` box reads `how long? 2 3d 1w fri`. Twenty-two
            columns, so it fits the narrowest pane the design promises; gone the
            moment there is anything to report; `$list` only when there is more
            than one list to address. No mode, no keymap, no second parser
      - **What went with the rejection:** `p` keeps its one field, the
        five-versus-four question is moot, and the two `$` questions were
        answered by the first piece — `$nosuchfile` is refused before the write
        rather than created, and with one list `$` parses the same and refuses
        anything but that list
- [x] **A date that does not exist is accepted in silence.** Found in use on
      2026-08-11: `@2026-13-45` resolves to nothing, so the whole word falls
      back to being part of the title — the file gets
      `- [ ] task @2026-13-45`, the task keeps no date, and neither the live
      preview nor the status line ever says so. The fallback itself is correct
      and stays: a word we did not understand is the user's text and we do not
      eat it. What is missing is that the preview goes quiet in exactly the
      moment it should speak.
      - [x] **The preview says so.** An `@` that can never become a date gets
            named under the input, in the colour the bottom line warns in.
            *Can never* rather than *does not yet*: the line redraws on every
            keystroke, and one that fires on `@2`, `@20`, `@202` on the way to
            `@2026-08-20` is one nobody reads by the time it is right. Caught
            by running the binary, not by the suite — the first version nagged
            through ten presses. See
            [docs/decisions.md](docs/decisions.md#settled)
      - [x] **The field-by-field date entry** — `↑ ↓` on the part under the
            cursor and eight digits filling `DD MM YYYY`, which is a keymap and
            a widget, not a message, and makes the invalid state unrepresentable
            rather than merely detectable. What it would cost is worked through
            in
            [notes.md](notes.md#the-date-field--a-proposal-not-a-decision-2026-08-11),
            and it became an entry in
            [docs/decisions.md](docs/decisions.md#settled) before a line of it
            was written —
            **done, as `tab`**, to the shape those notes argued their way to:
            the text box is untouched until you press it, `esc` gives it back,
            and the fast path is still `@thu`. The day is clamped to the month
            it is in, so the 31st of January arrowed into February is the 28th —
            the 29th in a leap year, because the length of a month is asked of
            the calendar and not of a table — and a month of `13` is
            unreachable rather than refused. It takes the same `tab` in the `p`
            box, where it writes the bare date `p` accepts past its horizon.
            The brackets around the focused part are what carry it under
            `NO_COLOR`, and they keep the row the same width wherever the
            cursor is
- [x] **`cargo publish`** — the email was verified on 2026-08-11 and the crate
      went up as **v0.3.0** the same day: `cargo install ratodo` is the install
      line now. It went out as a minor rather than a patch because the six
      commits after the `v0.2.0` tag include a breaking one — the `d`/`X` swap —
      and publishing `0.2.0` from a tree the tag does not point at would have
      been permanently wrong on crates.io
- [x] **The six integration tests that steer the binary with `$XDG_*`** —
      `#[cfg(unix)]` on each, the maintainer's call on 2026-08-12. `tests/cli.rs`
      points ratodo at a scratch directory by setting `XDG_CONFIG_HOME` and
      friends; `directories` ignores those off Linux and answers from the Known
      Folder API, so on Windows those six read the developer's *real* config and
      data directories. Same class as `which_files_count_as_lists`. **The gate is
      a gate, not a fix** — the rejected alternative was giving the config
      directory the `Derived` treatment (`dirs()` is read deep inside `lists`,
      `default_path`, `active_theme`, `backup_dir` and `ics_path`; resolving it
      once in `main` would make all six portable, which is the argument
      `src/main.rs` already makes for the backup and calendar paths). That is a
      refactor, and the six keep no Windows coverage of where the files land
      until somebody does it
- [ ] **Thunderbird** — the third and last calendar data point. Its Tasks view is
      a different code path from the month grid and is where a VTODO would land.
      `todoman` displays the file correctly and `khal` ignores it; Thunderbird is
      the one that decides whether the table in
      [docs/calendar.md](docs/calendar.md) is finished or still guessing. It is
      also what tells us how big an audience `--as-events` would actually buy,
      and that flag is already on the [v2 roadmap](docs/roadmap.md)
- [x] **`flake.nix` and an AUR `PKGBUILD`** — `rustPlatform.buildRustPackage` and
      a `PKGBUILD` against the tag. NixOS users will not `cargo install` into a
      profile, and Arch is the platform this was written on. Both pin a released
      version, which is why they came after the tag rather than before it —
      **done, and the two are not equally verified.**
      `packaging/PKGBUILD` was **built here**: `makepkg` against the `v0.3.0`
      tarball and its real sha256, `check()` running the suite, and the resulting
      package holding the binary, the three completions, the licence and the
      docs where `pacman` expects them. It also left `~/.local` alone, which is
      the failure this project has had once before. `.SRCINFO` is generated and
      committed beside it. What is **not** done is the AUR submission itself —
      that is an account and a push to `aur.archlinux.org`, not a file.
      `flake.nix` is **written and unbuilt**: there is no `nix` on this machine
      and no container runtime to borrow one from, so it has never been
      evaluated. It reads the version out of `Cargo.toml` and pins dependencies
      with `cargoLock.lockFile` so neither can rot; there is no `flake.lock`,
      for the same reason there is no build. The README says so rather than
      claiming otherwise. First person with `nix` closes this
      - [ ] **`nix build` once, by somebody who has nix.** Then the caveat comes
            out of the README and a `flake.lock` goes in

Open questions that block none of the above are in
[docs/decisions.md](docs/decisions.md#open-questions).

## 0 — Setup

- [x] Rust toolchain (1.97.1)
- [x] `git init`, remote configured
- [x] Verify the name is free — crates.io ✅, GitHub ✅, PATH ✅ (see [docs/naming.md](docs/naming.md))
- [x] Design record written up in `docs/`
- [x] `cargo init --name ratodo`
- [x] `Cargo.toml`: GPL-3.0, MSRV 1.88 — deps added per step, not all seven up front
- [x] Verify truecolor: `printf "\x1b[38;2;203;166;247mmauve\x1b[0m\n"`
- [x] Install a client — to see the `.ics` displayed, not just parsed —
      **done**: `todoman`. khal was the obvious guess and is the wrong tool; it
      draws events, we write todos

## 1 — Fixtures (no terminal needed)

- [x] `tests/fixtures/simple.md` — copy of [docs/examples/todo.md](docs/examples/todo.md), kept in sync by a test
- [x] `tests/fixtures/gnarly.md` — the deliberately awkward one
- [x] `crlf.md`, `no-final-newline.md`, `empty.md` — the byte-level edge cases
- [x] Expected parse results asserted in `tests/fidelity.rs`

## 2 — parse + write (no terminal needed) ← the heart of the product

- [x] `model.rs`: `Doc` / `Line` / `Item` / `Task`, each line keeping its own ending
- [x] `parse.rs`: line → `Task`. **The raw line is always kept**
- [x] `parse.rs`: `@date`, `@date HH:MM`, `#tag`, `!priority`, word-by-word, no regex
- [x] `capture.rs`: shorthand dates — `@today @tomorrow @mon…@sun @3d @2w` → ISO
- [x] `write.rs`: if `dirty == false`, write the raw line back untouched
- [x] `write.rs`: atomic write — temp → `fsync` → `rename`, `.bak` beforehand
- [x] `write.rs`: mtime check — if it changed since we read it, refuse and say so
- [x] **Round-trip test:** `parse(render(parse(x))) == parse(x)`
- [x] **Fidelity test:** toggling any one task changes exactly one byte, on every fixture
- [x] `ratodo list` and `ratodo add` → the product works from here on
- [x] `tests/property.rs`: 4000 generated documents, the generator its own oracle
- [x] `cargo mutants` clean over `parse` / `write` / `model` / `capture` / `text`

## 2.5 — What two design reviews found (no terminal needed)

Cheap fixes to things already built, plus the decisions that came out of the
2026-08-10 reviews. Details in [docs/decisions.md](docs/decisions.md#reversed),
the abandonment risk in [docs/risks.md](docs/risks.md).

- [x] `write.rs`: `.bak` goes to `~/.local/state/ratodo/`, not next to the list —
      a `.bak` in a dotfiles repo means `git status` is dirty after every capture.
      The backup directory is a **parameter**; `write.rs` reads no environment
- [x] `write.rs`: the backup is named after the whole target path, so two `--file`
      lists cannot overwrite each other's insurance
- [x] `model.rs`: `push_task` inserts after the last task, not at EOF. In a file
      ending with a table or `---` the captured task landed outside every `##`
- [x] `main.rs`: the empty-list message goes to **stderr**, so `list | wc -l` is honest
- [x] `main.rs`: `$RATODO_FILE` between `--file` and the XDG default
- [x] Single-quote every shell example in the README and docs: `!high` inside
      `"…"` is history expansion in bash and zsh, and the add never happens
- [x] Colour off when stdout is not a TTY. **Nothing to gate:** `ratodo list`
      prints no colour at all — the same bytes down a pipe as on a screen — and
      the only thing that emits any is the TUI, which already opens on a TTY and
      nowhere else. Settled and written up in [docs/cli.md](docs/cli.md);
      colouring `list` would be a feature, not this rule

## 3 — agenda + the scriptable surface (no terminal needed)

- [x] `agenda.rs`: `agenda(&[Task], today) -> Vec<Group>` — `today` is a **parameter**
- [x] Group tests: overdue / today / this week / later / undated
- [x] Boundary tests: exactly today 00:00, exactly +7 days, a past year, an invalid date
- [x] `list --tag` / `--prio` — the agenda says nothing about undated tasks, and
      most of a developer's list is undated
- [x] `list --porcelain` — tab-separated, stable, no colour. The contract behind
      `ratodo done "$(ratodo list --porcelain | fzf | cut -f3)"`
- [x] `ratodo status` and `--json` — `class` is the field waybar keys its CSS off
- [x] `status` exits non-zero when something is overdue
- [x] `done "<text>"`: unique match required; ambiguous → print candidates, exit 2,
      **write nothing**

## 4 — ratatui (the genuinely new part)

- [x] **Panic hook on day one** — a TUI that panics in raw mode wrecks the terminal.
      `ratatui::try_init` installs one that restores raw mode and the alternate
      screen; `Terminal`'s Drop puts the cursor back
- [x] A dumb list: print the task titles, `↑↓`, quit with `q`
- [x] **No fixed FPS** — draw on events; a wake-up with nothing to do draws
      nothing. Measured: 40 wake-ups in 20 idle seconds, zero CPU ticks
- [x] The TUI only opens on a TTY — `ratodo | wc -l` lists instead
- [x] Event loop: `crossterm::event::poll` + notify's mpsc channel. *(Was a
      blocking channel with a reader thread; reversed so `e` could exist — see
      [docs/decisions.md](docs/decisions.md#reversed))*
- [x] inotify: re-read when the file changes from outside. The watch is on the
      **directory**, because every safe writer renames over the file
- [x] The cursor stays on its task across a reload — by identity, see step 6

## 4.5 — ics (was step 3; moved behind the TUI)

Still v1, still one-way. It serves seed point 6, but the people it reaches are
Thunderbird and GNOME users, not the tiling-WM audience of seed point 2 — so it
does not get to block the screen that audience actually opens. See
[docs/decisions.md](docs/decisions.md#reversed).

- [x] `ics.rs`: VTODO output (~30 lines of string formatting, no crate)
- [x] `ics.rs`: stable UID, CRLF, 75-octet line folding
- [x] `ratodo sync`, and a regenerate after every capture
- [x] Real verification: the output parsed by Python's `icalendar` — a different
      implementation of the same RFC. Comma escaping, folding of a Turkish and
      emoji title, and the floating time all came back intact
- [x] The other half of it: a client actually **displaying** the file, which is
      what catches one that quietly ignores VTODO —
      **done**: it caught one on the first try. `todoman` lists all five tasks
      with dates, times, categories and priorities, and a change made in ratodo
      is there on the next `todo list` with no sync step. `khal` shows none of
      them, and a hand-written VEVENT in the same directory with the same config
      *did* appear — so it is VTODO being ignored, not our file being wrong.
      [docs/calendar.md](docs/calendar.md) had khal down as ✅ on nothing more
      than "it is file-based"; corrected
- [x] **`cargo test` rewrote the real `~/.local`** — `write_back` resolved the
      backup and calendar paths from the environment, so in-process tests
      regenerated the developer's own `todo.ics` from a fixture and left a
      `.bak` per case in `~/.local/state/ratodo`; `tests/cli.rs` set
      `XDG_STATE_HOME` and forgot `XDG_DATA_HOME` —
      **done**: both paths are resolved once in `dispatch` and carried, the
      integration tests set all four XDG directories through one helper, and a
      test pins that a write lands where the caller pointed it. Found because a
      calendar being read went empty, not by the suite
- Thunderbird is the one client still unchecked — see
  [What is left](#what-is-left)

## 5 — Theme

- [x] `theme.rs`: the `Theme` struct, 11 role keys
- [x] Built-in themes as `const` tables: catppuccin-mocha (default), catppuccin-latte, gruvbox-dark, nord, dracula, terminal
- [x] Every built-in ships `background = none` — transparency is opt-out, not opt-in
- [x] `theme.conf` parser (no serde): `key = value`, `#` comments
- [x] Value forms: `#rrggbb`, `#rgb`, ANSI index, ANSI name, `none`
- [x] Precedence: built-in → `theme =` → individual keys → `--theme` → `NO_COLOR`
- [x] Bad input never aborts: warn on stderr, fall back
- [x] `ratodo theme list` and `ratodo theme dump`
- [x] The theme reaches the screen — the selected row keeps its own colour, so
      an overdue task is still red under the cursor
- [x] Verify `background = none` by eye in a transparent terminal. Asserted in a
      test (every built-in ships `Color::Reset`), confirmed in a pty by the
      absence of a background escape, and looked at on 2026-08-11

## 6 — Assemble and apply the design

Screens and keymap: [docs/tui.md](docs/tui.md).

- [x] Draw the grouped agenda with header rules, `○ ✓ !` symbols, `▌` selection
- [x] ASCII fallback: `[ ]` `[x]` `[!]`, `>` selection — chosen from the locale,
      and it takes the frame and the punctuation with it
- [x] The bottom line: hints, results, warnings and the input field
- [x] Keys: `j k g G ctrl-d ctrl-u` · `spc` · `a o ⏎` · `d u X` · `h l z` · `e` ·
      `r` · `?` · `esc` · `q`
- [x] `h`/`l` fold the group under the cursor — lf/ranger/yazi muscle memory, not
      "fold LATER". A collapsed group is selectable, which is the only way back
- [x] Input mode: `⏎` save, `esc` cancel, `ctrl-c` cancel (**never quit**), and nothing else can open it
- [x] **Live parse preview** under the input — `@thu` resolves as you type. It
      costs the list a row while it is open — see
      [docs/decisions.md](docs/decisions.md#reversed)
- [x] `X` deletes immediately; `u` undoes delete / toggle. Edit joins it with the
      input mode. *(Was `d`; swapped with cancel on 2026-08-11 so that the key
      taking a line out of the file is the one asking for shift — see
      [docs/decisions.md](docs/decisions.md#reversed))*
- [x] Write-conflict line with `r` reload. A refusal while the input is open
      re-reads by itself and hands the typed text back to the field
- [x] Selection survives reload — by identity, not row index. `Task::identity`
      is the section and the title, and it is the same one the `.ics` UID is
      built from, so "the same task" has one definition
- [x] A toggled task does not change position until the next reload
- [x] Empty state with the file path and a worked example
- [x] `?` help overlay — only the keys that are built, and `esc` closes it
- [x] Progress on the right of the title rule — eight cells and a `3/8`, green
      because green already means finished. Only once something is ticked; the
      bar gives way below 60 columns and the count stays
- [x] Width breakpoints: ≥60 / 34–59 / <34, in the documented drop order
- [x] Height under 10 rows: collapse the hint bar
- [x] `NO_COLOR=1` on a bare TTY still reads correctly
- [x] `:` and `/` answer on the bottom line instead of doing nothing
- [x] `clap`: `ratodo` · `add` · `list` · `done` · `sync` · `theme`
- [x] `--file` and `--theme` global flags
- [x] Check column alignment with non-ASCII and emoji — display columns via
      ratatui's own width, so no eighth dependency

## 7 — Release

- [x] README: khal and Thunderbird subscription steps
- [x] README: `set autoread` for people with nvim open on the file in another pane
- [x] README: a `.chezmoiignore` note — `chezmoi apply` overwrites a live `todo.md`
- [x] `completions/ratodo.{bash,zsh,fish}` — hand-written, no `clap_complete`, and a
      test asks the binary what it answers to so they cannot rot quietly
- [x] Time a cold start; the `$mod+t` scratchpad makes it a spec, aim under 50 ms
      — measured 1.2 ms median for `list`, 20 runs
- [x] `cargo publish --dry-run` — 44 files, 157 KiB compressed. `exclude` keeps
      the machinery of working on the project out of it (`CLAUDE.md`, `.vscode`,
      `cliff.toml`, `scripts/`, `notes.md`, `todo.md`); `docs/` stays
- [x] Tag `v0.1.0`, generate the changelog with git-cliff — tagged, and a
      GitHub release with the binary attached

## 8 — Visual polish (deliberately last, and blocks nothing)

The screen works and is documented; this is the pass for making it feel less
plain. It comes **after** the tag on purpose — none of it is a bug, and shipping
a working v0.1.0 beats holding one back for looks.

The frame every item here has to fit is [docs/design.md](docs/design.md#rules),
and it is a tight one: one accent colour plus greys, two levels of hierarchy and
no third, one layout, nothing that depends on a Nerd Font, no meaning carried by
colour alone. Anything that needs a rule bent gets written up in
[docs/decisions.md](docs/decisions.md) first — an item on this list is a
**candidate**, not a decision.

The standing test for each: does it tell the reader something, or does it just
decorate? The progress bar earned its place by the first; the second is how a
side pane turns into a dashboard nobody leaves open.

- [x] Progress on the right of the title rule — the one already done, as the
      worked example of the standard above
- [x] **Wide panes waste their width.** Past 60 columns nothing new appears, the
      gap in the middle just stretches. A fourth breakpoint could show the full
      date and the section a dated task came from —
      **done as columns at ≥ 80**: date, priority and tags start in the same
      place on every row and the group rule stops at the title column. The
      section a task came from is still not shown; it is a second decision, not
      part of this one. See [docs/decisions.md](docs/decisions.md#reversed)
- [x] **Dated groups and the file's own `##` sections look identical.** `OVERDUE`
      and `Work` are both a bold word plus a rule, though one is ours and one is
      the user's. Careful: "two levels of hierarchy, there is no third" —
      **done**: the user's headings keep the `##` they have in the file. No
      second colour and no third level; see
      [docs/decisions.md](docs/decisions.md#settled)
- [x] **A completed task still shows how late it is** — `✓ review the deploy PR
      … 1d ago`. It is finished; the lateness stopped being true —
      **done**: it shows the plain date (`Aug 8`) instead, and stays in
      `OVERDUE`, where membership was always positional
- [x] **`!high` is easy to miss**, sitting dim next to the tags. It is the one
      field the user typed to mean *urgent* and the screen barely says so —
      **done**: it is bold and in the row's own colour. Weight, not a twelfth
      theme role, so it still reads under `NO_COLOR`. `!med` and `!low` unchanged
- [x] Empty screen and `?` overlay — both correct and both plain —
      **done**: the empty screen's example moved into the box `a` actually
      opens, drawn by the same code, so the live parse under it resolves
      `@tomorrow` before a key is pressed; under ten rows it goes back to being
      a line. The overlay's exit moved to the bottom border — no row spent, and
      the box is back to twelve on a fourteen-row pane. See
      [docs/decisions.md](docs/decisions.md#settled)
- [x] Decide what to do about the help overlay's `↓ ↑` under a non-UTF-8 locale.
      The main screen goes fully ASCII and the overlay does not; the buffer test
      never covered it because it does not open the overlay —
      **done**: `down up` and `ret`, and the test now opens the overlay. Two more
      escapes went with it: the `…` on a cut title and the `·` in the input
      preview. `LC_ALL=C` now puts nothing non-ASCII on the screen

## 9 — After v0.1.0, and what became v0.2.0

Tagged 2026-08-11. The three below are the release; the packaging and publishing
that were on this list have moved up to [What is left](#what-is-left).

- [x] **Several lists in one agenda** — every `*.md` in the config directory is
      read, the undated headings say which file they came from, a change goes
      back to the file it came from with that file's own mtime check and backup,
      and a capture goes to `todo.md`. The file is attached to a task only when
      there is more than one, so a single-file setup keeps its identities and its
      calendar UIDs. See [docs/cli.md](docs/cli.md#several-lists)
- [x] **A finished task is grey, and finishing one says nothing back.** Green is
      reserved for completed — [docs/design.md](docs/design.md) — and the only thing
      wearing it is the progress bar. Ticking a task should show in the row, and
      the file should record *when*: `✓2026-08-11`, a fourth field beside `@`,
      `#` and `!` — **done**: the row is green, the stamp is written and taken
      back off by unticking, and the date column on a finished row shows the day
      it was finished rather than the deadline that stopped applying
- [x] **There is no third state.** A task that is neither done nor still wanted
      can only be deleted, which loses the record of having decided against it.
      `- [-]` — the Obsidian/Logseq convention — with `d` to set it, out of the
      counts and never overdue — **done**: `✗` on screen in the grey a finished
      row gave up, out of the counts, never overdue, never exported, and `d`
      takes it back. `x` itself stays unbound, for the reason it always was.
      *(Shipped on `X`; swapped with delete on 2026-08-11)*
- [x] **Pushing a date out means retyping the whole line.** `⏎` reopens the
      input for a task whose only problem is that it is not today's problem.
      `p` should ask for how long — `2`, `3d`, `1w`, `fri` — and move `@` alone,
      keeping the time and everything the parser did not understand —
      **done**: the same input box with a different question, and a preview that
      answers it with the day it lands on

## Open questions blocking nothing

Tracked in [docs/decisions.md](docs/decisions.md#open-questions): whether a
completed task stays in place or moves to a `## Done` section, and whether a
list per repository needs `ratodo` to walk up the tree. The other four the code
answered on its way past them, and they have moved to
[resolved](docs/decisions.md#resolved-questions).
