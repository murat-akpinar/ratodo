# Command line and keys

Two entry paths, both in v1. The second one is the reason the product exists.

## Commands (v1)

| Command | What it does |
|---|---|
| `ratodo` | Opens the TUI on the agenda view |
| `ratodo add "<text>"` | Parses the text, appends the task, prints one line, exits. **The TUI never opens** |
| `ratodo list` | Prints the agenda to stdout and exits. Also the first thing that works during development |
| `ratodo done "<text>"` | Marks the matching task as done without opening the TUI |
| `ratodo sync` | Regenerates `todo.ics` by hand. See [calendar.md](calendar.md) |
| `ratodo theme list` | Lists the built-in themes |
| `ratodo theme dump` | Prints the active theme as a valid `theme.conf` |

Global flags:

| Flag | Meaning |
|---|---|
| `--file <path>` | Use a different file instead of `~/.config/ratodo/todo.md`. The escape hatch for "work list separate, personal list separate" |
| `--theme <name>` | Run once with a different theme, overriding `theme.conf`. See [theming.md](theming.md) |
| `--help` / `--version` | clap defaults |

`NO_COLOR=1` disables colour entirely; the `○ ✓ !` symbols still carry the
meaning.

### `add` syntax

Everything after the command is free text; `@date`, `#tag` and `!priority` are
extracted from it, whatever their position. Date shorthand is expanded to ISO
before it is written:

```
$ ratodo add "pay the invoice @tomorrow #home"
added: pay the invoice  ·  due tomorrow (2026-08-11)  ·  #home
```

Accepted shorthand: `@today @tomorrow @mon`…`@sun @3d @2w`. Full syntax in
[format.md](format.md).

## Keys (TUI, v1)

Vim-flavoured, but **not modal** — there is no normal/insert distinction to think
about. Full keymap, the reasoning, and the keys left deliberately unbound are in
[tui.md](tui.md#keys).

| Key | Action |
|---|---|
| `j` `k` / `↓` `↑` | Move the selection |
| `g` / `G` | Top / bottom |
| `spc` | Toggle done |
| `a` / `o` | Add a task (inline input on the bottom line) |
| `⏎` | Edit the selected task |
| `d` / `u` | Delete / undo |
| `l` / `z` | Fold LATER / fold the group under the cursor |
| `e` | Open `$EDITOR` on the file, re-read on exit |
| `?` | Key help |
| `q` / `Ctrl-C` | Quit |

`e` is the escape hatch: whatever the tool cannot do, the user can still do in
the file. Ten lines of code, and it fits the audience exactly.

## An alias is expected

`ratodo` is 6 characters and gets typed maybe 20 times a day. That friction is
real, and the README suggests fixing it:

```fish
alias r ratodo          # fish
```
```bash
alias r=ratodo          # bash / zsh
```

The name cannot be taken back; an alias can. See [naming.md](naming.md).

## Not in v1

`/` search, tag/priority filters and `ratodo archive` are v2. `ratodo status
--json` (for a waybar/eww module) is v4. See [roadmap.md](roadmap.md).
