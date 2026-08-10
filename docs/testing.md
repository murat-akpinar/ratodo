# Testing strategy

The big advantage of this project: **no test environment is required.** No
cluster, no server, no account. All you need is a handful of hand-written
`todo.md` files. Tests can be written from day one.

## The two tests that matter

Everything else is ordinary unit testing. These two are the product:

```
1. Round-trip:  parse(write(parse(x))) == parse(x)
2. Fidelity:    every untouched line is byte-for-byte identical
```

Test 2 is the stronger one. Concretely: if the user marks **one** task as done,
the other 20 lines in the file must come out byte-for-byte the same — including
the indentation, the double spaces and the emoji they typed themselves.

If that property breaks, the tool has corrupted somebody's hand-written file.
See [risks.md](risks.md).

## Fixtures

`tests/fixtures/simple.md` — a normal, well-formed list. The file at
[examples/todo.md](examples/todo.md) is exactly this; it gets copied over.

`tests/fixtures/gnarly.md` — deliberately awkward. Every line is a separate trap
the parser must **not** break:

```markdown
# My list

This is a paragraph. Not a task, do not touch it.

## Work
- [ ] write the deploy plan @2026-08-12 #ops !high
- [X] close the old PRs            <- capital X
* [ ] a star-bulleted item         <- * instead of -
  - [ ] an indented subtask        <- indentation must survive
- [ ]    three-space title         <- extra whitespace must survive
- [ ] invalid date @2026-13-45     <- must not parse, line must not break
- [ ] three tags #a #b #c @2026-09-01
- [ ] junk @ and # on their own
- [ ] non-ASCII: şğüöçİI ✓ emoji 🚀

> A quoted line. Do not touch.

| a | table |
|---|-------|
| this | too |

## Personal
- [ ] undated task
- [x] finished task

---
Trailing newline present or absent — both must be preserved.
```

Expected behaviour:

- Counted as tasks: `- [ ]`, `- [x]`, `- [X]`, `* [ ]`, and the indented one
- **Unchanged:** the heading, the paragraph, the quote, the table, `---`, the
  blank lines
- `@2026-13-45` → `due = None`, but the line is written back exactly as-is
- Non-ASCII and emoji survive; column alignment in the TUI must account for
  wide characters

## Layers

| Layer | How it is tested | Terminal needed |
|---|---|---|
| `parse` | Fixtures → expected `Vec<Task>` | No |
| `write` | Round-trip + fidelity, against every fixture | No |
| `agenda` | Fixed `today` value, snapshot the groups | No |
| `ics` | Snapshot **plus** real verification: feed the output to khal | No |
| `theme` | Parse a `theme.conf` → expected `Theme`; every bad-input case falls back instead of failing | No |
| `ui` | By eye | Yes |

Note what this table says: everything except the last row is a plain unit test.
That is the whole reason for the module layout in
[architecture.md](architecture.md).

## Boundary cases worth writing down

- Exactly today at 00:00, and exactly `today + 7` days — which group?
- A date in a past year (is "2 days ago" formatting still right?)
- An invalid date (`@2026-13-45`, `@2026-02-30`)
- An empty file, a file that is only a heading, a file with no trailing newline
- A task whose title is empty (`- [ ]` and nothing else)
- The same tag written twice on one line
- A file with CRLF line endings (a dotfiles repo shared with Windows)
- `theme.conf`: an unknown key, a malformed hex (`#zzz`, `#ab`), an ANSI index
  out of range, a missing `=`, an empty file, a `#` comment line — **every one of
  them must warn and continue, never abort**

## Running

```
cargo test              # everything
cargo test parse        # one module
cargo clippy -- -D warnings
cargo fmt --check
```
