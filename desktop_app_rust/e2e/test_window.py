#!/usr/bin/env python3
"""The window, driven like a user would (tauri-driver + WebKitWebDriver): every diagram of the
repository opens and is saved unchanged, undo and redo, unsaved changes, refused files, a file
open twice, the display modes, and the tabs coming back at the next start.

    npx tauri build --no-bundle          # the app with the page in it
    python3 e2e/test_window.py [OUT]     # OUT: where the screenshots go (default: target/e2e)

The app gets a settings folder of its own (ERDTEX_CONFIG_DIR), and works on copies of the files.
"""
import shutil
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from driver import Driver  # noqa: E402

RUST = HERE.parent
REPO = RUST.parent
APP = RUST / "target" / "release" / "erdtex"
SKIP = {".git", "build", "dist", "target", "node_modules"}

failures: list[str] = []


def check(ok: bool, what: str) -> None:
    print(("ok    " if ok else "FAIL  ") + what)
    if not ok:
        failures.append(what)


def diagrams() -> list[Path]:
    return sorted(p for p in REPO.rglob("*") if p.suffix in (".erd", ".gsmd")
                  and not SKIP & set(p.relative_to(REPO).parts))


def start(d: Driver, expr: str, *args) -> None:
    """Starts something that asks a question, without waiting for it to finish."""
    d.js(f"const args = arguments; {expr}; return 1", *args)


