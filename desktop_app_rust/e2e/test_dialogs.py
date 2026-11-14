#!/usr/bin/env python3
"""Preferences, GSM -> ER and the Relational Schema / SQL window with real clicks and keys
(tauri-driver + WebKitWebDriver): the window parts of the Python app's test_preferences.py,
test_gsm2er.py, test_relational_window.py and test_focus.py.  Drop-down lists are chosen
through the page (their popup is the system's).

    npx tauri build --no-bundle
    python3 e2e/test_dialogs.py
"""
import json
import shutil
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP, check, failures  # noqa: E402
from test_panels import Panels  # noqa: E402

EXAMPLES = json.loads((HERE.parent / "crates/core/tests/data/rm_examples.json").read_text())
REPO = HERE.parent.parent


class User(Panels):
    def select_in(self, css, value):
        self.d.js("const s = document.querySelector(arguments[0]); s.value = arguments[1];"
                  "s.dispatchEvent(new Event('change', {bubbles: true}));", css, value)
        self.d.frames(3)

    def settings(self) -> dict:
        return self.d.call("window.__erdtex.settings()")

    def shown(self, css) -> bool:
        return self.d.js("return !!document.querySelector(arguments[0])", css)

    def prefs_ok(self):
        """OK, and wait: it stores every page, then closes the dialog (or a message says why not)."""
        self.press_css("[data-testid=prefs-ok]")
        for _ in range(50):
            if not self.shown("[data-testid=preferences]") or self.state()["question"]:
                return
            time.sleep(0.1)

    def prefs(self, page=None):
        self.keys("Control", ",")
        if page is not None:
            self.press_css(f"[data-testid=preferences] [data-page='{page}']")


