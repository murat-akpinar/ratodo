# todo

The v1 task list, in build order. Decisions behind any of these live in
[docs/](docs/README.md); loose ends live in [notes.md](notes.md).

The order is deliberate: **once step 2 is done there is already a working CLI
todo** (ugly, but real). If step 4 stalls, the project does not die.

## 0 — Setup

- [x] Rust toolchain (1.97.1)
- [x] `git init`, remote configured
- [x] Verify the name is free — crates.io ✅, GitHub ✅, PATH ✅ (see [docs/naming.md](docs/naming.md))
- [x] Design record written up in `docs/`
- [x] `cargo init --name ratodo`
- [x] `Cargo.toml`: GPL-3.0, MSRV 1.88 — deps added per step, not all seven up front
- [x] Verify truecolor: `printf "\x1b[38;2;203;166;247mmauve\x1b[0m\n"`
- [ ] Install khal or Thunderbird — to see the `.ics` displayed, not just parsed

## 1 — Fixtures (no terminal needed)

- [x] `tests/fixtures/simple.md` — copy of [docs/examples/todo.md](docs/examples/todo.md), kept in sync by a test
- [x] `tests/fixtures/gnarly.md` — the deliberately awkward one
- [x] `crlf.md`, `no-final-newline.md`, `empty.md` — the byte-level edge cases
- [x] Expected parse results asserted in `tests/fidelity.rs`

## 2 — parse + write (no terminal needed) ← the heart of the product

- [x] `model.rs`: `Doc` / `Line` / `Item` / `Task`, each line keeping its own ending
- [x] `parse.rs`: line → `Task`. **The raw line is always kept**
- [x] `parse.rs`: `@date`, `@date HH:MM`, `#tag`, `!priority`, word-by-word, no regex
- [x] `capture.rs`: shorthand dates — `@today @tomorrow @mon…@sun @3d @2w` → ISO
- [x] `write.rs`: if `dirty == false`, write the raw line back untouched
- [x] `write.rs`: atomic write — temp → `fsync` → `rename`, `.bak` beforehand
- [x] `write.rs`: mtime check — if it changed since we read it, refuse and say so
- [x] **Round-trip test:** `parse(render(parse(x))) == parse(x)`
- [x] **Fidelity test:** toggling any one task changes exactly one byte, on every fixture
- [x] `ratodo list` and `ratodo add` → the product works from here on
- [x] `tests/property.rs`: 4000 generated documents, the generator its own oracle
- [x] `cargo mutants` clean over `parse` / `write` / `model` / `capture` / `text`

## 2.5 — What two design reviews found (no terminal needed)

