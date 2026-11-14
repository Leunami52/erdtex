#!/usr/bin/env python3
"""Editing with real clicks, drags and keys (tauri-driver + WebKitWebDriver): the Python app's
editor tests (tests/test_editor.py, and the editing parts of test_tabs.py and test_focus.py),
done through the palette, the canvas and the keyboard.

    npx tauri build --no-bundle
    python3 e2e/test_editing.py

The app gets its own settings folder and clipboard, so yours stay as they are.
"""
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402

APP = HERE.parent / "target" / "release" / "erdtex"
CM = 72.27 / 2.54
failures: list[str] = []


def check(ok: bool, what: str) -> None:
    print(("ok    " if ok else "FAIL  ") + what)
    if not ok:
        failures.append(what)


class User:
    """What a user does, in diagram centimetres."""

    def __init__(self, d: Driver):
        self.d = d

    def state(self) -> dict:
        return self.d.call("window.__erdtex.state()")

    def text(self) -> str:
        return self.d.call("window.__erdtex.text()")

    def at(self, x: float, y: float) -> tuple[float, float]:
        v = self.state()["view"]
        r = self.d.rect("[data-testid=canvas]")
        return r["x"] + x * CM * v["k"] + v["tx"], r["y"] - y * CM * v["k"] + v["ty"]

    def click(self, x, y, hold=(), button=0):
        sx, sy = self.at(x, y)
        self.d.pointer([("move", sx, sy), ("down", button), ("up", button)], hold)
        self.d.frames()

    def double(self, x, y):
        sx, sy = self.at(x, y)
        self.d.pointer([("move", sx, sy), ("down", 0), ("up", 0), ("pause", 60), ("down", 0), ("up", 0)])
        self.d.frames()

    def drag(self, a, b, hold=()):
        (ax, ay), (bx, by) = self.at(*a), self.at(*b)
        steps = [("move", ax, ay), ("down", 0)]
        for k in range(1, 6):
            t = k / 5
            steps += [("pause", 20), ("move", ax + t * (bx - ax), ay + t * (by - ay))]
        steps += [("up", 0), ("pause", 500)]          # not a double click with what comes next
        self.d.pointer(steps, hold)
        self.d.frames()

    def handle(self, name) -> tuple[float, float]:
        """Where a handle of the selected element is, in cm."""
        x, y = self.d.call("window.__erdtex.handles()")[name]
        return x / CM, y / CM

    def tool(self, name):
        r = self.d.rect(f'[data-tool="{name}"]')
        self.d.pointer([("move", r["x"] + r["w"] / 2, r["y"] + r["h"] / 2), ("down", 0), ("up", 0)])
        self.d.frames()

    def keys(self, *combo):
        self.d.keys(*combo)
        self.d.frames()

    def fresh(self, kind="er"):
        self.d.call("window.__erdtex.fresh(args[0])", kind)
        self.d.frames(3)