def preferences(u: User):
    u.load(r"\entity{p}(0,0){P}\attributes{p}{*k}")
    u.prefs()
    check(u.shown("[data-testid=preferences]"), "Ctrl+, opens Preferences")
    pages = u.d.js("return [...document.querySelectorAll('[data-page]')].map(b => b.textContent)")
    check(pages == ["Appearance", "Keyboard shortcuts", "New ER diagrams", "New GSM diagrams", "Editor", "Output"],
          f"the pages in order ({pages})")
    u.d.call("window.__erdtex.tool('entity')")
    u.press_css("[data-page='0']")
    u.keys("Escape")
    check(not u.shown("[data-testid=preferences]") and u.state()["tool"] == "entity",
          "Esc closes the dialog and leaves the tool (the dialog keeps its keys)")

    # shortcuts
    u.prefs(1)
    u.press_css("[data-testid=shortcuts] [data-id='er.entity']")
    u.press_css("[data-testid=shortcut-capture]")
    u.keys("Control", "s")
    msg = u.d.js("return document.querySelector('[data-testid=shortcut-message]').textContent")
    check("Ctrl+S" in msg and "Save" in msg, f"a conflict is refused and named ({msg})")
    u.keys("Control", "k")
    row = u.d.js("return document.querySelector(\"[data-id='er.entity'] .keys\").textContent")
    check(row == "Ctrl+K", f"a new combination ({row})")
    u.d.js("const i = document.querySelector('[data-testid=shortcut-search]'); i.value = 'subtype';"
           "i.dispatchEvent(new Event('input', {bubbles: true}))")
    u.d.frames(2)
    ids = u.d.js("return [...document.querySelectorAll('[data-testid=shortcuts] [data-id]')].map(b => b.dataset.id)")
    check(ids == ["gsm.subtype"], f"search ({ids})")
    u.press_css("[data-testid=prefs-cancel]")
    check(u.settings().get("shortcuts/er.entity") is None, "Cancel changes nothing")
    u.prefs(1)
    u.press_css("[data-testid=shortcuts] [data-id='er.entity']")
    u.press_css("[data-testid=shortcut-capture]")
    u.keys("Control", "k")
    u.prefs_ok()
    check(u.settings().get("shortcuts/er.entity") == "Ctrl+K", "OK stores it")
    u.d.call("window.__erdtex.focusCanvas()")
    u.keys("Control", "k")
    check(u.state()["tool"] == "entity", "and it works at once")
    u.keys("Escape")

    # new ER diagrams
    u.prefs(2)
    ph = u.d.js("return document.querySelector(\"[data-testid=defaults-er] input[data-key='entity fill']\").placeholder")
    check(ph == "gray!15", f"the packages' default in grey ({ph})")
    u.select_in("[data-testid=defaults-er] select[data-key='isa shape']", "diamond")
    u.press_css("[data-testid=defaults-er] input[data-key='entity fill']")
    u.d.type("blue!20")
    u.press_css("[data-testid=defaults-er] input[data-key='attr spread']")
    u.d.type("wide")
    u.prefs_ok()
    q = u.state()["question"] or ""
    check("attr spread" in q, f"a wrong value stops OK and says why ({q[:60]})")
    u.press_css("[data-testid=question] button")
    check(u.settings().get("new diagram/er/attr spread") is None, "nothing is stored")
    u.press_css("[data-testid=defaults-er] input[data-key='attr spread']")
    u.keys("Control", "a")
    u.keys("Backspace")
    before = u.text()
    u.press_css("[data-testid=use-er]")
    text = u.text()
    check("isa shape=diamond" in text and "entity fill=blue!20" in text and r"\entity{p}" in text,
          "Use for the open diagram")
    u.prefs_ok()
    s = u.settings()
    check(any(k.endswith("isa shape") and v == "diamond" for k, v in s.items()), "OK stores the defaults")
    u.keys("Control", "z")
    check(u.text() == before, "the open diagram: one step undone")
    u.d.call("window.__erdtex.fresh('er')")
    u.d.frames(2)
    check("entity fill=blue!20" in u.text(), "a new ER diagram gets them")

    # editor and output
    u.prefs(4)
    u.select_in("[data-testid=pref-snap]", "0.5")
    u.press_css("[data-testid=pref-grid]")
    u.press_css("[data-testid=pref-hints]")
    u.press_css("[data-page='5']")
    u.select_in("[data-testid=pref-rm-format]", "sqlite")
    u.press_css("[data-testid=pref-rm-drop]")
    u.prefs_ok()
    s = u.settings()
    check(s.get("snap") == "0.5" and s.get("show grid") == "false" and s.get("show hints") == "false",
          f"the Editor page ({s.get('snap')}, {s.get('show grid')}, {s.get('show hints')})")
    check(u.state()["snap"] == 0.5 and u.d.js("return document.querySelector('[data-testid=snap]').value") == "0.5",
          "the toolbar follows")
    check(s.get("rm/format") == "sqlite" and s.get("rm/drop") == "true", "the Output page")
    u.prefs()
    shown = u.d.js("return document.querySelector('[data-page].on').textContent")
    check(shown == "Output", f"the last page is shown again ({shown})")
    u.press_css("[data-page='0']")
    u.select_in("[data-testid=pref-mode]", "dark")
    u.select_in("[data-testid=pref-language]", "german")
    u.prefs_ok()
    s = u.settings()
    check(s.get("display mode") == "dark" and s.get("label language") == "german", "the Appearance page")
    bg = u.d.js("return getComputedStyle(document.body).getPropertyValue('--window').trim()")
    check(bg == "#2b2b2b", f"dark at once ({bg})")


