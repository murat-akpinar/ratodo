# The name — ratodo

**ratatui + todo.** Decided, and availability verified.

## Criteria

1. The todo/capture idea should be visible in the name
2. It gets typed **20 times a day** — it has to be short
3. The logo has to be drawable as one shape — single colour, readable as a 16px favicon
4. It should read the same in Turkish and English, without needing an explanation
5. It should not drown in noise on crates.io and in search results

## Why ratodo

The idea arrived as `ratado`, to echo ratatui. The direction was right, but
`ratado` fails criterion 1: **it describes the framework, not the product.** It is
like calling a text editor "Qtext" — users search for what a thing does, not what
it was written in. On top of that its logo collides directly with ratatui's
mouse, and it carries the `rata` (rat) association in Spanish and Portuguese.

One letter fixes it:

```
ratado  →  rat + ado      says nothing
ratodo  →  rat + todo     "todo" is literally inside it
```

`ratodo` keeps the kinship intact and closes the one flaw. The logo improves too:
not a plain mouse, but **a mouse holding a checklist** — related to ratatui's
logo without being confused for it.

⚠️ **The trap in the kinship:** the project may be taken for a ratatui plugin or
sub-project. The README's first sentence has to close that off:
*"A todo TUI, built with ratatui"* — **with**, not *for*.

## Rejected candidates

| Name | The idea | Why it lost |
|---|---|---|
| **ratado** | ratatui + (nothing) | Describes the framework, not the product. Logo collides with ratatui's mouse. `rata` = rat |
| **jot** | "jot down" — the product's verb | The cheapest to type at 3 letters, and the only real rival. But nothing about it says *todo*, and a command of that name already exists on BSD/macOS (a number generator) |
| **tuido** | TUI + do | Was the solid backup. Means nothing to someone who doesn't know "TUI", no ratatui kinship — **and it turns out to be taken on crates.io** |
| **tik** | Turkish "tik atmak" = English "tick" — same meaning, same sound in both languages | Too generic, high search noise, and a `tick` crate exists |
| **tock** | tick-tock — both a checkmark and time | Gets confused with time-tracking tools; we don't do time tracking |
| **rusto** | Rust + to-do | Forced, and it reads as "rustic" |
| **kap** | Turkish "kapmak" = to grab | Turkish-only, doesn't travel |
| **nudge** | a nudge | Reads as a notification tool, not a todo |

## The price we accepted

We gave up `jot`'s 3 characters. On a command typed 20 times a day that is not
nothing — but `alias r=ratodo` brings it down to one character and makes the
decision **reversible.** A name cannot be taken back; an alias can. The README
shows it as an example.

## Availability checks — done 2026-08-10

| Check | Result |
|---|---|
| crates.io | ✅ Free — `crate 'ratodo' does not exist` |
| GitHub | ✅ Clear — 8 substring hits, all Portuguese ("rato do…") with 0 stars, nothing named `ratodo` |
| `command -v ratodo` | ✅ Not on PATH |
| Backup name `tuido` | ❌ **Taken on crates.io** (published 2023, ~11k downloads) |

The name is clear. The backup is gone, which no longer matters — but it does mean
that if `ratodo` ever has to be abandoned, a new candidate has to be found from
scratch.

## Tagline

> A todo TUI, built with ratatui — one Markdown file, no cloud, no account.
