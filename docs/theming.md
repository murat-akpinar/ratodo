# Theming

People who live in kitty, konsole, alacritty and foot already theme everything on
their screen. A tool that hardcodes its colours is the one thing that looks wrong
in an otherwise coherent setup. So colours are user-configurable.

> **Decision reversal (2026-08-10).** A theme loader was previously rejected as
> YAGNI, with a fixed `theme.rs` of 11 constants. Reversed on the product owner's
> call: for this audience, theming is not a nice-to-have, it is what makes the
> tool belong on their desktop. See [decisions.md](decisions.md).
>
> What we kept from the old decision is the *restraint*: a flat config file, no
> new dependency, no hot reload in v1, no plugin system.

## The file

`~/.config/ratodo/theme.conf` — the user's, so it goes next to `todo.md` and into
their dotfiles.

The format is deliberately kitty-shaped, because that is the file this audience
already knows how to edit:

```conf
# ~/.config/ratodo/theme.conf

# start from a built-in theme
theme = catppuccin-mocha

# then override whatever you want
accent   = #cba6f7
overdue  = #f38ba8
background = none          # keep your terminal's background (transparency works)
```

Rules:

- One `key = value` per line.
- A `#` at the **start of a line** (after optional whitespace) is a comment.
  Everywhere else `#` is part of a colour value.
- Unknown keys and bad values are warned about on stderr and then ignored.
  **A broken theme file must never stop the program from starting.**
- No file at all → the built-in default. This is the common case and it must
  look good with zero configuration.

## Keys

Twelve roles, named after what they *do* — not after Catppuccin — so themes stay
portable. Each one has exactly one job, listed in
[design.md](design.md#what-each-colour-means):

| Key | What it colours | Default (Catppuccin Mocha) |
|---|---|---|
| `background` | the screen behind everything | `none` — see below |
| `foreground` | task titles, primary text | `#cdd6f4` |
| `dim` | dates, tags, secondary text | `#a6adc8` |
| `border` | frame and separators | `#6c7086` |
| `selection` | background of the selected row | `#313244` |
| `accent` | the tool's own voice: headings, the input box border, the focused date cell, the keys in `?` | `#cba6f7` |
| `overdue` | the overdue group and its `!` | `#f38ba8` |
| `today` | the today group | `#fab387` |
| `done` | the `✓` mark | `#a6e3a1` |
| `done_text` | the text of a completed task | `#7f849c` |
| `tag` | `#tag` | `#89b4fa` |
| `priority` | `!high` (bold) and `!med`; `!low` stays `dim` | `#f9e2af` |

The [design rules](design.md#rules) still apply on top of this: one accent plus
greys, red only for overdue, green only for done. A user can of course break
those rules in their own theme — that is their business.

## Value forms

| Form | Example | Note |
|---|---|---|
| 24-bit hex | `#cba6f7` | The usual case |
| Short hex | `#c9f` | Expanded to `#ccaa99`-style |
| ANSI index | `4`, `12` | 0–15, resolved by the terminal → follows the user's own terminal theme automatically |
| ANSI name | `blue`, `bright_black` | Same thing, readable |
| `none` / `default` | `background = none` | Paint nothing, let the terminal show through. **This is how transparency keeps working** |

`background = none` matters more than it looks: a lot of this audience runs a
translucent terminal, and a tool that paints an opaque background ruins it.

**Which is why every built-in theme ships `background = none`, including the
default.** The Catppuccin Mocha value `#1e1e2e` is the one you would set if you
wanted a painted background, and it is written down here for that purpose — but
opting *in* to opacity is the right way round. Someone on a translucent foot or
kitty who opens ratodo for the first time and sees a solid dark rectangle sitting
in their setup does not go looking for a config key; they close it. The other ten
keys are unaffected: only the background is `none` by default.

## Built-in themes

`theme = <name>` picks one; individual keys then override it.

| Name | Note |
|---|---|
| `catppuccin-mocha` | **Default.** Dark |
| `catppuccin-latte` | Light |
| `gruvbox-dark` | |
| `nord` | |
| `dracula` | |
| `terminal` | Uses only ANSI 0–15 — every colour comes from the user's own terminal palette. **The answer for anyone running pywal, wallust or base16:** "use my palette and get out of the way", with no twelfth colour file to keep in sync. Set it once and ratodo re-themes itself whenever the wallpaper does |

`terminal` does double duty: it is also the answer to "no truecolor" on a bare
TTY or inside old `screen`, which used to be an open risk. See
[risks.md](risks.md).

Built-in themes are `const` tables in `theme.rs` — the same twelve keys, filled
in. Adding one is a dozen lines and no new machinery.

## Precedence

Later wins:

```
built-in default
  → theme = <name>          in theme.conf
    → individual keys       in theme.conf
      → --theme <name>      on the command line
        → NO_COLOR=1        no colour at all, symbols only
```

`NO_COLOR` is respected because it is a standard and it costs nothing. The
`○ ✓ !` symbols already carry the meaning without colour — see
[design.md](design.md#rules) — so a no-colour run is still perfectly readable.
That was true before theming existed; it is what makes `NO_COLOR` cheap.

## Commands

| Command | What it does |
|---|---|
| `ratodo theme list` | List built-in themes |
| `ratodo theme dump` | Print the active theme as a valid `theme.conf` |
| `ratodo --theme <name>` | Run once with a different theme |

`dump` is the important one — it means a user never starts from an empty file:

```console
$ ratodo theme dump > ~/.config/ratodo/theme.conf
```

## Implementation notes

- **No new dependency.** The parser is `split_once('=')`, trim, and a hex decode.
  Roughly 40 lines. Using `serde` + TOML for this would cost a dependency and
  buy nothing — consistent with [architecture.md](architecture.md#dependencies).
- `theme.rs` grows from 11 constants to: the `Theme` struct, the built-in tables,
  a parser and a resolver. Still one file.
- The `Theme` struct is passed into the drawing code. No global, no `lazy_static`.
- Colour resolution happens **once at startup**, not per frame.

## Not in v1

- **Hot reload.** `notify` is already wired up for `todo.md`, so adding
  `theme.conf` to the watch list is cheap — but it is still extra state in the
  event loop, and editing a theme is a rare act. v2.
- **Themes as separate files** (`~/.config/ratodo/themes/mine.conf`,
  `theme = mine`). Wait until someone actually wants to share one.
- **Per-element style attributes** (bold, italic, underline). The two-level
  hierarchy in [design.md](design.md#rules) is deliberate; letting users add a
  third level is not a feature we owe them yet.
