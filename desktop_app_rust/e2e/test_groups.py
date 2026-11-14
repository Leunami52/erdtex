"""Tabs side by side (README, "Tabs side by side") with real pointer input and keys: tabs reordered,
split off to an edge, moved onto another group's tab bar, by the tab's menu and by the keys; the
splitter; the panels and the keys following the group clicked in; a group closing with its last tab.

    npx tauri build --no-bundle
    python3 e2e/test_groups.py
"""
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP, check, failures  # noqa: E402
from test_docks import Docks  # noqa: E402


class G(Docks):
    def groups(self):
        """Each group's tabs by name, left to right then top to bottom."""
        names = {t["name"]: t for t in self.state()["tabs"]}
        ids = self.d.js("return Object.fromEntries([...document.querySelectorAll('.gtab')].map(e => [e.dataset.tab, e.querySelector('span').textContent.replace('*', '')]))")
        rects = self.d.js("""return [...document.querySelectorAll('.dv-groupview')].map(g => {
            const r = g.getBoundingClientRect();
            return [r.left, r.top, [...g.querySelectorAll('.gtab')].map(e => e.dataset.tab)]; })""")
        rects.sort(key=lambda r: (r[0], r[1]))
        del names
        return [[ids[i][0] for i in r[2]] for r in rects]

    menu_seen: list = []

    def menu_visible(self) -> bool:
        """Whether the context menu is really on screen: each entry is the topmost element at its middle."""
        return self.d.js("""const bs = [...document.querySelectorAll('[data-testid=context-menu] button')];
            return bs.length > 0 && bs.every(b => { const r = b.getBoundingClientRect();
                return r.width > 0 && document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2) === b; });""")

    def tab_css(self, name):
        return self.d.js("return '[data-tab=\"' + [...document.querySelectorAll('.gtab')].find(e => e.textContent.trim().replace('*', '').startsWith(arguments[0])).dataset.tab + '\"]'", name)

    def group_rects(self):
        rs = self.d.js("return [...document.querySelectorAll('.dv-groupview')].map(g => { const r = g.getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; })")
        return sorted(rs, key=lambda r: (r[0], r[1]))

    def current(self):
        s = self.state()
        return s["tabs"][s["current"]]["name"][0]

    def click_in_group(self, k, fx=0.5, fy=0.75):
        x, y, w, h = self.group_rects()[k]
        self.d.pointer([("move", x + w * fx, y + h * fy), ("down", 0), ("up", 0)])
        self.d.frames(3)

    def tab_menu(self, name, label):
        x, y = self.center(self.tab_css(name))
        self.d.pointer([("move", x, y), ("down", 2), ("up", 2)])
        self.d.frames(2)
        labels = self.d.js("return [...document.querySelectorAll('[data-testid=context-menu] button')].map(b => b.textContent)")
        self.menu_seen.append(self.menu_visible())
        if label in labels:
            self.d.js("[...document.querySelectorAll('[data-testid=context-menu] button')].find(b => b.textContent === arguments[0]).click()", label)
            self.d.frames(3)
        else:
            self.keys("Escape")
        return labels


def restored(tmp: Path):
    """The tabs of the last session at the start: the saved one is shown, its canvas has its real size
    (grid and rulers), and it responds."""
    import json
    cfg = tmp / "cfg2"
    cfg.mkdir()
    paths = json.dumps([str(tmp / f"{n}.erd") for n in "abc"])
    (cfg / "settings.toml").write_text(f"'open tabs' = '{paths}'\n'active tab' = '0'\n")
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(cfg), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = G(d)
    try:
        d.frames(6)
        check(u.groups() == [["a", "b", "c"]] and u.current() == "a", f"at the start: the tabs again, the saved one shown ({u.groups()}, {u.current()})")
        sizes = d.js("""const c = [...document.querySelectorAll('[data-testid=canvas]')].find(e => e.offsetParent);
            const r = c.getBoundingClientRect(); return [r.width, r.height, ...[...c.querySelectorAll('canvas')].map(k => [k.width, k.height])];""")
        w, h, grid, top, left = sizes
        check(grid[0] >= w and grid[1] >= h and top[0] >= w - 2 and left[1] >= h - 2,
              f"its canvas knows its real size: the grid and rulers are drawn for it ({sizes})")
        x, y, gw, gh = u.group_rects()[0]
        d.call("window.__erdtex.focusCanvas()")
        u.keys("Control", "3")
        d.pointer([("move", x + gw * 0.3, y + gh * 0.7), ("down", 0), ("up", 0)])
        d.frames(3)
        s = u.state()
        check(s["tabs"][s["current"]]["modified"] and u.current() == "a", "and it responds: an entity placed in it")
    finally:
        d.quit()