def relational(u: User):
    u.load(EXAMPLES["EX"]["gen_cd"])
    u.keys("Control", "Shift", "e")
    check(u.shown("[data-testid=relational]"), "Ctrl+Shift+E opens the Relational Schema / SQL window")
    fmt = u.d.js("return document.querySelector('[data-testid=rm-format]').value")
    drop = u.d.js("return document.querySelector('[data-testid=rm-drop]').checked")
    check(fmt == "sqlite" and drop, "it starts with the stored choices (Output page)")
    u.select_in("[data-testid=rm-format]", "text")
    out = lambda: u.d.js("return document.querySelector('[data-testid=rm-output]').value")  # noqa: E731
    check("category" in out(), "the schema (a disjoint generalisation: categorised)")
    check(not u.d.js("return !!document.querySelector('[data-testid=rm-rules]')"), "no rule set to choose")
    u.load("\\setup{rm rules=db1}\n" + EXAMPLES["EX"]["gen_cd"])
    check("category" not in out().split("Foreign keys")[0], "an old file's rm rules=db1 keeps its result (not categorised)")
    u.load(EXAMPLES["EX"]["gen_cd"])
    # Markdown: keys by emphasis, with a note; LaTeX: a note that it needs the package
    note = lambda: u.d.js("return document.querySelector('[data-testid=rm-note]')?.textContent || ''")  # noqa: E731
    u.select_in("[data-testid=rm-format]", "markdown")
    md = out()
    check(md.startswith("## Relational schema") and "**" in md and "Markdown has no underline" in md,
          f"the schema in Markdown, with its legend ({md[:40]!r})")
    check("can't underline" in note(), f"and a note in the window ({note()[:40]!r})")
    u.select_in("[data-testid=rm-format]", "latex")
    check("erdiagram" in note() and u.shown("[data-testid=rm-save-packages]"),
          f"LaTeX: a note that it needs the erdiagram package, with Save LaTeX Packages… ({note()[:40]!r})")
    folder = Path(tempfile.mkdtemp(prefix="erdtex-packages-"))
    names = ["erdiagram-manual.pdf", "erdiagram.sty", "gsmdiagram-manual.pdf", "gsmdiagram.sty"]
    u.d.call("window.__erdtex.answerDialogs(args[0])", [str(folder)])
    u.press_css("[data-testid=rm-save-packages]")
    end = time.time() + 15
    while time.time() < end and sorted(p.name for p in folder.iterdir()) != names:
        time.sleep(0.25)
    saved = sorted(p.name for p in folder.iterdir())
    sty = (folder / "erdiagram.sty").read_text() if (folder / "erdiagram.sty").exists() else ""
    check(saved == names and "\\ProvidesPackage{erdiagram}" in sty
          and (folder / "erdiagram-manual.pdf").read_bytes()[:5] == b"%PDF-", f"the packages and manuals saved ({saved})")
    (folder / "gsmdiagram.sty").write_text("changed")
    u.d.call("window.__erdtex.answerDialogs(args[0])", [str(folder)])
    u.press_css("[data-testid=rm-save-packages]")
    end = time.time() + 10
    while time.time() < end and not u.state()["question"]:
        time.sleep(0.25)
    q = u.state()["question"] or ""
    check("already in that folder" in q, f"saving again asks before replacing ({q[:60]!r})")
    if q:
        u.press_css("[data-testid=question] button")      # Replace
        end = time.time() + 10
        while time.time() < end and (folder / "gsmdiagram.sty").read_text() == "changed":
            time.sleep(0.25)
    check((folder / "gsmdiagram.sty").read_text().startswith("%"), "and replaces them on Replace")
    shutil.rmtree(folder, ignore_errors=True)
    u.d.call("window.__erdtex.run('a_about')")
    end = time.time() + 10
    while time.time() < end and not u.state()["question"]:
        time.sleep(0.25)
    q = u.state()["question"] or ""
    check("Built-in LaTeX packages: erdiagram 3." in q and "gsmdiagram 3." in q and "Leunami52" in q,
          f"About names the built-in packages and the copyright ({q[:80]!r})")
    if q:
        u.d.js("document.querySelectorAll('[data-testid=question] button')[1].click()")   # Licences…
        end = time.time() + 10
        while time.time() < end and not u.shown("[data-testid=licenses]"):
            time.sleep(0.25)
        check(u.shown("[data-testid=licenses]"), "About → Licences… opens the licences")
        texts = {}
        for section in ("program", "packages", "fonts", "rust", "javascript"):
            u.d.js(f"document.querySelector(\"[data-testid=licenses] [data-section='{section}']\").click()")
            end = time.time() + 5
            while time.time() < end and (u.d.js("return document.querySelector('[data-testid=licenses-text]').textContent") or "").startswith("Loading"):
                time.sleep(0.25)
            texts[section] = u.d.js("return document.querySelector('[data-testid=licenses-text]').textContent") or ""
        check("MIT License" in texts["program"] and "Leunami52" in texts["program"], "the program's licence (MIT)")
        check("LaTeX Project Public License" in texts["packages"], "the packages' (LPPL)")
        check("GUST Font License" in texts["fonts"], "the fonts' (GUST)")
        check("Used by:" in texts["rust"] and "tauri" in texts["rust"], "the Rust crates' licences")
        check("svelte" in texts["javascript"] and "webcola" in texts["javascript"], "the JavaScript libraries' licences, workers' included")
        u.press_css("[data-testid=licenses-close]")

    u.load(EXAMPLES["DE"]["begleiten_ternary"])
    radios = u.d.js("return document.querySelectorAll('[data-testid=relational] input[type=radio]').length")
    check(radios == 4, f"the decision's four options ({radios})")
    result = u.d.js("return document.querySelector('[data-testid=relational] .result').textContent")
    check(" : {(" in result, f"each with its relations ({result[:40]!r})")
    u.press_css("[data-testid=relational] input[data-option='3']")
    check(any(ln.startswith(r"\relationship[rm=table") for ln in u.text().splitlines()), "choosing an option writes it into the diagram")
    u.keys("Control", "z")
    check("rm=table" not in u.text(), "undone")

    u.load(EXAMPLES["EX"]["hosts_1m"])
    check(u.d.js("return document.querySelectorAll('[data-testid=relational] input[type=radio]').length") == 0,
          "optional alternatives are collapsed")
    u.press_css("[data-testid=rm-minor]")
    u.press_css("[data-testid=relational] input[data-option='1']")
    check(u.d.js("return document.querySelector('[data-testid=rm-minor]').getAttribute('aria-pressed')") == "true"
          and "hosts" in out(), "and stay open while one is used")

    u.load(r"\entity{p}(0,0){Person}\attributes{p}{*PID}")
    check("Person" in out(), "the window follows the diagram")
    u.d.call("window.__erdtex.select(['p'])")
    u.d.frames(2)
    u.d.js("document.querySelector('[data-row=label] textarea').focus()")   # under the window here
    u.keys("Control", "a")
    u.d.type("Human\n")
    u.d.frames(3)
    check("Human" in out(), "also changes made in the panel")
    u.press_css("[data-testid=relational] [role=tab]:nth-child(2)")
    u.press_css("[data-testid=relational] input[data-row='0']")
    u.keys("Control", "a")
    u.d.type("bigint\n")
    u.d.frames(3)
    check("PID : bigint" in u.text(), "a data type set in the window")
    u.load(EXAMPLES["DE"]["mosaik_minmax"])
    u.press_css("[data-testid=relational] [role=tab]:nth-child(3)")
    probs = u.d.js("return document.querySelector('[data-testid=rm-problems]').textContent")
    check("Not enforced" in probs and "at least 4" in probs, "unenforced bounds in the Problems tab")
    u.press_css("[data-testid=relational] .x")
    check(not u.shown("[data-testid=relational]"), "Close")


