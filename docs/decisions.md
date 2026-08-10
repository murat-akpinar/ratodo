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
  user's job.
- ✅ **v1 scope: capture and check off.** Filter and search go to v2.
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

## Open questions

- [ ] Does a completed task stay where it is, or move to a `## Done` section?
      Staying in place bloats the file; moving means every completion shifts two
      lines in `git diff`. *Leaning: stay in place in v1, `ratodo archive` in v2.*
- [ ] Is `--file` enough for multiple lists (work / personal), or is a named-list
      concept needed? *Leaning: live with `--file` first and find out.*
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
