# Decisions

Three lists: what is settled, what was rejected, and what is still open.

## Settled

- ✅ **Name: `ratodo`** — ratatui + todo. The kinship is in the name, but the name
  describes the product, not the library. Rationale and rejected candidates in
  [naming.md](naming.md). Availability verified 2026-08-10.
- ✅ **Storage: a single Markdown file**, metadata inline. Because the user is
  already writing Markdown in vim, the file is still useful without the tool, and
  `git diff` is meaningful line by line.
- ✅ **Location: `~/.config/ratodo/todo.md`.** A deliberate XDG deviation — being
  in your dotfiles matters more than following the standard.
- ✅ **Round-trip fidelity.** Raw lines are preserved; an untouched line is written
  back byte-for-byte. This is the technical form of the whole product promise.
- ✅ **Writes are atomic, with a `.bak`.** We have to write, so the guarantee is
  not "cannot break anything" but "cannot lose anything".
- ✅ **On a concurrent edit, warn — do not merge.** A wrong merge loses data
  silently.
- ✅ **Calendar: one-way `.ics`, VTODO.** We generate the file; subscribing is the
  user's job. It is built **after** the TUI, not before — *(changed 2026-08-10,
  see below)*.
- ✅ **v1 scope: capture, check off, and narrow down.** `/` search and
  `ratodo archive` go to v2; `list --tag` / `--prio` stayed in v1.
  *(Changed from "filter and search go to v2" — see below.)*
