# ratodo — documentation

> Status: **design phase, no code yet** · Last updated: 2026-08-10

This directory is the project's decision record. Every document here answers a
different question, and each one states not only *what* was decided but *why*
and *what was given up*.

## Map

| Document | Answers |
|---|---|
| [product.md](product.md) | What is this, who is it for, why does it need to exist, what is deliberately out of scope |
| [format.md](format.md) | What does the file look like, where does it live, what does the tool touch |
| [architecture.md](architecture.md) | How does data flow, which modules exist, how are concurrent edits handled, which crates and why |
| [design.md](design.md) | Palette, layout, agenda grouping rules, symbols |
| [tui.md](tui.md) | Every screen and interaction state, the keymap, narrow-width behaviour |
| [theming.md](theming.md) | `theme.conf`, the 11 colour keys, built-in themes |
| [cli.md](cli.md) | Commands, flags, keybindings, output shapes |
| [calendar.md](calendar.md) | `.ics` export, VTODO, which calendar clients actually work |
| [testing.md](testing.md) | Test strategy, fixtures, the two tests that matter most |
| [roadmap.md](roadmap.md) | v1 through v5, and what belongs to which |
| [decisions.md](decisions.md) | Settled decisions, rejected ideas, still-open questions |
| [naming.md](naming.md) | Why "ratodo", which names lost, availability checks |
| [risks.md](risks.md) | What could kill this project and what we do about it |
| [examples/todo.md](examples/todo.md) | A real `todo.md`, exactly as a user's file would look |

## Reading order

New to the project? `product.md` → `format.md` → `architecture.md`. Those three
carry the whole idea. Everything else is detail hanging off them.

## Where things live

| File | Role |
|---|---|
| `docs/` | **Decisions.** Settled things. Changing a document here means changing a decision |
| `../notes.md` | **Raw thinking.** Loose ends, hunches, the idea graveyard. Anything that settles here moves into `docs/` |
| `../todo.md` | **Task list.** What gets built next, in order |
| `../CLAUDE.md` | Working rules for AI agents on this repo |

## The one-line rule

> The tool is the file's **guest**, not its owner.

Every architectural decision in this directory is derived from that sentence.
When a new idea comes up, the first question is whether it keeps that promise.
