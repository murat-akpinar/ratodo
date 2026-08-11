<div align="center">

# ratodo

**A todo TUI, built with ratatui — one Markdown file, no cloud, no account.**

[![crates.io](https://img.shields.io/badge/crates.io-v0.7.0-green)](https://crates.io/crates/ratodo)
[![license](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
[![rust](https://img.shields.io/badge/rust-1.88%2B-orange)](https://www.rust-lang.org)

<img src="https://raw.githubusercontent.com/murat-akpinar/ratodo/main/assets/demo.gif"
     alt="The agenda, grouped by date. A task is ticked off, and a new one is typed as a single line — buy coffee beans @fri #home !high — with the date resolving live underneath."
     width="820">

</div>

> **v0.7.0, on crates.io.** `cargo install ratodo`. The command line and the TUI
> are built and tested — capture, editing, undo, folding, themes, the `.ics`
> export, several lists in one agenda, `$work` to say which one a capture goes
> to, `tab` for a date field that cannot hold a day the calendar does not,
> ruled columns on a roomy pane, and an input box that opens on today's date.
> Twelve theme roles, one job each.
> Reasoning behind every decision is in [`docs/`](docs/README.md), what comes
> next in [`todo.md`](todo.md).

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
┌ ratodo — 5 open · 1 overdue ───────────────── ▰▱▱▱▱▱▱▱ 1/6 ┐
│  OVERDUE ───────────────────────────────────────────────── │
│▌ ! rotate the backup keys               2d ago  !high  #ops│
│                                                            │
│  TODAY ─────────────────────────────────────────────────── │
│  ○ pay the invoice                             today  #home│
│  ○ review the deploy PR                        16:00  #work│
│                                                            │
│  THIS WEEK ─────────────────────────────────────────────── │
│  ○ book a dentist appointment            Thu 09:30  #health│
│  ✓ migrate the server                                   Wed│
│                                                            │
│  LATER ─────────────────────────────────────────────────── │
│  ○ write the release notes                           Aug 20│
└────────────────────────────────────────────────────────────┘
 j k move  spc done  a add  ⏎ edit  d cancel  ? keys  q quit
```

Vim keys, no vim modes: `j` `k` `g` `G` `ctrl-d` to move, `spc` to tick, `a` to
add and `⏎` to edit, `y` to copy, `d` to cancel and `p` to put a date off, `X` to
delete and
`u` to take it back, `h` `l` `z` to fold a group, `e` for `$EDITOR` — one key per
action, and `?` for the rest. Delete is the only shifted key: it is the one that
takes a line out of the file.

Ticking something turns the row green and records the day in the file
(`✓2026-08-11`); the row then shows when it was finished rather than when it was
due. `d` is the third state — decided against, not done — which stays on the list
as `- [-]` in red instead of being deleted, out of the counts and never overdue. `p`
asks how long (`2`, `3d`, `1w`, `fri`) and moves the date alone, keeping the time
and everything else on the line. `y` opens the same box filled with the task
under the cursor, as a new one, for the task that is nearly one you already have
— without the tick or the completion stamp, since a copy is work to do.

Adding and editing are the only thing that opens a second mode, and `esc` or
`ctrl-c` always closes it — in there `ctrl-c` costs you the sentence, never the
session. It opens as a box over the middle of the list, where your eye already
is, and under a rule it shows what the line will become as you type:

```
┌────────────────────────────────────────────────────┐
│ add ▏call the accountant @thu !high                │
├────────────────────────────────────────────────────┤
│      due Thursday (2026-08-13) │ !high             │
└────────────────────────────────────────────────────┘
```

`tab` swaps that preview for a date field, for the days you were going to count
on your fingers:

```
┌────────────────────────────────────────────────────┐
│ add ▏renew the passport                            │
├────────────────────────────────────────────────────┤
│      [10] 08  2026  ↓ ↑                            │
└────────────────────────────────────────────────────┘
```

`↑` `↓` change the part in brackets, `←` `→` move between the three, and
`13082026` fills all three in eight keystrokes. It cannot produce a day the
calendar does not have — the 31st of January arrowed into February is the 28th,
and a month of `13` is unreachable. `⏎` writes it into the line, `esc` leaves it
alone, and `@thu` is still there for everything faster than that.

Only two things ever cover the list and you open both of them — that box and the
`?` help. Deleting is undoable with `u` instead of asking you to confirm, and the
screen degrades to a 34-column pane in a tiling layout. Every screen:
[`docs/tui.md`](docs/tui.md).

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
| `- [ ]` / `- [x]` / `- [-]` | open / completed / cancelled task |
| `## Heading` | section |
| `@2026-08-12` | due date (`@2026-08-12 16:00` with a time) |
| `#tag` | tag, any number of them |
| `!high` `!med` `!low` | priority |
| `✓2026-08-11` | when it was completed — written for you when you tick it |

When adding, shorthand is allowed — `@today`, `@tomorrow`, `@mon`…`@sun`, `@3d`,
`@2w` — and always stored as an ISO date. Full spec:
[`docs/format.md`](docs/format.md).

## Where things live

| What | Where |
|---|---|
| Your tasks | `~/.config/ratodo/todo.md` — in your dotfiles, versioned with your own git |
| Calendar export | `~/.local/share/ratodo/todo.ics` |

**Several lists, one screen.** Every `*.md` in that directory is a list, so
keeping work and home apart on disk costs nothing and shows up as one agenda:

```console
$ ls ~/.config/ratodo/
2026.md  personal.md  theme.conf  work.md

$ ratodo                    # all three, one agenda
$ ratodo --file ~/.config/ratodo/work.md    # only this one
```

Dated groups mix — overdue is overdue, whichever file it came from — and undated
headings say where they are from (`## Sprint (work.md)`). A change is written
back to the file it came out of and nowhere else; a capture goes to `todo.md`
unless the line says otherwise with `$`:

```console
$ ratodo add 'call the accountant @thu $work'   # -> work.md
```

`$work` works in the TUI's input box too, where the preview says `→ work.md`
before you press `⏎`. The word addresses the capture and never lands in the
file. Full rules:
[`docs/cli.md`](docs/cli.md#several-lists).

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
| `--file <path>` | work on exactly this list — or set `$RATODO_FILE` |
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

`status` exits non-zero when something is overdue, so nagging yourself needs no
extra flag:

```bash
ratodo status || notify-send "$(ratodo status)"
```

`--porcelain` is five tab-separated fields — state, date, title, tags, priority —
and column three is the title, which is what `done` wants:

```bash
ratodo done "$(ratodo list --porcelain | fzf | cut -f3)"
```

`done` takes any part of the title and needs it to match exactly one open task.
If two match it shows you both and changes nothing.

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
is also the answer for a bare TTY). Twelve keys, full spec:
[`docs/theming.md`](docs/theming.md).

## Calendar

Open tasks that have a date are exported to `~/.local/share/ratodo/todo.ics` as
VTODO entries, one-way. Every capture rewrites it; `ratodo sync` does it by hand.

A time in your file has no timezone, so it does not get one here either —
`@2026-08-13 09:30` is exported as a floating time and stays half past nine
wherever you are.

Generating the file is the easy part. **Pick a program that reads todos** —
these are VTODO entries, and most of what people call "the calendar" draws
events and nothing else:

| Client | Shows them |
|---|---|
| todoman | ✅ |
| Thunderbird | ⚠️ in the Tasks view, not the month grid |
| Evolution | ⚠️ version-dependent |
| khal | ❌ it is a calendar; it draws events |
| GNOME Calendar | ❌ wants `webcal://`, draws events |
| Google Calendar | ❌ ignores VTODO entirely |

Subscribing, with [todoman](https://github.com/pimutils/todoman):

```python
# ~/.config/todoman/config.py
path = "~/.local/share/ratodo"
date_format = "%Y-%m-%d"
time_format = "%H:%M"
```

```console
$ todo list
[ ] 1 !!! 2026-08-12 call the accountant  [home]
[ ] 2     2026-08-13 09:30 book a dentist appointment  [health]
```

It goes read-only by itself, which is the whole design: ratodo owns the file.
Thunderbird is **New Calendar → On My Computer**, pointed at the same file, and
the entries land in Tasks rather than on the grid.

Details: [`docs/calendar.md`](docs/calendar.md).

## Living in your dotfiles

`todo.md` is meant to be symlinked into your dotfiles repo. Two things are worth
knowing before you do it:

**chezmoi will overwrite your list.** `chezmoi apply` writes its source copy over
the live file, and its source copy is whatever it was when you last added it — so
every task captured since then disappears. Add it to `.chezmoiignore`:

```
.config/ratodo/todo.md
```

and let ratodo own the file. `stow` and a bare git repo have no such problem; they
symlink, and a symlink is exactly what ratodo expects.

**If you keep the file open in nvim**, ratodo's writes land underneath you and
`:w` will put your stale buffer back over them. `set autoread` fixes it:

```vim
set autoread
autocmd FocusGained,BufEnter * checktime
```

The reverse direction is already handled — ratodo watches the file and picks up
anything you save from an editor on its own.

## Completions

Hand-written, in [`completions/`](completions/):

```bash
cp completions/ratodo.bash ~/.local/share/bash-completion/completions/ratodo
cp completions/ratodo.zsh  ~/.zfunc/_ratodo          # with ~/.zfunc on $fpath
```
```fish
cp completions/ratodo.fish ~/.config/fish/completions/ratodo.fish
```

## Install

```console
$ cargo install ratodo
```

Rust 1.88 or newer, and no other build dependency. From source instead:

```console
$ git clone https://github.com/murat-akpinar/ratodo && cd ratodo
$ git checkout v0.7.0
$ cargo install --path .        # → ~/.cargo/bin/ratodo
```

Or take the binary from the [release
page](https://github.com/murat-akpinar/ratodo/releases) — x86_64 Linux, built
against the system glibc.

**Arch.** A `PKGBUILD` is in the repository, so the package builds with the
completions, the licence and the docs where `pacman` expects them:

```console
$ cd packaging && makepkg -si
```

It is not on the AUR yet — that is a submission, not a file.

**Nix.** A flake is in the repository:

```console
$ nix profile install github:murat-akpinar/ratodo
```

Honest caveat: there is no `nix` on the machine this was written on, so the
`PKGBUILD` above was built and installed and the flake was not. A report either
way is welcome.

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
