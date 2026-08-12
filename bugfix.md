# Windows findings — 2026-08-12

Investigated on Windows 11 Pro (tr-TR), PowerShell 5.1, ratodo 0.7.1, against
the real `%APPDATA%\ratodo\config\todo.md`. Nothing here is applied; every fix
below was written, run and then reverted, so the working tree is clean.

The reported symptom was: *"the app won't run — I captured one task, then every
key I pressed printed a syntax error in the terminal."*

---

## 1. Every write after the first fails — `backup_path` builds an illegal name

**The blocker. This is the reported bug.**

### What the user sees

```
Error: writing the backup C:\Users\<user>\AppData\Local\ratodo\cache\?-C:-Users-<user>-…-todo.md.bak

Caused by:
    Dosya adı, dizin adı veya birim etiketi sözdizimi hatalı. (os error 123)
```

`os error 123` is `ERROR_INVALID_NAME`. The Turkish text is Windows' own —
*"the file name, directory name or volume label syntax is incorrect"* — which is
the "syntax error" in the report. It does not come from ratodo and it does not
come from the shell.

### Why it looks like "the TUI doesn't start"

`save` only takes a backup when the file already exists ([write.rs:73](src/write.rs#L73)),
so the **first** capture into a fresh list succeeds and every later write dies:

| action | outcome |
|---|---|
| `ratodo add …` into a new list | works — nothing to back up |
| `ratodo add …` again | **os error 123** |
| `ratodo` → `spc`, `x`, `d`, `u`, `⏎` | **os error 123** |

In the TUI every one of those keys reaches `write_back`, whose non-`Conflict`
error arm re-raises ([main.rs:678](src/main.rs#L678)). That `?` unwinds through
`run` → `ratatui::restore()` → `dispatch` → process exit. So the TUI vanishes on
the first key that changes anything, prints the Windows message, and hands the
terminal back. Keys pressed after that go to PowerShell, which answers with
errors of its own — hence "whatever key I press".

Round-trip fidelity is not violated: the write is refused *before* the temp file
is written, so the list on disk is never damaged. It is unusable, not unsafe.

### Root cause

[write.rs:145](src/write.rs#L145):

```rust
fn backup_path(dir: &Path, target: &Path) -> PathBuf {
    let slug: String = target
        .to_string_lossy()
        .chars()
        .map(|c| if std::path::is_separator(c) { '-' } else { c })
        .collect();
    dir.join(format!("{}.bak", slug.trim_start_matches('-')))
}
```

`target` comes from `fs::canonicalize` ([write.rs:70](src/write.rs#L70)), which on
Windows returns a **verbatim** path — `\\?\C:\Users\you\todo.md`. Only the
separators are flattened, so the slug keeps two characters NTFS refuses:

```
\\?\C:\Users\you\todo.md  →  ?-C:-Users-you-todo.md.bak
                             ↑    ↑
                             both illegal in a Windows file name
```

The illegal set is `< > : " / \ | ? *`. On Linux `/home/you/todo.md` →
`home-you-todo.md.bak` and nothing is wrong, which is why this never showed up.

### Fix (written and verified, then reverted)

```rust
/// The full target path, flattened into one file name. A single `todo.md.bak`
/// slot would mean the backup of one `--file` list quietly replacing another's.
///
/// Everything Windows forbids in a name goes, not only the separators: there
/// `canonicalize` answers with a verbatim `\\?\C:\…` path, and a `?` or a `:`
/// left in the slug is os error 123 on every write past the first.
fn backup_path(dir: &Path, target: &Path) -> PathBuf {
    let slug = target
        .to_string_lossy()
        .split(|c: char| std::path::is_separator(c) || r#":?*"<>|"#.contains(c))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    dir.join(format!("{slug}.bak"))
}
```

`split` + `filter(non-empty)` also absorbs the old `trim_start_matches('-')` and
collapses the runs of separators the verbatim prefix leaves behind. **Existing
Linux backup names are unchanged** — `/home/you/todo.md` still slugs to
`home-you-todo.md.bak`, so nobody's `.bak` directory is orphaned by this.

### The test it needs

Pure, runs on both platforms, and fails on today's code with exactly the name
seen in the wild (`?-C:-Users-you-todo.md.bak`):

```rust
/// Every write after the first takes a backup, so a name the filesystem
/// refuses is the whole tool broken rather than one lost `.bak`.
///
/// Windows hands `canonicalize` a verbatim path — `\\?\C:\…` — and both `?`
/// and `:` are illegal in a file name there. Flattening only the separators
/// left them in, and every capture past the first died on os error 123.
#[test]
fn a_backup_name_holds_nothing_a_filesystem_refuses() {
    let windows = backup_path(Path::new("bak"), Path::new(r"\\?\C:\Users\you\todo.md"));
    let name = windows.file_name().unwrap().to_string_lossy();
    assert_eq!(name, "C-Users-you-todo.md.bak", "still unwritable on Windows");

    let unix = backup_path(Path::new("bak"), Path::new("/home/you/todo.md"));
    assert_eq!(
        unix.file_name().unwrap().to_string_lossy(),
        "home-you-todo.md.bak",
        "the name every existing backup already has"
    );
}
```

### Verification actually run

With the fix in place, against a directory whose name carries Turkish letters
**and** a space — `%TEMP%\ratodo dız ğş çİ\liste\todo.md`:

```
1. add  → added: faturayi ode · due tomorrow (2026-08-13) · #ev · !high
2. add  → added: sut al · #market          ← the write that used to fail
3. done → done: sut al

- [ ] faturayi ode @2026-08-13 #ev !high
- [x] sut al #market ✓2026-08-12

backup written: C-Users-…-Temp-ratodo dız ğş çİ-liste-todo.md.bak
```

---

## 2. `cargo test` does not compile on Windows at all

Two test cases reach for Unix-only APIs without a `cfg`, so the whole binary
test target fails to build and **the suite has never run on this platform**:

- [main.rs:1547](src/main.rs#L1547) `use std::os::unix::fs::PermissionsExt;` and two
  `Permissions::from_mode` calls — E0433, E0599. It makes a directory read-only
  to force a write failure; Windows has no equivalent.
- Gating it `#[cfg(unix)]` matches the precedent already in the repo
  (`a_symlinked_list_stays_a_symlink`, [write.rs:435](src/write.rs#L435)).

With both cases gated, the unit suites are green on Windows: **270 lib + 28 bin**.

---

## 3. Six integration tests fail on Windows — they steer the binary with XDG

`tests/cli.rs` points ratodo at a scratch directory by setting
`XDG_CONFIG_HOME` / `XDG_STATE_HOME` / `XDG_DATA_HOME`. `directories` ignores
those on Windows and answers from the Known Folder API, so the cases read the
developer's *real* config and data directories:

```
the_default_path_is_the_config_directory                        cli.rs:739
the_backup_lands_in_the_state_directory_and_nowhere_near_the_list  cli.rs:132
the_calendar_is_written_beside_no_one_and_kept_up_to_date       cli.rs:1313
only_open_tasks_reach_the_calendar                              cli.rs:528
a_broken_theme_file_warns_and_never_stops_anything              cli.rs:1431
the_theme_flag_overrides_the_file                               cli.rs:1404
```

Same class as `which_files_count_as_lists` ([main.rs:2210](src/main.rs#L2210)).

Two ways out, and the choice is a decision rather than a fix:

1. **`#[cfg(unix)]` on the six.** One line each, honest about what they steer
   with, and CI is Linux so nothing is really lost. Windows keeps no coverage of
   where the files land.
2. **Give the config directory the `Derived` treatment.** `dirs()` is read deep
   inside `lists`, `default_path`, `active_theme`, `backup_dir` and `ics_path`;
   resolving it once in `main` and passing it down would make all six portable —
   and would be the same argument [main.rs:272](src/main.rs#L272) already makes for
   the backup and calendar paths, for the same reason.

Option 2 is the one consistent with the note already in the source. It is also a
real refactor, not a bugfix, so it belongs in `todo.md` rather than in a patch
that goes out with the fix above.

---

## 4. `scripts/check-docs.py` reports a false failure on Windows

```
FAIL tests\fixtures\simple.md:5  missing file  -> ../format.md
1 broken link(s)
```

[check-docs.py:9](scripts/check-docs.py#L9) skips `"tests/fixtures"`, and
[line 44](scripts/check-docs.py#L44) compares with
`str(p.relative_to(ROOT)).startswith(s)`. On Windows that string is
`tests\fixtures\simple.md`, which never matches the forward-slash prefix, so a
test fixture gets linted as documentation.

One-line fix — compare on a normalised path:

```python
and not any(p.relative_to(ROOT).as_posix().startswith(s) for s in SKIP)
```

---

## Not bugs — answers to the questions asked

### Where the files live on Windows

`directories` follows the Known Folder API, so there is no `~/.config`:

| | Linux | Windows |
|---|---|---|
| the list | `~/.config/ratodo/todo.md` | `%APPDATA%\ratodo\config\todo.md` |
| the calendar | `~/.local/share/ratodo/todo.ics` | `%APPDATA%\ratodo\data\todo.ics` |
| the backups | `~/.local/state/ratodo/` | `%LOCALAPPDATA%\ratodo\cache\` |

The backups land in the cache directory because `state_dir()` is `None` off
Linux and [main.rs:299](src/main.rs#L299) falls back to `cache_dir()`. Worth a line
in `docs/cli.md` — the current text names only the XDG paths.

### Turkish characters and spaces in the user name

No problem, and this was tested rather than assumed (see §1). Paths are carried
as `PathBuf` and never go through a shell, so spaces need no quoting; a Windows
path is UTF-16 and every Turkish letter survives `to_string_lossy` intact. The
`.bak` slug keeps both, and NTFS accepts both.

One caveat that is not ratodo's: PowerShell 5.1's `Get-Content` and `>` default
to the ANSI code page, so reading or writing `todo.md` **through those** mangles
`ç ğ ı ş İ`. ratodo itself reads and writes UTF-8 in both directions. Use
`Get-Content -Encoding UTF8`, or just let ratodo do it.

### `#`, `@` and `!` on the command line

Not a ratodo bug — the shell eats them before the process starts. In PowerShell
`#` opens a comment and the rest of the line is gone:

```powershell
ratodo add fatura ode @tomorrow #ev !high     # → "- [ ] fatura ode"
ratodo add "fatura ode @tomorrow #ev !high"   # → date, tag and priority all land
```

bash does the same thing with `#`, which is why the help text and the README
already show the quoted form. Nothing to change in the code; the TUI's `a` key
is the way that never has this problem, and §1 is what stops people reaching it.

---

## Suggested order

1. §1 — the fix and its test. This is the release-blocker; nothing else on
   Windows works until it lands.
2. §2 — two `#[cfg(unix)]` lines, or the suite cannot be run on Windows to
   confirm §1.
3. §4 — one line, unblocks the documented check.
4. §3 — a decision first (`cfg` vs. threading the config directory through
   `main`), then the work. Not part of the fix above.
5. The `docs/cli.md` paths table.
