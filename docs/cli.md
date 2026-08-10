# Command line and keys

Two entry paths, both in v1. The second one is the reason the product exists.

## Commands (v1)

| Command | What it does |
|---|---|
| `ratodo` | Opens the TUI on the agenda view |
| `ratodo add '<text>'` | Parses the text, appends the task, prints one line, exits. **The TUI never opens** |
| `ratodo list` | Prints the agenda to stdout and exits. Also the first thing that works during development |
| `ratodo done '<text>'` | Marks the matching task as done without opening the TUI |
| `ratodo status` | Prints the counts on one line, for a bar. See below |
| `ratodo sync` | Regenerates `todo.ics` by hand. See [calendar.md](calendar.md) |
| `ratodo theme list` | Lists the built-in themes |
| `ratodo theme dump` | Prints the active theme as a valid `theme.conf` |

`list` flags:

| Flag | Meaning |
|---|---|
| `--tag <name>` | Only tasks carrying `#name`. Repeatable; repeats mean OR |
| `--prio <level>` | Only `!high` / `!med` / `!low` |
| `--porcelain` | Machine-readable output. See below |

Global flags:

| Flag | Meaning |
|---|---|
| `--file <path>` | Use a different file instead of `~/.config/ratodo/todo.md`. The escape hatch for "work list separate, personal list separate" |
| `--theme <name>` | Run once with a different theme, overriding `theme.conf`. See [theming.md](theming.md) |
| `--help` / `--version` | clap defaults |

Path precedence: `--file` → `$RATODO_FILE` → `$XDG_CONFIG_HOME/ratodo/todo.md`.
The environment variable exists so that `direnv` can give a repository its own
list without an alias per checkout.

## Behaving like a Unix program

The audience pipes things. Three rules, decided before `ui.rs` exists so they do
not have to be retrofitted:

- **Colour is for terminals.** `NO_COLOR=1` disables it, and so does stdout not
  being a TTY (`std::io::IsTerminal`, no dependency). The `○ ✓ !` symbols carry
  the meaning without it.
- **stdout carries data, stderr carries talk.** "nothing here yet", warnings and
  conflict notices go to stderr, so `ratodo list | wc -l` counts tasks and
  nothing else.
- **Exit codes mean something.** `0` success, `1` an error, `2` a request that
  could not be answered — no match, or an ambiguous one.

### `list --porcelain`

One task per line, tab-separated, no colour, no summary line, no group headings:

```
$ ratodo list --porcelain
open	2026-08-12	write the deploy plan	ops	high
open		call the bank
done	2026-08-09	close the old PRs	ops
```

Fields: `state`, ISO date (empty if none), title, then tags and priority. Stable
across versions — this is the contract that makes

```
ratodo done "$(ratodo list --porcelain | fzf | cut -f3)"
```

work. The default `list` output is for humans and is *not* a stable interface;
note that it prints `[!]` for overdue tasks, so `grep '\[ \]'` over it silently
misses exactly the tasks that matter. That is what `--porcelain` is for.

### `status`

```
$ ratodo status
3 open · 1 overdue
$ ratodo status --json
{"text":"3 ○ 1!","tooltip":"3 open, 1 overdue","class":"overdue"}
```

The `class` field is the load-bearing one: waybar and eww key their CSS off it
(`ok`, `due`, `overdue`). Hand-formatted — a JSON object this shape does not
justify `serde`. In a bar config it is one line:

```json
"custom/todo": { "exec": "ratodo status --json", "return-type": "json", "interval": 60 }
```

Exits `1` when something is overdue, so `ratodo status || notify-send "$(ratodo status)"`
works with no extra flag.

### `done` matching

The text is matched against task titles, case-insensitively, as a substring.
**A unique match is required.** On several matches ratodo prints the candidates,
exits `2` and writes nothing; on none, the same without the list. Silently
ticking the wrong task is precisely the trust break that the whole round-trip
guarantee exists to prevent — see [testing.md](testing.md).

### `add` syntax

Everything after the command is free text; `@date`, `#tag` and `!priority` are
extracted from it, whatever their position. Date shorthand is expanded to ISO
before it is written:

```
$ ratodo add 'pay the invoice @tomorrow #home'
added: pay the invoice  ·  due tomorrow (2026-08-11)  ·  #home
```

Accepted shorthand: `@today @tomorrow @mon`…`@sun @3d @2w`. Full syntax in
[format.md](format.md).

### Quote with `'`, not `"`

The single quotes above are not a style choice. Two of the three metadata
characters are special to a shell, and one of them **fails loudly**:

| Character | bash / zsh | fish |
|---|---|---|
| `#tag` | Comment — only when unquoted | Comment — only when unquoted |
| `!high` | **History expansion, even inside `"…"`** → `bash: !high: event not found`, and nothing is added | Fine. fish has no history expansion |
| `@2026-08-12` | Fine | Fine |

Single quotes turn all three off in every shell, so every example in the docs and
the README uses them. This is a documentation problem, not a code one — there is
nothing ratodo can do about text its own process never receives.

### Shells

The commands themselves are shell-agnostic. Only three things differ, and the
README spells each out:

| | bash / zsh | fish |
|---|---|---|
| Alias | `alias r=ratodo` | `alias r ratodo` |
| Per-repo list | `export RATODO_FILE=…` | `set -x RATODO_FILE …` |
| Completions | `completions/ratodo.bash`, `.zsh` | `completions/ratodo.fish` |

Completions are **hand-written static files in `completions/`**, not generated:
`clap_complete` would be an eighth dependency, and the subcommand list is short
and fixed. If it ever stops being fixed, revisit that.

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
| `h` / `l` | Fold / unfold the group under the cursor |
| `z` | Same, as one toggle — `z` is the vim fold prefix |
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

`/` search, filtering from inside the TUI and `ratodo archive` are v2 — the flags
on `list` are the whole of v1's filtering. `notify-send` on overdue tasks, and
packaging for AUR and Nix, are v4. See [roadmap.md](roadmap.md).