def quit_app(d):
    """Quit (File → Quit), and wait until the window has closed (the settings are written then)."""
    d.js("window.__erdtex.run('a_quit'); return 1")
    for _ in range(100):
        try:
            d.js("return 1")
            time.sleep(0.1)
        except Exception:  # noqa: BLE001
            return


def kept(tmp: Path):
    """The layout of the groups kept for the next start: the groups and what each shows; a file changed
    by another program in a group that isn't the active one; a file gone by the next start."""
    for name in "pqrs":
        (tmp / f"{name}.erd").write_text(f"\\entity{{{name}}}(0,0){{Entity {name.upper()}}}\n")
    env = {"ERDTEX_CONFIG_DIR": str(tmp / "cfg3"), "ERDTEX_PRIVATE_CLIPBOARD": "1"}
    d = Driver(APP, env)
    u = G(d)
    try:
        d.frames(4)
        for name in "pqrs":
            d.call("window.__erdtex.open(args[0])", str(tmp / f"{name}.erd"))
        d.call("window.__erdtex.open(args[0])", str(tmp / "r.erd"))            # shown, then split off at the right
        d.call("window.__erdtex.run('a_split_right')")
        d.call("window.__erdtex.open(args[0])", str(tmp / "s.erd"))
        d.call("window.__erdtex.run('a_split_down')")
        d.call("window.__erdtex.open(args[0])", str(tmp / "q.erd"))            # the left group shows q
        d.frames(3)
        before = u.groups()
        check(before == [["p", "q"], ["s"], ["r"]], f"a layout of three groups ({before})")
        quit_app(d)
    finally:
        d.quit()
    shown = lambda: d.js("""return [...document.querySelectorAll('.dv-groupview')].map(g => { const r = g.getBoundingClientRect();
        const t = g.querySelector('.dv-tab.dv-active-tab .gtab span'); return [r.left, r.top, t ? t.textContent[0] : null]; })
        .sort((a, b) => a[0] - b[0] || a[1] - b[1]).map(x => x[2])""")
    d = Driver(APP, env)
    u = G(d)
    try:
        d.frames(6)
        check(u.groups() == before, f"at the next start: the same groups, the same tabs in each ({u.groups()})")
        check(shown() == ["q", "s", "r"] and u.current() == "q", f"each group shows the tab it showed, the current one is current ({shown()}, {u.current()})")
        sizes = d.js("""return [...document.querySelectorAll('[data-testid=canvas]')].filter(e => e.offsetParent).map(c => {
            const r = c.getBoundingClientRect(); const g = c.querySelector('canvas'); return [Math.round(r.width), Math.round(r.height), g.width, g.height]; })""")
        check(len(sizes) == 3 and all(gw >= w and gh >= h for w, h, gw, gh in sizes),
              f"every group's canvas knows its real size ({sizes})")
        (tmp / "r.erd").write_text("\\entity{r}(0,0){Entity R}\n\\entity{changed}(3,0){Changed}\n")
        time.sleep(3.5)                                                  # the files are looked at every 1.5 s
        d.call("window.__erdtex.open(args[0])", str(tmp / "r.erd"))
        check("changed" in d.call("window.__erdtex.text()"), "a file changed by another program is reloaded in its group")
        d.call("window.__erdtex.open(args[0])", str(tmp / "q.erd"))
        quit_app(d)
    finally:
        d.quit()
    (tmp / "s.erd").unlink()
    d = Driver(APP, env)
    u = G(d)
    try:
        d.frames(6)
        status = u.state()["status"]
        check(u.groups() == [["p", "q"], ["r"]] and "s.erd" in status,
              f"a file gone by then: its group is left out, and the status says so ({u.groups()}, {status!r})")
    finally:
        d.quit()


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-groups-"))
    for name in ("a", "b", "c", "d"):
        (tmp / f"{name}.erd").write_text(f"\\entity{{{name}}}(0,0){{Entity {name.upper()}}}\n\\entity{{{name}2}}(5,-2){{Second}}\n")
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp / "cfg"), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = G(d)
    t0 = time.time()
    try:
        d.frames(4)
        for name in ("a", "b", "c"):
            d.call("window.__erdtex.open(args[0])", str(tmp / f"{name}.erd"))
        d.frames(4)
        check(u.groups() == [["a", "b", "c"]], f"one group with the three tabs ({u.groups()})")
        k = u.state()["zoom"]
        d.call("window.__erdtex.run('a_fit')")
        d.frames(2)
        check(abs(u.state()["zoom"] - k) < 1e-6, f"an opened diagram is fitted to its canvas's real size ({k:.3f} -> {u.state()['zoom']:.3f})")

        # reordered within the tab bar: a after c
        a = u.center(u.tab_css("a"))
        cx, cy = u.center(u.tab_css("c"))
        cw = d.rect(u.tab_css("c"))["w"]
        u.drag(a, (cx + cw / 2 - 4, cy))
        check(u.groups() == [["b", "c", "a"]], f"a tab dragged within its tab bar moves there ({u.groups()})")

        # split off to the right edge, then moved onto the other group's tab bar
        r = u.group_rects()[0]
        u.drag(u.center(u.tab_css("c")), (r[0] + r[2] - 30, r[1] + r[3] / 2))
        check(u.groups() == [["b", "a"], ["c"]], f"a tab dragged to the right edge splits off ({u.groups()})")
        bar = d.js("const b = [...document.querySelectorAll('.dv-tabs-and-actions-container')].map(e => e.getBoundingClientRect()).sort((p, q) => p.left - q.left)[1]; return [b.left, b.top, b.width, b.height]")
        u.drag(u.center(u.tab_css("b")), (bar[0] + bar[2] - 20, bar[1] + bar[3] / 2))
        check(u.groups() == [["a"], ["c", "b"]], f"a tab dropped on another group's tab bar moves there ({u.groups()})")

        # the group clicked in: current tab, keys, editing, the Element pane
        u.click_in_group(0)
        check(u.current() == "a", f"a click in a group makes its tab current ({u.current()})")
        u.keys("Control", "3")
        d.frames(2)
        check(u.state()["tool"] == "entity", f"Ctrl+3 there: the entity tool ({u.state()['tool']})")
        u.click_in_group(0, 0.2, 0.85)
        s = u.state()
        check(s["tabs"][s["current"]]["modified"] and u.current() == "a", "placing goes into that tab")
        u.click_in_group(1, 0.9, 0.9)
        check(u.current() == "b" and u.state()["tool"] == "select", f"the other group: its tab, with its own tool ({u.current()}, {u.state()['tool']})")
        d.call("window.__erdtex.select(['b'])")
        d.frames(3)
        props = lambda: d.js("const p = document.querySelector('[data-testid=properties]'); return p.innerText + ' ' + [...p.querySelectorAll('input, textarea')].map(e => e.value).join(' ')")
        check("Entity B" in props(), "the Element pane shows the current group's selection")
        u.click_in_group(0, 0.9, 0.15)
        check(u.current() == "a" and "Entity B" not in props(), "and follows the group clicked in")

        # the keys: next group, Ctrl+Tab within the group
        d.call("window.__erdtex.focusCanvas()")
        u.keys("Control", "Alt", "PgDown")
        d.frames(3)
        check(u.current() == "b", f"Ctrl+Alt+PgDown: the next group ({u.current()})")
        u.keys("Control", "Tab")
        d.frames(3)
        check(u.current() == "c", f"Ctrl+Tab: the next tab of the group ({u.current()})")
        u.keys("Control", "Alt", "PgUp")
        d.frames(3)
        check(u.current() == "a", f"Ctrl+Alt+PgUp: the previous group ({u.current()})")

        # the splitter (before any right click: see the menu's part below)
        g0 = u.group_rects()[0]
        sash = d.js("""const edge = arguments[0];
            const s = [...document.querySelectorAll('.dv-sash')].map(e => e.getBoundingClientRect())
                .filter(r => r.height > 100 && Math.abs(r.left + r.width / 2 - edge) < 12)[0];
            return s ? [s.left + s.width / 2, s.top + s.height / 2] : null;""", g0[0] + g0[2])
        before = g0[2]
        if sash:
            u.drag((sash[0], sash[1]), (sash[0] - 120, sash[1]))
        after = u.group_rects()[0][2]
        check(sash is not None and abs((before - after) - 120) < 10, f"the splitter changes the sizes ({before:.0f} -> {after:.0f})")

        # the tab's menu: a group's only tab can't split, but can move to the other group
        labels = u.tab_menu("a", "Move to Other Group")
        check("Split Right" not in labels and "Move to Other Group" in labels, f"the menu of a group's only tab ({labels})")
        check(u.groups() == [["c", "b", "a"]], f"Move to Other Group; the empty group closes ({u.groups()})")
        # after a right click WebKit (through WebDriver) sends no more pointerdowns: tabs still move
        a = u.center(u.tab_css("a"))
        cx, cy = u.center(u.tab_css("c"))
        u.drag(a, (cx - 10, cy))
        check(u.groups() == [["a", "c", "b"]], f"after a right click, a tab is still dragged ({u.groups()})")
        u.drag(u.center(u.tab_css("a")), (u.center(u.tab_css("b"))[0] + 30, cy))
        check(u.groups() == [["c", "b", "a"]], f"and back ({u.groups()})")
        labels = u.tab_menu("b", "Split Down")
        check(u.groups() == [["c", "a"], ["b"]] and len(u.group_rects()) == 2 and u.group_rects()[1][1] > u.group_rects()[0][1] + 50,
              f"Split Down from the tab's menu: below ({u.groups()}, {labels})")
        d.call("window.__erdtex.select([])")
        u.click_in_group(0)
        d.call("window.__erdtex.focusCanvas()")
        u.keys("Control", "Alt", "Right")
        d.frames(3)
        g = u.groups()
        check(len(g) == 3 and sum(len(x) for x in g) == 3, f"Ctrl+Alt+Right: the current tab splits off at the right ({g})")

        # New and Open go into the active group
        u.click_in_group(0)
        before = u.groups()
        d.call("window.__erdtex.open(args[0])", str(tmp / "d.erd"))
        d.frames(3)
        after = u.groups()
        grew = [i for i in range(len(after)) if len(after[i]) == len(before[i]) + 1] if len(after) == len(before) else []
        check(len(grew) == 1 and "d" in after[grew[0]], f"an opened diagram goes into the active group ({before} -> {after})")

        # closing a group's last tab closes the group
        n = len(u.groups())
        only = next(x for x in u.groups() if len(x) == 1)[0]
        u.press_css(f"{u.tab_css(only)} .close")
        d.frames(3)
        if u.state()["question"]:
            d.js("[...document.querySelectorAll('[data-testid=question] button')].find(b => /Discard|Don/.test(b.textContent)).click()")
            d.frames(3)
        check(len(u.groups()) == n - 1, f"closing a group's last tab closes the group ({u.groups()})")
        check(u.menu_seen and all(u.menu_seen), f"the tab's menu is really on screen ({u.menu_seen})")
        x, y, w, h = u.group_rects()[0]
        d.pointer([("move", x + w / 2, y + h / 2), ("down", 2), ("up", 2)])
        d.frames(2)
        check(u.menu_visible(), "the canvas's menu is really on screen")
        u.keys("Escape")
    finally:
        d.quit()
    restored(tmp)
    kept(tmp)
    print(f"({time.time() - t0:.0f} s)")
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