def place_er(u: User):
    u.fresh()
    u.tool("entity")
    u.click(0, 0)
    u.tool("relationship")
    u.click(4, 0)
    u.tool("entity")
    u.click(8, 0)
    u.tool("link")
    u.drag((0, 0), (4, 0))
    u.drag((4, 0), (8, 0))
    u.tool("select")


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-edit-"))
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = User(d)
    try:
        d.call("window.__erdtex.setMode('light')")
        # -- placing and connecting --------------------------------------------------------
        place_er(u)
        t = u.text()
        check("\\entity{entity}(0,0){Entity}" in t and "\\relationship{relates}(4,0){relates}" in t
              and "\\entity{entity2}(8,0){Entity}" in t, "palette and clicks place entity, relationship, entity")
        check("\\link{entity}{relates}" in t and "\\link{relates}{entity2}" in t, "dragging with the line tool connects")
        u.tool("link")
        u.drag((0, 0), (8, 0))
        st = u.state()
        check(u.text().count("\\link") == 2 and "relationship" in st["status"],
              f"entity to entity is refused, and says why: {st['status']!r}")
        u.keys("Escape")
        check(u.state()["tool"] == "select", "Esc goes back to the select tool")
        # -- moving, snapping, undo -----------------------------------------------------------
        u.drag((8, 0), (9.13, 1.9))
        check("\\entity{entity2}(9.25,2){Entity}" in u.text(), "dragging moves, snapped to 0.25 cm")
        u.keys("Control", "z")
        check("\\entity{entity2}(8,0){Entity}" in u.text(), "Ctrl+Z undoes the move")
        u.keys("Control", "Shift", "z")
        check("\\entity{entity2}(9.25,2){Entity}" in u.text(), "Ctrl+Shift+Z redoes it")
        u.drag((9.25, 2), (10.13, 2), hold=("Alt",))
        line = next((l for l in u.text().split("\n") if "entity2" in l), "")
        x = float(line.split("(")[1].split(",")[0])
        check(abs(x - 10.13) < 0.02 and abs(x / 0.25 - round(x / 0.25)) > 0.01, f"with Alt, without snapping: {line}")
        u.keys("Control", "z")
        # -- nudging ----------------------------------------------------------------------------
        u.click(9.25, 2)
        u.keys("Right")
        check("\\entity{entity2}(9.5,2){Entity}" in u.text(), "an arrow key nudges by the snap step")
        u.keys("Shift", "Up")
        check("\\entity{entity2}(9.5,3){Entity}" in u.text(), "Shift+arrow nudges by 1 cm")
        # -- handles: bend and way points ---------------------------------------------------------
        place_er(u)
        u.click(2, 0)                                   # on the first edge
        check(u.state()["selected"] == 1, "a click on an edge selects it")
        mx, my = u.handle("bend")
        u.drag((mx, my), (mx, my + 0.6))
        t = u.text()
        check("bend=" in t, "dragging the round handle bends the edge")
        bx, by = u.handle("bend")
        u.double(bx, by)
        t = u.text()
        check("via=" in t and "bend=" not in t.split("\\link")[1], "a double click adds a way point instead of the bend")
        # -- delete ---------------------------------------------------------------------------------
        place_er(u)
        u.click(0, 0)
        u.keys("Delete")
        t = u.text()
        check("entity}(0,0)" not in t and "\\link{entity}" not in t and "relates" in t, "Delete takes the entity and its line")
        # -- rubber band and copy/paste --------------------------------------------------------
        place_er(u)
        u.drag((-3, 3), (11, -3))
        check(u.state()["selected"] == 5, f"a rectangle selects everything in it ({u.state()['selected']})")
        sx, sy = u.at(5, -6)
        d.pointer([("move", sx, sy)])
        u.keys("Control", "c")
        u.keys("Control", "v")
        t = u.text()
        check(t.count("\\entity{") == 4 and t.count("\\link") == 4, "Ctrl+C, Ctrl+V copies nodes with their lines")
        check("(4,-6)" in t or "(4.5,-6.5)" in t or "(5,-6)" in t, "it lands where the pointer is")
        u.keys("Control", "d")
        check(u.text().count("\\entity{") == 6, "Ctrl+D duplicates")
        # -- across tabs and kinds ---------------------------------------------------------------
        u.keys("Control", "a")
        u.keys("Control", "c")
        u.keys("Control", "Shift", "n")                 # a new GSM diagram
        d.frames(3)
        sx, sy = u.at(0, 0)
        d.pointer([("move", sx, sy)])
        u.keys("Control", "v")
        st = u.state()
        check(st["question"] is not None and "can't be pasted into a GSM diagram" in st["question"],
              "ER elements are refused in a GSM diagram, with the reason")
        if st["question"]:
            d.call("window.__erdtex.answer('OK')")
        u.keys("Control", "n")
        d.frames(3)
        u.keys("Control", "v")
        check(u.text().count("\\entity{") == 6, "pasting into another ER tab")
        # -- GSM: IS-A rules ---------------------------------------------------------------------
        u.fresh("gsm")
        u.tool("abstract type")
        u.click(0, 0)
        u.tool("subtype")
        u.click(0, -3)
        u.click(4, -3)
        u.tool("generalization")
        u.drag((0, 0), (0, -3))
        check("\\generalization" not in u.text(), "a generalization from the abstract type is refused")
        u.drag((0, -3), (0, 0))
        u.drag((4, -3), (0, -3))
        check(u.text().count("\\generalization") == 2, "subtype to abstract type, and subtype to subtype")
        u.drag((0, -3), (4, -3))
        check(u.text().count("\\generalization") == 2, "a cycle is refused")
        # -- weak entities: double lines and the direction ------------------------------------------
        u.fresh()
        u.keys("Control", "7")                           # weak entity
        u.click(0, 0)
        u.keys("Control", "Shift", "1")                  # weak relationship
        u.click(4, 0)
        u.keys("Control", "7")
        u.click(8, 0)
        u.keys("Control", "2")                           # line
        u.drag((0, 0), (4, 0))
        check("\\link[double]{weak}{has}" in u.text(), "the line to the only weak entity is double")
        u.drag((4, 0), (8, 0))
        check("[double]" not in u.text(), "a second weak entity leaves the direction open")
        u.keys("Control", "z")
        check("\\link[double]{weak}{has}" in u.text() and u.text().count("\\link") == 1, "and undo brings the double line back")
        # -- subdomains ---------------------------------------------------------------------------
        u.fresh()
        u.keys("Control", "Shift", "6")                  # subdomain
        u.drag((0, 0), (4, -3))
        check("\\subdomain[color=blue!60]{Subdomain}{(0,0) (4,-3)}" in u.text(), "a subdomain by dragging a rectangle")
        u.keys("Escape")                                 # tools stay chosen after placing
        mx, my = u.handle("m0")                          # the round handle in the middle of the top
        u.drag((mx, my), (mx, my + 1))
        check("(0,0) (2,1) (4,0) (4,-3) (0,-3)" in u.text(), f"dragging a side's circle adds a corner")
        u.double(2, 1)
        check(u.text().count("(") - u.text().count("\\subdomain") * 0 >= 1 and "(2,1)" not in u.text(),
              "a double click on a corner removes it")
        # -- context menu ----------------------------------------------------------------------------
        place_er(u)
        u.click(2, 0)
        mx, my = u.handle("bend")
        u.drag((mx, my), (mx, my + 0.6))
        bx, by = u.handle("bend")
        u.click(bx, by, button=2)
        items = d.js("return [...document.querySelectorAll('[data-testid=context-menu] button')].map(b => b.textContent)")
        check(items[:1] == ["Straighten"] and "Delete" in items, f"right click on an edge: {items}")
        r = d.js("const b = [...document.querySelectorAll('[data-testid=context-menu] button')].find(b => b.textContent === 'Straighten');"
                 "const r = b.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2];")
        d.pointer([("move", r[0], r[1]), ("down", 0), ("up", 0)])
        d.frames()
        check("bend" not in u.text(), "Straighten")
        # -- the snap setting --------------------------------------------------------------------------
        d.js("const s = document.querySelector('[data-testid=snap]'); s.value = '1'; s.dispatchEvent(new Event('change', {bubbles: true}));")
        d.frames()
        u.drag((8, 0), (9.4, 1.4))
        check("\\entity{entity2}(9,1){Entity}" in u.text() and u.state()["snap"] == 1,
              f"Snap: 1 cm ({u.state()['snap']}): " + next((l for l in u.text().split("\n") if "entity2" in l), ""))
    finally:
        d.quit()
    # -- a changed shortcut (kept in the settings) is used at the next start ------------------
    (tmp / "settings.toml").write_text('"shortcuts/er.entity" = "Ctrl+K"\n')
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = User(d)
    try:
        u.keys("Control", "k")
        check(u.state()["tool"] == "entity", "a changed shortcut: Ctrl+K picks the entity tool")
        u.keys("Escape")
        u.keys("Control", "3")
        check(u.state()["tool"] == "select", "and Ctrl+3 doesn't any more")
        title = d.js("return document.querySelector('[data-tool=entity]').title")
        check("Ctrl+K" in title, f"the palette's tooltip says so: {title!r}")
    finally:
        d.quit()
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
