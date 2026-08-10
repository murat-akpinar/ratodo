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
- Untimed tasks use a `VALUE=DATE` due; timed ones use a UTC date-time.
- Completed (`[x]`) tasks are not exported at all in v1.

## Verification

Snapshot-testing the output is not enough. The real test is feeding the file to
khal and confirming it is actually displayed — that is step 3 in
[../todo.md](../todo.md).
