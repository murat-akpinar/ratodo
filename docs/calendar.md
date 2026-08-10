# Calendar export

## What we give, and what we don't

Open tasks **that have a date** are written to `todo.ics` as VTODO entries. One
direction only. An edit made in the calendar does not come back to `todo.md`, and
that is deliberate — coming back means conflict resolution, which is a
sub-project of its own.

When it is written: after every `todo.md` write, and manually via `ratodo sync`.

## ⚠️ There is no such thing as "the Linux calendar"

This section has to be read honestly, because it is where expectations break:

| Client | Reads a local `.ics` file? | Note |
|---|---|---|
| khal | ✅ | Already file-based; you just point it at the directory |
| Thunderbird | ✅ | "New Calendar → On My Computer / from file" |
| Evolution | ⚠️ | Varies by version; needs an "On This Computer" source |
| GNOME Calendar | ⚠️ | Mostly expects `webcal://` / HTTPS; local files are unreliable |
| Google Calendar | ❌ | Does not read local files, and **ignores VTODO entirely** |

So producing the `.ics` is easy; **getting it subscribed is the user's job.** In
v1 we generate the file and document the subscription steps for khal and
Thunderbird in the README. No automatic registration, no talking to a calendar
service over dbus.

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

Still to do — khal or Thunderbird actually **displaying** it, which is a
different question from parsing it, and the one that catches a client quietly
ignoring VTODO.
