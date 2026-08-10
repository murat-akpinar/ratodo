# Known risks

| Risk | Impact | What we do about it |
|---|---|---|
| **Round-trip fidelity breaks** | The user's hand-written file gets corrupted → trust is gone, the tool is uninstalled. **The number one risk** | Keep the raw line, byte-for-byte tests, `.bak`, atomic writes. See [testing.md](testing.md) |
| Scope creep | **The thing that actually kills projects like this** | The "out of scope" list in [product.md](product.md#out-of-scope). Every new idea goes there first |
| A broken `theme.conf` stops the program | The tool becomes unusable over a typo in a colour value — an unacceptable trade for a cosmetic feature | Warn on stderr and ignore the bad line. A theme file can never prevent startup. See [theming.md](theming.md) |
| GNOME Calendar won't read a local `.ics` | The calendar promise feels half-delivered | Be honest in the README: khal ✅ Thunderbird ✅ GNOME ⚠️ Google ❌ |
| Google Calendar ignores VTODO | "It doesn't show up in my calendar" | An `--as-events` flag in v2 |
| The name is read as a ratatui sub-project | Not seen as an independent product; gets filed as "a ratatui plugin" | The README's first sentence: "built **with** ratatui". The logo is not a mouse, it is a **mouse holding a checklist** |
| `ratodo` is 6 characters, typed 20 times a day | Friction — this was `jot`'s whole argument | `alias r=ratodo`, given as an example in the README |
| First TUI project — the event loop stalls | Development stops before anything is usable | The build order is deliberate: after step 2 there is already a working CLI todo. If step 4 stalls, the project does not die. See [../todo.md](../todo.md) |
| A panic in raw mode leaves the terminal broken | The user's terminal is wrecked, which is the worst first impression a tool can make | `std::panic::set_hook` on day one, not later |
| Non-ASCII / emoji width in the TUI | Columns get misaligned for Turkish characters and emoji | Fixtures deliberately contain `şğüöçİI` and 🚀 |

## Retired risks

| Risk | How it resolved |
|---|---|
| `ratodo` might be taken → the name, the `~/.config/` path and the package name would all have to change | Checked 2026-08-10: free on crates.io, clear on GitHub, no PATH conflict. Closed. See [naming.md](naming.md) |
| No truecolor on a bare TTY or in old `screen` → colours look wrong | Solved as a side effect of theming: the built-in `terminal` theme uses only ANSI 0–15. `ratodo --theme terminal`. See [theming.md](theming.md) |
