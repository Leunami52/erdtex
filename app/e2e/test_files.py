#!/usr/bin/env python3
"""The LaTeX preview, export and files with the real window (tauri-driver + WebKitWebDriver):
the Python app's test_export.py, test_recent.py, the file-watching parts of test_tabs.py, Copy
\\input Line, and taking over the Python app's settings.  Native save dialogs are answered through
the test hook (`savePathOnce`).

    npx tauri build --no-bundle
    python3 e2e/test_files.py
"""
import shutil
import struct
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402
from test_editing import APP, check, failures  # noqa: E402
from test_dialogs import User  # noqa: E402

REPO = HERE.parent.parent
TEX = shutil.which("pdflatex") is not None
CAIRO = shutil.which("pdftocairo") is not None


def wait(cond, timeout=40.0, step=0.25):
    end = time.time() + timeout
    while time.time() < end:
        v = cond()
        if v:
            return v
        time.sleep(step)
    return cond()


def preview_done(u, runs_before):
    st = u.d.call("window.__erdtex.preview()")
    return st if st and st["runs"] > runs_before and st["status"] != "Compiling…" else None


def preview(u: User, tmp: Path):
    if not TEX:
        print("(no pdflatex: the preview is skipped)")
        return
    shutil.copy(REPO / "app/test_diagrams/railway.erd", tmp / "railway.erd")
    check(not u.shown("[data-testid=preview]"), "the preview starts hidden, as in the Python app")
    u.keys("F6")
    st0 = u.d.call("window.__erdtex.preview()")
    u.d.call("window.__erdtex.open(args[0])", str(tmp / "railway.erd"))
    st = wait(lambda: (lambda s: s if s["image"] and s["image"][0] > 1000 and s["status"] == "Up to date." else None)(
        u.d.call("window.__erdtex.preview()")))
    check(bool(st), f"the preview shows the diagram as LaTeX sets it ({st})")
    good = st["image"] if st else None
    runs = st["runs"] if st else st0["runs"]
    u.d.call("window.__erdtex.select(['train'])")
    u.d.frames(2)
    u.d.js("document.querySelector('[data-row=label] textarea').focus()")
    u.keys("Control", "a")
    u.d.type("Flight \\undefinedmacro\n")
    st = wait(lambda: preview_done(u, runs))
    check(bool(st) and st["status"].startswith("LaTeX reported errors") and any(e.startswith("line ") for e in st["errors"]),
          f"an error is reported with its line ({st and st['errors']})")
    check(bool(st) and st["image"] == good, "and the last good result stays")
    runs = st["runs"] if st else runs
    u.keys("Control", "z")
    st = wait(lambda: preview_done(u, runs))
    check(bool(st) and st["status"] == "Up to date." and not st["errors"], "undo: up to date again")
    runs = st["runs"] if st else runs
    u.press_css("[data-testid=preview-grid]")
    st = wait(lambda: preview_done(u, runs))
    check(bool(st) and st["grid"] and st["image"] != good, f"with the package's grid ({st and st['image']})")
    u.press_css("[data-testid=preview-grid]")
    # several changes during a run: one more run, with the newest text
    runs = u.d.call("window.__erdtex.preview()")["runs"]
    for k in range(4):
        u.d.call("window.__erdtex.load(args[0], 'er')", f"\\entity{{e}}(0,0){{V{k}}}\n")
        time.sleep(0.15)
    st = wait(lambda: preview_done(u, runs + 1))
    check(bool(st) and st["status"] == "Up to date.", "changes during a run: the newest one counts")
    u.keys("F6")
    check(not u.shown("[data-testid=preview]"), "F6 hides the preview")
    u.keys("F6")
    check(u.shown("[data-testid=preview]"), "and shows it again")


def png_size(path: Path):
    b = path.read_bytes()
    return struct.unpack(">II", b[16:24]), b[25]


def magic(path: Path, fmt: str) -> bool:
    if not path.exists() or path.stat().st_size < 1000:
        return False
    b = path.read_bytes()
    return {"pdf": b.startswith(b"%PDF"), "png": b.startswith(b"\x89PNG"), "svg": b"<svg" in b[:2000]}[fmt]


def export(u: User, tmp: Path):
    shutil.copy(REPO / "app/test_diagrams/railway.erd", tmp / "export.erd")
    u.d.call("window.__erdtex.open(args[0])", str(tmp / "export.erd"))
    u.d.frames(3)
    sources = ["editor"] + (["latex"] if TEX else [])
    for source in sources:
        for fmt in ("png", "pdf", "svg"):
            if source == "latex" and fmt == "svg" and not CAIRO:
                continue
            out = tmp / f"{source}.{fmt}"
            u.d.call("window.__erdtex.savePathOnce(args[0])", str(out))
            u.keys("Control", "e")
            u.press_css(f"[data-testid=export-{source}]")
            u.select_in("[data-testid=export-format]", fmt)
            if fmt == "png":
                u.d.js("const i = document.querySelector('[data-testid=export-dpi]'); i.value = '150';"
                       "i.dispatchEvent(new Event('input', {bubbles: true}))")
            u.press_css("[data-testid=export-ok]")
            wait(lambda: u.state()["status"] == f"Exported {out}" or u.state()["question"], 60)
            q = u.state()["question"]
            if q:
                u.press_css("[data-testid=question] button")
            check(magic(out, fmt) and not q, f"export: the {source} result as {fmt.upper()} ({q or out.stat().st_size if out.exists() else 'missing'})")
    (w, _), _ = png_size(tmp / "editor.png")
    check(1000 < w < 1250, f"the editor rendering at 150 dpi: about 18.7 cm ({w} px)")
    s = u.settings()
    check(s.get("export/format") == "svg" and s.get("export/dpi") == "150", "the choices are kept")


