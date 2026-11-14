#!/usr/bin/env python3
"""The panels with real clicks and typing (tauri-driver + WebKitWebDriver): the Python app's
focus tests (tests/test_focus.py) and the panel tests that need a window (the label field,
the attribute table with Shift+Enter, ticks, the cardinality picker, questions, the Problems
panel).  Drop-down lists are chosen through the page (their popup is the system's).

    npx tauri build --no-bundle
    python3 e2e/test_panels.py
"""
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP, User, check, failures  # noqa: E402

BASE = r"\entity{base}(3,3){Base}\attributes{base}{*x}"


class Panels(User):
    def load(self, text, kind="er"):
        self.d.call("window.__erdtex.load(args[0], args[1])", text, kind)
        self.d.frames(3)
        self.d.call("window.__erdtex.focusCanvas()")                    # the keyboard on the canvas
        self.keys("Escape")

    def focused(self) -> str:
        return self.d.call("window.__erdtex.focused()")

    def count(self) -> int:
        return self.text().count("\\")

    def press_css(self, css, dx=None, dy=None):
        self.d.js("document.querySelector(arguments[0]).scrollIntoView({block: 'nearest'})", css)
        r = self.d.rect(css)
        x = r["x"] + (r["w"] / 2 if dx is None else dx)
        y = r["y"] + (r["h"] / 2 if dy is None else dy)
        self.d.pointer([("move", x, y), ("down", 0), ("up", 0)])
        self.d.frames()

    def choose(self, row_id, value):
        """A drop-down of the panel: as if chosen from its list."""
        self.d.js("const s = document.querySelector(`[data-row='${arguments[0]}'] select`);"
                  "s.value = arguments[1]; s.dispatchEvent(new Event('change', {bubbles: true}));", row_id, value)
        self.d.frames(3)

    def panel(self) -> dict:
        return self.d.call("window.__erdtex.panel()")

    def place(self, tool, x=0, y=0):
        self.tool(tool)
        if tool == "subdomain":
            self.drag((x - 1, y + 1), (x + 3, y - 2))
        else:
            self.click(x, y)
        self.d.frames(3)


def after_placing(u: Panels):
    for kind, tools in (("er", ("entity", "weak entity", "relationship", "identification dependency", "note", "subdomain")),
                        ("gsm", ("abstract type", "subtype", "note", "subdomain"))):
        for tool in tools:
            u.load(BASE if kind == "er" else "", kind)
            before = u.count()
            u.place(tool, 8, -2)
            check(u.count() > before, f"{kind} {tool}: placed")
            check(u.focused() == "textarea:label", f"{kind} {tool}: typing can start at once ({u.focused()})")
            u.keys("Control", "z")
            check(u.count() == before, f"{kind} {tool}: Ctrl+Z from the label field takes it back")
            u.place(tool, 8, -2)
            u.keys("Escape")
            st = u.state()
            check(st["tool"] == "select" and u.focused() == "canvas",
                  f"{kind} {tool}: Esc from the label field: select tool, keyboard on the canvas ({st['tool']}, {u.focused()})")


