# File format

One file. A Markdown checklist. Metadata inline, on the task's own line.

A complete working example lives in [examples/todo.md](examples/todo.md).

## Syntax

| # | Syntax | Example | Meaning |
|---|---|---|---|
| 1 | `- [ ]` | `- [ ] pay the invoice` | open task |
| 2 | `- [x]` | `- [x] review the PR` | completed task |
| 3 | `- [-]` | `- [-] rewrite the docs` | **cancelled** — decided against |
| 4 | `## Heading` | `## Work` | section |
| 5 | `@YYYY-MM-DD` | `@2026-08-12` | due date |
| 6 | `@YYYY-MM-DD HH:MM` | `@2026-08-12 16:00` | due date with a time |
| 7 | `#tag` | `#ops #home` | tag; a task may have several |
| 8 | `!high` `!med` `!low` | `!high` | priority |
| 9 | `✓YYYY-MM-DD` | `✓2026-08-11` | **when it was completed** |
| 10 | everything else | free text | the task's title |
| 11 | **an unrecognised line** | `> quote`, a table, a blank line, `---` | **untouched, preserved exactly** |

Row 11 is not a detail, it is a product decision. Half of a user's file may be
things we do not understand; all of it stays exactly where it is.

### The three states

`[ ]` open, `[x]` done, `[-]` cancelled — the last of these is the Obsidian and
Logseq convention, and it exists because a list whose only exit is deletion
cannot record *having decided against something*. A cancelled task:

- is **not** open — it is out of `ratodo status`, out of the progress bar, and
  `ratodo done` will not match it
- is **never overdue**, however far past its date
- is **not** exported to the calendar — the `.ics` is work still to do
- shows as `✗` on screen (`[-]` in ASCII), in the same grey as a finished task
  rather than the green

`X` sets it and `X` takes it back, exactly like `spc` for done.

### The completion stamp

`✓2026-08-11` is written when a task is ticked and removed when it is unticked.
It is the one non-ASCII thing the tool writes, which was a deliberate choice —
see [decisions.md](decisions.md#settled). Two consequences worth knowing:

- The date is **required**. A bare `✓`, or `✓` followed by anything that is not
  an ISO date, is your own text and stays in the title untouched.
- Only the **first** stamp on a line counts, like the first `@date`. A second is
  title text.

A task finished before the stamp existed — or ticked by hand in `vim` — simply
has no stamp, and gets one the next time ratodo ticks it.

## Input is flexible, storage is strict

When writing, shorthand is allowed. What lands in the file is always an ISO date:

```
ratodo add 'pay the invoice @tomorrow'  →  - [ ] pay the invoice @2026-08-11
ratodo add 'report @mon !high'          →  - [ ] report @2026-08-17 !high
ratodo add 'run a backup @3d'           →  - [ ] run a backup @2026-08-13
```

Accepted shorthand: `@today @tomorrow @mon`…`@sun @3d @2w`.

They never appear in the file. The file has to read the same to a machine and to
a human, next year as much as today.

## What the tool writes

- New tasks are always written as `- [ ]`, with an ISO date.
- Changing the state rewrites **only** the byte between the brackets, and — when
  ticking — appends the stamp with one space in front of it. Nothing between the
  two moves: not your spacing, not the order you put your own fields in, not
  anything the parser did not understand.
- Unticking removes the stamp again, with the one space it was given, so a line
  that goes out and comes back is the line that went out.
- `p` moves the `@date` and only that. A time stays where it is — putting
  "Friday at 09:30" off by a week is still half past nine — and a task with no
  date at all gets one appended.
- Any line the user typed themselves and we have not modified is written back
  byte-for-byte. This is [round-trip fidelity](architecture.md#round-trip-fidelity),
  the single most important technical property of the project.

Reading is more permissive than writing: `- [X]` (capital), `* [ ]` and `+ [ ]`
are all read as tasks, because Markdown treats all of them as list items — but
whatever we write, we write as `- [ ]`. (Still an open question, see
[decisions.md](decisions.md#open-questions).)

## Where files live

There is a **deliberate deviation from XDG here.** Technically user data belongs
in `$XDG_DATA_HOME` (`~/.local/share/`). But the whole point is that the file
ends up in your dotfiles, and nobody puts `~/.local/share` in their dotfiles.
Following the standard would break the product's main promise, so we don't:

| What | Where | Why |
|---|---|---|
| `todo.md` | `~/.config/ratodo/todo.md` | **The user's.** Goes into dotfiles, hand-edited, versioned in git. This is the XDG deviation, and it is on purpose |
| `todo.ics` | `~/.local/share/ratodo/todo.ics` | **Derived.** Pointless to back up, regenerated if deleted. XDG is right here |
| the `.bak` | `~/.local/state/ratodo/` | **Derived.** Written before every write, cheap insurance — but *not* next to the list. `todo.md` is usually symlinked into a dotfiles repo, and a `.bak` beside it means `git status` reports an untracked file after every single capture. The file is named after the whole target path with the separators flattened (`-home-you-.config-ratodo-todo.md.bak`), so two `--file` lists cannot overwrite each other's backup |
| `theme.conf` | `~/.config/ratodo/theme.conf` | **The user's.** Optional — colours fall back to the built-in default. See [theming.md](theming.md) |
| `config.toml` | `~/.config/ratodo/config.toml` | v2. There is **no** general config file in v1 (`theme.conf` is separate and deliberately not TOML) |

Overrides: `$XDG_CONFIG_HOME` and `--file <path>`. Every `*.md` in the config
directory is a list and they are read as one agenda; `--file` narrows a run to
exactly one of them — see [cli.md](cli.md#several-lists).
