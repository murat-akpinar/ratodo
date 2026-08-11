# todo

The build list. Decisions behind any of these live in [docs/](docs/README.md);
loose ends live in [notes.md](notes.md).

**v1 shipped as `v0.1.0` and the next batch as `v0.2.0`, both on 2026-08-11.**
Steps 0–8 below are the record of how that was built and are kept for the
reasoning in them, not because there is anything left to do in them. The work
that is actually open is the short list directly under this line.

## What is left

Four open: one is code in the product and came out of a day of real use, the
other three are packaging. Nothing here blocks anything else, so the order is by
reach rather than by dependency. The ticked ones are kept here rather than moved
down, because the reasoning in them is about things that were asked for and are
not being built — a key, and a box split into fields.

- [x] **Copying a task means retyping it.** A task that is nearly one you already
      have — same tag, same shape, different day — had no way in but `a` and the
      whole line again. `y` opens the input box pre-filled with the selected
      task, as a new one, and `⏎` saves it. Asked for as `y`/`p` with a register;
      `p` is taken and the register bought nothing, because a capture lands in
      the capture target wherever the cursor is. See
      [docs/decisions.md](docs/decisions.md#settled)

- [x] **`$list` in the input, and the input as four fields.** Asked for on
      2026-08-11, and it is two pieces that arrived in one sentence. The first
      shipped. The second was measured and **rejected**, and what it was really
      after shipped instead as one dim row.
      - [x] **`$work` routes the capture.** `a` wrote to `todo.md` and nothing
            else — [cli.md](docs/cli.md#several-lists) rule 4 — so capturing into
            `work.md` meant leaving the TUI for
            `ratodo --file ~/.config/ratodo/work.md add '...'`. `$` puts it in
            the box: a fourth sigil beside `@` `#` `!`, read by the one
            tokenizer `capture::parts` already is, previewed as `→ work.md` the
            way `@thu` is previewed as a date. Rule 4 said a fixed target on
            purpose — "`a` must not mean a different file depending on what the
            cursor happens to be over" — and `$` does not break that: the target
            is a **word the user typed**, not the cursor position. That
            distinction is the whole reversal and it went in
            [docs/decisions.md](docs/decisions.md#a-capture-always-goes-to-todomd--work-picks-the-list-2026-08-11)
            before the code did —
            **done**, and it works on `ratodo add` as well as in the box. The
            word never reaches the file, `$work` and `$work.md` are one list,
            the first `$` wins, `$50` is money, and a `$` on `⏎` is refused
            rather than swallowed, because `capture` drops the word and silence
            would eat the title. Two of the open questions below went with it.
            One thing the suite would not have caught and running it did: the
            preview nagged `no list w.md` through every keystroke of `$work`, so
            it now waits until the word can no longer become one of the lists —
            the same rule the date warning already had
      - [x] **The box becomes four fields with `tab` between them** — **no.**
            The reversal was drafted and then measured, and the measurement
            killed it. It is **five** fields once `!high` is counted, the box is
            `min(70, pane − 4)` wide, and a 34-column pane leaves **28 columns**
            — five fields and four separators is three characters a field. At 60
            it is forty-two columns and `2026-08-13` alone is ten. Drawable at
            eighty and nowhere else, which buys a second input mode for the
            narrow pane the product was designed around.
            The invariant argument is the one that settles it: keeping one
            tokenizer means joining the fields back into a line for
            `capture::parts`, at which point the boundaries are decoration over
            the same string, paid for with a focus state, `tab`/`shift-tab`,
            five carets and five scroll windows. Not joining them is a second
            parser, and two parsers of the same box eventually disagree about
            what it will write. A tag *field* also cannot hold `#home #work`,
            which one line already does. Written up in
            [docs/decisions.md](docs/decisions.md#settled)
      - [x] **What it was actually after — discoverability — shipped instead.**
            An empty box now reads `@thu #home !high $list` in the dim, exactly
            as the empty `p` box reads `how long? 2 3d 1w fri`. Twenty-two
            columns, so it fits the narrowest pane the design promises; gone the
            moment there is anything to report; `$list` only when there is more
            than one list to address. No mode, no keymap, no second parser
      - **What went with the rejection:** `p` keeps its one field, the
        five-versus-four question is moot, and the two `$` questions were
        answered by the first piece — `$nosuchfile` is refused before the write
        rather than created, and with one list `$` parses the same and refuses
        anything but that list
- [ ] **A date that does not exist is accepted in silence.** Found in use on
      2026-08-11: `@2026-13-45` resolves to nothing, so the whole word falls
      back to being part of the title — the file gets
      `- [ ] task @2026-13-45`, the task keeps no date, and neither the live
      preview nor the status line ever says so. The fallback itself is correct
      and stays: a word we did not understand is the user's text and we do not
      eat it. What is missing is that the preview goes quiet in exactly the
      moment it should speak.
      - [x] **The preview says so.** An `@` that can never become a date gets
            named under the input, in the colour the bottom line warns in.
            *Can never* rather than *does not yet*: the line redraws on every
            keystroke, and one that fires on `@2`, `@20`, `@202` on the way to
            `@2026-08-20` is one nobody reads by the time it is right. Caught
            by running the binary, not by the suite — the first version nagged
            through ten presses. See
            [docs/decisions.md](docs/decisions.md#settled)
      - [ ] **The field-by-field date entry** — `↑ ↓` on the part under the
            cursor and eight digits filling `DD MM YYYY`, which is a keymap and
            a widget, not a message, and makes the invalid state unrepresentable
            rather than merely detectable. What it would cost is worked through
            in
            [notes.md](notes.md#the-date-field--a-proposal-not-a-decision-2026-08-11),
            and it becomes an entry in
            [docs/decisions.md](docs/decisions.md) before a line of it is written
- [ ] **`cargo publish`** — `--dry-run` passes (44 files, 157 KiB), and the only
      thing in the way is a verified email address on crates.io. One command
      after that. This is the item that decides whether anyone outside this
      machine can install the thing with a tool they already have
- [ ] **Thunderbird** — the third and last calendar data point. Its Tasks view is
      a different code path from the month grid and is where a VTODO would land.
      `todoman` displays the file correctly and `khal` ignores it; Thunderbird is
      the one that decides whether the table in
      [docs/calendar.md](docs/calendar.md) is finished or still guessing. It is
      also what tells us how big an audience `--as-events` would actually buy,
      and that flag is already on the [v2 roadmap](docs/roadmap.md)
- [ ] **`flake.nix` and an AUR `PKGBUILD`** — `rustPlatform.buildRustPackage` and
      a `PKGBUILD` against the tag. NixOS users will not `cargo install` into a
      profile, and Arch is the platform this was written on. Both pin a released
      version, which is why they come after the tag rather than before it

Open questions that block none of the above are in
[docs/decisions.md](docs/decisions.md#open-questions).

## 0 — Setup

- [x] Rust toolchain (1.97.1)
- [x] `git init`, remote configured
- [x] Verify the name is free — crates.io ✅, GitHub ✅, PATH ✅ (see [docs/naming.md](docs/naming.md))
- [x] Design record written up in `docs/`
- [x] `cargo init --name ratodo`
- [x] `Cargo.toml`: GPL-3.0, MSRV 1.88 — deps added per step, not all seven up front
- [x] Verify truecolor: `printf "\x1b[38;2;203;166;247mmauve\x1b[0m\n"`
- [x] Install a client — to see the `.ics` displayed, not just parsed —
      **done**: `todoman`. khal was the obvious guess and is the wrong tool; it
      draws events, we write todos

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
- [x] The other half of it: a client actually **displaying** the file, which is
      what catches one that quietly ignores VTODO —
      **done**: it caught one on the first try. `todoman` lists all five tasks
      with dates, times, categories and priorities, and a change made in ratodo
      is there on the next `todo list` with no sync step. `khal` shows none of
      them, and a hand-written VEVENT in the same directory with the same config
      *did* appear — so it is VTODO being ignored, not our file being wrong.
      [docs/calendar.md](docs/calendar.md) had khal down as ✅ on nothing more
      than "it is file-based"; corrected
- [x] **`cargo test` rewrote the real `~/.local`** — `write_back` resolved the
      backup and calendar paths from the environment, so in-process tests
      regenerated the developer's own `todo.ics` from a fixture and left a
      `.bak` per case in `~/.local/state/ratodo`; `tests/cli.rs` set
      `XDG_STATE_HOME` and forgot `XDG_DATA_HOME` —
      **done**: both paths are resolved once in `dispatch` and carried, the
      integration tests set all four XDG directories through one helper, and a
      test pins that a write lands where the caller pointed it. Found because a
      calendar being read went empty, not by the suite
- Thunderbird is the one client still unchecked — see
  [What is left](#what-is-left)

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
- [x] Keys: `j k g G ctrl-d ctrl-u` · `spc` · `a o ⏎` · `d u X` · `h l z` · `e` ·
      `r` · `?` · `esc` · `q`
- [x] `h`/`l` fold the group under the cursor — lf/ranger/yazi muscle memory, not
      "fold LATER". A collapsed group is selectable, which is the only way back
- [x] Input mode: `⏎` save, `esc` cancel, `ctrl-c` cancel (**never quit**), and nothing else can open it
- [x] **Live parse preview** under the input — `@thu` resolves as you type. It
      costs the list a row while it is open — see
      [docs/decisions.md](docs/decisions.md#reversed)
- [x] `X` deletes immediately; `u` undoes delete / toggle. Edit joins it with the
      input mode. *(Was `d`; swapped with cancel on 2026-08-11 so that the key
      taking a line out of the file is the one asking for shift — see
      [docs/decisions.md](docs/decisions.md#reversed))*
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

## 9 — After v0.1.0, and what became v0.2.0

Tagged 2026-08-11. The three below are the release; the packaging and publishing
that were on this list have moved up to [What is left](#what-is-left).

- [x] **Several lists in one agenda** — every `*.md` in the config directory is
      read, the undated headings say which file they came from, a change goes
      back to the file it came from with that file's own mtime check and backup,
      and a capture goes to `todo.md`. The file is attached to a task only when
      there is more than one, so a single-file setup keeps its identities and its
      calendar UIDs. See [docs/cli.md](docs/cli.md#several-lists)
- [x] **A finished task is grey, and finishing one says nothing back.** Green is
      reserved for completed — [docs/design.md](docs/design.md) — and the only thing
      wearing it is the progress bar. Ticking a task should show in the row, and
      the file should record *when*: `✓2026-08-11`, a fourth field beside `@`,
      `#` and `!` — **done**: the row is green, the stamp is written and taken
      back off by unticking, and the date column on a finished row shows the day
      it was finished rather than the deadline that stopped applying
- [x] **There is no third state.** A task that is neither done nor still wanted
      can only be deleted, which loses the record of having decided against it.
      `- [-]` — the Obsidian/Logseq convention — with `d` to set it, out of the
      counts and never overdue — **done**: `✗` on screen in the grey a finished
      row gave up, out of the counts, never overdue, never exported, and `d`
      takes it back. `x` itself stays unbound, for the reason it always was.
      *(Shipped on `X`; swapped with delete on 2026-08-11)*
- [x] **Pushing a date out means retyping the whole line.** `⏎` reopens the
      input for a task whose only problem is that it is not today's problem.
      `p` should ask for how long — `2`, `3d`, `1w`, `fri` — and move `@` alone,
      keeping the time and everything the parser did not understand —
      **done**: the same input box with a different question, and a preview that
      answers it with the day it lands on

## Open questions blocking nothing

Tracked in [docs/decisions.md](docs/decisions.md#open-questions): whether a
completed task stays in place or moves to a `## Done` section, and whether a
list per repository needs `ratodo` to walk up the tree. The other four the code
answered on its way past them, and they have moved to
[resolved](docs/decisions.md#resolved-questions).