def typing(u: Panels):
    u.load(BASE)
    before = u.count()
    u.place("entity", 8, -2)
    u.d.type("Hund")
    u.d.frames()
    field = "document.querySelector('[data-row=label] textarea').value"
    check(u.d.js(f"return {field}") == "Hund", "typed into the label field")
    u.keys("Control", "z")
    check(u.d.js(f"return {field}") != "Hund", "Ctrl+Z takes back the typing first")
    check(u.count() == before + 1, "... and leaves the element")
    for _ in range(8):
        if u.count() == before:
            break
        u.keys("Control", "z")
    check(u.count() == before, "then Ctrl+Z takes back the element")

    u.load(BASE)
    before = u.count()
    u.place("entity", 8, -2)
    u.d.type("Hund")
    u.keys("Escape")
    check(r"{Hund}" in u.text() and u.state()["tool"] == "select", "Esc keeps what was typed")
    u.keys("Control", "z")
    check(r"{Entity}" in u.text(), "after Esc, Ctrl+Z takes back the label ...")
    u.keys("Control", "z")
    check(u.count() == before, "... then the element")

    u.load(BASE)
    u.place("entity", 8, -2)
    u.d.type("Dog")
    u.d.keys("Enter")
    u.d.frames(3)
    check(r"\entity{dog}" in u.text() and r"{Dog}" in u.text(), "Enter commits; a generated id follows the label")
    u.place("entity", 3, -5)                         # within the view on the check's 1920 x 1200 display
    u.d.type("Big Red Dog")
    u.keys("Control", "Backspace")
    u.d.frames()
    check(u.d.js(f"return {field}") == "Big Red ", f"Ctrl+Backspace deletes the word before the cursor ({u.d.js(f'return {field}')!r})")
    u.keys("Control", "Left")
    u.keys("Control", "Delete")
    u.d.frames()
    # "Big Red |" -> Ctrl+Left: "Big |Red " -> Ctrl+Delete deletes "Red", the space after it stays
    check(u.d.js(f"return {field}") == "Big  ", f"Ctrl+Left moves by a word, Ctrl+Delete deletes the word after it ({u.d.js(f'return {field}')!r})")
    u.keys("Escape")
    u.keys("Escape")
    u.keys("Escape")
    check(u.state()["selected"] == 0, "a second Esc clears the selection")


def elsewhere(u: Panels):
    u.load(BASE)
    u.tool("entity")
    check(u.state()["tool"] == "entity" and u.focused() == "canvas", "the palette leaves the keyboard on the canvas")
    u.keys("Escape")
    check(u.state()["tool"] == "select", "and Esc works at once")

    u.load(BASE)
    before = u.count()
    u.place("part of", 8, -2)
    u.tool("entity")
    u.keys("Control", "z")
    check(u.count() == before, "undo after only choosing a tool")

    u.load(BASE)
    u.d.call("window.__erdtex.select(['base'])")
    u.d.frames(2)
    u.d.call("window.__erdtex.tool('entity')")
    u.press_css("[data-row=id] input")
    check(u.focused() == "input:id", f"a line field has the focus ({u.focused()})")
    u.keys("Escape")
    check(u.state()["tool"] == "select" and u.focused() == "canvas", "Esc from a line field")

    u.load(BASE)
    before = u.count()
    u.place("part of", 8, -2)
    u.press_css("[data-row=pos] input[data-axis=x]")
    u.keys("Control", "z")
    check(u.count() == before, "Ctrl+Z from a number field undoes the diagram")

    u.load(r"\point{p}(1,2)")
    u.d.call("window.__erdtex.select(['p'])")
    u.d.frames(2)
    p = u.panel()
    check(p["title"] == "Junction point" and any(r["type"] == "position" for r in p["rows"]), "a junction point shows its position")


def attribute_lists(u: Panels):
    def items():
        t = u.text()
        i = t.index(r"\attributes{base}{") + len(r"\attributes{base}{")
        return t[i:t.index("}", i)]

    u.load(BASE)
    u.place("attribute tree", 3, 3)
    check(items() == "*x, attribute", f"an attribute added to the list ({items()})")
    check(u.focused() == "input:attributes", f"typing its name can start at once ({u.focused()})")
    u.keys("Control", "z")
    check(items() == "*x", "Ctrl+Z takes it back")

    u.load(BASE)
    u.place("attribute tree", 3, 3)
    u.keys("Escape")
    check(u.state()["tool"] == "select" and u.focused() == "canvas" and items() == "*x, attribute",
          "Esc from the name: keyboard on the canvas, the attribute stays")

    u.load(r"\entity{base}(3,3){Base}\attributes{base}{first}")
    u.d.call("window.__erdtex.select(['base'])")
    u.d.frames(2)
    u.click(5.2, 3)                                   # the list beside the entity
    u.press_css("input[data-name='0']")
    for name in ("PID", "name", "born"):
        sel = u.d.js("const e = document.activeElement; return [e.selectionStart, e.selectionEnd, e.value.length]")
        check(sel[0] == 0 and sel[1] == sel[2], f"the name is selected, typing replaces it ({sel})")
        u.d.type(name)
        u.keys("Shift", "Enter")
        u.d.frames(3)
    check(u.d.js("return document.activeElement.value") == "attribute", "Shift+Enter starts the next attribute")
    u.d.type("last")
    u.d.keys("Enter")
    u.d.frames(3)
    check(items() == "PID, name, born, last", f"Shift+Enter types a whole list ({items()})")

    u.load(r"\entity{base}(3,3){Base}\attributes{base}{a, b}")
    u.click(5.2, 3)
    u.press_css("input[data-key='1']")
    check(items() == "a, *b", f"the key tick ({items()})")
    u.press_css("input[data-name='1']")
    u.press_css("[data-field=attributes] .buttons button:nth-child(3)")     # Up
    check(items() == "*b, a", f"Up takes the current row up ({items()})")


