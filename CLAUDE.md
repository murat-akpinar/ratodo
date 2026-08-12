# CLAUDE.md

Working rules for AI agents. The decisions live in
[docs/README.md](docs/README.md) — read it before touching anything;
[docs/map.md](docs/map.md) indexes *which file to open*.

**ratodo** — a todo TUI in Rust + ratatui. Single Markdown file, no cloud, no
account. Eleven flat modules in `src/`
([docs/architecture.md](docs/architecture.md#module-layout)).

---

## Workflow — this order, every time

1. **Write the code.**
2. **Review the diff yourself**, before running anything: does it break a hard invariant below, add a dependency, touch a line of the user's file it had no business touching, or `unwrap` where runtime can fail?
3. **Test** — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `python3 scripts/check-docs.py`. Green tests are not working software: also run the binary on a throwaway file and say what you actually saw. After changing `parse`, `write`, `model`, `capture` or `text`, run `cargo mutants --timeout 90` too — a **MISSED** mutant is a hole in the fidelity guarantee and gets a test, not an excuse ([docs/testing.md](docs/testing.md)).
4. **Commit only if all of it passed.** Never commit red — fix it or report it.
5. **Changelog:** `git cliff -o CHANGELOG.md`, committed separately as `chore(changelog): update`.
6. **Push once**, at the end, after the changelog commit — not twice.

Never skip 2 or 3. Never report "done" for work that was not tested.

## Releasing — the maintainer sees it first

**`cargo publish` is permanent** (yanking is not withdrawing), and releases have
shipped what looked right in a test and wrong on a real screen. So:

1. `cargo install --force --path .` — without `--force` an unchanged version number leaves yesterday's binary on `PATH`.
2. **Stop, say it is installed, and wait.** The maintainer runs `ratodo` in their own terminal; a pty here shows it *works*, not that it *reads* well.
3. Only once they say so: bump the version, tag, `cargo publish`, point the PKGBUILD at the new tag.
4. **A GitHub release, every time, no exceptions** — skipped for v0.6.0/v0.7.0/v0.7.1, all backfilled later. `gh release create vX.Y.Z`, the `x86_64-linux` binary attached as `ratodo-vX.Y.Z-x86_64-linux`, a title saying what changed rather than the number again, a body written for a reader rather than the generated changelog pasted in. The tag is not the release; people find the binary on that page.

Never publish on a green suite alone, and never in the same turn as the change
being published.

## Commits — attribution

- Commits go out under the repository owner's signature **only**.
- **Never** add `Co-Authored-By: Claude`, `Generated with Claude Code`, `🤖` or any other AI attribution — not to commit messages, PR bodies, the changelog, or code comments.
- Do not change `user.name` / `user.email`; use whatever git is configured with.

## Commits — format

`<type>(<scope>): <subject>`, mandatory — `cliff.toml` sets
`filter_unconventional = true`, so anything else silently vanishes from the
changelog. E.g. `feat(parse): keep the raw line for every task`.

- **types:** `feat` `fix` `docs` `perf` `refactor` `style` `test` `chore` `ci` `revert`
- **scopes:** `parse` `write` `agenda` `ics` `ui` `cli` `theme` `docs`
- **subject:** English, imperative, lowercase, no trailing period
- **breaking:** `feat(format)!: ...` plus a `BREAKING CHANGE:` footer

## Language

English in the repository — code, identifiers, comments, UI strings, docs,
commit messages. Chat with the maintainer in **Turkish**.

## Hard invariants — do not break these without an explicit decision

1. **Round-trip fidelity** — the parser keeps every task's raw line; a line the tool did not modify is written back byte-for-byte. The project's most important property ([docs/architecture.md](docs/architecture.md#round-trip-fidelity)).
2. **Never reorder, reformat or reflow the user's file.** Unrecognised lines are untouchable.
3. **Writes are atomic** — temp file → `fsync` → `rename`, a `.bak` first, an mtime check before overwriting. On a concurrent edit: warn, do not merge.
4. **`agenda(&[Task], today)` takes `today` as a parameter** — no `Local::now()` inside it, ever, or it stops being testable.
5. **Panic hook restores the terminal** — a panic in raw mode wrecks the user's shell.
6. **No fixed FPS** — draw on events only; block when idle.
7. **Never write `- [!]`** — `!` is a screen symbol derived from the date. The file holds exactly three states, `[ ]`, `[x]` and `[-]`, and `[-]` is the only one ever added ([docs/decisions.md](docs/decisions.md#settled)).
8. **A broken `theme.conf` must never prevent startup** — warn on stderr, fall back.
9. **No new dependencies** without asking. The seven allowed: `ratatui`, `crossterm`, `clap`, `chrono`, `notify`, `directories`, `anyhow` — in particular no `tokio`, `serde`, `regex`, `icalendar`, each for a reason ([docs/architecture.md](docs/architecture.md#dependencies)).

## Before adding a feature

Check [docs/product.md](docs/product.md#out-of-scope) first — scope creep is the
named number-one risk. Out of scope means no, unless the maintainer reverses the
decision explicitly, and a reversal gets recorded in
[docs/decisions.md](docs/decisions.md) rather than applied silently.

## Code style

- **Few comments** — the reasoning lives in `docs/` and goes stale when copied into the source. One `//!` line per module pointing at its document; otherwise comment only what the code cannot say itself: a non-obvious invariant, a deliberate deviation, a trap.
- Delimit a block only if it genuinely needs marking out (`// -- ALAN START: round-trip --` … `// -- ALAN END --`), sparingly — if it needs a marker to be understandable, ask first whether it should be its own function.
- rustfmt defaults; clippy clean at `-D warnings`.
- `anyhow` for errors; no `unwrap()` outside tests (`expect` with a real message is acceptable in `main`).
- Modules stay flat — the eleven files in [docs/architecture.md](docs/architecture.md#module-layout) are the whole plan. No `mod.rs` pyramid, no trait layer.
- Pure functions stay pure: `parse`, `write`, `agenda`, `ics`, `theme` take input and return output. No clock, no terminal, no globals.

## Repository layout

| Path | What it is |
|---|---|
| `docs/` | The decision record. Editing a file here means changing a decision |
| `notes.md` | Raw thinking, loose ends, the idea graveyard |
| `todo.md` | The task list. Work top to bottom |
| `README.md` | User-facing. English, and the first sentence says "built **with** ratatui" — never "for" |
| `scripts/check-docs.py` | Verifies every relative Markdown link and anchor resolves |
| `scripts/demo.py` | Records `assets/demo.gif` for the README. Needs kitty, menyoki, ffmpeg, X11 |
| `cliff.toml` | git-cliff config. Conventional commits required |
| `CHANGELOG.md` | Generated. Never edit by hand |

## Documentation upkeep

When a decision changes, update the document that owns it and note the reversal
in `docs/decisions.md` — never leave two documents disagreeing. Check a finished
`todo.md` task off in the same commit as the work.
