"""The rule books in the real window: Help → Rule Books… and Shift+F1, the books and rules, a page with its examples
drawn by the core, the search, links in a page and back, and a problem, a refused line, a decision of the
Relational Schema window and a question of the GSM → ER dialog opening their rules.

    npx tauri build --no-bundle
    python3 e2e/test_rulebook.py
"""
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP, User, check, failures  # noqa: E402
from test_docks import Docks  # noqa: E402


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-rulebook-"))
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = Docks(d)
    t0 = time.time()
    js = d.js
    try:
        d.frames(4)
        # the desktop's Help menu: the website and the source code; no privacy note (the website's)
        js("[...document.querySelectorAll('.menubar .title')].find(b => b.textContent === 'Help').click()")
        d.frames(2)
        labels = js("return [...document.querySelectorAll('.menubar .dropdown .label')].map(l => l.textContent)")
        check("Open the Website" in labels and "Source Code on GitHub" in labels and "Privacy" not in labels
              and "Imprint" not in labels, f"the Help menu's links ({labels})")
        js("document.querySelector('.menubar .title').click()")   # closes it
        d.frames(2)
        u.menu("Help", "Rule Books")
        d.frames(6)
        check(u.shown("[data-testid=rulebook]"), "Help → Rule Books… opens the window")
        books = js("return [...document.querySelectorAll('[data-testid=rulebook] .book')].map(b => b.textContent.replace(/^[▸▾]/, ''))")
        check(books[:6] == ["Modelling ER", "Modelling GSM", "Drawing", "GSM → ER", "ER → Relational", "Notations"], f"the books ({books})")
        title = js("return document.querySelector('[data-testid=rulebook-page] h2')?.textContent")
        check(title == "Modelling ER", f"it starts with the first book's introduction ({title!r})")
        # the books are collapsed; a click on one lists its rules
        listed = js("return document.querySelectorAll('[data-rule-item]').length")
        check(listed == 0, f"the books start collapsed ({listed} rules listed)")
        js("document.querySelector('[data-book=modelling-er]').click()")
        d.frames(3)
        listed = js("return [...document.querySelectorAll('[data-rule-item]')].map(b => b.dataset.ruleItem)")
        check(listed[:1] == ["ER-1"] and all(r.startswith("ER-") for r in listed), f"a click on a book lists its rules ({len(listed)})")
        # a rule: its badge, what it applies to, its examples drawn
        js("document.querySelector(\"[data-rule-item='ER-1']\").click()")
        d.frames(8)
        page = js("const p = document.querySelector('[data-testid=rulebook-page]'); return {h: p.querySelector('h2').textContent,"
                  " badge: p.querySelector('.badge').textContent, svgs: p.querySelectorAll('[data-example] svg').length,"
                  " applies: p.querySelectorAll('.element svg').length}")
        check("ER-1" in page["h"] and page["badge"] == "Hint", f"a rule's page: its id, title and badge ({page})")
        check(page["svgs"] == 3 and page["applies"] == 2, f"its examples drawn, and the symbols of what it applies to ({page})")
        # the search
        u.d.js("const i = document.querySelector('[data-testid=rulebook-search]'); i.value = 'amphibious';"
               "i.dispatchEvent(new Event('input', {bubbles: true}))")
        d.frames(3)
        listed = js("return [...document.querySelectorAll('[data-rule-item]')].map(b => b.dataset.ruleItem)")
        check(listed == ["GSM-8"], f"the search finds the rule ({listed})")
        js("const i = document.querySelector('[data-testid=rulebook-search]'); i.value = '';"
           "i.dispatchEvent(new Event('input', {bubbles: true}))")
        d.frames(2)
        # a link in a page, and back
        js("document.querySelector(\"[data-testid=rulebook] .book:nth-of-type(1)\").click()")
        d.frames(4)
        js("document.querySelector(\"[data-testid=rulebook-page] .contents [data-rule='ER-1']\").click()")
        d.frames(4)
        h = js("return document.querySelector('[data-testid=rulebook-page] h2').textContent")
        js("document.querySelector('[data-testid=rulebook] .nav[aria-label=Back]').click()")
        d.frames(4)
        back = js("return document.querySelector('[data-testid=rulebook-page] h2').textContent")
        check("ER-1" in h and back == "Modelling ER", f"a link opens its rule, Back returns ({h!r}, {back!r})")
        js("document.querySelector('[data-testid=rulebook] .x').click()")
        d.frames(3)
        check(not u.shown("[data-testid=rulebook]"), "× closes it")
        d.call("window.__erdtex.focusCanvas()")
        u.keys("Shift", "F1")
        d.frames(6)
        check(u.shown("[data-testid=rulebook]"), "Shift+F1 opens it")
        js("document.querySelector('[data-testid=rulebook] .x').click()")
        d.frames(3)
        # a problem names its rule and opens it
        d.call("window.__erdtex.load(args[0], 'er')", r"\entity{a}(0,0){A}\attributes{a}{x}")
        d.frames(4)
        rule = js("return document.querySelector('[data-testid=problems] li button.rule')?.dataset.rule")
        check(rule == "ER-1", f"the problem shows its rule ({rule!r})")
        js("document.querySelector('[data-testid=problems] li button.rule').click()")
        d.frames(8)
        h = js("return document.querySelector('[data-testid=rulebook-page] h2')?.textContent")
        check(u.shown("[data-testid=rulebook]") and "ER-1" in (h or ""), f"a click opens the rule in the rule books ({h!r})")
        open_books = js("return [...document.querySelectorAll('[data-book][aria-expanded=true]')].map(b => b.dataset.book)")
        check(open_books == ["modelling-er"], f"with its book opened in the list, the others collapsed ({open_books})")
        js("document.querySelector('[data-testid=rulebook] .x').click()")
        d.frames(3)
        # a refused connection names its rule in the status bar, as a link
        c = User(d)  # drawing on the canvas (Docks' drag moves panes)
        c.fresh()
        c.tool("entity")
        c.click(0, 0)
        c.tool("entity")
        c.click(8, 0)
        c.tool("link")
        c.drag((0, 0), (8, 0))
        d.frames(3)
        status = u.state()["status"]
        link = js("return document.querySelector('[data-testid=status] button.rule')?.dataset.rule")
        check("(DRAW-1)" in status and link == "DRAW-1", f"the status bar names the rule of a refused line ({status!r}, {link!r})")
        js("document.querySelector('[data-testid=status] button.rule').click()")
        d.frames(8)
        h = js("return document.querySelector('[data-testid=rulebook-page] h2')?.textContent")
        check("DRAW-1" in (h or ""), f"and a click opens it ({h!r})")
        js("document.querySelector('[data-testid=rulebook] .x').click()")
        d.frames(3)
        # a decision of the Relational Schema window names its rule and opens it
        d.call("window.__erdtex.load(args[0], 'er')", r"\entity{p}(0,0){Person}\attributes{p}{*SSN}\isa{g}(0,-2){p}{cd}"
               r"\entity{e}(-2,-4){Employee}\entity{s}(2,-4){Student}\link{e}{g}\link{s}{g}")
        d.frames(4)
        d.call("window.__erdtex.focusCanvas()")
        u.keys("Control", "Shift", "e")
        d.frames(8)
        rule = js("return document.querySelector('[data-testid=relational] fieldset.decision button.rule')?.dataset.rule")
        check(rule == "REL-17", f"a decision of the Relational Schema window shows its rule ({rule!r})")
        js("document.querySelector('[data-testid=relational] fieldset.decision button.rule').click()")
        d.frames(8)
        h = js("return document.querySelector('[data-testid=rulebook-page] h2')?.textContent")
        check("REL-17" in (h or ""), f"and a click opens it ({h!r})")
        js("document.querySelector('[data-testid=rulebook] .x').click()")
        d.frames(3)
        u.keys("Escape")
        d.frames(3)
        # a question of the GSM -> ER dialog names its rule and opens it
        d.call("window.__erdtex.load(args[0], 'gsm')", r"\abstract{person}(0,0){Person}\abstract{city}(5,0){City}"
               r"\attribute[bend left=25]{person}{city}{lives in}\multiattribute[bend left=25]{city}{person}{home of}")
        d.frames(4)
        d.call("window.__erdtex.run('a_convert')")
        d.frames(8)
        rule = js("return document.querySelector('[data-testid=convert] fieldset button.rule')?.dataset.rule")
        check(rule == "G2E-7", f"a question of the GSM → ER dialog shows its rule ({rule!r})")
        js("document.querySelector('[data-testid=convert] fieldset button.rule').click()")
        d.frames(8)
        h = js("return document.querySelector('[data-testid=rulebook-page] h2')?.textContent")
        check("G2E-7" in (h or ""), f"and a click opens it ({h!r})")
    finally:
        d.quit()
    print(f"({time.time() - t0:.0f} s)")
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
