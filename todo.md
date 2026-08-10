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
- [ ] `cargo init --name ratodo`
- [ ] `Cargo.toml`: the seven crates, GPL-3.0, MSRV
- [ ] Verify truecolor: `printf "\x1b[38;2;203;166;247mmauve\x1b[0m\n"`
- [ ] Install khal or Thunderbird (to verify the `.ics` output)

## 1 — Fixtures (no terminal needed)

- [ ] `tests/fixtures/simple.md` — copy of [docs/examples/todo.md](docs/examples/todo.md)
- [ ] `tests/fixtures/gnarly.md` — the deliberately awkward one, draft in [docs/testing.md](docs/testing.md)
- [ ] Write the expected `Vec<Task>` for each fixture

## 2 — parse + write (no terminal needed) ← the heart of the product

- [ ] `model.rs`: `Task { raw, line_no, done, title, due, tags, priority, dirty }`
- [ ] `parse.rs`: line → `Task`. **The raw line is always kept**
- [ ] `parse.rs`: `@date`, `@date HH:MM`, `#tag`, `!priority`, word-by-word, no regex
- [ ] `parse.rs`: shorthand dates — `@today @tomorrow @mon…@sun @3d @2w` → ISO
- [ ] `write.rs`: if `dirty == false`, write the raw line back untouched
- [ ] `write.rs`: atomic write — temp → `fsync` → `rename`, `.bak` beforehand
- [ ] `write.rs`: mtime check — if it changed since we read it, warn and refuse
- [ ] **Round-trip test:** `parse(write(parse(x))) == parse(x)`
- [ ] **Fidelity test:** every untouched line byte-for-byte identical
- [ ] `ratodo list` → `println!`. The product works from here on

## 3 — agenda + ics (no terminal needed)

- [ ] `agenda.rs`: `agenda(&[Task], today) -> Vec<Group>` — `today` is a **parameter**
- [ ] Group tests: overdue / today / this week / later / undated
- [ ] Boundary tests: exactly today 00:00, exactly +7 days, a past year, an invalid date
- [ ] `ics.rs`: VTODO output (~30 lines of string formatting, no crate)
- [ ] `ics.rs`: stable UID, CRLF, 75-octet line folding
- [ ] Snapshot test **plus** real verification: feed the output to khal

## 4 — ratatui (the genuinely new part)

- [ ] **Panic hook on day one** — a TUI that panics in raw mode wrecks the terminal
- [ ] A dumb list: print the task titles, `↑↓`, quit with `q`
- [ ] Event loop: `crossterm::event::poll` + notify's mpsc channel
- [ ] **No fixed FPS** — draw on events, block when idle (0% CPU at rest)
- [ ] inotify: re-read when the file changes from outside

## 5 — Theme

- [ ] `theme.rs`: the `Theme` struct, 11 role keys
- [ ] Built-in themes as `const` tables: catppuccin-mocha (default), catppuccin-latte, gruvbox-dark, nord, dracula, terminal
- [ ] `theme.conf` parser (~40 lines, no serde): `key = value`, `#` comments
- [ ] Value forms: `#rrggbb`, `#rgb`, ANSI index, ANSI name, `none`
- [ ] Precedence: built-in → `theme =` → individual keys → `--theme` → `NO_COLOR`
- [ ] Bad input never aborts: warn on stderr, fall back
- [ ] `ratodo theme list` and `ratodo theme dump`
- [ ] Verify `background = none` in a transparent terminal

## 6 — Assemble and apply the design

- [ ] Draw the grouped agenda, `○ ✓ !` symbols
- [ ] ASCII fallback: `[ ]` `[x]` `[!]`
- [ ] `a` add · `⏎` toggle · `d` delete · `e` `$EDITOR` · `l` expand LATER · `q` quit
- [ ] `clap`: `ratodo` · `add` · `list` · `done` · `sync` · `theme`
- [ ] `--file` and `--theme` global flags
- [ ] Check column alignment with non-ASCII and emoji

## 7 — Release

- [ ] README: khal and Thunderbird subscription steps
- [ ] `cargo publish --dry-run`
- [ ] Tag `v0.1.0`, generate the changelog with git-cliff

## Open questions blocking nothing

Tracked in [docs/decisions.md](docs/decisions.md#open-questions): whether a
completed task stays in place, whether `--file` is enough for multiple lists,
when `.ics` gets regenerated, and whether `* [ ]` is recognised.
