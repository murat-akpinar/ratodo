# Product

## In one sentence

A single-binary Rust + ratatui TUI that lets someone running i3 / Hyprland / sway
capture a task from the terminal in seconds without breaking their flow, keeping
the data in **one Markdown file** that lives in their dotfiles, and sending
nothing to any cloud.

## Who it is for

Someone who uses a tiling window manager, lives in the terminal, and keeps their
dotfiles in git. They are **already** doing `vim ~/todo.md`. We have to give them
a reason to put that down — not a reason to hand over their file.

## Why it exists — where the gap is

| Tool | What it does | What it lacks |
|---|---|---|
| `vim ~/todo.md` | What everyone does today. Zero dependencies | No answer to "what's due today?". Date math happens in your head, overdue work is invisible |
| taskwarrior | Powerful data model, mature CLI | Data lives in `~/.task/*.data` — **its own format**. Unreadable without the tool, does not belong in dotfiles. Steep learning curve; the TUI (`vit`) is a separate project |
| todo.txt / todo.sh | Plain text, a real standard, an ecosystem | No TUI. Weak date/agenda support, no calendar export. It's a shell script |
| taskell | Markdown + TUI, exactly our territory | **Kanban-first** — columns, but no dates, no agenda, no calendar |
| dstask | Git-based, flat files | Requires a git repo, and one YAML file per task. Not "a single file" |
| Todoist / TickTick TUI clients | They sync | Account required, data leaves the machine, offline story is weak |

The gap: nothing gives you **plain Markdown + fast capture + an agenda** at the
same time. Every tool picks two and drops the third.

## The core promise

The critical distinction is that this is not a data store — it is **the file itself**:

```
taskwarrior  : ~/.task/pending.data
               delete the tool and the data is unreadable. The format belongs to the tool.

ratodo       : ~/.config/ratodo/todo.md
               delete the tool and the file still works. The format belongs to the user.
```

> **The tool is the file's guest, not its owner.**

This one sentence is the product's spine. Every architectural decision derives
from it — see [architecture.md](architecture.md) for the technical form it takes
(round-trip fidelity).

## Product decisions

- **The file belongs to the user.** It must stay hand-editable. The tool never
  touches a line it does not recognise, never reformats, never reorders.
- **Two ways in, both in v1:**

  ```
  ratodo                              → opens the TUI
  ratodo add "pay the invoice @tomorrow"   → writes and exits, TUI never opens
  ```

  The second one is the reason this product exists: getting the thing that just
  popped into your head into the file in two seconds. Forcing a TUI open kills
  that flow.
- **`e` opens `$EDITOR`.** An escape hatch from the TUI into vim. Ten lines of
  code, and exactly right for this audience — a guarantee that anything the tool
  cannot do, you can still do in the file. Nobody feels locked in.
- **Local and offline.** No account, no server, no telemetry. Sync is the user's
  own git — not our business, and it should stay that way.
- **v1 writes, but is not destructive:** atomic writes (temp file + `rename`) and
  a `todo.md.bak` before every write. The guarantee is not "we can't break
  anything" — we have to write, after all — it is "**we can't lose anything**".

## Out of scope

**This is the most important section in the document.**

What kills a project like this is not technical difficulty, it is **scope creep**.
A todo tool can grow forever; everyone has one feature it obviously needs. The
following are deliberately absent:

- ❌ **Cloud sync / accounts / a server.** Git already exists. We do not walk back
  the "your data stays put" guarantee.
- ❌ **Two-way CalDAV sync.** ETags, conflicts, an offline queue. That is a
  separate product.
- ❌ **Kanban / board view.** taskell does this well. Don't rewrite it.
- ❌ **Recurring tasks (RRULE).** A week of work on its own. v3.
- ❌ **Subtasks / dependency graphs.** That is taskwarrior's territory.
- ❌ **Time tracking / pomodoro.** Different product, different moment of use.
- ❌ **Plugin systems**, and theme *hot reload* (v2). Colours themselves **are**
  configurable — see [theming.md](theming.md); that rejection was reversed.
- ❌ **A general-purpose Markdown editor.** `e` opens vim. That's enough.
- ❌ **Windows / macOS.** crossterm and notify are portable and we won't
  deliberately break them — but the XDG paths and the audience are Linux. Not
  tested, not promised.

Rejected ideas and the reasoning behind each are in
[decisions.md](decisions.md#rejected).