def input_line(u: User, tmp: Path):
    u.d.call("window.__erdtex.open(args[0])", str(tmp / "export.erd"))
    u.d.call("window.__erdtex.focusCanvas()")
    u.keys("Control", "Shift", "c")
    status = u.state()["status"]
    check(status == "Copied: \\inputerdiagram[max width=\\linewidth]{export.erd}", f"Copy \\input Line ({status})")


def recent_and_refused(u: User, tmp: Path):
    files = {"a.erd": "\\entity{a}(0,0){A}\n", "new.gsmd": "", "B.ERD": "\\entity{a}(0,0){A}\n",
             "c.tex": "\\entity{a}(0,0){A}\n", "notes.txt": "[Rules]\ncount=1\n"}
    for name, text in files.items():
        (tmp / name).write_text(text)
    for name in ("a.erd", "new.gsmd", "B.ERD", "c.tex"):
        u.d.call("window.__erdtex.open(args[0])", str(tmp / name))
    u.d.js("window.__erdtex.open(arguments[0]); return 1", str(tmp / "notes.txt"))   # waits for the message
    q = wait(lambda: u.state()["question"], 5) or ""
    check("not a diagram" in q, f"a text file is refused ({q[:60]})")
    if q:
        u.press_css("[data-testid=question] button")
    check((tmp / "notes.txt").read_text() == files["notes.txt"], "and left as it was")
    rec = u.d.call("window.__erdtex.recent()")
    want = [str(tmp / n) for n in ("c.tex", "B.ERD", "new.gsmd", "a.erd")]
    check(rec[:4] == want and str(tmp / "notes.txt") not in rec, f"Open Recent lists the diagrams, newest first ({rec[:4]})")


def watching(u: User, tmp: Path):
    a, b = tmp / "wa.erd", tmp / "wb.erd"
    a.write_text("\\entity{a}(0,0){A}\n")
    b.write_text("\\entity{b}(0,0){B}\n")
    u.d.call("window.__erdtex.open(args[0])", str(a))
    u.d.call("window.__erdtex.open(args[0])", str(b))
    time.sleep(1.0)
    a.write_text("\\entity{n}(0,0){Changed}\n")
    ok = wait(lambda: any(t["path"] == str(a) for t in u.state()["tabs"]) and "Reloaded" in (u.state()["status"] or ""), 8)
    st = u.state()
    check(bool(ok) and st["tabs"][st["current"]]["path"] == str(b), f"a file changed on disk is reloaded in its tab; b stays in front ({st['status']})")
    check("Changed" not in u.text(), "the tab in front is the other one")
    # with unsaved changes: asked first
    u.d.call("window.__erdtex.open(args[0])", str(a))
    check("Changed" in u.text(), "the reloaded text")
    u.d.call("window.__erdtex.select(['n'])")
    u.d.frames(2)
    u.keys("Delete")
    time.sleep(1.0)
    a.write_text("\\entity{m}(0,0){Again}\n")
    q = wait(lambda: u.state()["question"], 8) or ""
    check("changed on disk" in q, f"with unsaved changes, it asks ({q[:50]})")
    if q:
        u.press_css("[data-testid=question] button")      # Yes
        u.d.frames(2)
    check("Again" in u.text(), "and reloads on Yes")


