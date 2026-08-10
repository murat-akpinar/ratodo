# Roadmap

Each version has to be usable on its own. If development stops after v1, what
exists must still be a tool worth having.

## v1 — Capture and check off

`ratodo` (TUI) + `ratodo add` + the agenda + `.ics` export. One screen, one
binary.

- Markdown parse/write with round-trip fidelity
- Agenda grouping: overdue / today / this week / later / undated
- Quick capture from the command line
- `e` → `$EDITOR`
- One-way `todo.ics` (VTODO)
- User-configurable colours via `theme.conf`, six built-in themes

The task-by-task breakdown is in [../todo.md](../todo.md).

## v2 — Filter / search / archive

- Filter by tag and priority
- `/` search
- `ratodo archive` — move completed tasks into `## Done`
- `config.toml` arrives here (and with it, `serde`)
- Theme hot reload — `theme.conf` joins the `notify` watch list
- `--as-events` flag for calendar clients that ignore VTODO

## v3 — Recurrence and deferral

- A subset of RRULE: daily / weekly / monthly
- `~date` syntax — hide until this date
- These two together are the single biggest chunk of work in the roadmap, which
  is why they are this far out

## v4 — Desktop integration

Probably the biggest real win for this audience:

- `ratodo status --json` → a waybar / eww module. Seeing "3 open · 1 overdue" in
  your bar
- Overdue notifications via `notify-send`
- Example tmux popup / Hyprland scratchpad bindings in the README

## v5 — CalDAV

Opt-in, by writing into a vdirsyncer directory. We still do not implement
two-way sync ourselves — see [product.md](product.md#out-of-scope).

## What is never coming

Cloud sync, accounts, a Kanban board, subtasks, time tracking, a plugin system.
The reasoning for each is in [product.md](product.md#out-of-scope).