def element_fields(u: Panels):
    src = (r"\entity{a}(0,0){A}\attributes{a}{*k}\relationship[dependency=Id]{r}(4,0){R}"
           r"\entity[weak]{w}(8,0){W}\attributes{w}{*n}\link{a}{r}{1}\link[double]{r}{w}{n}")
    u.load(src)
    u.d.call("window.__erdtex.select(['w'])")
    u.d.frames(2)
    u.press_css("[data-row='weak'] input")
    check(r"\link{r}{w}{n}" in u.text(), "unticking Weak removes the double line")
    u.press_css("[data-row='weak'] input")
    label = u.d.js("return document.querySelector('[data-testid=fix-all]').textContent")
    check(label == "Fix all (1)", f"ticking it again: a problem with a fix ({label})")
    u.press_css("[data-testid=fix-all]")
    check(r"\link[double]{r}{w}{n}" in u.text(), "Fix all brings the double line back")

    u.d.call("window.__erdtex.select(['r'])")
    u.d.frames(2)
    u.choose("kind", "E")
    check(r"\relationship[dependency=E]{r}" in u.text(), "the relationship's Kind")
    words = u.d.js("return document.querySelector('[data-testid=properties]').textContent")
    check("Existence dependency only" in words, "with its explanation")

    u.load(r"\entity{h}(0,0){Halter}\relationship{x}(3,0){besitzt}\entity{d}(6,0){Hund}\link{h}{x}{1}\link{x}{d}{n}")
    u.click(1.5, 0)                                    # the first edge
    check(u.panel()["title"] == "Line", "an edge selected by a click")
    u.press_css("[data-field=card] button[data-value='cn']")
    check(r"\link{h}{x}{cn}" in u.text(), "a cardinality button")
    sentence = u.d.js("return document.querySelector('[data-field=card] .sentence').textContent")
    check(sentence == "Each Hund has any number (also none) Halter.", f"and what it means ({sentence})")
    u.press_css("[data-field=card] .deviate")
    q = u.state()["question"]
    check(q is not None and "opposite ends" in q, "changing the notation asks first")
    u.press_css("[data-testid=question] button")      # Yes
    check(r"\link{h}{x}{(1,*)}" in u.text(), f"and converts the relationship ({u.text()[-60:]!r})")
    u.press_css("[data-field=card] input[data-part=lo]")
    u.keys("Control", "a")
    u.d.type("3")
    u.press_css("[data-field=card] input[data-part=hi]")
    u.keys("Control", "a")
    u.d.type("2\n")
    u.d.frames(3)
    bad = u.d.js("const e = document.querySelector('[data-field=card] .bad'); return e ? e.textContent : ''")
    check("can't be smaller" in bad, f"impossible bounds are refused at the field ({bad})")

    u.load(BASE)
    u.d.call("window.__erdtex.select([])")
    u.d.frames(2)
    u.choose("notation", "minmax")
    check(u.state()["question"] is not None, "switching the diagram's notation asks")
    u.keys("Escape")
    check("card notation" not in u.text(), "and No leaves it")
    u.press_css("[data-row='sans serif'] input")
    check("sans serif" in u.text(), "a tick of the diagram's style")
    u.keys("Control", "z")
    check("sans serif" not in u.text(), "undone with Ctrl+Z")


