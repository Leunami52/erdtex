"""Arranging a diagram in the real window: Edit → Arrange Diagram…,
tidy and anew, the result bar with its figures, Keep and Undo, an ER and a GSM diagram; then every
diagram of the repository tidied: no overlaps left, never worse than before, one Ctrl+Z takes it back;
Arrange Selection… (nothing else changes) and keeping the selection in place.

    npx tauri build --no-bundle
    python3 e2e/test_arrange.py
"""
import shutil
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP, check, failures  # noqa: E402
from test_docks import Docks  # noqa: E402


def wait(cond, timeout=30.0):
    t = time.time()
    while time.time() - t < timeout:
        v = cond()
        if v:
            return v
        time.sleep(0.2)
    return None


def arrange(u: Docks, mode: str):
    u.menu("Edit", "Arrange Diagram")
    check(u.shown("[data-testid=arrange]"), "Edit → Arrange Diagram… opens the dialog")
    u.press_css(f"[data-testid=arrange-{mode}]")
    t0 = time.time()
    u.press_css("[data-testid=arrange-ok]")
    r = wait(lambda: u.d.call("window.__erdtex.arrangeResult()"), 60)
    return r, time.time() - t0


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-arrange-"))
    for name in ("railway.erd", "imkerei.erd", "ensemble.gsmd"):
        shutil.copy(REPO / "app" / "test_diagrams" / name, tmp / name)
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp / "cfg"), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = Docks(d)
    t0 = time.time()
    try:
        d.frames(4)
        d.call("window.__erdtex.open(args[0])", str(tmp / "railway.erd"))
        d.frames(3)
        original = d.call("window.__erdtex.text()")
        before = d.call("window.__erdtex.figures()")
        r, secs = arrange(u, "tidy")
        check(r is not None, f"tidying gives a result ({secs:.1f} s)")
        if r:
            check(r["after"]["overlaps"] == 0 and before["overlaps"] > 0, f"no overlaps left ({before['overlaps']} -> {r['after']['overlaps']})")
            check(r["after"]["through"] <= before["through"] and r["after"]["crossings"] <= before["crossings"],
                  f"lines through elements and crossings no more than before ({r['before']} -> {r['after']})")
            check(u.shown("[data-testid=arrange-result]"), "the result bar says what changed")
            check(d.call("window.__erdtex.text()") != original, "the diagram changed")
            u.press_css("[data-testid=arrange-keep]")
            d.frames(2)
            check(not u.shown("[data-testid=arrange-result]"), "Keep closes the bar")
            d.call("window.__erdtex.focusCanvas()")
            u.keys("Control", "z")
            d.frames(3)
            check(d.call("window.__erdtex.text()") == original, "one Ctrl+Z takes the whole arrangement back")
        # anew, then Undo in the bar
        d.call("window.__erdtex.open(args[0])", str(tmp / "imkerei.erd"))
        d.frames(3)
        original = d.call("window.__erdtex.text()")
        r, secs = arrange(u, "anew")
        check(r is not None and r["after"]["overlaps"] == 0, f"laying out anew gives a result without overlaps ({secs:.1f} s, {r and r['name']})")
        u.press_css("[data-testid=arrange-undo]")
        d.frames(3)
        check(d.call("window.__erdtex.text()") == original and not u.shown("[data-testid=arrange-result]"), "Undo in the bar takes it back")
        # a GSM diagram
        d.call("window.__erdtex.open(args[0])", str(tmp / "ensemble.gsmd"))
        d.frames(3)
        r, secs = arrange(u, "tidy")
        check(r is not None or "as tidy as" in u.state()["status"], f"a GSM diagram ({secs:.1f} s, {r and r['after']})")
        s = d.call("window.__erdtex.settings()")
        check(s.get("arrange/mode") == "tidy" and s.get("arrange/spacing") == "0.5", f"the choices are kept ({s.get('arrange/mode')}, {s.get('arrange/spacing')})")
        # the selection only: nothing outside it changes in the file
        copy = tmp / "selection.erd"
        shutil.copy(REPO / "app" / "test_diagrams" / "railway.erd", copy)
        d.call("window.__erdtex.open(args[0])", str(copy))
        d.frames(3)
        original = d.call("window.__erdtex.text()")
        d.call("window.__erdtex.select(args[0])", ["dep", "arr"])
        d.frames(2)
        u.menu("Edit", "Arrange Selection")
        check(u.shown("[data-testid=arrange]") and d.js("return document.querySelector('[data-testid=arrange-selection]').checked"),
              "Edit → Arrange Selection… opens the dialog with the selection chosen")
        d.call("window.__erdtex.clearStatus()")
        u.press_css("[data-testid=arrange-ok]")
        r = wait(lambda: d.call("window.__erdtex.arrangeResult()"), 60)
        text = d.call("window.__erdtex.text()")
        outside = [ln for ln in original.splitlines() if not any(k in ln for k in ("{dep}", "{arr}", "\\grid"))]
        changed = [ln for ln in outside if ln not in text.splitlines()]
        check(r is not None and r["after"]["overlaps"] < r["before"]["overlaps"] and not changed,
              f"arranging the selection separates it and changes nothing else ({r and r['before']['overlaps']} -> {r and r['after']['overlaps']} overlaps, changed: {changed[:3]})")
        u.press_css("[data-testid=arrange-undo]")
        d.frames(3)
        # the selection kept in place: its elements' lines stay as they are
        d.call("window.__erdtex.select(args[0])", ["person", "train"])
        d.frames(2)
        u.menu("Edit", "Arrange Diagram")
        u.press_css("[data-testid=arrange-pin]")
        d.call("window.__erdtex.clearStatus()")
        u.press_css("[data-testid=arrange-ok]")
        r = wait(lambda: d.call("window.__erdtex.arrangeResult()"), 60)
        text = d.call("window.__erdtex.text()")
        kept = [ln for ln in original.splitlines() if ln.startswith(("\\entity{person}", "\\entity[minimum width=2cm]{train}"))]
        check(r is not None and len(kept) == 2 and all(ln in text.splitlines() for ln in kept) and text != original,
              f"keeping the selection in place: it stays, the rest is arranged ({r and r['after']['overlaps']} overlaps)")
        u.press_css("[data-testid=arrange-undo]")
        d.frames(3)
        check(d.call("window.__erdtex.text()") == original, "Undo gives back the file")
        # every test diagram (the dialog's choices are kept: tidy, Enter arranges)
        files = sorted((REPO / "app" / "test_diagrams").glob("*.[eg]*d"))
        files = [f for f in files if f.suffix in (".erd", ".gsmd")]
        bad, arranged = [], 0
        for i, f in enumerate(files):
            copy = tmp / f"all{i}{f.suffix}"
            shutil.copy(f, copy)
            d.call("window.__erdtex.open(args[0])", str(copy))
            d.frames(3)
            original = d.call("window.__erdtex.text()")
            before = d.call("window.__erdtex.figures()")
            d.call("window.__erdtex.clearStatus()")
            d.call("window.__erdtex.run('a_arrange')")
            d.frames(2)
            u.press_css("[data-testid=arrange-ok]")
            r = wait(lambda: d.call("window.__erdtex.arrangeResult()") or ("as tidy as" in u.state()["status"] and "tidy"), 60)
            if not r:
                bad.append(f"{f.name}: no end")
                continue
            if r == "tidy":  # nothing better found: the diagram stays as it is
                if d.call("window.__erdtex.text()") != original:
                    bad.append(f"{f.name}: changed without a result")
                continue
            arranged += 1
            after = d.call("window.__erdtex.figures()")
            if after["overlaps"]:
                bad.append(f"{f.name}: {after['overlaps']} overlaps")
            if after["moved_out"] or after["box_overlaps"] > before["box_overlaps"]:
                bad.append(f"{f.name}: boxes ({after['moved_out']} out, {before['box_overlaps']} -> {after['box_overlaps']} overlapping)")
            if after["close"] > before["close"]:
                bad.append(f"{f.name}: lines on top of one another ({before['close']} -> {after['close']})")
            if after["score"] > before["score"] + 1e-6:
                bad.append(f"{f.name}: worse ({before['score']:.0f} -> {after['score']:.0f})")
            d.call("window.__erdtex.focusCanvas()")
            u.keys("Control", "z")
            d.frames(3)
            if d.call("window.__erdtex.text()") != original:
                bad.append(f"{f.name}: Ctrl+Z doesn't restore it")
        check(not bad and arranged >= len(files) // 2, f"every repository diagram ({arranged} of {len(files)} arranged) tidied without overlaps or lines on top of one another, not worse, undone by Ctrl+Z {bad}")
    finally:
        d.quit()
    print(f"({time.time() - t0:.0f} s)")
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
