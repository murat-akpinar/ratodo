#!/usr/bin/env python3
"""Record assets/demo.gif — the agenda being used, for the README.

    python3 scripts/demo.py

Needs kitty, menyoki, ffmpeg and an X11 or XWayland display. It opens one
throwaway kitty window, drives a release build of ratodo inside it against a
throwaway XDG tree, and lets menyoki record that window. The real
~/.config/ratodo is never in scope.

The window size is the compositor's call under a tiling WM, so the pty is
sized from the window rather than the other way round.
"""

import fcntl
import os
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target/release/ratodo"
OUT = ROOT / "assets/demo.gif"
FPS = 12

TODO = """\
# todo

## Ops

- [ ] rotate the backup keys @2026-08-09 #ops !high
- [ ] review the deploy PR @2026-08-11 16:00 #work
- [ ] plan the server migration @2026-09-01 #ops
- [x] close the old PRs #work

## Home

- [ ] pay the invoice @2026-08-11 #home
- [ ] book a dentist appointment @2026-08-14 09:30 #health
- [ ] fatura öde @2026-08-17 #ev !med
- [ ] update the keyboard firmware #hobby
"""

# (pause before, keys) — one tuple per beat. None means TYPED, a character
# at a time, so the live preview is seen resolving.
SCRIPT = [
    (1.6, ""),  # the agenda, as it opens
    (0.6, "j"),
    (0.5, "j"),
    (0.9, " "),  # tick it: the row goes green, the progress bar moves
    (1.6, ""),
    (0.6, "a"),  # the capture box, with its dim example
    (1.0, None),
    (1.2, "\r"),
    (1.8, ""),
]
TYPED = "buy coffee beans @fri #home !high"


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
    inner = (
        f"sleep 1.2; menyoki -q record --focus --border 0 --countdown 0 "
        f"'python3 {__file__} --drive' gif --fps {FPS} --quality 80 "
        f"save '{raw}'"
    )
    subprocess.run(
        [
            "kitty",
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
        check=True,
    )

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
