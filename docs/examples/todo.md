# todo

This file does two jobs:

1. **A format example** — the syntax table in [../format.md](../format.md), made
   real. A user's `~/.config/ratodo/todo.md` looks exactly like this.
2. **The first test fixture** — it gets copied to `tests/fixtures/simple.md`.

This paragraph is itself a test: the tool does not touch lines it does not
recognise, and when it rewrites the file these stay byte-for-byte identical.

Reference "today" for the expectations below: **2026-08-10**

## Ops

- [ ] rotate the backup keys @2026-08-08 #ops !high
- [ ] review the deploy PR @2026-08-10 16:00 #work
- [ ] plan the server migration @2026-09-01 #ops
- [x] close the old PRs #work

## Home

- [ ] pay the invoice @2026-08-10 #home
- [ ] book a dentist appointment @2026-08-14 09:30 #health
- [ ] fatura öde @2026-08-17 #ev !med
- [ ] something with no date, whenever
- [x] migrate the server #ops

## Someday

- [ ] finish chapter 13 of the Rust book !low
- [ ] update the keyboard firmware #hobby

---

> This quote is preserved too. So is the table below.

| Expected group | Which tasks |
|---|---|
| OVERDUE | rotate the backup keys (2 days late) |
| TODAY | review the deploy PR (16:00), pay the invoice |
| THIS WEEK | book a dentist appointment (Aug 14), fatura öde (Aug 17) |
| LATER | plan the server migration (Sep 1) |
| *(undated)* | under their `##` sections, in file order |
| *(completed)* | close the old PRs, migrate the server |

One task is deliberately left in Turkish (`fatura öde`, `#ev`) — non-ASCII
characters have to survive parsing, writing and TUI column alignment.
