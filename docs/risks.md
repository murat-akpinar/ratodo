# Known risks

| Risk | Impact | What we do about it |
|---|---|---|
| **Round-trip fidelity breaks** | The user's hand-written file gets corrupted → trust is gone, the tool is uninstalled. **The number one risk** | Keep the raw line, byte-for-byte tests, `.bak`, atomic writes. See [testing.md](testing.md) |
| Scope creep | **The thing that actually kills projects like this** | The "out of scope" list in [product.md](product.md#out-of-scope). Every new idea goes there first |
| **Quiet abandonment in week three** | Not a bug, not a crash — the tool simply stops being opened, and nobody files an issue about it. The most likely way this project fails after shipping | See below |
| The tool never gets into the setup | It stays a program you *run* rather than part of the desktop. A todo you have to remember to open is a todo you forget | `ratodo status` for the bar and `list --porcelain` for `fzf` moved into v1 for exactly this. [decisions.md](decisions.md#reversed) |
| A broken `theme.conf` stops the program | The tool becomes unusable over a typo in a colour value — an unacceptable trade for a cosmetic feature | Warn on stderr and ignore the bad line. A theme file can never prevent startup. See [theming.md](theming.md) |
| GNOME Calendar won't read a local `.ics` | The calendar promise feels half-delivered | Be honest in the README: khal ✅ Thunderbird ✅ GNOME ⚠️ Google ❌ |
| Google Calendar ignores VTODO | "It doesn't show up in my calendar" | An `--as-events` flag in v2 |
| The name is read as a ratatui sub-project | Not seen as an independent product; gets filed as "a ratatui plugin" | The README's first sentence: "built **with** ratatui". The logo is not a mouse, it is a **mouse holding a checklist** |
| `ratodo` is 6 characters, typed 20 times a day | Friction — this was `jot`'s whole argument | `alias r=ratodo`, given as an example in the README |
| First TUI project — the event loop stalls | Development stops before anything is usable | The build order is deliberate: after step 2 there is already a working CLI todo. If step 4 stalls, the project does not die. See [../todo.md](../todo.md) |
| A panic in raw mode leaves the terminal broken | The user's terminal is wrecked, which is the worst first impression a tool can make | `std::panic::set_hook` on day one, not later |
| Non-ASCII / emoji width in the TUI | Columns get misaligned for Turkish characters and emoji | Fixtures deliberately contain `şğüöçİI` and 🚀 |

## Week three

Written down at length because it is the one risk with no test to catch it. Two
design reviews from the two target profiles, run separately on 2026-08-10,
produced the same story:

> Week 1 they alias `r=ratodo` and it feels great. Week 2 they have captured
> forty things, six of which have dates. Week 3 they open it and see six dated
> rows above an undifferentiated dump of thirty-four undated ones — no filter, no
> search, no idea which repo any of them belonged to — while `rg TODO` and
> `gh issue list` still answer both questions.

The mechanism: **the agenda organises by date, and the list is mostly undated.**
Capture is fast, so the file grows; the one screen that was supposed to give the
file structure has nothing to say about most of it. The tool solves calendar
pressure, which this audience does not feel, and not context switching, which
they do.

What is in place against it:

- `list --tag` / `--prio` pulled into v1 — the smallest thing that lets someone
  say "just the work ones". [decisions.md](decisions.md#reversed)
- `$RATODO_FILE`, so a repository can have its own list without an alias per
  checkout, which is most of the "which repo was this" problem
- `ratodo status` in the bar: the list stays visible without being opened

What is deliberately *not* in place: interactive filters, saved views, contexts,
projects. Those are v2 at the earliest, and several are on the out-of-scope list.
The check is behavioural, not technical — **use it for three weeks and see
whether it is still open in week four.** If it is not, the fix is not another
feature; re-read this row first.

## Retired risks

| Risk | How it resolved |
|---|---|
| `ratodo` might be taken → the name, the `~/.config/` path and the package name would all have to change | Checked 2026-08-10: free on crates.io, clear on GitHub, no PATH conflict. Closed. See [naming.md](naming.md) |
| No truecolor on a bare TTY or in old `screen` → colours look wrong | Solved as a side effect of theming: the built-in `terminal` theme uses only ANSI 0–15. `ratodo --theme terminal`. See [theming.md](theming.md) |