Cheap fixes to things already built, plus the decisions that came out of the
2026-08-10 reviews. Details in [docs/decisions.md](docs/decisions.md#reversed),
the abandonment risk in [docs/risks.md](docs/risks.md).

- [x] `write.rs`: `.bak` goes to `~/.local/state/ratodo/`, not next to the list —
      a `.bak` in a dotfiles repo means `git status` is dirty after every capture.
      The backup directory is a **parameter**; `write.rs` reads no environment
- [x] `write.rs`: the backup is named after the whole target path, so two `--file`
      lists cannot overwrite each other's insurance
- [x] `model.rs`: `push_task` inserts after the last task, not at EOF. In a file
      ending with a table or `---` the captured task landed outside every `##`
- [x] `main.rs`: the empty-list message goes to **stderr**, so `list | wc -l` is honest
- [x] `main.rs`: `$RATODO_FILE` between `--file` and the XDG default
- [x] Single-quote every shell example in the README and docs: `!high` inside
      `"…"` is history expansion in bash and zsh, and the add never happens
- [x] Colour off when stdout is not a TTY. **Nothing to gate:** `ratodo list`
      prints no colour at all — the same bytes down a pipe as on a screen — and
      the only thing that emits any is the TUI, which already opens on a TTY and
      nowhere else. Settled and written up in [docs/cli.md](docs/cli.md);
      colouring `list` would be a feature, not this rule

## 3 — agenda + the scriptable surface (no terminal needed)

- [x] `agenda.rs`: `agenda(&[Task], today) -> Vec<Group>` — `today` is a **parameter**
- [x] Group tests: overdue / today / this week / later / undated
- [x] Boundary tests: exactly today 00:00, exactly +7 days, a past year, an invalid date
- [x] `list --tag` / `--prio` — the agenda says nothing about undated tasks, and
      most of a developer's list is undated
- [x] `list --porcelain` — tab-separated, stable, no colour. The contract behind
      `ratodo done "$(ratodo list --porcelain | fzf | cut -f3)"`
- [x] `ratodo status` and `--json` — `class` is the field waybar keys its CSS off
- [x] `status` exits non-zero when something is overdue
- [x] `done "<text>"`: unique match required; ambiguous → print candidates, exit 2,
      **write nothing**

## 4 — ratatui (the genuinely new part)

- [x] **Panic hook on day one** — a TUI that panics in raw mode wrecks the terminal.
      `ratatui::try_init` installs one that restores raw mode and the alternate
      screen; `Terminal`'s Drop puts the cursor back
- [x] A dumb list: print the task titles, `↑↓`, quit with `q`
- [x] **No fixed FPS** — draw on events; a wake-up with nothing to do draws
      nothing. Measured: 40 wake-ups in 20 idle seconds, zero CPU ticks
- [x] The TUI only opens on a TTY — `ratodo | wc -l` lists instead
- [x] Event loop: `crossterm::event::poll` + notify's mpsc channel. *(Was a
      blocking channel with a reader thread; reversed so `e` could exist — see
      [docs/decisions.md](docs/decisions.md#reversed))*
- [x] inotify: re-read when the file changes from outside. The watch is on the
      **directory**, because every safe writer renames over the file
- [x] The cursor stays on its task across a reload — by identity, see step 6

## 4.5 — ics (was step 3; moved behind the TUI)

Still v1, still one-way. It serves seed point 6, but the people it reaches are
Thunderbird and GNOME users, not the tiling-WM audience of seed point 2 — so it
does not get to block the screen that audience actually opens. See
[docs/decisions.md](docs/decisions.md#reversed).

- [x] `ics.rs`: VTODO output (~30 lines of string formatting, no crate)
- [x] `ics.rs`: stable UID, CRLF, 75-octet line folding
- [x] `ratodo sync`, and a regenerate after every capture
- [x] Real verification: the output parsed by Python's `icalendar` — a different
      implementation of the same RFC. Comma escaping, folding of a Turkish and
      emoji title, and the floating time all came back intact
- [ ] The other half of it: khal or Thunderbird actually **displaying** the file,
      which is what catches a client that quietly ignores VTODO

## 5 — Theme

- [x] `theme.rs`: the `Theme` struct, 11 role keys
- [x] Built-in themes as `const` tables: catppuccin-mocha (default), catppuccin-latte, gruvbox-dark, nord, dracula, terminal
- [x] Every built-in ships `background = none` — transparency is opt-out, not opt-in
- [x] `theme.conf` parser (no serde): `key = value`, `#` comments
- [x] Value forms: `#rrggbb`, `#rgb`, ANSI index, ANSI name, `none`
- [x] Precedence: built-in → `theme =` → individual keys → `--theme` → `NO_COLOR`
- [x] Bad input never aborts: warn on stderr, fall back
- [x] `ratodo theme list` and `ratodo theme dump`
- [x] The theme reaches the screen — the selected row keeps its own colour, so
      an overdue task is still red under the cursor
- [x] Verify `background = none` by eye in a transparent terminal. Asserted in a
      test (every built-in ships `Color::Reset`), confirmed in a pty by the
      absence of a background escape, and looked at on 2026-08-11

## 6 — Assemble and apply the design

Screens and keymap: [docs/tui.md](docs/tui.md).

- [x] Draw the grouped agenda with header rules, `○ ✓ !` symbols, `▌` selection
- [x] ASCII fallback: `[ ]` `[x]` `[!]`, `>` selection — chosen from the locale,
      and it takes the frame and the punctuation with it
- [x] The bottom line: hints, results, warnings and the input field
- [x] Keys: `j k g G ctrl-d ctrl-u` · `spc` · `a o ⏎` · `d u` · `h l z` · `e` ·
      `r` · `?` · `esc` · `q`
- [x] `h`/`l` fold the group under the cursor — lf/ranger/yazi muscle memory, not
      "fold LATER". A collapsed group is selectable, which is the only way back
- [x] Input mode: `⏎` save, `esc` cancel, `ctrl-c` cancel (**never quit**), and nothing else can open it
- [x] **Live parse preview** under the input — `@thu` resolves as you type. It
      costs the list a row while it is open — see
      [docs/decisions.md](docs/decisions.md#reversed)
- [x] `d` deletes immediately; `u` undoes delete / toggle. Edit joins it with the
      input mode
- [x] Write-conflict line with `r` reload. A refusal while the input is open
      re-reads by itself and hands the typed text back to the field
- [x] Selection survives reload — by identity, not row index. `Task::identity`
      is the section and the title, and it is the same one the `.ics` UID is
      built from, so "the same task" has one definition
- [x] A toggled task does not change position until the next reload
- [x] Empty state with the file path and a worked example
- [x] `?` help overlay — only the keys that are built, and `esc` closes it
- [x] Progress on the right of the title rule — eight cells and a `3/8`, green
      because green already means finished. Only once something is ticked; the
      bar gives way below 60 columns and the count stays
- [x] Width breakpoints: ≥60 / 34–59 / <34, in the documented drop order
- [x] Height under 10 rows: collapse the hint bar
- [x] `NO_COLOR=1` on a bare TTY still reads correctly
- [x] `:` and `/` answer on the bottom line instead of doing nothing
- [x] `clap`: `ratodo` · `add` · `list` · `done` · `sync` · `theme`
- [x] `--file` and `--theme` global flags
- [x] Check column alignment with non-ASCII and emoji — display columns via
      ratatui's own width, so no eighth dependency

## 7 — Release

- [x] README: khal and Thunderbird subscription steps
- [x] README: `set autoread` for people with nvim open on the file in another pane
- [x] README: a `.chezmoiignore` note — `chezmoi apply` overwrites a live `todo.md`
- [x] `completions/ratodo.{bash,zsh,fish}` — hand-written, no `clap_complete`, and a
      test asks the binary what it answers to so they cannot rot quietly
- [x] Time a cold start; the `$mod+t` scratchpad makes it a spec, aim under 50 ms
      — measured 1.2 ms median for `list`, 20 runs
- [x] `cargo publish --dry-run` — 44 files, 157 KiB compressed. `exclude` keeps
      the machinery of working on the project out of it (`CLAUDE.md`, `.vscode`,
      `cliff.toml`, `scripts/`, `notes.md`, `todo.md`); `docs/` stays
- [x] Tag `v0.1.0`, generate the changelog with git-cliff — tagged, and a
      GitHub release with the binary attached

## 8 — Visual polish (deliberately last, and blocks nothing)

The screen works and is documented; this is the pass for making it feel less
plain. It comes **after** the tag on purpose — none of it is a bug, and shipping
a working v0.1.0 beats holding one back for looks.

The frame every item here has to fit is [docs/design.md](docs/design.md#rules),
and it is a tight one: one accent colour plus greys, two levels of hierarchy and
no third, one layout, nothing that depends on a Nerd Font, no meaning carried by
colour alone. Anything that needs a rule bent gets written up in
[docs/decisions.md](docs/decisions.md) first — an item on this list is a
**candidate**, not a decision.

The standing test for each: does it tell the reader something, or does it just
decorate? The progress bar earned its place by the first; the second is how a
side pane turns into a dashboard nobody leaves open.

- [x] Progress on the right of the title rule — the one already done, as the
      worked example of the standard above
- [x] **Wide panes waste their width.** Past 60 columns nothing new appears, the
      gap in the middle just stretches. A fourth breakpoint could show the full
      date and the section a dated task came from —
      **done as columns at ≥ 80**: date, priority and tags start in the same
      place on every row and the group rule stops at the title column. The
      section a task came from is still not shown; it is a second decision, not
      part of this one. See [docs/decisions.md](docs/decisions.md#reversed)
- [x] **Dated groups and the file's own `##` sections look identical.** `OVERDUE`
      and `Work` are both a bold word plus a rule, though one is ours and one is
      the user's. Careful: "two levels of hierarchy, there is no third" —
      **done**: the user's headings keep the `##` they have in the file. No
      second colour and no third level; see
      [docs/decisions.md](docs/decisions.md#settled)
- [x] **A completed task still shows how late it is** — `✓ review the deploy PR
      … 1d ago`. It is finished; the lateness stopped being true —
      **done**: it shows the plain date (`Aug 8`) instead, and stays in
      `OVERDUE`, where membership was always positional
- [x] **`!high` is easy to miss**, sitting dim next to the tags. It is the one
      field the user typed to mean *urgent* and the screen barely says so —
      **done**: it is bold and in the row's own colour. Weight, not a twelfth
      theme role, so it still reads under `NO_COLOR`. `!med` and `!low` unchanged
- [x] Empty screen and `?` overlay — both correct and both plain —
      **done**: the empty screen's example moved into the box `a` actually
      opens, drawn by the same code, so the live parse under it resolves
      `@tomorrow` before a key is pressed; under ten rows it goes back to being
      a line. The overlay's exit moved to the bottom border — no row spent, and
      the box is back to twelve on a fourteen-row pane. See
      [docs/decisions.md](docs/decisions.md#settled)
- [x] Decide what to do about the help overlay's `↓ ↑` under a non-UTF-8 locale.
      The main screen goes fully ASCII and the overlay does not; the buffer test
      never covered it because it does not open the overlay —
      **done**: `down up` and `ret`, and the test now opens the overlay. Two more
      escapes went with it: the `…` on a cut title and the `·` in the input
      preview. `LC_ALL=C` now puts nothing non-ASCII on the screen

## After v0.1.0

- [x] **Several lists in one agenda** — every `*.md` in the config directory is
      read, the undated headings say which file they came from, a change goes
      back to the file it came from with that file's own mtime check and backup,
      and a capture goes to `todo.md`. The file is attached to a task only when
      there is more than one, so a single-file setup keeps its identities and its
      calendar UIDs. See [docs/cli.md](docs/cli.md#several-lists)
- [ ] `cargo publish` — blocked on a verified email address on crates.io
- [ ] `flake.nix` (`rustPlatform.buildRustPackage`) and an AUR `PKGBUILD`

## Open questions blocking nothing

Tracked in [docs/decisions.md](docs/decisions.md#open-questions): whether a
completed task stays in place, whether `--file` is enough for multiple lists,
when `.ics` gets regenerated, and whether `* [ ]` is recognised.
