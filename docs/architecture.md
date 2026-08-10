# Architecture

What drives the design here is not performance — the file is a few KB — it is
**data fidelity**. That is the central architectural decision:

## Round-trip fidelity

> The parser keeps every task's **raw line**. If the tool did not change a field,
> the line is written back byte-for-byte.

The user's hand-written indentation, extra spaces, and the note they added
themselves do not disappear. This is the technical form of "the file belongs to
the user", and the good news is that testing it needs no terminal at all:

```
parse(write(parse(x))) == parse(x)
and: every untouched line, byte-for-byte identical
```

If this property ever breaks, the tool has corrupted somebody's hand-written
file — trust is gone and the tool gets deleted. It is the project's number one
risk; see [risks.md](risks.md).

## Data flow

```
~/.config/ratodo/todo.md
  → parse   : line -> Task { raw line + parsed fields }
  → model   : Vec<Task>, file order preserved
  → agenda  : (Vec<Task>, today) -> Vec<Group>      ← the product lives here
  → ratatui : draw only when an event arrives

~/.config/ratodo/theme.conf
  → theme   : key = value -> Theme, resolved once at startup ─┘
  ← write   : Task -> line (raw line if unmodified), atomic + .bak
  → ics     : open tasks that have a date -> VTODO
```

Two things are worth noticing about this pipeline:

1. **`parse`, `agenda`, `write` and `ics` are pure functions.** String in,
   struct out. No side effects, no terminal, no clock. That means the actual
   value of the product is testable without ever opening a TUI.
2. **`today` is a parameter, not a call.** `agenda(&[Task], today)` never calls
   `Local::now()` internally — otherwise it cannot be tested.

## The event loop

- **No fixed FPS.** Draw when a key is pressed or the file changes; block when
  idle → 0% CPU at rest. A todo tool must not burn battery in the background.
- **The panic hook restores the terminal.** A TUI that panics in raw mode leaves
  the user's terminal broken. `std::panic::set_hook` puts the screen back in
  every case. This gets written on day one, not later.
- Event sources: `crossterm::event::poll` for keys, and an mpsc channel from
  `notify` for file changes.

## Concurrent editing

The file is the single source of truth, not the in-memory model. Therefore:

- While the TUI is open, if the file changes from outside (vim, `ratodo add` in
  another terminal, `git pull`) → `notify`/inotify catches it and the file is
  re-read.
- Before writing, mtime is checked. If it changed since we read it, **we do not
  overwrite** — the user is warned.
- Writes are atomic: write to a temp file → `fsync` → `rename`. A half-written
  file cannot exist, even if the power drops.

We are not doing a clever merge. Conflicts are rare, and a wrong merge loses data
silently — warning and backing off is the honest behaviour.

## Module layout

```
src/
  main.rs      clap subcommands, terminal setup/teardown, panic hook
  model.rs     Task, Section, Due, Priority
  parse.rs     todo.md -> Vec<Task>, raw line preserved      ← the product lives here
  write.rs     Vec<Task> -> todo.md, atomic + backup
  agenda.rs    (Vec<Task>, today) -> Vec<Group>              ← the product lives here
  ics.rs       Vec<Task> -> todo.ics (VTODO)
  theme.rs     Theme struct, built-in themes, theme.conf parser
  ui.rs        ratatui drawing
tests/
  fixtures/    hand-written todo.md files — well-formed ones and deliberately awkward ones
```

Eight files. No `mod.rs` pyramid, no trait layer, no plugin system.

The `Task` struct, roughly:

```rust
struct Task {
    raw: String,        // the line exactly as it was read
    line_no: usize,
    done: bool,
    title: String,
    due: Option<Due>,
    tags: Vec<String>,
    priority: Option<Priority>,
    dirty: bool,        // false -> write `raw` back untouched
}
```

`dirty` is what makes round-trip fidelity mechanical rather than a matter of
discipline.

## Dependencies

```toml
ratatui       # TUI
crossterm     # terminal backend + events
clap          # add / done / list subcommands
chrono        # date parsing + "tomorrow / 3d" math
notify        # inotify — file changed from outside
directories   # XDG paths
anyhow        # errors
```

Seven crates, all of them required.

Deliberately **absent**, and why:

- **No `tokio`.** There is no need for async here — a single local file, and
  blocking IO is more than enough. The event loop is `crossterm::event::poll`
  plus notify's mpsc channel. An async runtime would grow compile times and
  binary size for free.
- **No `serde`.** We write the Markdown parser ourselves (it is the heart of the
  product anyway). `theme.conf` is a flat `key = value` file precisely so that it
  can be parsed in ~40 lines instead of pulling in serde + a TOML crate. serde
  arrives with `config.toml` in v2, if at all.
- **No `regex`.** `@date`, `#tag` and `!priority` are parsed by scanning
  word by word — faster than regex here, and much easier to produce good error
  messages from.
- **No `icalendar` crate.** VTODO output is ~30 lines of string formatting. Not
  worth a dependency. See [calendar.md](calendar.md).

## Difficulty is unevenly distributed

This is the first TUI project on this codebase, and the architecture is shaped
around that fact:

| | Hard / new | Easy / familiar |
|---|---|---|
| What | ratatui event loop, terminal raw mode, inotify | `parse` / `write` / `agenda`: pure functions |
| Why | Terminal goes into raw mode, events are awaited, the terminal must be handed back even on panic | Input is a `String`, output is a `Vec<Task>`. No side effects |
| Testing | Hard, by eye | **Easy — plain unit tests against fixture files, no terminal at all** |

So the real value of the product (parse + agenda) can be written and tested
completely independently of the TUI. That removes most of the risk for someone
new to TUI work — and it is why the build order in [../todo.md](../todo.md) puts
the pure functions first.
