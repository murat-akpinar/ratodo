# Calendar export

## What we give, and what we don't

Open tasks **that have a date** are written to `todo.ics` as VTODO entries. One
direction only. An edit made in the calendar does not come back to `todo.md`, and
that is deliberate — coming back means conflict resolution, which is a
sub-project of its own.

When it is written: after every `todo.md` write, and manually via `ratodo sync`.

## ⚠️ There is no such thing as "the Linux calendar"

This section has to be read honestly, because it is where expectations break:

**A calendar app and a todo app are not the same program**, and that is the
whole of this section. We write VTODO; most of what people call "the calendar"
draws VEVENT and nothing else.

| Client | Shows ratodo's entries? | Note |
|---|---|---|
| todoman | ✅ | **Verified 2026-08-11.** Reads VTODO, which is what it is for. Point it straight at `~/.local/share/ratodo`; it goes read-only on its own because there is more than one todo in the file |
| khal | ❌ | **Verified 2026-08-11 — this used to say ✅ and was wrong.** khal wants a *vdir* (a directory), not a file, and even given one it draws VEVENT only: a control VEVENT dropped into the same directory appeared, our five VTODOs did not |
| Thunderbird | ⚠️ | "New Calendar → On My Computer / from file". It has a Tasks view, which is where these land — not the month grid. Not verified by us |
| Evolution | ⚠️ | Varies by version; needs an "On This Computer" source |
| GNOME Calendar | ❌ | Expects `webcal://` / HTTPS, and draws events |
| Google Calendar | ❌ | Does not read local files, and **ignores VTODO entirely** |

So producing the `.ics` is easy; **getting it subscribed is the user's job**, and
picking a program that reads todos at all comes before that. In v1 we generate
the file and document the steps for todoman in the README. No automatic
registration, no talking to a calendar service over dbus.

The `--as-events` flag on the [roadmap](roadmap.md) for v2 is the answer for the
other half of that table, and this is the evidence that it is worth building:
every ❌ above is a client that would have shown the same tasks as events.

## VTODO or VEVENT

A task is semantically a VTODO, and that is the correct choice. But a lot of
clients do not display VTODO. Writing a hybrid (timed ones as VEVENT, untimed as
VTODO) is confusing — so: **VTODO in v1**, and an `--as-events` flag in v2 for
people whose client only shows events.

## Implementation

No `icalendar` crate. The output is ~30 lines of string formatting; it is not
worth a dependency. Shape of what gets emitted:

```
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//ratodo//EN
BEGIN:VTODO
UID:<stable hash of the task>
DTSTAMP:<now, UTC>
DUE:<20260812 or 20260812T160000>
SUMMARY:<title>
CATEGORIES:<tags>
PRIORITY:<1 / 5 / 9>
STATUS:NEEDS-ACTION
END:VTODO
END:VCALENDAR
```

Points to get right:

- **UID must be stable.** If it is regenerated on every write, calendar clients
  see delete-and-recreate every time. Derive it from the task's content.
- Lines must be CRLF-terminated and folded at 75 octets — RFC 5545 is picky, and
  khal will tell you so.
- Untimed tasks use a `VALUE=DATE` due; timed ones use a **floating** local
  date-time — `DUE:20260813T093000`, with no trailing `Z`.

  *(This corrects an earlier line here that said UTC.)* `@2026-08-13 09:30` in
  the file carries no timezone, because the person who typed it meant half past
  nine where they are. Converting that with the machine's offset at the moment of
  writing bakes today's timezone into the file, and the task moves on screen the
  first time its owner travels or the clocks change. RFC 5545 has floating times
  for exactly this, and it is what the data actually is.
- Completed (`[x]`) tasks are not exported at all in v1.
- **The UID comes from the title and the section**, not the whole line. Changing
  a date or adding a tag then *updates* the entry the client is already showing,
  where deriving it from the raw line would delete and recreate it. Two genuinely
  identical tasks get an occurrence number mixed in, because two VTODOs sharing
  a UID is one entry as far as any client is concerned — the second task would
  simply not appear.
- **The hash is written out rather than taken from `DefaultHasher`**, whose
  output the standard library is explicitly allowed to change between releases.
  A UID that moves with the toolchain is the bug this whole section is about.

## Verification

Snapshot-testing the output is not enough: a snapshot only says the output has
not changed, never that it was right to begin with.

Done so far — a generated `todo.ics` parsed by Python's `icalendar`, which is a
different implementation by different people reading the same RFC. It confirmed
the comma escape survives a round trip rather than splitting a summary in two,
that a 130-character Turkish and emoji title unfolds back to itself, and that a
timed task arrives as a floating datetime rather than one pinned to an offset.

Done 2026-08-11 — a real client **displaying** it, which is a different
question from parsing it, and the one that catches a client quietly ignoring
VTODO. It caught one immediately: `todoman` lists all five tasks with their
dates, times, categories and priorities, and a change made in ratodo shows up on
the next `todo list` with no sync step in between; `khal`, which this document
had listed as ✅ on nothing more than "it is file-based", shows none of them. The
control that makes that a finding rather than a guess is a hand-written VEVENT
dropped into the same directory with the same config — khal drew that one.

Still to do — Thunderbird, whose Tasks view is a different code path from the
month grid and is where these would land.