def dialogs(u: User, tmp: Path):
    """The file dialogs, answered by the test: what each one is asked with."""
    def ask(*answers):
        u.d.call("window.__erdtex.answerDialogs(args[0])", list(answers))

    asked = lambda: u.d.call("window.__erdtex.dialogsAsked()")
    tabs = lambda: [t["path"] for t in u.state()["tabs"]]
    filters = lambda a: [f["name"] for f in a["filters"]]
    # Open takes several files, each in a tab of its own
    one, two = tmp / "several1.erd", tmp / "several2.gsmd"
    one.write_text("\\entity{a}(0,0){A}\n")
    two.write_text("\\abstract{p}(0,0){P}\n")
    ask([str(one), str(two)])
    u.d.call("window.__erdtex.run('a_open')")
    wait(lambda: str(two) in tabs(), 10)
    a = asked()
    check(str(one) in tabs() and str(two) in tabs(), f"Open: several files at once ({tabs()[-2:]})")
    check(a and a[0]["multiple"] and filters(a[0]) == ["Diagram files", "ER diagrams", "GSM diagrams", "All files"],
          f"Open offers every kind ({a})")
    # Save As offers only the diagram's own kind, and Open starts where it saved
    project = tmp / "project"
    project.mkdir()
    u.d.call("window.__erdtex.run('a_new_gsm')")
    ask(str(project / "aspects"))
    u.d.call("window.__erdtex.run('a_save_as')")
    wait(lambda: (project / "aspects.gsmd").exists(), 10)
    a = asked()
    check((project / "aspects.gsmd").exists() and a and filters(a[0]) == ["GSM diagrams", "All files"],
          f"Save As of a GSM diagram offers GSM diagrams only ({a and filters(a[0])})")
    u.d.call("window.__erdtex.run('a_new_er')")
    ask(None)
    u.d.call("window.__erdtex.run('a_open')")
    a = asked()
    check(a and a[0].get("defaultPath") == str(project), f"Open starts where Save As saved ({a and a[0].get('defaultPath')})")

    # Export: next to the diagram first, then where this diagram's last export went
    def export_asked(answer):
        ask(answer)
        u.keys("Control", "e")
        u.press_css("[data-testid=export-editor]")
        u.select_in("[data-testid=export-format]", "png")
        u.press_css("[data-testid=export-ok]")
        if answer:
            wait(lambda: u.state()["status"] == f"Exported {answer}", 30)
        a = asked()
        return str(Path(a[0]["defaultPath"]).parent) if a else None

    figures = tmp / "figures"
    figures.mkdir()
    u.d.call("window.__erdtex.open(args[0])", str(one))
    first = export_asked(str(figures / "several1.png"))
    check(first == str(tmp), f"Export starts next to the diagram ({first})")
    again = export_asked(None)
    check(again == str(figures), f"then where this diagram's last export went ({again})")
    u.d.call("window.__erdtex.open(args[0])", str(project / "aspects.gsmd"))
    other = export_asked(None)
    check(other == str(project), f"another diagram starts next to itself ({other})")


def examples(u: User, tmp: Path):
    """File → Open Example: each sample in a tab of its own, not saved anywhere; unchanged it closes without
    asking, once edited it counts as changed."""
    tab = lambda: (lambda s: s["tabs"][s["current"]])(u.state())  # noqa: E731
    before = len(u.state()["tabs"])
    u.d.call("window.__erdtex.run('a_example_er')")
    wait(lambda: tab()["name"] == "festival.erd", 10)
    t = tab()
    check(t["name"] == "festival.erd" and t["kind"] == "er" and t["path"] is None and not t["modified"],
          f"File → Open Example → ER Diagram: the sample, unsaved and unchanged ({t})")
    check("\\entity{person}" in u.text() and "\\subdomain" in u.text(), "with its content")
    u.d.call("window.__erdtex.run('a_example_er')")
    u.d.frames(3)
    check(len(u.state()["tabs"]) == before + 1, "opening it again shows the open one")
    u.d.call("window.__erdtex.run('a_example_gsm')")
    wait(lambda: tab()["name"] == "festival.gsmd", 10)
    check(tab()["kind"] == "gsm" and "\\abstract{person}" in u.text(), "GSM Diagram: the GSM sample")
    u.d.call("window.__erdtex.run('a_close_tab')")
    u.d.frames(3)
    check(not u.state()["question"] and len(u.state()["tabs"]) == before + 1, "unchanged, it closes without asking")
    u.d.call("window.__erdtex.run('a_example_er')")   # back to the ER sample
    wait(lambda: tab()["name"] == "festival.erd", 10)
    u.d.call("window.__erdtex.select(['person'])")
    u.d.frames(2)
    # Edit → Delete (a key could go to a field earlier parts left focused)
    u.d.js("[...document.querySelectorAll('.menubar .title')].find(b => b.textContent === 'Edit').click()")
    u.d.frames(2)
    u.d.js("[...document.querySelectorAll('.menubar .row')].find(b => b.querySelector('.label').textContent === 'Delete').click()")
    u.d.frames(3)
    check(tab()["modified"], "an edited sample counts as changed")
    u.d.js("void window.__erdtex.run('a_close_tab')")   # it waits for the answer
    q = wait(lambda: u.state()["question"], 5) or ""
    check("unsaved changes" in q, f"and asks before closing ({q[:40]!r})")
    if q:
        u.d.js("[...document.querySelectorAll('[data-testid=question] button')].find(b => b.textContent === 'Discard').click()")
        u.d.frames(3)


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-files-"))
    d = Driver(APP, {"ERDTEX_CONFIG_DIR": str(tmp / "config"), "ERDTEX_PRIVATE_CLIPBOARD": "1"})
    u = User(d)
    t0 = time.time()
    try:
        d.frames(3)
        d.call("window.__erdtex.setMode('light')")
        for part in (preview, export, input_line, recent_and_refused, watching, dialogs, examples):
            print(f"-- {part.__name__}")
            part(u, tmp)
    finally:
        d.quit()
    print(f"({time.time() - t0:.0f} s)")
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
