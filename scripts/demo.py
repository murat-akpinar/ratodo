#!/usr/bin/env python3
"""Record assets/demo.gif — the agenda being used, for the README.

    python3 scripts/demo.py

Needs kitty, menyoki, ffmpeg and an X11 or XWayland display. It opens one
throwaway kitty window, drives a release build of ratodo inside it against a
throwaway XDG tree, and lets menyoki record that window. The real
~/.config/ratodo is never in scope.

On Hyprland the window is floated and sized before recording starts, because a
tall tile leaves a third of the GIF empty below the last group. Everywhere else
the size is the compositor's call and the pty is sized from the window rather
than the other way round.
"""

import fcntl
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
from datetime import date, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target/release/ratodo"
OUT = ROOT / "assets/demo.gif"
FPS = 12
WM_CLASS = "ratodo-demo"
# Pixels, at font_size 12 and 8 of padding: about 100 columns by 34 rows.
SIZE = (1040, 760)

# Relative to the day it is recorded, so re-recording never produces a screen
# where every group is OVERDUE. The `✓` stamps are what the band's week
# sparkline and the stats screen read, so two of them are the point.


def _day(offset):
    return (date.today() + timedelta(days=offset)).isoformat()


TODO = f"""\
# todo

## Ops

- [ ] rotate the backup keys @{_day(-2)} #ops !high
- [ ] review the deploy PR @{_day(0)} 16:00 #work
- [ ] plan the server migration @{_day(20)} #ops
- [x] close the old PRs #work ✓{_day(-1)}
- [x] tag the release #ops ✓{_day(0)}

## Home

- [ ] pay the invoice @{_day(0)} #home
- [ ] book a dentist appointment @{_day(2)} 09:30 #health
- [ ] fatura öde @{_day(5)} #ev !med
- [ ] update the keyboard firmware #hobby
"""

# (pause before, keys) — one tuple per beat. None means TYPED, a character
# at a time, so the live preview is seen resolving.
#
# What it has to show is what v0.8.0 changed: the band and the boxes as it
# opens, the form `a` now opens rather than the one-line box, and the stats
# screen. It ends back on the list, because a GIF loops and a cut from the
# stats screen to the agenda reads as a glitch.
SCRIPT = [
    (1.6, ""),  # the agenda, as it opens
    (0.6, "j"),
    (0.5, "j"),
    (0.9, " "),  # tick it: the row goes green, the band's counts move
    (1.4, ""),
    (0.8, "a"),  # the form
    (1.0, None),  # typed, so each word lands in its own box as it goes
    (0.9, "\t"),  # date — already holding the day `@fri` resolved to
    (0.4, "\t"),  # time
    (0.4, "\t"),  # priority
    (0.7, "\x1b[C"),  # → high, and the PREVIEW says what will be written
    (1.0, "\r"),
    (1.5, ""),  # written, and the list has it
    (0.8, "s"),  # the stats screen
    (2.2, "\x1b"),
    (1.2, ""),
]
TYPED = "buy coffee beans @fri #home"


def drive():
    """Run ratodo on a pty and mirror it to this terminal, typing the script."""
    import pty

    pid, fd = pty.fork()
    if pid == 0:
        os.execvp(str(BIN), [str(BIN)])
    rows, cols = struct.unpack(
        "HHHH", fcntl.ioctl(1, termios.TIOCGWINSZ, b"\0" * 8)
    )[:2]
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

    done = threading.Event()

    def pump():
        while not done.is_set():
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            if not data:
                break
            os.write(1, data)

    threading.Thread(target=pump, daemon=True).start()

    for delay, keys in SCRIPT:
        time.sleep(delay)
        if keys is None:
            for ch in TYPED:
                os.write(fd, ch.encode())
                time.sleep(0.07)
        elif keys:
            os.write(fd, keys.encode())

    # No `q`: quitting tears the alternate screen down, and a bare prompt is
    # the frame a looping GIF would rest on. Closing the pty ends the session
    # with the agenda still up.
    done.set()
    os.close(fd)
    os.kill(pid, 9)
    os.waitpid(pid, 0)


