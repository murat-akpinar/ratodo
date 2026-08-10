# File format

One file. A Markdown checklist. Metadata inline, on the task's own line.

A complete working example lives in [examples/todo.md](examples/todo.md).

## Syntax

| # | Syntax | Example | Meaning |
|---|---|---|---|
| 1 | `- [ ]` | `- [ ] pay the invoice` | open task |
| 2 | `- [x]` | `- [x] review the PR` | completed task |
| 3 | `## Heading` | `## Work` | section |
| 4 | `@YYYY-MM-DD` | `@2026-08-12` | due date |
| 5 | `@YYYY-MM-DD HH:MM` | `@2026-08-12 16:00` | due date with a time |
| 6 | `#tag` | `#ops #home` | tag; a task may have several |
| 7 | `!high` `!med` `!low` | `!high` | priority |
| 8 | everything else | free text | the task's title |
| 9 | **an unrecognised line** | `> quote`, a table, a blank line, `---` | **untouched, preserved exactly** |

Row 9 is not a detail, it is a product decision. Half of a user's file may be
things we do not understand; all of it stays exactly where it is.

## Input is flexible, storage is strict

When writing, shorthand is allowed. What lands in the file is always an ISO date:

```
ratodo add "pay the invoice @tomorrow"  →  - [ ] pay the invoice @2026-08-11
ratodo add "report @mon !high"          →  - [ ] report @2026-08-17 !high
ratodo add "run a backup @3d"           →  - [ ] run a backup @2026-08-13
```

Accepted shorthand: `@today @tomorrow @mon`…`@sun @3d @2w`.

They never appear in the file. The file has to read the same to a machine and to
a human, next year as much as today.

## What the tool writes

- New tasks are always written as `- [ ]`, with an ISO date.
- Marking something done rewrites **only** the `[ ]` → `[x]` on that line. The
  rest of the line stays byte-for-byte identical.
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
| `todo.md.bak` | next to `todo.md` | Written before every write. Cheap insurance |
| `theme.conf` | `~/.config/ratodo/theme.conf` | **The user's.** Optional — colours fall back to the built-in default. See [theming.md](theming.md) |
| `config.toml` | `~/.config/ratodo/config.toml` | v2. There is **no** general config file in v1 (`theme.conf` is separate and deliberately not TOML) |

Overrides: `$XDG_CONFIG_HOME` and `--file <path>`. The second one is the escape
hatch for "work list separate, personal list separate" — before writing a
multi-list feature, let's find out whether that is already enough.
