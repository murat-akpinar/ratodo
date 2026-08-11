# CLAUDE.md

Working rules for AI agents in this repository. Read [docs/README.md](docs/README.md)
before touching anything — the decisions live there, not here.

**Project:** ratodo — a todo TUI in Rust + ratatui. Single Markdown file, no
cloud, no account. v0.1.0 is tagged; the code is the eight flat modules in
`src/` described in [docs/architecture.md](docs/architecture.md#module-layout).

---

## Workflow — follow this order, every time

1. **Write the code.**
2. **Review it yourself.** Re-read the diff before running anything. Look for:
   does it break a hard invariant below, does it add a dependency, does it touch
   a line of the user's file it had no business touching, is there an `unwrap`
   in a path that can fail at runtime.
3. **Test.**
   ```
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   python3 scripts/check-docs.py
   ```
   Green tests are not the same as working software: run the binary against a
   throwaway file too, and say what you actually saw.

   After changing `parse`, `write`, `model`, `capture` or `text`, also run
   `cargo mutants --timeout 90`. A **MISSED** mutant is a change to the source
   that no test objected to — in those modules that is a hole in the fidelity
   guarantee, and it gets a test rather than an excuse. See docs/testing.md.
4. **Only if all of it passes: commit.** If anything fails, fix it or report the
   failure — never commit red.
5. **Update the changelog:** `git cliff -o CHANGELOG.md`, then commit it
   separately as `chore(changelog): update`.
6. **Push.** One push at the end, after the changelog commit — not two.

Never skip 2 or 3. Never report "done" for work that was not tested.

## Releasing — the maintainer sees it first

**`cargo publish` is permanent.** A published version cannot be withdrawn, only
yanked, and the last two releases each shipped something that looked right in a
test and wrong on a real screen. So a release stops for a human look:

1. `cargo install --force --path .` — the same version number means `--force` is
   not optional, or the old binary stays on `PATH` and everyone is looking at
   yesterday's build.
2. **Stop. Say it is installed, and wait.** The maintainer runs `ratodo` in their
   own terminal and says whether it is right. Driving it on a pty here is
   evidence that it *works*; it is not evidence that it *reads* well, and that is
   the half a release cannot take back.
3. Only after they say so: bump the version, tag, `cargo publish`, point the
   PKGBUILD at the new tag.

Never publish on the strength of a green suite alone, and never bundle the
publish into the same turn as the change it is publishing.

## Commits — attribution rules

- **Commits go out under the repository owner's signature only.**
- **Never** add `Co-Authored-By: Claude`, `Generated with Claude Code`,
  `🤖`, or any other AI attribution — not to commit messages, not to PR bodies,
  not to the changelog, not to code comments.
- Do not change `user.name` / `user.email`; use whatever git is configured with.

## Commits — format

Conventional Commits are **mandatory**. `cliff.toml` sets
`filter_unconventional = true`, so a non-conventional message silently
disappears from the changelog.

```
<type>(<scope>): <subject>
```

- **types:** `feat` `fix` `docs` `perf` `refactor` `style` `test` `chore` `ci` `revert`
- **scopes:** `parse` `write` `agenda` `ics` `ui` `cli` `theme` `docs`
- **subject:** English, imperative, lowercase, no trailing period

```
feat(parse): keep the raw line for every task
fix(write): preserve the trailing newline when the file has none
docs: split the decision record into docs/
test(agenda): cover the today-at-midnight boundary
```

Breaking changes: `feat(format)!: ...` plus a `BREAKING CHANGE:` footer.

## Language

Everything that lands in the repository is **English**: code, identifiers,
comments, UI strings, documentation, commit messages. *(The docs were originally
written in Turkish and translated on 2026-08-10.)*

Chat with the maintainer in **Turkish**.

## Hard invariants — do not break these without an explicit decision

1. **Round-trip fidelity.** The parser keeps every task's raw line. A line the
   tool did not modify is written back byte-for-byte. This is the project's most
   important property — [docs/architecture.md](docs/architecture.md#round-trip-fidelity).
2. **Never reorder, reformat or reflow the user's file.** Unrecognised lines are
   untouchable.
3. **Writes are atomic** — temp file → `fsync` → `rename` — with a `.bak` first
   and an mtime check before overwriting. On a concurrent edit: warn, do not merge.
4. **`agenda(&[Task], today)`** takes `today` as a parameter. No `Local::now()`
   inside it, ever, or it stops being testable.
5. **Panic hook restores the terminal.** A TUI that panics in raw mode wrecks the
   user's shell.
6. **No fixed FPS.** Draw on events only; block when idle.
7. **Never write `- [!]`** to the file. `!` is a screen symbol derived from the
   date. The file contains exactly three states — `[ ]`, `[x]` and `[-]` — and
   `[-]` is the only one ever added to that list
   ([docs/decisions.md](docs/decisions.md#settled)).
8. **A broken `theme.conf` must never prevent startup.** Warn on stderr, fall back.
9. **No new dependencies** without asking. The seven allowed crates: `ratatui`,
   `crossterm`, `clap`, `chrono`, `notify`, `directories`, `anyhow`. In
   particular there is no `tokio`, no `serde`, no `regex`, no `icalendar`, and
   each of those has a reason — [docs/architecture.md](docs/architecture.md#dependencies).

## Before adding a feature

Check [docs/product.md](docs/product.md#out-of-scope) first. Scope creep is the
named number-one project risk. If it is on the out-of-scope list, the answer is
no unless the maintainer reverses the decision explicitly — and a reversal gets
recorded in [docs/decisions.md](docs/decisions.md), not applied silently.

## Code style

- **Few comments.** The reasoning lives in `docs/`; repeating it in the source is
  noise that goes stale. One `//!` line per module pointing at the document that
  explains it, and otherwise comment only what the code cannot say itself: a
  non-obvious invariant, a deliberate deviation, a trap.
- When a block genuinely needs marking out, delimit it:
  ```rust
  // -- ALAN START: round-trip --
  ...
  // -- ALAN END --
  ```
  Sparingly. If a section needs a marker to be understandable, first ask whether
  it should be its own function.
- rustfmt defaults; clippy clean at `-D warnings`
- `anyhow` for errors; no `unwrap()` outside tests (`expect` with a real message
  is acceptable in `main`)
- Keep modules flat — the eight files in
  [docs/architecture.md](docs/architecture.md#module-layout) are the whole plan.
  No `mod.rs` pyramid, no trait layer
- Pure functions stay pure: `parse`, `write`, `agenda`, `ics`, `theme` take input
  and return output. No clock, no terminal, no globals

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
in `docs/decisions.md`. Do not leave two documents disagreeing. When a task in
`todo.md` is finished, check it off in the same commit as the work.