def label_field(u: Panels):
    u.load(r"\note(0,0){one}")
    u.click(0, 0)
    h = lambda: u.d.js("return document.querySelector('[data-row=label] textarea').offsetHeight")  # noqa: E731
    one = h()
    u.press_css("[data-row=label] textarea")
    u.keys("Control", "End")
    for k in range(5):
        u.keys("Shift", "Enter")
        u.d.type(f"line {k}")
    five = h()
    check(five > one, f"the label field grows with its text ({one} -> {five})")
    for k in range(20):
        u.keys("Shift", "Enter")
        u.d.type(f"more {k}")
    many = h()
    check(many > five and many < 400, f"up to a limit ({many})")
    u.keys("Escape")
    check(r"\note(0,0){one\\line 0" in u.text(), "the lines become \\\\")
    u.click(0, 0)
    r = u.d.rect("[data-testid=label-grip]")
    x, y = r["x"] + r["w"] / 2, r["y"] + r["h"] / 2
    before = h()
    u.d.pointer([("move", x, y), ("down", 0), ("move", x, y + 60), ("move", x, y + 120), ("up", 0)])
    u.d.frames()
    check(h() > before + 60, f"the grip sizes it ({before} -> {h()})")
    u.d.pointer([("move", x, y + 120), ("down", 0), ("up", 0), ("pause", 50), ("down", 0), ("up", 0)])
    u.d.frames()
    check(abs(h() - before) <= 2, f"a double click on the grip fits it again ({h()})")


def problems_panel(u: Panels):
    u.load(r"\entity{a}(0,0){A}\entity{b}(5,0){B}\attributes{b}{*k}")
    summary = u.d.js("return document.querySelector('[data-testid=problem-summary]').textContent")
    check("1 hint" in summary, f"the summary counts ({summary})")
    u.press_css("[data-testid=problems] li button")
    check(u.state()["selected"] == 1 and u.panel()["title"] == "Entity type", "a click on a problem shows its element")
    badge = u.d.js("return document.querySelectorAll('.badge').length")
    check(badge == 1, f"a badge on the canvas ({badge})")
    u.press_css("[data-testid=problems] .bar label input")
    rows = u.d.js("return document.querySelectorAll('[data-testid=problems] li').length")
    check(rows == 0 and u.d.js("return document.querySelectorAll('.badge').length") == 0, "without hints: none listed, no badge")
    u.press_css("[data-testid=problems] .bar label input")
    u.press_css("[data-testid=inspector] [role=tab]:nth-child(2)")
    src = u.d.js("return document.querySelector('[data-testid=source]').value")
    check(src.startswith(r"\entity{a}") and "% hint:" in src, f"the Source tab: the text and its problems ({src!r})")
    u.press_css("[data-testid=inspector] [role=tab]:nth-child(1)")


def add_list_key(u: Panels):
    """Ctrl+L (Add attribute list): the Element pane's button shows the key and does the same."""
    u.load(r"\entity{a}(0,0){A}\relationship{r}(4,0){R}\entity{b}(8,0){B}\link{a}{r}{1}\link{r}{b}{n}")
    d = u.d
    d.call("window.__erdtex.select(['a'])")
    d.frames(3)
    label = d.js("return document.querySelector('[data-row=add-tree] button')?.textContent")
    check(label == "Add attribute list (Ctrl+L)", f"the button shows its key ({label!r})")
    d.call("window.__erdtex.focusCanvas()")
    u.keys("Control", "l")
    d.frames(4)
    check("\\attributes{a}" in u.text(), "Ctrl+L adds an attribute list to the selected entity")
    check(u.focused() == "input:attributes", f"with the cursor in its first name ({u.focused()})")
    u.keys("Escape")
    # the same with the Element pane closed
    d.call("window.__erdtex.select(['r'])")
    d.js("document.querySelector('[data-pane=element] [data-act=close]').click()")
    d.frames(3)
    d.call("window.__erdtex.focusCanvas()")
    u.keys("Control", "l")
    d.frames(3)
    check("\\attributes{r}" in u.text(), "also with the Element pane closed (a relationship)")
    d.call("window.__erdtex.select([])")
    d.frames(2)
    u.keys("Control", "l")
    d.frames(2)
    check("Select an entity" in u.state()["status"], f"with nothing selected, the status says what to do ({u.state()['status']!r})")
    d.js("[...document.querySelectorAll('.menubar .title')].find(b => b.textContent === 'View').click()")
    d.frames(2)
    d.js("[...document.querySelectorAll('.menubar .row')].find(b => b.querySelector('.label').textContent.startsWith('Element')).click()")
    d.frames(3)