- ✅ **`list --today` narrows the groups, not the tasks.** `agenda()` has already
  done the date arithmetic; a second calculation inside `list` is how two answers
  start to drift. **Overdue stays in it** — late work is today's work, and a
  `--today` that hides yesterday's miss is a lie on a morning check. See
  [cli.md](cli.md#list---today). *(0.8.2, 2026-08-21.)*
- ✅ **The tool is scriptable, not just interactive.** `ratodo status` for a bar,
  `list --porcelain` for `fzf` and `grep`. A tool this audience cannot pipe is a
  tool that stays outside their setup. See [cli.md](cli.md).
- ✅ **Only `todo.md` lives in the user's directory.** `.bak` is derived and goes
  to `~/.local/state/ratodo/` — writing it next to the list leaves an untracked
  file inside somebody's dotfiles repo after every capture.
- ✅ **`$RATODO_FILE` overrides the default path**, below `--file`. Two lines, and
  `direnv` then gives per-repository lists for free.
- ✅ **Built-in themes default to `background = none`.** Transparency survives
  unless the user asks for a painted background, not the other way round.
- ✅ **`ratodo done "<text>"` requires a unique match.** On an ambiguous one it
  prints the candidates, exits non-zero and writes nothing. Silently ticking the
  wrong task is the exact trust break round-trip fidelity exists to prevent.
- ✅ **`e` → `$EDITOR`.** An escape hatch, ten lines, exactly right for the audience.
- ✅ **One view mode (agenda).** Two modes means state management, key conflicts
  and two drawing paths.
- ✅ **Vim keys, not vim modes.** `j k g G ctrl-d o z`, but no normal/insert
  distinction, no command mode, no pending-operator state. The whole state
  machine is list mode plus an input mode that only exists while adding or
  editing. See [tui.md](tui.md).
- ✅ **One multiplexed bottom line** carrying hints, results and warnings, and
  never changing size. Two things cover the list, both of them opened by a key
  and closed by `esc`: the help overlay (`?`) and the input box (`a` / `o` /
  `⏎`). *(The input moved off this line — see below.)*
- ✅ **Delete is immediate, with `u` to undo.** No confirmation prompt: a prompt
  taxes every delete to protect against the rare wrong one.
- ✅ **`spc` toggles done, `⏎` edits.** *(Changed from "`⏎` toggles" — see below.)*
- ✅ **Narrow width is the normal case**, not an edge case. Degradation order:
  spacing → tags → priority → date → truncate the title, never below 12
  characters. Under 34 columns the frame is dropped entirely.
- ✅ **The selection survives a reload**, tracked by task identity rather than row
  index, and a toggled task does not jump position until the next reload. A list
  that moves under you is unusable as a side pane.
- ✅ **No `tokio`, no `serde`, no `regex`, no `icalendar`.** Reasons in
  [architecture.md](architecture.md#dependencies).
- ✅ **Palette: Catppuccin Mocha, accent mauve** — as the *default*.
- ✅ **Colours are user-configurable** via `~/.config/ratodo/theme.conf`, in v1.
  A flat kitty-style `key = value` file, 12 role keys, six built-in themes, no
  new dependency. Hot reload is v2. See [theming.md](theming.md).
  *(This reverses an earlier rejection — see below.)*
- ✅ **Interface language: English.** The terms and the search results are English
  anyway, and so is the audience if this is ever opened up. No i18n (YAGNI);
  splitting it out later is cheaper than building it now.
- ✅ **Documentation language: English**, same reasoning. *(Decided 2026-08-10;
  the documents were originally written in Turkish and translated.)*
- ✅ **No test environment needed.** All that is required is a few hand-written
  `todo.md` files. Tests can be written on day one.
- ✅ **The file watch is on the directory** *(2026-08-11)*. vim, git and our own
  writer all replace the file by renaming a new one over it, and an inotify watch
  follows the inode that just stopped being the list.
- ✅ **A timed task is exported as a floating time, not UTC** *(2026-08-11)*.
  `@2026-08-13 09:30` carries no timezone because its author meant half past
  nine where they are; converting with today's offset makes the entry wrong the
  first time they travel. *(This corrects a line in
  [calendar.md](calendar.md#implementation) that said UTC.)*
- ✅ **The `.ics` UID is derived from title and section**, hashed with an FNV-1a
  written out in the source *(2026-08-11)*. `DefaultHasher` is allowed to change
  between Rust releases, and a UID that moves is every calendar entry deleted
  and recreated. See [calendar.md](calendar.md#implementation).
- ✅ **The ASCII fallback is chosen by locale, not by `NO_COLOR`** *(2026-08-11)*.
  Two questions, two signals: `$LC_ALL`/`$LC_CTYPE`/`$LANG` decide the glyphs,
  `NO_COLOR` and the theme decide the colours. See
  [tui.md](tui.md#no-colour-no-nerd-font).
- ✅ **Display width comes from ratatui, not a new crate** *(2026-08-11)*.
  `Span::width` already does the Unicode arithmetic, so `şğüöç` and 🚀 line up
  without an eighth dependency.
- ✅ **The user's own headings keep their `##`** *(2026-08-11)*. `OVERDUE` is
  ours and `## Work` came out of the file, and as the same bold word plus the
  same rule nothing on the screen said which was which. The marker is already in
  the file, so it costs no second colour and no third level of hierarchy, and it
  survives the ASCII fallback. The alternatives — dropping the rule from the
  user's headings, or indenting ours — each spent a level of hierarchy the
  design does not have. See [tui.md](tui.md#main-screen).
- ✅ **The input field is coloured by the parse, not by the leading character**
  *(2026-08-11)*. `@thu`, `#home` and `!high` light up as they are typed;
  `@notaday` stays plain, because that is what the file will hold. Rejected on
  the way: splitting the input into `text | date | tag` sub-fields with `tab`
  between them — it puts a focus state inside the input mode, and an edit would
  have to take an existing line apart into fields and put it back, which a line
  with two tags or a title after the tag does not survive intact. `capture::parts`
  is the one tokenizer both the field and the parse read. See
  [tui.md](tui.md#adding). **Asked for again on 2026-08-11 and rejected again**,
  now on arithmetic and on the invariant rather than on taste:
  - **It is five fields, not four.** `!high` was not in the ask and `$list` was,
    so the row is `title │ date │ tag │ priority │ list`.
  - **They do not fit the pane the product is aimed at.** The box is
    `min(70, pane − 4)` wide, so a 34-column pane gives it **28 columns**. Five
    fields and four separators leave sixteen columns of content — three
    characters a field. At 60 it is forty-two, and `2026-08-13` alone is ten. It
    is drawable at eighty and nowhere else, which means a second input for the
    narrow pane — and the narrow pane in the corner of a tiling layout is the
    audience this was designed for, not the fallback.
  - **Joined back up, the fields buy nothing.** Keeping one tokenizer means
    joining the five fields into a line and handing it to `capture::parts` — at
    which point the boundaries are decoration over the same string, paid for
    with a focus state, `tab`/`shift-tab`, five carets and five scroll windows.
    Not joining them means a second parser, and the day the two disagree the box
    is lying about what it will write. That is invariant 1 territory.
  - **A field is narrower than the line it replaces.** One tag field cannot hold
    `#home #work`, and the measured round-trip on 2026-08-11 —
    `- [ ] pay #home the invoice @thu #work !high` retyped and saved intact — is
    the thing that would be given up.
  - **What the ask actually wanted is discoverability**, and that is one dim row:
    an empty box now reads `@thu #home !high $list`, the way the empty `p` box
    reads `how long? 2 3d 1w fri`. Twenty-two columns, fits the 34-column pane,
    no mode, no keymap, and `$list` only appears when there is more than one
    list to address.
- ✅ **The ASCII fallback covers the overlay too** *(2026-08-11)*. `↓ ↑` and `⏎`
  were literals in the key list, `…` was a literal in `shorten`, and `·` came out
  of `text::fields`. The separator is now the caller's, because stdout does not
  fall back and the screen does. Warnings that carried an `—` were reworded
  instead: the bottom line has to read on any terminal. See
  [tui.md](tui.md#no-colour-no-nerd-font).
- ✅ **The date column borrows the row's colour when it presses** *(2026-08-11)*.
  `overdue` for a late task, `today` for one due today, dim for everything else
  — the two roles the title already uses, so nothing new to theme. Colouring
  every date was the alternative and it flattens the row: the right-hand fields
  are secondary on purpose. See [tui.md](tui.md#main-screen).
- ✅ **The columns get rules, and `y` says `copy`** *(2026-08-11)*. Both out of
  the same report: the row was read as one run-on line, and the box filled by
  `y` looked exactly like the box filled by `a`. Past `COLUMNS_AT` the fields
  already line up, so a dim `│` between them costs one column each and turns
  three fields that happen to be near each other into a table. **An empty cell
  keeps its rules** — a row with no date and no priority draws them in the same
  places as a row with both, which is the whole difference — and the rule that
  opens the tag column is reserved in `Columns` rather than spent out of what is
  left, or a long title would push it off exactly the rows with nothing to show
  there. Below the breakpoint there are no rules, because there are no columns.
  The input box's preview uses the same `│`, so the screen has one separator
  language. `y` now opens a box labelled `copy` in the accent rather than `add`:
  the label is the only thing that says `⏎` will *not* rewrite the line it was
  just filled from, and it was the one label worth reading in the quietest
  colour on the screen. See [design.md](design.md#rules) and
  [tui.md](tui.md#copying--y).
- ✅ **A date field, opened by `tab` and not standing in anybody's way**
  *(2026-08-11)*. `↑ ↓` on the part under the cursor and eight digits filling
  `DD MM YYYY`, which makes a month of `13` **unreachable** rather than merely
  detectable — the stronger half of the complaint whose cheap half is the
  "@… is not a date" line above. It is built to the shape
  [notes.md](../notes.md#the-date-field--a-proposal-not-a-decision-2026-08-11)
  argued its way to, and each of the three objections recorded there is answered
  by that shape rather than waved past:
  - **"A second mode inside a mode."** It is, and it is one you open on purpose
    with `tab` and leave with `esc`, which is the same in-and-out the help
    overlay has. Nothing about the box changes until you press it: `a` still
    opens one line of text with the same five keys on it.
  - **"It competes with the thing that already works."** It cannot, because it
    is not on the path. `@thu`, `@3d` and `@tomorrow` are still what the common
    case types, and the field is what you reach for when you were going to count
    days on your fingers anyway. The empty box still advertises `@thu`, not
    `tab`.
  - **"Two entry methods for one field is the trap."** Avoided by there being
    exactly one at a time. While the field is open the digits are positional and
    the text box is not being typed into; `⏎` writes one `@YYYY-MM-DD` word back
    into the line and hands the keyboard back. Free text never has to guess
    whether `12` means December.

  The date it writes is always a date: the day is clamped to the month it is in,
  so arrowing January 31st into February gives the 28th (or the 29th) rather
  than a word the parser will refuse. It edits the `@` word already in the line
  if there is one and appends if there is not, and in the `p` box — which asks
  *how long* and not *which day* — it writes the bare ISO date, which is the one
  form `p` accepts past its year horizon.
- ✅ **The packaging lives in this repository, and neither file carries a
  version** *(2026-08-11)*. `flake.nix` reads the version out of `Cargo.toml`
  with `fromTOML` and pins its dependencies with `cargoLock.lockFile`; the
  `PKGBUILD` carries `pkgver` and the tarball's `sha256`, because that is what
  `makepkg` is. A `cargoHash` in the flake was the alternative and it is a
  second copy of what the lock already says — the copy that rots. There is **no
  `flake.lock`**: it cannot be generated on a machine with no `nix`, and an
  invented one would be worse than none. The AUR package is a `PKGBUILD` in
  `packaging/`, not a submission — submitting is an account and a push, and it
  is somebody's decision rather than a build step.
- ✅ **A length of time stops at a year** *(2026-08-11)*. Reported from use: a
  keyboard that stutters turns `22` into `2222` in the `p` box, and both are
  perfectly good arithmetic — twenty-two days and six years — so the file took
  the second one without a word. `capture::later` now refuses past 365 days in
  every form that can carry a doubled digit (`2222`, `2222d`, `222w`), and the
  refusal names the way out. The horizon is on `p` alone and **an ISO date is
  not measured against it**: `p` asks *how long*, and past a year that has
  stopped being the question, while `@2032-09-10` is a day somebody meant. A
  digit-count check was the alternative and it is the wrong shape — `366` is two
  wrong digits and `22` is two right ones. See [tui.md](tui.md#putting-a-date-off--p).
- ✅ **`!high` is bold, and that is all it gets** *(2026-08-11)*. The one field
  the user typed to mean *urgent* sat in the same grey as the date and the tags.
  Weight rather than a twelfth theme role: a priority colour would have to be
  themed, would collide with `overdue` on the rows that have both, and would say
  nothing under `NO_COLOR`. `!med` and `!low` stay dim. See
  [tui.md](tui.md#main-screen). *(Widened 2026-08-12: the priority is the accent
  in two weights — [reversed](#reversed), and each objection answered by using
  the accent rather than a new hue.)*
- ✅ **A finished task is never late** *(2026-08-11)*. `1d ago` on a ticked line
  states something that stopped being true, and the counts already left finished
  work out of `overdue`. It shows the plain date instead, and keeps its place in
  `OVERDUE`, where membership was always positional.
- ✅ **The empty screen shows the real input box, and the overlay puts its exit
  on the border** *(2026-08-11)*. The example was a line of text saying
  `Try:  a  then  buy milk @tomorrow #home`; it is now the same field `a` opens,
  drawn by the same code, with the live parse under it already reading
  `due tomorrow (2026-08-11) · #home`. The shorthand is the thing worth teaching
  and it now teaches itself before a key is pressed. No new concept and no new
  colour: the frame's own border colour, because the accent border is what marks
  the box that has the keyboard. Under eleven rows it goes back to being a line —
  the example is the last thing a short pane is allowed to lose. In the help
  overlay `? esc  this, and away again` left the key list for the bottom border
  as `esc or ? to close`, which costs no row and takes the box back to twelve.
  Grouping the keys with blank lines was the alternative and it costs four rows,
  which is `q  ctrl-c` falling off a fourteen-row pane. See
  [tui.md](tui.md#empty).
- ✅ **A rule splits the input box in two** *(2026-08-11)*. The field and the
  live parse sat in one box with nothing between them, and the caret read as
  something that could be moved down into the preview. The box costs a fifth
  row; on a pane too short for one the rule goes rather than the preview. It is
  drawn as text inside the block, so the two cells where it meets the frame are
  set to `├` and `┤` afterwards — a rule butting into `│` reads as a frame that
  broke. See [tui.md](tui.md#adding).
- ✅ **Every `*.md` in the config directory is a list** *(2026-08-11)*. Somebody
  who keeps `work.md`, `personal.md` and `2026.md` apart on disk still wants one
  screen, and the alternative — a manifest naming the lists — is a second config
  file plus a step to forget every time a file is added. Reading them was the
  easy half. What the decision actually cost:
  a `Task` carries the file it came from, so a change goes back to that file with
  its own mtime check and its own backup; `Kind::Section` carries it too, so
  `## Work` in two files is two headings rather than one that pulls tasks
  upwards; and `Task::identity` gains it, or two files holding `- [ ] fix the
  tap` are one task to `done`, to the cursor and to the calendar UID.
  **The file is attached only when there is more than one**, which is what keeps
  every single-file setup — its identities, its UIDs, its `done` — exactly as it
  was. A capture goes to `todo.md` rather than to whatever the cursor is over:
  `a` meaning a different file depending on the scroll position is the kind of
  surprise that loses a task. See [cli.md](cli.md#several-lists).
  *`$work` in the sentence picks another list, since the same day and for the
  same reason — the target is a word you typed, not a scroll position. See
  [Reversed](#a-capture-always-goes-to-todomd--work-picks-the-list-2026-08-11).*
- ✅ **A reader that closes the pipe is not an error** *(2026-08-11)*.
  `ratodo list | head` made `println!` panic. Every stdout write goes through
  `writeln!`, and `BrokenPipe` alone exits 0. See
  [cli.md](cli.md#behaving-like-a-unix-program).
- ✅ **A third state: `- [-]`, cancelled** *(2026-08-11)*. A list whose only exit
  is deletion cannot record having *decided against* something — the task and
  the decision go together. `[-]` is the Obsidian and Logseq convention, so a
  file with one in it still reads correctly in the two tools most likely to open
  it next. It is out of the counts, never overdue, and not exported. `X` sets it
  and `X` takes it back. Hard invariant 7 said the file only ever contains `[ ]`
  and `[x]`; this widens it to three and no further — `[!]` is still derived from
  the date and still never written. See [format.md](format.md#the-three-states).
- ✅ **The completion date is stamped: `✓2026-08-11`** *(2026-08-11)*. Ticking
  something recorded the fact and lost the day, which is the half people
  actually want later. Three shapes were on the table: `%2026-08-11`, a new
  sigil in the same family as `@` and `#`; `done:2026-08-11`, no sigil but
  colliding with any title containing `done:`; and `✓2026-08-11`, matching the
  symbol already on the screen. **`✓` was chosen**, and it is the one non-ASCII
  thing the tool writes — a deliberate exception, since ASCII fallback is a rule
  about the *screen* and the file has always been free to hold any UTF-8. The
  cost is real and known: it is harder to grep and harder to type by hand.
  Mitigated by requiring the date — a bare `✓` in a title is the user's and is
  never written over, which the `gnarly.md` fixture caught the first time it was
  not. See [format.md](format.md#the-completion-stamp).
- ✅ **`p` puts a date off, and takes a bare number of days** *(2026-08-11)*.
  Moving a date meant reopening the whole line with `⏎` and retyping it. `p`
  reuses the input box — same caret, same rule, same way out — but asks a
  different question, and the preview answers it with the day it lands on. It
  takes everything `@` takes plus a bare number, which is `p`'s alone: a box
  that has just asked *how long* has one reading of `2`, and `@2` in a sentence
  does not. It moves `@` and nothing else; the time stays put. This is **not**
  the `~date` deferral of [roadmap.md](roadmap.md) v3 — that hides a task until
  a date, this changes when it is due. See [tui.md](tui.md#putting-a-date-off--p).
- ✅ **The preview warns about an `@` that can never be a date** *(2026-08-11)*.
  Found on the first day of real use: `@2026-13-45` resolves to nothing, so the
  word stays in the title and the task keeps no date — correct, and silent. The
  fallback is not what changed; the silence is. The condition is **can never
  be**, not **is not yet**: the preview redraws on every keystroke, so warning on
  anything unresolved would warn through all of `@2`, `@20`, `@202` on the way to
  a good `@2026-08-20`, and a line that is wrong ten times per date is one people
  stop reading. `@2026-0` is still on its way somewhere; `@2026-13` is not, and
  that is where it speaks. This is the cheap half of the complaint — the
  field-by-field date entry is the other half and is still
  [a proposal](../notes.md#the-date-field--a-proposal-not-a-decision-2026-08-11).
  See [tui.md](tui.md#adding).
- ✅ **`y` copies a task, and there is no paste** *(2026-08-11)*. Asked for as
  yank-and-paste with a register: `y` here, `p` there. Two things were in the
  way. `p` has put a date off since v0.2.0 and is not free. More to the point, a
  capture lands in the capture target no matter where the cursor is, so
  "paste here" and "paste there" would have been one key doing one thing — the
  register was buying nothing. What is left is `y`: the input box, pre-filled
  with the selected task, as a new one. The completion stamp and the state do
  not come with it. Rejected along the way: pasting a copy into the file first
  and editing it afterwards, which is two writes, two mtime checks, and a stretch
  where two tasks share a title — and `Task::identity` is the title, so it is
  also a stretch where they share an `.ics` UID. See
  [tui.md](tui.md#copying--y).

### The box is "one field, not five labelled ones" → `a` opens a form, and the box is what it falls back to (2026-08-12)

**Was, twice.** [tui.md](tui.md#adding) says the input is one field and not five
labelled ones, and the second ask for labelled fields was
[measured and rejected](#settled) the day before this: five fields and four
separators in a 34-column pane is three characters a field, and joining them back
into a line for `capture::parts` makes the boundaries decoration over the same
string — paid for with a focus state, `tab`/`shift-tab`, five carets and five
scroll windows.

**Now:** `a` opens a **form** — but the arithmetic that killed the last attempt is
untouched, because the reversal **narrows** rather than dies:

- **It is a screen, not a row.** The rejection was about five fields *inside the
  one-line box*, at `min(70, pane − 4)` columns. The form is a centred overlay
  with a row per field and the whole pane to lay them out in. Nothing about
  28 columns is being argued with; that case is simply not this one.
- **"One field, not five labelled ones" still governs the narrow pane**, which
  is the case its arithmetic was always about. Under **15 rows or 40 columns**
  `a` opens the one-line box instead. A form that half-fits is worse than a box
  that always fits, and the box is already built and already tested.
- **`p` and `y` keep the box at every width.** Each asks one question, and a form
  for one question is a form nobody wants.

**And the invariant is kept the way the rejection demanded, not in spite of it.**
The old objection was that five fields mean either joining them back into a line
(decoration) or a second parser (drift). The form does neither: **the line is the
model.** The text box holds the whole line exactly as the one-line box does, and
Due, Time, Priority, Tags and List are *views* of it — each reads
`capture::parts` to know what is selected, and each writes back by replacing the
span that tokenizer claimed. There is one string, one tokenizer and one truth,
and a tag field that cannot hold `#home #work` is not a problem this design can
have, because there is no tag field holding anything.

**What the form adds that the box could not**, and it is the reason to build it
at all: a `PREVIEW`, with its own label and its own rule above it. The difference
between a form that happens to show a line and a form whose *conclusion* is a
line. A Todoist form saves into a database and can tell you nothing; this one
saves into your file, so the file is the last word on the screen.

**The question field holds the sentence and nothing else** *(amended
2026-08-12, after looking at it)*. It was the whole line — `call the plumber @fri
14:00 !med #home` — because the line is the model and that was the most direct
way to say so. On the screen it read as the syntax the boxes underneath were
already showing, twice. The field is now a **view like every other control**: it
holds the run of words `parts` did not claim, and the one place the whole line
appears is the `PREVIEW`, which is what the preview was for.

Nothing about the invariant moves — there is still one string and one tokenizer,
and the question field writes back through the same `set_parts` every other row
does. Two things follow and are worth writing down:

- **A date typed into the sentence still works**, and it now visibly *moves*:
  the word goes into the line, `parts` claims it, and it is in the date box
  before the keystroke is over. It leaves the sentence when the field gives up
  the keyboard.
- **`a`'s opening date lives in the date box** rather than behind the caret,
  which is a better place for a guess: it can be seen and changed without
  deleting anything first. It still steps aside for a date the user types, once,
  and only while the line still holds ours untouched.

**The date and its time share a row, and the date is typed rather than picked**
*(amended 2026-08-12, after looking at it)*. The first drawing gave `Due` a row
of radios — `○ none ◉ today ○ tomorrow ○ pick` — and `Time` a row of its own.
Two rows for one thought, and a fixed set of days the form had invented. As one
row, `Date / Time  [ 2026-08-12▏]  [ 09:30 ]`, it is a row shorter and the field
takes anything `capture` resolves: `thu`, `3d`, `2026-08-14`. The `PREVIEW` is
what says which day `thu` came out as, which is the same live parse the one-line
box has always had, and `↑` `↓` still open the three-part picker on it. Below
about fifty columns the two boxes do not fit side by side and take a row each.

**Six fields and no seventh.** Title, date, time, tags, priority and which list —
exactly the six a one-line format carries. No Description and no Project, because
there is nowhere in a line to put them, and no Section picker, because that means
teaching the writer to *insert* into the middle of a file it only appends to
today. See [redesign.md](redesign.md#screen-2--a-the-add-screen----replaced).

**Rejected on the way, again:** a separate text field per token. It is the same
proposal as the one measured down the page, moved to a bigger screen, and it
brings back the same two problems in a room where they are easier to hide.

### The form is a modal, and `design.md` said there were none (2026-08-12)

[design.md](design.md#rules) says *"One layout, no split panes. No sidebar, no
modal"*, and [tui.md](tui.md#help) calls the help overlay *the one overlay in
the product*. The form in the step above is a centred overlay, so both sentences
had to move rather than be quietly outlived.

**Amended:** the rule is **no sidebar and no split pane**, which is what it was
protecting — a layout that divides the pane permanently and works at 68 columns
and nowhere else. An overlay is not that: it is opened by a key, closed by `esc`,
covers nothing you were mid-way through reading, and gives the whole pane back
the moment it goes. The product already had one and the input box is a second;
the form is a third of the same kind and it falls back to the box at the width
where an overlay stops fitting.

The rule that does **not** move: nothing may be *permanently* beside the list.

### `s` — a second screen, and the first one this product has had (2026-08-12)

**Nothing is reversed by this.** A new screen is a new promise, so it gets an
entry either way.

`ratodo` had exactly one screen and one overlay, and "there is only one screen"
was the most common thing said about it. The answer is not a sidebar and not a
split pane — both are refused in [design.md](design.md#rules) and both were
drawn and rejected in [redesign.md](redesign.md) — it is a **second screen one
key away**, closed by the key that opened it.

- **`stats(&[Task], today, period) -> Stats` is pure**, with `today` a parameter.
  The same shape as `agenda` and testable for the same reason, and it lives in
  `agenda.rs` beside it: it has that function's exact signature and exact
  purity, so a twelfth file would be the first brick of the `mod.rs` pyramid
  [architecture.md](architecture.md#module-layout) forbids.
- **Nothing is stored and nothing is added to the format.** Every number is
  arithmetic over `✓` stamps the file already carries. That is the principle the
  whole redesign hangs off — the tool must not know something the file does not.
- **No new dependency.** The bars are `█` and `░` by hand; ratatui's `BarChart`
  and `Gauge` would have been a widget's opinion about a layout this screen has
  its own opinion about.
- **A screen, not an overlay.** It replaces the list rather than covering it,
  because nothing on it is glanced at mid-task — and while it is up the list's
  keys do not act, or `spc` would tick a task nobody can see.
- **No boxes and no rules between the blocks**, which is the restraint being
  spent here on purpose rather than argued about later. A statistics screen is
  exactly where a tool starts trying to look like Grafana.

**What it costs on the two bars it has to appear on**, measured rather than
assumed. In `?` it is the eleventh key and gets a row of its own — eleven plus
two of border is thirteen, and a fourteen-row pane holds it with one row spare.
On the hint bar it goes on the **end** of the greedy fill, which is ordered by
how often a key is reached for; the consequence is that `[s] stats` needs about
106 columns before it appears there at all, so `?` is where the key is actually
found. That is the trade the ordering rule implies and it is written down rather
than discovered later.

## Rejected

These are not "we'll look at it later" — they were looked at and the answer was
no. Reopening one requires new information.

| Idea | Why not |
|---|---|
| TOML / JSON storage | Parsing is free, but it is not hand-editable and `git diff` gets noisy. Kills the core promise |
| SQLite storage | Fast, but binary — no `git diff`, doesn't open in vim |
| The todo.txt standard | Has an ecosystem, but weak date/recurrence support and nothing for calendar export |
| Two-way CalDAV sync | ETags, conflict resolution, an offline queue, credential storage. A sub-project on its own |
| Kanban / board view | taskell already does this, and does it well |
| Cloud sync / accounts | "Your data stays put" is the product's strongest sentence. It cannot be taken back |
| `tokio` | No need for async — one local file, blocking IO is enough |
| Theme loader — **reversed 2026-08-10**, see below | ~~YAGNI~~ |
| Two view modes (agenda / file) | Two modes = state management + key conflicts + two drawing paths |
| Strikethrough for completed tasks | Inconsistent terminal support; unreadable for half of users |
| An encrypted list | No. The file stays plain text — that is the entire logic of the product |
| Verifying the `.ics` in Thunderbird — **off the build list 2026-08-21** | It needs Thunderbird on this machine. `todoman` reading all five tasks and `khal` ignoring them are already two data points and the finding they were for: the file is a real VTODO, and some clients drop VTODO on the floor. [calendar.md](calendar.md) and the README say "not verified by us" instead of guessing, which is the honest version and costs nobody anything |
| Building `flake.nix` here — **off the build list 2026-08-21** | There is no `nix` on the machine this was written on and no container runtime to borrow one from. The flake reads the version out of `Cargo.toml` and pins with `cargoLock.lockFile` so it cannot rot silently; the README says it is unbuilt. First person with `nix` closes it, and until then it is not work anybody here can do |
| Automatic git commits | Tempting, but touching the user's git is dangerous even opt-in. Maybe an explicit `--commit` flag much later |

## Reversed

### The blank row between groups becomes the group's bottom edge (2026-08-12)

**Was:** [design.md](design.md#rules) — *"Generous whitespace. The blank lines
between groups are half of the design."* — and the code said it too:
`ui::rows`'s own doc comment called the spacer *a row, not a margin*. A group was
a heading with a rule after it, its tasks, and a blank row.

**Now:** a group is a **box**. The heading moves into the top edge, the blank row
becomes the bottom edge, and the column dividers meet them at `┬` and `┴`.

**Why, and it is a correction rather than a decoration.** The screen drew three
line systems that never touched each other: the group rule stopped in mid-air at
column 39, the column dividers began at column 40 out of nothing, attached to
neither, and the group ended in a blank row that closed nothing. Three sets of
strokes sharing a screen without ever meeting is most of why a screen where
every field is correct still read as unfinished — [redesign.md](redesign.md).
Nothing floats now: every stroke starts at a corner and ends at one.

**What it costs, honestly:**

- **No rows at 60 columns and up.** One heading, *n* tasks, one spacer becomes
  one top edge, *n* tasks, one bottom edge. The last group gains the one row it
  used to end without.
- **One row per group between 34 and 59 columns**, which is the width that had
  already dropped the spacer. This is the real price and it is not free: a short
  pane at that width shows one group fewer.
- **Five columns of row** — a side either end, one of inset after the left one,
  and two the box holds back so it does not close flush against the frame.
  `COLUMNS_AT` went from 76 to 71 with it, so that eighty columns — the width a
  terminal opens at — keeps the columns it has rather than losing them to the
  border.
- **Nothing below 34 columns.** The frame goes at that width and the box goes
  with it.

**What is kept:**

- **A folded group stays a bare rule.** An empty two-row box to say a group is
  closed is exactly backwards: the difference between a container and a line
  *is* the open/closed signal.
- **The furniture stays furniture.** The box is drawn in `border`, the colour
  design.md already reserves for frames and rules. The group's *name* keeps its
  accent. No new colour, no new role.
- **A group with no name still gets a box**, with nothing written on its top
  edge — the run of tasks above a file's first heading. Still no "(no section)"
  nobody wrote, and no rows left floating beside the boxed ones either.

**The rule this walks into second**, and it is amended in the same place:
*"A rule between two columns, and nowhere else."* As written it forbids exactly
what the correction does. It was written to stop three characters of noise per
row; an edge is not per-row noise, it is the container the per-row rules run
inside, and it is what they now end on.

### `!high` is bold and that is all it gets → the priority gets a colour of its own (2026-08-12)

**Was:** the priority borrowed the row's own colour and added weight when it was
`!high` and the task was open; `!med` and `!low` sat in the same grey as the
tags. The reasoning was that a priority colour would need a twelfth theme role,
would collide with `overdue` on the rows that have both, and would say nothing
under `NO_COLOR`.

**Now:** a **twelfth role, `priority`** — yellow by default. `!high` is that
colour and bold, `!med` is that colour, `!low` stays dim. Three levels the eye
can sort without reading them, which is what the column is for.

**It went to the accent first, on 2026-08-12, and that lasted an hour.** The
accent was already the group headings, the input box border, the focused date
cell and the keys in `?`; adding the priority to it — and, the same day, the box
labels — made mauve the answer to six different questions, and the screen read
as noise. That is the failure [design.md](design.md#what-each-colour-means) now
exists to prevent, and the rule it states is the finding: **one colour, one
job.** A colour that answers two questions answers neither.

**A twelfth role rather than a borrowed one, because every other colour was
already spoken for:** red is the negative outcome, green is finished, orange is
today, blue is a tag, mauve is the tool's own voice. Yellow was the only hue in
the palette with no job, and a priority is the only thing left that needs one.

**What survives from the old decision:**

- **The weight.** `!high` is still the bold one, so `NO_COLOR=1` reads exactly
  as it did and nothing here is carried by colour alone.
- **`!low` stays dim.** Three loud rows teach nothing about which is which.

*(Amended the same day: a **ticked or cancelled row keeps its priority colour**.
It was dim at first, on the old rule that finished work is not urgent — but that
rule is about the **date**, which stops applying the moment a task is ticked. A
priority does not: it is a fact about the task, the `✓` and the `✗` already say
the work is over, and a finished `!med` going grey beside an open `!high` read as
the colour having failed rather than as the task being done.)*

**And one thing it fixes.** `!high` used to be drawn in the *row's* colour, so on
a late row the date and the priority were the same red — the one row where the
two most need telling apart. A field now keeps its own job's colour whatever the
row is doing.

The input box paints it the same two ways, because a box teaching a colour the
list then contradicts is worse than either of them alone. On the `terminal`
theme it is ANSI 6, the one index the other eleven roles had left.

### Only `copy` is lit → every label is → lit by weight, and upper case (2026-08-12)

**Was, and is again:** the input box's first word is dim for `add`, `edit` and
`put off`, and in the accent only for `copy`. `copy` is the one with something to
say — `⏎` will **not** rewrite the line the box was just filled from.

**The hour in between:** all four were put in the accent, on the grounds that the
box is a mode and a mode whose name is the dimmest thing in it is a mode you have
to look for. What that missed is that **the box's own border is already the
accent**, and it is the border that says *you are in a box*. The label an inch
inside it in the same colour said the same thing twice, and spent the one
distinction `copy` had. Together with the priority moving to the accent the same
day, it made mauve mean six things at once — see
[design.md](design.md#what-each-colour-means), which was written out of exactly
this.

**And that is what it became, an hour later.** Asked for a third time — *can the
labels be a different colour* — and the answer the scheme allows is the one the
line above already named: **`foreground` and bold**, full brightness against the
dim caret beside them, with `COPY` alone keeping the accent because `COPY` is
the only one with news. Lit, and not a seventh meaning on the screen.

**A thirteenth role was the alternative and it was refused.** Catppuccin Mocha
does have fourteen accent colours — the palette was never the limit. `nord`,
`gruvbox-dark`, `dracula` and a bare ANSI `terminal` are, because a role has to
be fillable in all six built-ins, and more than that: every hue added is a
meaning the reader has to learn. Twelve is the ceiling.

**They are upper case now**, the way `OVERDUE`, `TODAY` and `LATER` are: both
are the tool's own word rather than the user's, and the screen says so the same
way twice. A heading out of the user's file keeps its `##` *and* its own casing,
which is what still tells the two apart.

### The stats screen has no boxes, deliberately → every block is a box (2026-08-12)

**Was:** *"No boxes and no rules between the blocks. The list is a grid because
its rows line up and are read across; this is five paragraphs read one at a
time, and a frame round each would be furniture with nothing to hold. A
statistics screen is exactly where a tool starts trying to look like Grafana,
and the restraint is spent here rather than argued about later."*

**Now:** five boxes — `TOTALS`, `DONE THIS WEEK`, `PRIORITY`, `SECTIONS`/`LISTS`
and `PACE` — the same box the agenda draws a group in, touching each other the
way the agenda's groups do.

**Why the argument was wrong, having been looked at:** it defended against
decoration and the screen did not have a decoration problem, it had a
*containment* problem. The maintainer's word for it was **başı boş** — loose,
uncontained, as though nothing had been categorised. Every other surface in this
product is a container; a heading with nothing holding what is under it was the
one place where a word was expected to do a box's job, and two headings side by
side over two ragged half-width columns was the worst of it — neither column had
an edge, so neither heading visibly owned anything.

A box here is not the Grafana move the old rule was worried about. It is the
same stroke, the same corners and the same rule the list already draws, which is
the opposite of a new visual idea: the screen now reads as the same program.

**What it cost, and what paid for it:**

- **Two headings for one.** `PRIORITY` and `SECTIONS` were a two-column block
  and are now a box each, stacked. Stacking costs rows, and rows are what this
  screen had spare — the bottom third of it was empty at any normal height.
- **`TOTALS` and `PACE` got names.** They were the two blocks with no word over
  them, which is exactly the reader having to work out what a row of numbers is.
- **Eight rows at the top end, and a new drop order at the bottom.** `SECTIONS`
  goes first, then `PRIORITY`, then the day labels, then the histogram, with
  `TOTALS` and `PACE` left standing — and blocks go **whole**, because a box cut
  off at the bottom of a pane loses its own bottom edge.
- **Nothing below 34 columns.** The frame goes there and the boxes go with it,
  exactly as they do on the list.

**What did not change:** no new theme role, no new dependency, no new data. The
edges are `border` and the headings are `accent`, which is what the agenda's
group boxes already were.

### The arrows open the date picker and step it → the first press only opens it (2026-08-12)

**Was:** `↑` on the form's date field built the picker and immediately stepped
it, so opening it on 2026-08-12 showed you the 13th. Written up as deliberate —
the arrow that opened it "did something", rather than looking dead once.

**Now:** the first press opens it on the date the line already means and changes
nothing; the second steps it. Costs one keystroke in the case where the next
thing you wanted *was* tomorrow, and buys the rule that no key edits the date on
its way to showing you the date. The keystroke it costs is a key you pressed on
purpose; the one it stops is a value you have to notice changed and then undo,
which is the more expensive of the two by a long way.

**What it does not change:** `esc` still closes the picker and leaves the line
alone, and `tab` in the one-line box has always opened its field without
stepping — this is the form catching up with the box rather than a new rule.

**Two discoverability fixes went out beside it**, both the same shape: a key
that exists and is named nowhere.

- **The form's border names `shift-tab`.** It walked the fields backwards since
  it was built and said only `tab · next field`, so the way back was found by
  guessing or not at all. Below 43 columns the border has no room and names
  `tab` alone.
- **The date field's row names `← →`.** The brackets say which of the three
  parts has the cursor; nothing said how to move them. First thing dropped when
  the row runs out of room, since `↓ ↑` was always the half that was named.

### The input box opens empty → it opens on today (2026-08-12)

**Was:** `a` gave you an empty line, and the date was one more thing to type.
`tab` opened the date field on today, which is the same answer two keystrokes
further away and behind a key most people never press.

**Now:** the box opens with ` @2026-08-12` in it and the caret **in front of
it**. Today is the date a new task has more often than every other date put
together, and this is the one field the tool can guess right most of the time.
Guessing it in the *box* rather than at the write is what keeps it honest: it is
on the screen, under the preview, and a few keystrokes from gone. The date sits
behind the caret and not in front of it because it is the field the tool
guessed — the title is the one the user came to type, and it goes where the
written line and the row on the screen both put it, first.

**What it decides on the way past:**

- **A typed `@thu` takes its place.** `capture` gives the line to the first `@`,
  and the first one here is the one nobody typed — so without this the shorthand
  in every screenshot in [tui.md](tui.md#adding) would lose, *and* be left
  sitting in the title. It goes on the `@` keystroke, once, and only while the
  line still holds it untouched. `bob@work` in a title takes nothing with it.
- **A line of fields and no words is refused.** `@2026-08-12` on its own is a
  date, not a task, and `a`+`⏎` is now two keys away from writing a titleless
  line. It was reachable by typing `@thu` before this, and wrote one.
- **The empty-box hint stays**, for a box you have emptied. It is what `@thu`
  means that it teaches, and someone deleting the date is exactly who is
  looking for it.
- **`p` still opens empty.** Pre-filling a length of time with a guess makes the
  common case *delete* something before typing — the opposite of this.

### A capture always goes to `todo.md` → `$work` picks the list (2026-08-11)

**Was:** rule 4 of [cli.md](cli.md#several-lists). A capture goes to `todo.md`,
full stop; capturing into another list meant leaving the TUI for
`ratodo --file ~/.config/ratodo/work.md add '...'`.

**Now:** a `$work` anywhere in the input sends that one capture to `work.md`. It
is a fourth sigil beside `@` `#` `!`, read by the same `capture::parts`, and the
preview under the field answers it with `→ work.md` the way it answers `@thu`
with a date. Without a `$`, the target is `todo.md` exactly as before.

**Why the old reason survives intact.** It was never "one fixed file" for its
own sake — it was *`a` must not mean a different file depending on what the
cursor happens to be over*. A capture whose destination moves with the scroll
position is how a task gets lost. `$work` is not that: the target is a **word
the user typed**, in front of them, previewed before `⏎`. Same key, same
sentence, same file, every time.

**What it decides on the way past:**

- **A `$` that names no list is refused before the write.** Not created. A
  capture that quietly invents a file in the config directory is a surprise, and
  a new list is `touch work.md` — the feature already has no manifest and no
  setting, so it needs no back door either.
- **`$` is capture only.** On `⏎`, moving an existing task between files is two
  writes against two mtimes, and it is not this key. A `$` in an edit is refused
  and says so, rather than being swallowed — `capture` drops the word from the
  title, so silence would eat the user's text.
- **`$50` is money.** The word after `$` has to start with a letter, which is
  the reading a shell gives it too.
- **The first `$` wins**, the way the first `@` does.
- **With one list it still parses**, and refuses unless it names that list. `$`
  has nothing to do there, and a syntax that means one thing on a one-file setup
  and another on a two-file setup is worse than one that is simply unnecessary.

### `d` deletes and `X` cancels → `d` cancels and `X` deletes (2026-08-11)

**Was:** `d` deleted the task under the cursor and `X` marked it cancelled. The
capital was on cancel because "cancelling should be harder to reach than
finishing" — see [the entry below](#x-unbound--still-unbound-and-x-cancels-2026-08-11).

**Now:** the two are swapped. `d` cancels, `X` deletes. `x` stays unbound, and
the reasoning for that is unchanged.

**Why:** the old pairing put the shift on the wrong key. Cancelling is
reversible — `d` again takes it back, the row stays in the file, and the state
is one of the three the format already carries. Deleting takes a line out of the
user's file, and `u` is one level of undo that a `q` or a crash spends. The key
that costs the most is the one that should cost a shift, and `d` sitting one row
from `j` and `k` made the cheap key the destructive one.

The cost: `d` no longer means what it means in vim, and that is a real loss for
the audience this keymap is aimed at. It was accepted because `X` still points
at vim's `x`, only shifted, and because the wrong keystroke on `d` now costs a
second `d` rather than an undo.

### Red — only for overdue, then for the negative outcome (2026-08-11)

**Was:** "**Red is only for overdue.** Nowhere else." A cancelled row was drawn
in the grey a finished row had just given up.

**Now:** a cancelled row is `overdue` too. The rule reads "red is the negative
outcome": overdue, and cancelled.

**Why:** grey said *finished*, and a cancelled task is the opposite of finished
— it is the one that will not be. Three states wanted three colours, and the
alternatives were both worse: a twelfth theme role for one row, or leaving
cancelled looking like something that had been dealt with.

The cost is real and was weighed: red is the screen's loudest colour and a
cancelled task needs the least attention on it, so a cancelled row now pulls the
eye harder than the overdue one above it. What makes that survivable is the rule
this design has had from the start — **nothing is carried by colour alone**. `✗`
and `!` are different symbols, they are different in the ASCII fallback
(`[-]` and `[!]`), and they are different under `NO_COLOR`, where the whole
question disappears. If it turns out to read badly in practice, the way back is
a twelfth role, not a return to grey.

### A finished row — grey, then green (2026-08-11)

**Was:** a completed task was drawn in `done_text`, a grey. `done` — the green —
existed as a theme role and was spent on one thing: the progress bar in the
title rule.

**Now:** the row itself is green. The grey moves to cancelled rows, where it
belongs.

**Why:** [design.md](design.md#rules) reserved green for completed and then
never gave it to the thing that completes. Ticking a task was the one action on
the screen that said nothing back — the row simply went quiet. The old reasoning
was that finished work should recede, and it should, but receding and being
unacknowledged are different. The `✓` and the position already do the receding;
the colour is the acknowledgement. Nothing new was added: `done` was in every
built-in theme from the start, waiting.

### `x` unbound → still unbound, and `X` cancels (2026-08-11)

**Was:** `x` was deliberately bound to nothing — "in vim it deletes a character,
in a checklist it means tick the box; two strong and opposite intuitions on one
key, so it gets neither."

**Now:** `x` is *still* unbound. When cancelling needed a key, `x` was the
obvious candidate and was rejected on the same grounds: a **third** meaning on a
key already pulling two ways is worse than either of them. `X` took the job.

**Why:** the capital reads as a bigger version of the tick it is related to, it
was free, and shift makes it the deliberate act that "decided against" ought to
be. Cancelling should be harder to reach than finishing.

*Superseded the same day: `X` now deletes and `d` cancels — see the entry at the
top of this section. `x` is still unbound, on the grounds given here.*

### The help overlay — `:` and `/` listed, then not (2026-08-11)

**Was:** the overlay listed `:  /  answer, for now`, so that the two unbound
keys were documented where people look.

**Now:** the row is gone. Both keys still answer when pressed, on the status
line.

**Why:** the overlay's own rule is that it lists keys that *do something*, and
those two do not — it was one row short of consistent with itself. Pressing
either is the moment they teach anything, and that already works. The row they
were costing is what keeps `X  p` inside the twelve that fit a fourteen-row pane
— see [tui.md](tui.md#help).

### The input — the bottom line, then a box over the list (2026-08-11)

**Was:** the input opened on the bottom line, borrowing a second row from the
list for its parse preview. "No dialog ever covers the list" was the rule, and
the input was written to obey it.

**Now:** `a`, `o` and `⏎` open a box over the middle of the list, four rows tall
— border, field, preview — centred, and four columns short of the pane so the
frame stays visible around it. The bottom line goes back to one fixed row and
shows `⏎ save   esc cancel` for as long as the box is open.

**Why:** the rule was right about the wrong thing. What makes a dialog an
interruption is the screen changing under you; the bottom line was chosen to
avoid that. But this tool lives in a pane in the corner of a tiling layout, which
puts that line at the **bottom edge of the screen** — so every capture and every
edit meant looking down there, away from the row being worked on. The head
movement is the interruption. A box that appears where the eye already is costs
nothing but the rows it covers, and gives them back on `esc`.

**What it cost:** the box covers up to four rows of list while it is open, which
the fixed line never did. It is clipped rather than moved on a short pane, and
under three rows of list it is not drawn at all — the bottom line still names the
way out, so nobody is stranded in a mode they cannot leave. The keys left the
preview line for the bottom line, which is why they are now in the same place
whether the box is open or not.

### The right-hand fields — right-aligned, then columns past eighty (2026-08-11)

**Was:** "the date column is right-aligned … the eye reads down it", and group
headers get "a rule to the right edge". One layout at every width above sixty.

**Now:** a fourth breakpoint. Past eighty columns the date, the priority and the
tags become real left-aligned columns starting in the same place on every row,
the title column is sized to the widest title in the list, and the group rule
stops where that column ends. Below eighty nothing changes.

**Why:** the right-aligned block only reads down its edge when the rows all end
in the same field. They do not — `3d ago  !high  #ops` and `1d ago  #home` are
right-aligned as a blob, so the dates land in a different place on every row and
the eye gets a ragged list rather than a table. On a wide pane the middle of the
screen was also fifty columns of nothing, growing with the terminal: past sixty
the layout had no use for the width and just stretched the gap.

**Why a breakpoint and not the layout:** a column costs every row its width
whether or not that row uses it. One `!low` in a list buys a priority column
that every other row then carries as blank space, and at sixty columns paying
for it meant cutting titles — the exact inversion of the drop order, where the
title is the last thing to go. Eighty columns is where there is room to spend.

**What it cost:** the `LATER (3)` fold key moved from the right edge to the end
of the shortened rule, because a key stranded thirty columns past the rule it
belongs to is not an instruction. Tags kept no reservation, so a row with more
tags than room drops the last ones whole rather than cutting the title.

Recorded against [tui.md](tui.md#width), which now carries all four
breakpoints, and item 1 of step 8 in `todo.md`.

### The bottom line — one row, two while the input is open (2026-08-11)

**Was:** "there is one reserved line under the frame … nothing shifts the layout,
and **the list never moves under you**." One row, four jobs.

**Now:** the input takes two — the field, and the parse preview under it — so
opening it costs the list one row until `⏎` or `esc`. Everything else is
unchanged: hints, results and warnings still get exactly one.

**Why:** [tui.md](tui.md) contradicted itself. The rule said one row and the
*Adding* sketch drew two, and the sketch is the half that matters: the preview
is the reason the input exists at all. `@thu` resolving to a real date while you
type is what teaches the syntax, catches the typo, and proves the shorthand did
what you meant — that does not fit beside the text it is commenting on.

The rule survives, read the way it was meant: **the list does not move on its
own.** A row given up because the user pressed `a`, and taken back on `esc`, is
not the screen rearranging itself under a reader — it is the same deliberate
exception the help overlay already is. What the rule forbids is a layout that
shifts while you are only looking at it, and that is still forbidden.

**What it cost:** one row, and only while typing. The preview is also the half
that gets dropped first: under ten rows, or in a pane dragged down to two, the
field stays and the preview goes.

### The event loop — a blocking channel, then `poll` again (2026-08-11)

**Was:** a thread parked in `crossterm::event::read` sending keys down the same
mpsc channel `notify` uses, with the loop blocked on `recv`. Genuinely idle:
measured at six seconds open for zero seconds of CPU, and that measurement went
out in a commit message.

**Now:** `event::poll` with a 500 ms timeout in one thread, `try_recv` for file
changes.

**Why:** `e` → `$EDITOR`, which was already settled above. The editor wants the
terminal that the reader thread is parked on, so while vim runs the thread eats
its keystrokes — racily, which would have read as an intermittent bug rather
than a design mistake. A thread blocked in a read cannot be interrupted to stop
it, so one of the two had to go, and `e` is the escape hatch for everything the
tool cannot do.

**What it actually cost:** measured after the change, 40 wake-ups in 20 idle
seconds and zero CPU ticks at the kernel's 10 ms accounting granularity. The
timeout bounds nothing a user waits on — a key returns from `poll` immediately —
only how stale an outside edit can be, and half a second of that is invisible.
Drawing is still event-driven, so a wake-up with nothing to do draws nothing.

**What the old decision still buys us:** the shape of it. Keys and file changes
still meet in one loop, and the channel still carries nothing but "it changed".

### Theme loader — rejected, then accepted (2026-08-10)

**Was:** "YAGNI. A `theme.rs` with 11 constants is enough."

**Now:** colours are configurable in v1 through `theme.conf`.

**What changed:** the original rejection weighed the loader as engineering cost
against a default palette that already fits the audience. That misread what the
audience is. People running kitty, konsole, alacritty or foot theme *everything*
on their screen; a tool with hardcoded colours is the one thing that looks out of
place in an otherwise coherent setup. Theming here is not a power-user extra, it
is the difference between the tool belonging on someone's desktop and not.

**What the old decision still buys us** — the reversal is scoped, not open-ended:

- a flat `key = value` file, **not** TOML, **not** `serde` → no new dependency
- 11 keys, matching the 11 roles that already existed
- built-in themes are `const` tables, not files to discover and load
- no hot reload in v1, no per-element style attributes, no plugin system

It also retired an open risk for free: the built-in `terminal` theme uses only
ANSI 0–15, which is the answer to "no truecolor on a bare TTY".

Full spec: [theming.md](theming.md).

### `⏎` toggles → `spc` toggles, `⏎` edits (2026-08-10)

**Was:** `⏎` marks a task done, and there is no inline edit.

**Now:** `spc` marks done, `⏎` opens the task for editing.

**Why:** two reasons, and the second is the real one.

1. Space-to-toggle and Enter-to-open are the conventions people arrive with from
   every other list UI they use.
2. `⏎` is also the accept key in the add/edit input. Having one key mean both
   "accept this text" and "toggle this task" a moment apart is exactly the class
   of mistake that makes someone stop trusting a tool with their file.

### `.ics` before the TUI → after it (2026-08-10)

**Was:** the build order was fixtures → parse/write → **agenda + `.ics`** → TUI,
and `ratodo status --json` (the waybar/eww module) sat in v4.

**Now:** `status` and `list --porcelain` are v1 and get built with the agenda;
`.ics` moves behind the TUI.

**Why:** two independent design reviews, run from the two user profiles this
tool is aimed at — a tiling-WM ricer and a terminal-bound developer — arrived at
the same objection without seeing each other's notes. `.ics` serves seed point 6
(calendar integration), but the people it actually reaches are Thunderbird and
GNOME Calendar users. That is not the audience in seed point 2. A khal user does
not want a loose `.ics` either; they want a vdirsyncer collection, which is v5.
Meanwhile [notes.md](../notes.md) had already written down that a bar module is
"probably the single biggest win for this audience" — and then filed it three
versions away.

The ordering was upside down: the feature aimed at the audience was last, and
the feature aimed at somebody else was first.

**What did not change:** `.ics` is still in v1 and still one-way. Seed point 6
stands; only its position in the queue moved.

### Filter in v2 → `list --tag` / `--prio` in v1 (2026-08-10)

**Was:** "v1 scope: capture and check off. Filter and search go to v2."

**Now:** `ratodo list` takes `--tag` and `--prio` in v1. Interactive filtering
and `/` search stay in v2.

**Why:** the agenda groups by date, and a developer's list is mostly undated.
Per [design.md](design.md#agenda-grouping-rules-v1) undated tasks fall below
LATER, in file order — so on a realistic list the majority of the file gets no
structure from the one screen that was supposed to provide it. Someone who has
captured forty things and dated six of them opens the tool in week three, sees
thirty-four undifferentiated rows, and goes back to `rg TODO`.

The v1/v2 line was drawn to stop scope creep, and that instinct was right. This
reversal is deliberately the narrowest version of it: two flags over an already
parsed `Task`, on a command that already exists. No filter state, no UI, nothing
to undo.

**What the old decision still buys us:** `/` search, interactive filters, saved
views and `ratodo archive` are all still v2.

## Open questions

- [ ] Does a completed task stay where it is, or move to a `## Done` section?
      Staying in place bloats the file; moving means every completion shifts two
      lines in `git diff`. *Leaning: stay in place in v1, `ratodo archive` in v2.*
- [ ] Does a developer with one list per repository need `ratodo` to walk up the
      tree looking for a `TODO.md`? `--file`, `$RATODO_FILE` and every `*.md` in
      the config directory all shipped, and none of them is the per-repository
      case. *Leaning: `direnv` already solves per-directory, and the env var is
      what it would set. Still waiting on someone who wants it.*

## Resolved questions

- ✅ **Is `ratodo` available?** Checked 2026-08-10: free on crates.io, no notable
  GitHub project by that name, no binary conflict on PATH. Notably the backup
  name `tuido` **is taken** on crates.io — so the backup plan is gone, but it is
  no longer needed. Details in [naming.md](naming.md).
- ✅ **The README's first sentence:** "A todo TUI, built **with** ratatui" —
  not *for*. The name's kinship risks it being read as a ratatui plugin; the
  first sentence has to close that off.
- ✅ **Where does a captured task go in a file that ends with a table, a rule or
  a paragraph?** *(2026-08-11)* After the last recognised task, falling back to
  EOF — the leaning, as shipped. `Doc::push_task` finds the last `Item::Task` by
  `rposition` and inserts after it, so a capture into a file ending in a table
  lands inside the last `##` section rather than below everything. It also gives
  the previous line an ending when that line is a final one without one.
- ✅ **Do the docs mislead `chezmoi` users?** *(2026-08-11)* They did; it is a
  README paragraph now, not code. `chezmoi apply` writes the source copy over a
  live `todo.md` and every task captured since the last `chezmoi add` is gone.
  The fix is `.chezmoiignore`, and telling people so is cheaper — and more
  honest — than a tool that tries to detect it.
- ✅ **Is `.ics` regenerated on every `add`, or only when the TUI closes?**
  *(2026-08-11)* On every write. `quietly_sync` runs after a capture, after a
  `done` from the command line, and after every write the TUI makes. It is a few
  hundred bytes of string formatting over a list that fits in memory, and the
  alternative is a calendar that is stale for as long as the TUI stays open —
  which, given the scratchpad binding this is designed around, is all day.
- ✅ **Are `* [ ]` and `+ [ ]` recognised?** *(2026-08-11)* Yes on the way in,
  never on the way out — the leaning, as shipped. `parse.rs` accepts `-`, `*` and
  `+`, and new tasks are always composed as `- [ ]`. The asymmetry is not a
  compromise: round-trip fidelity means an existing `* [ ]` line keeps its `*`
  forever, because ticking it rewrites one byte inside the brackets and touches
  nothing else. We only choose a marker for lines we are creating.
