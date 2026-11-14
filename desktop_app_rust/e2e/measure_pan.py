#!/usr/bin/env python3
"""How smooth panning is: frame times while the middle button drags the railway diagram (and
while the wheel zooms), in the real window.  Prints the median and 95th percentile of the frame
intervals and how many frames took longer than 25 ms.

    python3 e2e/measure_pan.py
"""
import shutil
import statistics
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP  # noqa: E402

IDLE = 1
RECORD = """
window.__frames = []; let last = performance.now();
const tick = (t) => { window.__frames.push(t - last); last = t; if (window.__rec) requestAnimationFrame(tick); };
window.__rec = true; requestAnimationFrame(tick);
"""


def stats(d, what):
    frames = d.js("window.__rec = false; return window.__frames.slice(2)")
    print("  as they came:", " ".join(f"{f:.0f}" for f in frames[:60]))
    frames.sort()
    p95 = frames[int(len(frames) * 0.95)] if frames else 0
    long = sum(1 for f in frames if f > 25)
    print(f"{what}: {len(frames)} frames, median {statistics.median(frames):.1f} ms, "
          f"95% {p95:.1f} ms, longer than 25 ms: {long}")


def main():
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-pan-"))
    shutil.copy(HERE.parent.parent / "desktop_app_rust/test_diagrams/railway.erd", tmp / "railway.erd")
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp / "cfg"), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    try:
        d.call("window.__erdtex.setMode('light')")
        d.call("window.__erdtex.open(args[0])", str(tmp / "railway.erd"))
        d.session("POST", "/window/maximize", {})
        d.frames(10)
        d.js("window.__erdtex.run('a_zoom_in'); window.__erdtex.run('a_zoom_in')")
        d.frames(10)
        r = d.rect("[data-testid=canvas]")
        cx, cy = r["x"] + r["w"] / 2, r["y"] + r["h"] / 2
        d.js(RECORD)
        import time as _t
        _t.sleep(1.0)
        stats(d, "idle (requestAnimationFrame alone)")
        d.js(RECORD)
        steps = [("move", cx, cy), ("down", 1)]
        for k in range(500):           # two seconds of dragging, an event every 4 ms (a real mouse)
            dx = 300 * ((k % 125) / 125 if (k // 125) % 2 == 0 else 1 - (k % 125) / 125)
            steps += [("pause", 4), ("move", cx + dx - 150, cy + dx / 2 - 75)]
        steps += [("up", 1)]
        d.pointer(steps)
        stats(d, "panning")
    finally:
        d.quit()


if __name__ == "__main__":
    main()
