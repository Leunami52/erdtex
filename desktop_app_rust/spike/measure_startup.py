#!/usr/bin/env python3
"""Start-up time of a program until its window has settled, measured the same way for any app.

Runs the command on an invisible X display (Xvfb), takes a screenshot every few milliseconds
for RECORD seconds, and reports the first moment the screen is within NEAR of its final state.
("The screen stops changing" doesn't work: scrollbars fade and status texts change later.)
Each run gets a fresh, empty settings folder (XDG_CONFIG_HOME), so nothing real is read or
written.

    measure_startup.py [--runs N] -- COMMAND [ARGS...]
"""
import argparse
import os
import statistics
import subprocess
import sys
import tempfile
import time
import zlib

from PIL import Image, ImageChops
from Xlib import X, display as xdisplay

RECORD = 6.0         # seconds of screenshots per run
NEAR = 0.005         # share of pixels that may still differ from the final screen
W, H = 1280, 900
PAUSE = 8.0          # between runs: a WebView started again at once waits for the last one


def differing(a: bytes, b: bytes) -> float:
    """Share of pixels that differ visibly between two screenshots."""
    ia = Image.frombytes("RGB", (W, H), a, "raw", "BGRX").resize((W // 2, H // 2))
    ib = Image.frombytes("RGB", (W, H), b, "raw", "BGRX").resize((W // 2, H // 2))
    hist = ImageChops.difference(ia, ib).convert("L").point(lambda v: 255 if v > 24 else 0).histogram()
    return hist[255] / (W * H / 4)


def run_once(cmd: list[str], disp: str, record: float = RECORD) -> float | None:
    with tempfile.TemporaryDirectory() as cfg:
        env = dict(os.environ, DISPLAY=disp, XDG_CONFIG_HOME=cfg, GDK_BACKEND="x11",
                   QT_QPA_PLATFORM="xcb", NO_AT_BRIDGE="1")
        env.pop("WAYLAND_DISPLAY", None)
        d = xdisplay.Display(disp)
        root = d.screen().root
        frames: list[tuple[float, bytes]] = []
        start = time.perf_counter()
        proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                start_new_session=True)
        try:
            last = None
            while time.perf_counter() - start < record:
                data = root.get_image(0, 0, W, H, X.ZPixmap, 0xFFFFFFFF).data
                h = zlib.crc32(data)
                if h != last:
                    last = h
                    frames.append((time.perf_counter() - start, data))
                if proc.poll() is not None:
                    return None
                time.sleep(0.005)
        finally:
            try:
                os.killpg(proc.pid, 9)
            except ProcessLookupError:
                pass
            proc.wait()
            time.sleep(PAUSE)
            d.close()
        final = frames[-1][1]
        return next(t for t, data in frames if differing(data, final) <= NEAR)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--record", type=float, default=RECORD,
                    help="seconds of screenshots per run (less than a status message stays)")
    ap.add_argument("cmd", nargs=argparse.REMAINDER)
    a = ap.parse_args()
    cmd = a.cmd[1:] if a.cmd and a.cmd[0] == "--" else a.cmd
    disp = ":97"
    xvfb = subprocess.Popen(["Xvfb", disp, "-screen", "0", f"{W}x{H}x24", "-nolisten", "tcp"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1.0)
    try:
        first = run_once(cmd, disp, a.record)    # also warms the disk cache
        times = [run_once(cmd, disp, a.record) for _ in range(a.runs)]
    finally:
        xvfb.terminate()
        xvfb.wait()
    ok = [t for t in times if t is not None]
    print(f"first run: {first:.2f} s" if first is not None else "first run: failed")
    if ok:
        print(f"warm runs: median {statistics.median(ok):.2f} s, "
              f"min {min(ok):.2f} s, max {max(ok):.2f} s ({len(ok)} of {len(times)} runs)")
    else:
        print("warm runs: all failed")
        sys.exit(1)


if __name__ == "__main__":
    main()
