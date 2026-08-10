<div align="center">

# ratodo

**A todo TUI, built with ratatui — one Markdown file, no cloud, no account.**

[![status](https://img.shields.io/badge/status-v1%20in%20progress-orange)](todo.md)
[![license](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
[![rust](https://img.shields.io/badge/rust-1.97%2B-orange)](https://www.rust-lang.org)

</div>

> ⚠️ **Status: v1 in progress.** `ratodo add` and `ratodo list` work today; the
> TUI below is designed but not built yet. Progress in [`todo.md`](todo.md),
> reasoning in [`docs/`](docs/README.md).
>
> ```console
> $ cargo run -- --file ./todo-test.md add 'try ratodo @tomorrow #test'
> ```

## What it is

You are running i3, Hyprland or sway. You are in the middle of something, and a
task pops into your head. You type one command, it lands in a Markdown file that
is already in your dotfiles, and you go back to what you were doing.

```console
$ ratodo add 'pay the invoice @tomorrow #home'
added: pay the invoice  ·  due tomorrow (2026-08-11)  ·  #home
```

Then, when you want the overview, `ratodo` opens the agenda:

```
┌─ ratodo ────────────────────────────── 5 open · 1 overdue ─┐
│                                                            │
│ OVERDUE ─────────────────────────────────────────────────  │
│   ! rotate the backup keys                   2d ago  #ops  │
│                                                            │
│ TODAY ───────────────────────────────────────────────────  │
│ ▌ ○ pay the invoice                                 #home  │
│   ○ review the deploy PR                     16:00  #work  │
│                                                            │
│ THIS WEEK ───────────────────────────────────────────────  │
│   ○ book a dentist appointment         Thu 09:30  #health  │
│   ✓ migrate the server                                     │
│                                                            │
│ LATER (3) ──────────────────────────────────────────── l   │
│                                                            │
└────────────────────────────────────────────────────────────┘

 j k move   spc done   a add   ⏎ edit   d del   e $EDITOR   ? keys   q quit
```

Vim keys, no vim modes: `j` `k` `g` `G` `ctrl-d` to move, one key per action, and
`?` for the rest. Nothing pops over the list, deleting is undoable with `u`
instead of asking you to confirm, and it degrades to a 34-column pane in a tiling
layout. Every screen: [`docs/tui.md`](docs/tui.md).

## Why

Everyone already does `vim ~/todo.md`. That gives you a file you own, but no
answer to "what is due today?". Every tool that gives you the agenda takes the
file away in exchange — taskwarrior stores `~/.task/*.data`, todo apps store it
in someone else's cloud.

> **The tool is the file's guest, not its owner.**

Delete ratodo and your `todo.md` still works. That is the whole idea, and every
decision in [`docs/`](docs/README.md) follows from it.

## The file

```markdown
## Work

- [ ] rotate the backup keys @2026-08-08 #ops !high
- [ ] review the deploy PR @2026-08-10 16:00 #work
- [x] close the old PRs #work

> Any line ratodo doesn't recognise — this quote, a table, your own
> notes — is preserved byte-for-byte. It is your file.
```

| Syntax | Meaning |
|---|---|
| `- [ ]` / `- [x]` | open / completed task |
| `## Heading` | section |
| `@2026-08-12` | due date (`@2026-08-12 16:00` with a time) |
| `#tag` | tag, any number of them |
| `!high` `!med` `!low` | priority |

When adding, shorthand is allowed — `@today`, `@tomorrow`, `@mon`…`@sun`, `@3d`,
`@2w` — and always stored as an ISO date. Full spec:
[`docs/format.md`](docs/format.md).

## Where things live

| What | Where |
|---|---|
| Your tasks | `~/.config/ratodo/todo.md` — in your dotfiles, versioned with your own git |
| Calendar export | `~/.local/share/ratodo/todo.ics` |

Sync is your git. There is no account, no server, no telemetry, and nothing
leaves the machine.

## Commands

| | |
|---|---|
| `ratodo` | open the TUI |
| `ratodo add '<text>'` | capture a task and exit |
| `ratodo list` | print the agenda |
| `ratodo list --tag ops` | just the `#ops` ones — also `--prio high` |
| `ratodo list --porcelain` | one tab-separated line per task, for `fzf` and `grep` |
| `ratodo done '<text>'` | mark a task done |
| `ratodo status` | `3 open · 1 overdue`, for your bar — also `--json` |
| `ratodo sync` | regenerate `todo.ics` |
| `ratodo theme list` / `dump` | list built-in themes / print the active one |
| `--file <path>` | use a different list — or set `$RATODO_FILE` |
| `--theme <name>` | run with a different theme |

**Use `'single quotes'`, not `"double"`.** In bash and zsh, `!high` inside double
quotes is history expansion: you get `bash: !high: event not found` and nothing
is added. fish does not have this problem, but single quotes are right in all
three.

It gets typed a lot, so alias it:

```bash
alias r=ratodo          # bash / zsh
```
```fish
alias r ratodo          # fish
```

In your bar, it is one line:

```json
"custom/todo": { "exec": "ratodo status --json", "return-type": "json", "interval": 60 }
```

## Theming

Your terminal is themed, so this is too. Drop a `~/.config/ratodo/theme.conf`:

```conf
theme = catppuccin-mocha    # or catppuccin-latte, gruvbox-dark, nord, dracula, terminal

accent     = #cba6f7        # override anything you like
overdue    = #f38ba8
background = none           # keep your terminal's background — transparency works
```

Start from the current colours instead of an empty file:

```console
$ ratodo theme dump > ~/.config/ratodo/theme.conf
```

`--theme <name>` overrides it for one run, `NO_COLOR=1` turns colour off
entirely, and `theme = terminal` uses only your terminal's own 16 colours (which
is also the answer for a bare TTY). Eleven keys, full spec:
[`docs/theming.md`](docs/theming.md).

## Calendar

Open tasks that have a date are exported to `todo.ics` as VTODO entries, one-way.
Generating the file is the easy part; subscribing to it is up to your client, and
they differ a lot:

| Client | Local `.ics` |
|---|---|
| khal | ✅ |
| Thunderbird | ✅ |
| Evolution | ⚠️ version-dependent |
| GNOME Calendar | ⚠️ mostly wants `webcal://` |
| Google Calendar | ❌ ignores VTODO entirely |

Details and subscription steps: [`docs/calendar.md`](docs/calendar.md).

## Install

Not published yet. To build it now:

```console
$ git clone https://github.com/murat-akpinar/ratodo && cd ratodo
$ cargo build --release
$ ./target/release/ratodo --help
```

Once v1 lands: `cargo install ratodo`.

## Documentation

| | |
|---|---|
| [docs/product.md](docs/product.md) | what this is, who it is for, what is out of scope |
| [docs/format.md](docs/format.md) | the file format, in full |
| [docs/architecture.md](docs/architecture.md) | data flow, modules, dependencies |
| [docs/design.md](docs/design.md) | palette, layout, agenda rules |
| [docs/tui.md](docs/tui.md) | every screen, the keymap, narrow-width behaviour |
| [docs/theming.md](docs/theming.md) | `theme.conf`, colour keys, built-in themes |
| [docs/cli.md](docs/cli.md) | commands and keybindings |
| [docs/calendar.md](docs/calendar.md) | `.ics` export and client support |
| [docs/testing.md](docs/testing.md) | test strategy and fixtures |
| [docs/roadmap.md](docs/roadmap.md) | v1 → v5 |
| [docs/decisions.md](docs/decisions.md) | settled, rejected, and open questions |
| [todo.md](todo.md) | what is being built next |

## Not planned

Cloud sync, accounts, a Kanban board, subtasks, time tracking, recurring tasks
(v3), two-way CalDAV (v5), a plugin system. Reasoning for each is in
[docs/product.md](docs/product.md#out-of-scope).

## License

[GPL-3.0](LICENSE)

---

<div align="center">
<sub>Built <b>with</b> ratatui — not a ratatui plugin.</sub>
</div>
