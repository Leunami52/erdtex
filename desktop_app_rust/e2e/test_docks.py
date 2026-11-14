#!/usr/bin/env python3
"""The panes with real drags and clicks: detached by dragging the title and docked at another
side, folded, closed and brought back from View, the tools reflowing to the pane's width, and the
layout kept for the next start (as the Python app's dock widgets).

    npx tauri build --no-bundle
    python3 e2e/test_docks.py
"""
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP, check, failures  # noqa: E402
from test_dialogs import User  # noqa: E402


class Docks(User):
    def drag(self, a, b):
        steps = [("move", *a), ("down", 0)]
        for k in range(1, 9):
            steps += [("pause", 20), ("move", a[0] + (b[0] - a[0]) * k / 8, a[1] + (b[1] - a[1]) * k / 8)]
        self.d.pointer(steps + [("up", 0)])
        self.d.frames(3)

    def center(self, css):
        r = self.d.rect(css)
        return r["x"] + r["w"] / 2, r["y"] + r["h"] / 2

    def pane(self, id_):
        return self.d.call("window.__erdtex.docks()")["panes"][id_]

    def menu(self, title, label):
        self.d.js("[...document.querySelectorAll('.menubar .title')].find(b => b.textContent === arguments[0]).click()", title)
        self.d.frames(2)
        self.d.js("[...document.querySelectorAll('.menubar .row')].find(b => b.querySelector('.label').textContent.startsWith(arguments[0])).click()", label)
        self.d.frames(3)