def all_attributes(u: Panels):
    """An entity's list of all its attributes: its own, then the inherited ones with their trace; a double
    click opens one where it is defined."""
    text = (REPO / "app" / "test_diagrams" / "railway.erd").read_text()
    u.load(text)
    d = u.d
    d.call("window.__erdtex.select(['engineer'])")
    d.frames(3)
    p = u.panel()
    check(p["title"] == "Entity type", f"the full name of the element in the title ({p['title']!r})")
    rows = next((r["rows"] for r in p["rows"] if r["type"] == "lineage"), [])
    names = [(r["name"], r["from"]) for r in rows]
    check(names[:2] == [("driving hours", ""), ("license", "")] and ("empID", "Member") in names and ("PID", "Person") in names,
          f"its own attributes first, then the inherited ones with where they come from ({names})")
    pid = next(r for r in rows if r["name"] == "PID")
    check(pid["key"] and "Engineer → Member (IS_A) → Person (IS_A)" in pid["trace"], f"the key marked, the trace complete ({pid['trace']!r})")
    shown = d.js("return [...document.querySelectorAll('[data-lineage]')].map(r => r.title).length")
    check(shown == len(rows), f"each row has its trace as a tooltip ({shown} of {len(rows)})")
    k = [r["name"] for r in rows].index("address")
    css = f"[data-lineage='{k}']"
    d.js("document.querySelector(arguments[0]).scrollIntoView({block: 'nearest'})", css)
    r = d.rect(css)
    d.pointer([("move", r["x"] + 20, r["y"] + r["h"] / 2), ("down", 0), ("up", 0), ("pause", 60), ("down", 0), ("up", 0)])
    d.frames(4)
    p = u.panel()
    value = d.js("return document.activeElement && document.activeElement.value")
    check(p["title"] == "Attribute list" and value == "address",
          f"a double click opens it in Person's attribute list, the cursor in its name ({p['title']!r}, {value!r})")
    u.keys("Escape")


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-panels-"))
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = Panels(d)
    t0 = time.time()
    try:
        d.frames(3)
        for part in (after_placing, typing, elsewhere, attribute_lists, element_fields, label_field, problems_panel, add_list_key, all_attributes):
            print(f"-- {part.__name__}")
            part(u)
    finally:
        d.quit()
    # -- a cancel key that is a letter types in a field ------------------------------------------
    (tmp / "settings.toml").write_text('"shortcuts/cancel" = "K"\n')
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = Panels(d)
    try:
        u.d.call("window.__erdtex.load(args[0], 'er')", BASE)
        u.d.frames(3)
        u.place("entity", 8, -2)
        u.d.type("K")
        u.d.frames()
        value = u.d.js("return document.querySelector('[data-row=label] textarea').value")
        check("K" in value and u.state()["tool"] == "entity", f"a cancel key that is a letter types in a field ({value})")
        u.d.call("window.__erdtex.focusCanvas()")
        u.keys("k")
        check(u.state()["tool"] == "select", "and cancels on the canvas")
    finally:
        d.quit()
    print(f"({time.time() - t0:.0f} s)")
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