def place_the_window():
    """Float the demo window at a fixed size, on the one WM that can be asked.

    Left to a tiling WM the window is whatever the layout gives it, and a tall
    tile puts a third of the GIF below the last group: the footer is pinned to
    the bottom of the pane, so the dead band is in the *middle* and no crop
    reaches it. The size is a terminal somebody would actually run — wide enough
    for the columns and the form, short enough that the list fills it.

    Dispatched at the window rather than declared as a rule, because a rule is
    config the script would then owe the user a way back out of. Anywhere but
    Hyprland this does nothing and the window is the compositor's call, which is
    what the rest of this script has always assumed.
    """
    if not shutil.which("hyprctl") or not os.environ.get("HYPRLAND_INSTANCE_SIGNATURE"):
        return
    where = f"class:^({WM_CLASS})$"
    for _ in range(40):
        time.sleep(0.1)
        clients = subprocess.run(
            ["hyprctl", "clients"], capture_output=True, text=True, check=False
        )
        if WM_CLASS in clients.stdout:
            break
    else:
        return
    subprocess.run(["hyprctl", "dispatch", "setfloating", where], check=False)
    subprocess.run(
        ["hyprctl", "dispatch", "resizewindowpixel", f"exact {SIZE[0]} {SIZE[1]},{where}"],
        check=False,
    )


def record():
    if not BIN.exists():
        sys.exit(f"no release build at {BIN} — cargo build --release")
    tmp = Path(tempfile.mkdtemp(prefix="ratodo-demo-"))
    env = dict(os.environ, HOME=str(tmp))
    for var in ("CONFIG", "DATA", "STATE", "CACHE"):
        d = tmp / var.lower()
        d.mkdir()
        env[f"XDG_{var}_HOME"] = str(d)
    (tmp / "config/ratodo").mkdir(parents=True)
    (tmp / "config/ratodo/todo.md").write_text(TODO)

    raw = tmp / "raw.gif"
    # Long enough that the window has been floated and resized before menyoki
    # reads its geometry: it records a rectangle, not a window that moves.
    inner = (
        f"sleep 3; menyoki -q record --focus --border 0 --countdown 0 "
        f"'python3 {__file__} --drive' gif --fps {FPS} --quality 80 "
        f"save '{raw}'"
    )
    kitty = subprocess.Popen(
        [
            "kitty",
            "--class", WM_CLASS,
            "-o", "linux_display_server=x11",
            "-o", "remember_window_size=no",
            "-o", "font_size=12",
            "-o", "window_padding_width=8",
            "-o", "hide_window_decorations=yes",
            "-o", "confirm_os_window_close=0",
            "-o", "cursor_blink_interval=0",
            "-e", "bash", "-c", inner,
        ],
        env=env,
    )
    place_the_window()
    if kitty.wait() != 0:
        sys.exit(f"kitty exited {kitty.returncode}")

    # menyoki writes a palette per frame; one shared palette plus rectangle
    # diffing takes a terminal recording down by an order of magnitude
    palette = tmp / "palette.png"
    ff = ["ffmpeg", "-loglevel", "error", "-y"]
    subprocess.run(
        ff + ["-i", str(raw), "-vf", "palettegen=stats_mode=diff", str(palette)],
        check=True,
    )
    OUT.parent.mkdir(exist_ok=True)
    subprocess.run(
        ff
        + [
            "-i", str(raw),
            "-i", str(palette),
            "-lavfi", "paletteuse=dither=none:diff_mode=rectangle",
            str(OUT),
        ],
        check=True,
    )
    print(f"{OUT} — {OUT.stat().st_size // 1024} KiB")


if __name__ == "__main__":
    drive() if "--drive" in sys.argv else record()