def convert(u: User, tmp: Path):
    gsm = tmp / "aspects.gsmd"
    shutil.copy(REPO / "desktop_app_rust/test_diagrams/ensemble.gsmd", gsm)
    before = gsm.read_bytes()
    u.d.call("window.__erdtex.open(args[0])", str(gsm))
    u.d.frames(3)
    u.d.call("window.__erdtex.run('a_convert')")
    u.d.frames(3)
    check(u.shown("[data-testid=convert]"), "the GSM -> ER dialog")
    svg = u.d.js("return document.querySelector('[data-testid=convert] svg').innerHTML.length")
    check(svg > 1000, f"with a preview ({svg})")
    report = lambda: u.d.js("return document.querySelector('[data-testid=convert-report]').textContent")  # noqa: E731
    check("key Person ID added" in report(), "the assumptions are listed")
    u.d.js("document.querySelectorAll('[data-testid=convert] input[type=radio]')[1].click()")
    u.d.frames(3)
    checked = u.d.js("return [...document.querySelectorAll('[data-testid=convert] input[type=radio]')].map(r => r.checked)")
    check(checked[:2] == [False, True], f"a choice converts again, and stays chosen ({checked[:2]})")
    u.select_in("[data-testid=convert-names]", "type")
    check(u.settings().get("gsm2er/attr_names") == "type", "the attribute names are remembered")
    tabs = len(u.state()["tabs"])
    u.press_css("[data-testid=convert-ok]")
    st = u.state()
    tab = st["tabs"][st["current"]]
    check(len(st["tabs"]) == tabs + 1 and tab["kind"] == "er" and tab["modified"] and tab["name"] == "aspects_er.erd",
          f"a new, unsaved ER tab named after the GSM file ({tab})")
    check("diamond" in u.text(), "with the defaults of new ER diagrams (isa shape from Preferences)")
    check(gsm.read_bytes() == before, "the GSM file is untouched")


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-dialogs-"))
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = User(d)
    t0 = time.time()
    try:
        d.frames(3)
        d.call("window.__erdtex.setMode('light')")
        for part in (preferences, relational):
            print(f"-- {part.__name__}")
            part(u)
        print("-- convert")
        convert(u, tmp)
    finally:
        d.quit()
    print(f"({time.time() - t0:.0f} s)")
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
