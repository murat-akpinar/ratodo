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
- ✅ **One multiplexed bottom line** carrying hints, the input field, results and
  warnings. No dialog ever covers the list — the help overlay (`?`) is the single
  exception, and it is one you asked for.
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
  A flat kitty-style `key = value` file, 11 role keys, six built-in themes, no
  new dependency. Hot reload is v2. See [theming.md](theming.md).
  *(This reverses an earlier rejection — see below.)*
- ✅ **Interface language: English.** The terms and the search results are English
  anyway, and so is the audience if this is ever opened up. No i18n (YAGNI);
  splitting it out later is cheaper than building it now.
- ✅ **Documentation language: English**, same reasoning. *(Decided 2026-08-10;
  the documents were originally written in Turkish and translated.)*
- ✅ **No test environment needed.** All that is required is a few hand-written
  `todo.md` files. Tests can be written on day one.
- ✅ **The event loop is one channel, not a poll with a timeout** *(2026-08-11)*.
  Keys arrive from a thread parked in `crossterm::event::read`, file changes from
  `notify`'s own thread, both down the same mpsc channel, and the loop blocks on
  `recv`. Any poll timeout has to choose between waking up forever and being slow
  to notice an edit; this chooses neither. See
  [architecture.md](architecture.md#the-event-loop).
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
- ✅ **A reader that closes the pipe is not an error** *(2026-08-11)*.
  `ratodo list | head` made `println!` panic. Every stdout write goes through
  `writeln!`, and `BrokenPipe` alone exits 0. See
  [cli.md](cli.md#behaving-like-a-unix-program).

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
| Automatic git commits | Tempting, but touching the user's git is dangerous even opt-in. Maybe an explicit `--commit` flag much later |

## Reversed

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
- [ ] Is `--file` plus `$RATODO_FILE` enough for multiple lists, or does a
      developer with one list per repository need `ratodo` to walk up the tree
      looking for a `TODO.md`? *Leaning: the env var buys most of it — `direnv`
      already solves per-directory. Ship that, then find out.*
- [ ] Where does a captured task go in a file that ends with a table, a rule or a
      paragraph? Appending at EOF puts it outside every `##` section.
      *Leaning: insert after the last recognised task, fall back to EOF.*
- [ ] Do the docs promise dotfiles integration in a way that misleads `chezmoi`
      users, whose `apply` will overwrite a live `todo.md` from a stale source
      copy? *Leaning: a README paragraph, not code.*
- [ ] Should `.ics` be regenerated on every `ratodo add`, or only when the TUI
      closes? *Leaning: on every add — it is simple, and the file is small.*
- [ ] Besides `- [ ]`, should `* [ ]` and `+ [ ]` be recognised? (Markdown treats
      all of them as list items.) *Leaning: recognise them when reading, always
      write `- [ ]`.*

## Resolved questions

- ✅ **Is `ratodo` available?** Checked 2026-08-10: free on crates.io, no notable
  GitHub project by that name, no binary conflict on PATH. Notably the backup
  name `tuido` **is taken** on crates.io — so the backup plan is gone, but it is
  no longer needed. Details in [naming.md](naming.md).
- ✅ **The README's first sentence:** "A todo TUI, built **with** ratatui" —
  not *for*. The name's kinship risks it being read as a ratatui plugin; the
  first sentence has to close that off.
