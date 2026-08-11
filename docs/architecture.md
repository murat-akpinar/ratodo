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
- Event sources: **`crossterm::event::poll` with a 500 ms timeout, plus an mpsc
  channel from `notify`.** One thread does everything.

  A key returns from `poll` the instant it arrives, so nothing the user does
  waits on that timeout. It bounds one thing only: how stale an outside edit can
  be before the screen catches up. Half a second on a `git pull` is invisible.

  Drawing is still driven by events. A wake-up that finds nothing to do draws
  nothing, so "no fixed FPS" survives the timeout intact.

  **This reverses a design that parked a thread in `event::read` and blocked on
  the channel** — genuinely idle, and unable to support `e`. Two readers of the
  same terminal means the thread eating `$EDITOR`'s keystrokes, and a thread
  parked in a blocking read cannot be interrupted to stop it. Measured after the
  change: 40 wake-ups in 20 idle seconds, and zero CPU ticks accumulated at the
  kernel's 10 ms accounting granularity. See [decisions.md](decisions.md#reversed).

- **Watch the directory, not the file.** Every safe writer — vim, `git`, our own
  `write.rs` — replaces a file by creating a new one and renaming it over the
  top. An inotify watch is on the inode, so it goes quiet at exactly the moment
  something interesting happened. Watching the parent and filtering by file name
  is the fix, and the filter is its own tested function because getting it
  backwards means either reloading on every unrelated file or never reloading at
  all.

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
  lib.rs       the pure core, exposed so tests/ can reach it
  model.rs     Doc, Line, Task, Due, Priority
  parse.rs     todo.md -> Doc, raw line preserved            ← the product lives here
  write.rs     Doc -> todo.md, atomic + backup, and the file IO
  capture.rs   free text -> Task, resolving @tomorrow and friends
  text.rs      human-facing strings for the command line
  agenda.rs    (&[Task], today) -> Vec<Group>                ← the product lives here
  ics.rs       &[Task] -> todo.ics (VTODO)
  theme.rs     Theme struct, built-in themes, theme.conf parser
  ui.rs        ratatui drawing
tests/
  fidelity.rs  round-trip and byte-for-byte tests over every fixture
  property.rs  generated documents, checked against the same invariants
  fixtures/    hand-written todo.md files — well-formed ones and deliberately awkward ones
```

Eleven files, flat. No `mod.rs` pyramid, no trait layer, no plugin system.

Three of them were not in the original plan and are worth naming:

- **`lib.rs`** exists because a binary-only crate cannot be reached from
  `tests/`. The core is a library and `main.rs` is a thin shell over it, which is
  the same split the "difficulty is unevenly distributed" table below describes.
- **`capture.rs`** is separate from `parse.rs` on purpose. They look similar but
  they enforce opposite rules: `parse` is strict because it reads the file,
  `capture` is permissive because it reads a human. Merging them would put a
  flag in the middle of the one function that must never get this wrong.
- **`text.rs`** exists because mutation testing found that everything living in
  `main.rs` was untested — including which tasks count as overdue, which is
  product logic, not printing. Anything with a decision in it moves out of the
  binary so it can be tested. `main.rs` keeps argument parsing and IO.

A document is every line of the file in order, each carrying its own ending, so
that a mixed-endings file and a file with no final newline both survive:

```rust
struct Doc  { lines: Vec<Line> }
struct Line { item: Item, ending: Ending }   // Ending: Lf | CrLf | None
enum   Item { Task(Task), Text(String) }     // Text = everything we don't touch

struct Task {
    raw: String,        // the line exactly as it was read
    line_no: usize,
    done: bool,
    title: String,
    due: Option<Due>,
    tags: Vec<String>,
    priority: Option<Priority>,
    section: Option<String>,
    dirty: bool,        // false -> write `raw` back untouched
    checkbox: usize,    // byte index of the character between the brackets
}
```

`dirty` is what makes round-trip fidelity mechanical rather than a matter of
discipline. `checkbox` is what makes it survive the most common write of all:
marking something done replaces that one ASCII byte inside `raw` instead of
re-rendering the line, so the user's spacing and anything we failed to understand
come through untouched.

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
  blocking IO is more than enough. The event loop is one `poll` and one channel;
  `notify`'s own watcher thread is the whole of the concurrency in this program.
  An async runtime would grow compile times and binary size for free.
- **No `serde`.** We write the Markdown parser ourselves (it is the heart of the
  product anyway). `theme.conf` is a flat `key = value` file precisely so that it
  can be parsed in ~40 lines instead of pulling in serde + a TOML crate. serde
  arrives with `config.toml` in v2, if at all.
- **No `regex`.** `@date`, `#tag` and `!priority` are parsed by scanning
  word by word — faster than regex here, and much easier to produce good error
  messages from.
- **No `icalendar` crate.** VTODO output is ~30 lines of string formatting. Not
  worth a dependency. See [calendar.md](calendar.md).

Two notes on how the two terminal crates are declared, both of which are the kind
of thing that is invisible until it bites:

- **`ratatui` runs with `default-features = false`.** The defaults bring a
  calendar widget which brings its own date crate, alongside the `chrono` already
  here. We take `crossterm` and `layout-cache` and nothing else.
- **`crossterm` has to stay on the version ratatui re-exports.** A key event from
  two different crossterm versions is two different types that read identically
  in a compiler error. `cargo tree -i crossterm` printing one entry is the check.

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
