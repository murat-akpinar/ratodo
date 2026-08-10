# Testing strategy

The big advantage of this project: **no test environment is required.** No
cluster, no server, no account. All you need is a handful of hand-written
`todo.md` files. Tests can be written from day one.

## A green suite is not the same as a working one

The failure mode that matters here is not a test that fails — it is a test that
passes and proves nothing. This project is one where that would be expensive:
the promise is "we cannot corrupt your file", and a suite that quietly stopped
checking that would leave the promise standing with nothing behind it.

Three specific ways a suite lies, and what is in place against each:

| The lie | What it looks like | The guard |
|---|---|---|
| **The test cannot fail** | An assertion that holds no matter what the code does | `cargo mutants` — deliberately break the code and confirm something goes red |
| **The generator never generates the interesting case** | 4000 random documents that all happen to be empty | `the_generator_produces_what_we_think_it_does` asserts the corpus really contains CRLF, tabs, `[X]`, emoji, invalid dates, near-miss task lines |
| **The comparison is too weak to notice** | Comparing parsed models when the bytes are what matter | `the_checker_would_notice_damage` feeds the comparison known-damaged input and requires it to reject each one |

One real example, from writing these: a test asserted that `@9999999d` was
rejected as an overflow. It was not — 27,000 years from now is a perfectly valid
`NaiveDate`, so the assertion was wrong and the code was right. The test failed,
which is the only reason anyone found out. A weaker assertion would have passed
and quietly documented a behaviour that does not exist.

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

## Three layers of input

| Layer | File | What it is for |
|---|---|---|
| Unit | `src/*.rs` `#[cfg(test)]` | One function, with today's date injected rather than read from the clock |
| Fixtures | `tests/fixtures/*.md` | The cases we thought of, readable and hand-checked |
| Generated | `tests/property.rs` | The cases we did not think of — 4000 documents from a fixed seed |
| CLI | `tests/cli.rs` | The real binary, driven the way a user drives it |
| Mutants | `cargo mutants` | Not input at all: it breaks the *code* and checks that a test notices |

`tests/cli.rs` is deliberately free of anything depending on today's date. A test
that passes this week and fails next week is another way for a suite to mislead
you, so date phrasing is asserted in `text.rs` where `today` is a parameter, and
the CLI tests only use absolute dates and shapes.

The generated documents are built from a seeded xorshift, so a failure is
reproducible from the seed printed in the message. No `proptest` or `quickcheck`
dependency: the generator is 40 lines and the shrinking those crates provide is
not worth a dev-dependency here, since a failing document is already small.

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
cargo test                          # everything
cargo test --test property          # the generated corpus
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python3 scripts/check-docs.py       # every Markdown link and anchor
```

And, before trusting the suite rather than the code:

```
cargo mutants --timeout 60
```

Anything reported as **MISSED** is a change to the source that no test objected
to. Some of those are fine — a mutation inside a `Display` impl usually is — but
a missed mutant in `parse`, `write` or `model` is a hole in the fidelity
guarantee and gets a test.

**TIMEOUT is a catch, not a miss.** Every one of them so far is the same shape: a
mutation to a loop counter (`+=` becoming `*=`, `-=` becoming `/=`) leaves the
loop unable to advance, so the mutant hangs instead of returning a wrong answer.
Hanging is detection.

Two kinds of MISSED are worth naming, because the honest response to each is
different:

- **An equivalent mutant** cannot be killed, because it does not change the
  program. `n > 0` and `n >= 0` are the same answer where the loop around them
  never runs at `n == 0`. Writing a test that appears to cover it is worse than
  leaving it; the fix is to restructure so the ambiguous branch is not there —
  which is how `Screen::move_by` came to read its sign once, outside the loop.
- **A weak assertion** is the common case and the useful one. A mutant surviving
  usually means a test looked at half of what it claimed to. Every one found so
  far was real: a backup asserted absent but never asserted present, a keymap
  with no coverage at all, a `sync` count checked against a list where three
  different wrong filters all give the same number.
