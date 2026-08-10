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
| `--tag <name>` | Only tasks carrying `#name`. Repeatable; repeats mean OR. Case-insensitive, so `#Ops` answers to `--tag ops` |
| `--prio <level>` | Only `high`, `med` or `low` — the exact level, not "and above". Anything else is rejected before the file is opened |
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

One task per line, tab-separated, no colour, no summary line, no group headings.
Agenda order, so the most urgent line is the first one:

```
$ ratodo list --porcelain
open	2026-08-09	close the old PRs	ops	
open	2026-08-12	write the deploy plan	ops,home	high
open			call the bank		
```

**Five fields, always all five**, even when they are empty:

| # | Field | Notes |
|---|---|---|
| 1 | state | `open` or `done`. Overdue is not a state here — the date is right there in field 2, and a script that wants both can have both |
| 2 | due date | `YYYY-MM-DD`, empty if none. **Date only**; a time is display, not data a bar needs |
| 3 | title | Control characters, tabs included, are replaced with `�` so the field count cannot be forged by a line in the file |
| 4 | tags | Comma-separated, no `#`, empty if none |
| 5 | priority | `high` / `med` / `low`, empty if none |

The fixed count is the point: `cut -f5` has to mean priority on every line, which
it cannot if tags each take a column of their own. The format grows by **appending**
a sixth column, never by changing what one to five mean.

An empty result prints nothing at all — not even the "nothing here yet" hint,
which is help for a human — and still exits `0`. A filter that matches nothing is
an answer, not an error.

Stable across versions — this is the contract that makes

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

The `class` field is the load-bearing one: waybar and eww key their CSS off it,
so those three words are an interface — renaming one silently unstyles somebody's
bar.

| `class` | When |
|---|---|
| `overdue` | anything is past its date |
| `due` | nothing is late, but something is due today |
| `ok` | neither |

`text` gains its ` N!` only when something is overdue, and `tooltip` names the
due-today count only when there is one, so a quiet list reads `{"text":"2 ○",
"tooltip":"2 open, 1 due today","class":"due"}`.

Hand-formatted — a JSON object this shape does not justify `serde`, and it is
only safe to hand-format because every value in it is a number or one of those
fixed words. **No text from the user's file reaches it.** Putting a task title in
the tooltip is the change that would need escaping first.

A list that does not exist yet is `0 open · 0 overdue` and exit `0`, not an
error: the bar starts polling before the user has captured anything.

In a bar config it is one line:

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