def state(d: Driver) -> dict:
    return d.call("window.__erdtex.state()")


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else RUST / "target" / "e2e"
    shutil.rmtree(out, ignore_errors=True)
    tmp = Path(tempfile.mkdtemp(prefix="erdtex-e2e-"))
    cfg = tmp / "config"
    env = {"ERDTEX_CONFIG_DIR": str(cfg)}
    d = Driver(APP, env)
    try:
        d.call("window.__erdtex.setMode('light')")
        files = diagrams()
        # -- every diagram opens, is drawn, and is saved unchanged ----------------------
        for k, src in enumerate(files):
            copy = tmp / "files" / str(k) / src.name
            copy.parent.mkdir(parents=True)
            shutil.copy(src, copy)
            rel = src.relative_to(REPO)
            ok = d.call("window.__erdtex.open(args[0])", str(copy))
            d.frames(3)
            st = state(d)
            tab = st["tabs"][st["current"]]
            check(ok and tab["path"] == str(copy.resolve()) and not tab["modified"] and st["question"] is None,
                  f"opens {rel}")
            d.screenshot(out / "rust" / f"{k:02d}_{src.stem}.png", "[data-testid=canvas]")
            d.call("window.__erdtex.save()")
            check(copy.read_bytes() == src.read_bytes(), f"saved unchanged: {rel}")
        check(len(state(d)["tabs"]) == len(files), f"one tab per file ({len(files)})")
        # -- a file that's open is shown, not opened again --------------------------------
        first = tmp / "files" / "0" / files[0].name
        d.call("window.__erdtex.open(args[0])", str(first))
        st = state(d)
        check(len(st["tabs"]) == len(files) and st["current"] == 0, "opening an open file shows its tab")
        # -- undo and redo, also with the keys -----------------------------------------------
        er = next(k for k, f in enumerate(files) if f.suffix == ".erd" and "show types" not in f.read_text())
        d.call("window.__erdtex.open(args[0])", str(tmp / "files" / str(er) / files[er].name))
        before = d.call("window.__erdtex.text()")
        d.call("window.__erdtex.run('a_types')")
        st = state(d)
        check("\\setup{show types}" in d.call("window.__erdtex.text()") and st["tabs"][st["current"]]["modified"],
              "Show data types changes the diagram")
        d.keys("Control", "z")
        st = state(d)
        check(d.call("window.__erdtex.text()") == before and not st["tabs"][st["current"]]["modified"], "Ctrl+Z undoes it")
        d.keys("Control", "Shift", "z")
        check("\\setup{show types}" in d.call("window.__erdtex.text()"), "Ctrl+Shift+Z redoes it")
        d.keys("Control", "y")
        d.keys("Control", "z")
        check(d.call("window.__erdtex.text()") == before, "Ctrl+Y does nothing more, Ctrl+Z undoes again")
        d.call("window.__erdtex.run('a_types')")
        # -- unsaved changes are asked about ------------------------------------------------------
        start(d, "window.__erdtex.run('a_close_tab')")
        d.wait("return !!window.__erdtex.state().question")
        q = state(d)["question"]
        check("has unsaved changes" in q, f"closing asks: {q!r}")
        d.call("window.__erdtex.answer('Cancel')")
        check(len(state(d)["tabs"]) == len(files), "Cancel keeps the tab")
        start(d, "window.__erdtex.run('a_close_tab')")
        d.wait("return !!window.__erdtex.state().question")
        d.call("window.__erdtex.answer('Discard')")
        d.frames()
        check(len(state(d)["tabs"]) == len(files) - 1, "Discard closes it")
        check((tmp / "files" / str(er) / files[er].name).read_bytes() == files[er].read_bytes(), "the file stays as it was")
        # -- refused files ------------------------------------------------------------------------
        notes = tmp / "notes.txt"
        notes.write_text("just some text\n")
        start(d, "window.__erdtex.open(args[0])", str(notes))
        d.wait("return !!window.__erdtex.state().question")
        q = state(d)["question"]
        check("is not a diagram" in q, f"a text file is refused: {q!r}")
        d.call("window.__erdtex.answer('OK')")
        gsm_copy = tmp / "x.gsmd"
        shutil.copy(next(f for f in files if f.suffix == ".gsmd"), gsm_copy)
        d.call("window.__erdtex.open(args[0])", str(tmp / "files" / "0" / files[0].name))  # an ER diagram with a file
        start(d, "window.__erdtex.saveAs(args[0])", str(gsm_copy))
        d.wait("return !!window.__erdtex.state().question")
        q = state(d)["question"]
        check("is a GSM diagram file" in q, f"an ER diagram isn't saved over a GSM file: {q!r}")
        d.call("window.__erdtex.answer('OK')")
        check(gsm_copy.read_bytes() == next(f for f in files if f.suffix == ".gsmd").read_bytes(), "the GSM file stays as it was")
        # -- display modes ------------------------------------------------------------------------
        air = next(k for k, f in enumerate(files) if f.name == "railway.erd")
        d.call("window.__erdtex.open(args[0])", str(tmp / "files" / str(air) / "railway.erd"))
        for mode in ("light", "dark", "intermediate", "system"):
            d.call("window.__erdtex.setMode(args[0])", mode)
            d.frames(3)
            d.screenshot(out / "modes" / f"{mode}.png")
        check(all((out / "modes" / f"{m}.png").exists() for m in ("light", "dark", "intermediate", "system")),
              "the four display modes")
        tabs_before = [t["path"] for t in state(d)["tabs"] if t["path"]]
        start(d, "window.__erdtex.run('a_quit')")
        for _ in range(100):                     # until the window has closed (settings written)
            try:
                d.js("return 1")
                time.sleep(0.1)
            except Exception:  # noqa: BLE001
                break
    finally:
        d.quit()
    # -- the tabs come back at the next start ------------------------------------------------
    d = Driver(APP, env)
    try:
        d.frames(3)
        st = state(d)
        back = [t["path"] for t in st["tabs"]]
        check(back == tabs_before, f"{len(tabs_before)} tabs come back at the next start"
              + ("" if back == tabs_before else f" (got {len(back)})"))
    finally:
        d.quit()
    shutil.rmtree(tmp, ignore_errors=True)
    print(f"\n{len(failures)} failed" if failures else "\nall passed")
    print(f"screenshots in {out}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