def tabs(u: Docks, tmp: Path):
    """The tab bar with real pointer input: dragging moves a tab, the right button opens its menu, and
    WebKit's own page menu (Back, Forward, Stop, Reload) appears nowhere but in text fields."""
    d = u.d
    for name in ("one", "two", "three"):
        (tmp / f"{name}.erd").write_text(f"\\entity{{{name}}}(0,0){{{name}}}\n")
        d.call("window.__erdtex.open(args[0])", str(tmp / f"{name}.erd"))
    names = lambda: d.js("return [...document.querySelectorAll('.gtab span')].map(e => e.textContent.replace('*', ''))")
    start = names()
    css = lambda name: f"[data-tab='{d.js('return [...document.querySelectorAll(\'[data-tab]\')].find(e => e.textContent.includes(arguments[0])).dataset.tab', name)}']"
    a = u.center(css("one.erd"))
    b = u.center(css("three.erd"))
    u.drag(a, (b[0] + 20, b[1]))
    check(names()[-3:] == ["two.erd", "three.erd", "one.erd"], f"dragging a tab moves it ({start} -> {names()})")
    check(u.state()["tabs"][[t["name"] for t in u.state()["tabs"]].index("one.erd")]["path"] == str(tmp / "one.erd")
          and d.js("return document.querySelector('.dv-tab.dv-active-tab .gtab').textContent").startswith("one.erd"),
          "the pressed tab is shown")
    x, y = u.center(css("two.erd"))
    d.pointer([("move", x, y), ("down", 2), ("up", 2)])
    d.frames(2)
    labels = d.js("return [...document.querySelectorAll('[data-testid=context-menu] button')].map(b => b.textContent)")
    visible = d.js("""return [...document.querySelectorAll('[data-testid=context-menu] button')].every(b => {
        const r = b.getBoundingClientRect(); return document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2) === b; })""")
    check(visible, "the tab's menu is really on screen")
    check(labels[:3] == ["Close Tab", "Close Other Tabs", "Close Tabs to the Right"], f"the tab's own menu ({labels})")
    d.js("[...document.querySelectorAll('[data-testid=context-menu] button')].find(b => b.textContent === 'Close Other Tabs').click()")
    d.frames(3)
    check(names() == ["two.erd"], f"Close Other Tabs ({names()})")
    page_menu = ("const e = new MouseEvent('contextmenu', {bubbles: true, cancelable: true});"
                 "document.querySelector(arguments[0]).dispatchEvent(e); return e.defaultPrevented")
    check(d.js(page_menu, ".dv-tabs-and-actions-container") and d.js(page_menu, ".menubar") and d.js(page_menu, "[data-pane=element] header"),
          "no page menu on the tab bar, the menus or a pane")
    d.call("window.__erdtex.select(['two'])")
    d.frames(3)
    check(not d.js(page_menu, "[data-testid=properties] input"), "a text field keeps its own menu")


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-docks-"))
    env = {"ERDTEX_CONFIG_DIR": str(tmp / "cfg"), "ERDTEX_PRIVATE_CLIPBOARD": "1"}
    d = Driver(APP, env)
    u = Docks(d)
    t0 = time.time()
    try:
        d.frames(4)
        shown = [p for p in ("tools", "element", "preview", "problems") if u.shown(f"[data-pane={p}]")]
        check(shown == ["tools", "element", "problems"], f"the panes at the start, the preview hidden ({shown})")
        title = d.js("return document.querySelector('[data-pane=problems] .title').textContent")
        check(title.startswith("Problems ("), f"the Problems pane says how many ({title})")

        # the tools reflow to the pane's width
        cols = lambda: d.js("return getComputedStyle(document.querySelector('[data-testid=palette] .grid')).gridTemplateColumns.split(' ').length")  # noqa: E731
        wide = cols()
        r = d.rect("[data-pane=tools]")
        u.drag((r["x"] + r["w"] + 2, r["y"] + 200), (r["x"] + 62, r["y"] + 200))
        check(cols() == 1 < wide, f"a narrow tools pane: one column ({wide} -> {cols()})")

        # folded and unfolded
        u.press_css("[data-pane=tools] [data-act=fold]")
        check(u.shown("[data-area=left] [data-unfold=tools]") and not u.shown("[data-testid=palette]"), "folded: a narrow strip")
        u.press_css("[data-unfold=tools]")
        check(u.shown("[data-testid=palette]"), "unfolded again")

        # detached, and docked at the right
        hx, hy = u.center("[data-pane=element] .title")
        u.drag((hx, hy), (500, 140))              # high up: not over the Problems pane's title
        check(u.pane("element")["floating"], "dragging the title detaches a pane")
        size = lambda: d.js("const r = document.querySelector('[data-pane=element]').getBoundingClientRect(); return [r.width, r.height]")
        before = size()
        d.frames(30)
        check(size() == before, f"a floating pane keeps its size ({before} -> {size()})")
        hx, hy = u.center("[data-pane=problems] .title")
        u.drag((hx, hy), (d.js("return window.innerWidth") - 20, 400))
        p = u.pane("problems")
        check(p["area"] == "right" and not p["floating"],
              f"dropped at the right edge, it docks there ({p['area']}, floating {p['floating']}, x {p['x']:.0f} of {d.js('return window.innerWidth')})")
        hx, hy = u.center("[data-pane=element] .title")
        d.pointer([("move", hx, hy), ("down", 0), ("up", 0), ("pause", 60), ("down", 0), ("up", 0)])
        d.frames(3)
        check(not u.pane("element")["floating"], "a double click on the title docks it again")

        # closed, and back from View
        u.press_css("[data-pane=problems] [data-act=close]")
        check(not u.shown("[data-pane=problems]"), "closed")
        u.menu("View", "Problems")
        check(u.shown("[data-pane=problems]"), "View → Problems brings it back")
        u.menu("View", "LaTeX Preview")
        check(u.shown("[data-pane=preview]"), "View → LaTeX Preview")
        time.sleep(0.6)                                  # the layout is saved a moment after a change
        tabs(u, tmp)
    finally:
        d.quit()
    d = Driver(APP, env)
    u = Docks(d)
    try:
        d.frames(4)
        p = u.pane("problems")
        check(p["area"] == "right" and u.pane("preview")["visible"], "the layout is kept for the next start")
    finally:
        d.quit()
    print(f"({time.time() - t0:.0f} s)")
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
