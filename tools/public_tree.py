#!/usr/bin/env python3
"""The public repository's files: what of this repository goes public, and a check that nothing private
or from course material comes along (website plan, W2 and W23).

    tools/public_tree.py --list           the files that go public
    tools/public_tree.py --export DIR     write them into DIR (an empty folder), with public/ laid over
    tools/public_tree.py --check          export into a temporary folder and check it:
                                            no excluded file, no course words, no personal details,
                                            no paths of this computer, no secrets (gitleaks), and
                                            not the program's old name

The files come from git (tracked files only). In the public repository itself (no `desktop_app/`), every
tracked file is checked. `public/` holds the public versions of files that differ
from this repository's (the root README, .gitignore, ...): they replace or add to the tree.
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# what goes public: these folders and files (prefixes of tracked paths) ...
INCLUDE = [
    "LICENSE",
    "desktop_app_rust/",
    "latex_packages/",
    "tools/build_rust_release.sh",
    "tools/check_rust.sh",
    "tools/check_packages.sh",
    "tools/check_sql.py",
    "tools/licenses.sh",
    "tools/public_tree.py",
    "tools/release_github.sh",
    "tools/release_notes.md",
    "tools/rust_release/",
    "tools/sync_shared.py",
    "tools/test_rust_release.sh",
]
# ... except these (prefixes)
EXCLUDE = [
    "desktop_app_rust/oracle/",           # the comparison with the Python app, which stays private
    "desktop_app_rust/crates/oracle/",
    "desktop_app_rust/e2e/python_canvas.py",
    "latex_packages/tests/v1/",           # converted course diagrams
    "latex_packages/tests/v1_files.tex",
]
# files of the public tree that are changed on the way: path -> [(old, new)]
PATCHES = {
    # the workspace without the comparison's crate
    "desktop_app_rust/Cargo.toml": [('"crates/oracle", ', "")],
}

# words that must not appear (case-insensitive, whole words)
DENIED_WORDS = [
    r"slides?", r"lectures?", r"Vorlesung\w*", r"Übung\w*", r"exercises?", r"Exercise\w*",
    r"Exemplar", r"DB1", r"CDM", r"script \d", r"gmail",
]
# ... and words this file mustn't name, since it is public itself (names, places): one per line in
# tools/public_tree_private.txt (never exported; untracked in the public repository), or in the
# environment variable PUBLIC_TREE_DENIED (an Actions secret there), one per line or comma-separated
PRIVATE_WORDS = ROOT / "tools/public_tree_private.txt"


def private_words() -> list[str]:
    text = os.environ.get("PUBLIC_TREE_DENIED", "")
    if PRIVATE_WORDS.is_file():
        text += "\n" + PRIVATE_WORDS.read_text()
    words = [w.strip() for w in text.replace(",", "\n").splitlines()]
    return [re.escape(w) for w in words if w and not w.startswith("#")]

# where a denied word is fine: the key old files contain, its values in the code
ALLOWED = [
    r"rm rules=db1", r"rm rules=cdm", r'"db1"', r'"cdm"', r"'db1'", r"Some\(\"db1\"",
    r"slide outwards",          # erdiagram.sty: a label slides along its edge
    r'contains\("slide"\)',     # the rule books' test that refuses the word
    r"`db1`",
]
# denied with their case: exercise sheets' names (UE03), not WebDriver's key codes (\ue009)
DENIED_CASED = [r"UE\d\d\w*"]
# files only checked for secrets and paths (generated, third-party texts)
TEXT_ONLY = ["desktop_app_rust/licenses/", "latex_packages/LICENSE", "desktop_app_rust/fonts/GUST-FONT-LICENSE.txt"]
# the program's old name (up to 3.5), in any spelling, in names and contents (also of binary files), and
# its abbreviation in names of the code (DS_CONFIG_DIR, window.__ds): nothing public says it (2026-11-14)
OLD_NAME = re.compile(rb"diagram[\s_-]?studio", re.IGNORECASE)
OLD_ABBREVIATION = re.compile(r"\bDS_[A-Z]|__ds\b|\bds-[a-z]")
BINARY = {".pdf", ".otf", ".ttf", ".woff", ".woff2", ".png", ".ico", ".icns", ".jpg", ".wasm"}


def tracked() -> list[str]:
    out = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True).stdout
    return [p for p in out.decode().split("\0") if p]


def public_files() -> list[str]:
    if not (ROOT / "desktop_app").exists():   # the public repository itself: all of it
        return [p for p in tracked() if (ROOT / p).is_file()]

    def wanted(p: str) -> bool:
        return any(p == i or (i.endswith("/") and p.startswith(i)) for i in INCLUDE) and not any(
            p.startswith(e) for e in EXCLUDE)
    files = [p for p in tracked() if wanted(p) and (ROOT / p).is_file()]
    overlay = [str(p.relative_to(ROOT / "public")) for p in (ROOT / "public").rglob("*") if p.is_file()] \
        if (ROOT / "public").is_dir() else []
    return sorted(set(files) | set(overlay))


def export(dest: Path) -> list[str]:
    files = public_files()
    for p in files:
        src = ROOT / "public" / p if (ROOT / "public" / p).is_file() else ROOT / p
        (dest / p).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest / p)
    private = (ROOT / "desktop_app").exists()   # the public repository is already patched
    for p, changes in (PATCHES.items() if private else []):
        f = dest / p
        text = f.read_text()
        for old, new in changes:
            if old not in text:
                sys.exit(f"public_tree: patch for {p} no longer applies: {old!r}")
            text = text.replace(old, new)
        f.write_text(text)
    return files


def check(dest: Path, files: list[str]) -> list[str]:
    problems = []
    words = re.compile(r"(?<![\w-])(?:" + "|".join(DENIED_WORDS + private_words()) + r")(?![\w-])", re.IGNORECASE)
    cased = re.compile(r"(?<![\w-])(?:" + "|".join(DENIED_CASED) + r")(?![\w-])")
    allowed = re.compile("|".join(ALLOWED))
    private = re.compile(r"(?<![\w-])(?:" + "|".join(private_words() or [r"(?!)"]) + r")(?![\w-])", re.IGNORECASE)
    home = str(Path.home())
    for p in files:
        if any(p.startswith(e) for e in EXCLUDE):
            problems.append(f"{p}: an excluded file")
        f = dest / p
        if p != "tools/public_tree.py" and (OLD_NAME.search(p.encode()) or OLD_NAME.search(f.read_bytes())):
            problems.append(f"{p}: the program's old name")
        if f.suffix.lower() in BINARY:
            data = f.read_bytes()
            if home.encode() in data:
                problems.append(f"{p}: contains this computer's home folder")
            continue
        try:
            text = f.read_text()
        except UnicodeDecodeError:
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if home in line:
                problems.append(f"{p}:{n}: this computer's home folder")
            if any(p.startswith(t) for t in TEXT_ONLY):
                continue
            if p == "tools/public_tree.py":   # its own lists of words: only the private ones are refused
                problems += [f"{p}:{n}: a private word" for _ in private.finditer(line)]
                continue
            if OLD_ABBREVIATION.search(line):
                problems.append(f"{p}:{n}: the old name's abbreviation: {line.strip()[:120]}")
            rest = allowed.sub("", line)
            for m in [*words.finditer(rest), *cased.finditer(rest)]:
                problems.append(f"{p}:{n}: '{m.group(0)}': {line.strip()[:120]}")
    leaks = shutil.which("gitleaks") or str(Path.home() / ".local/bin/gitleaks")
    if Path(leaks).exists():
        r = subprocess.run([leaks, "detect", "--no-git", "--source", str(dest), "--no-banner", "--redact",
                            "--exit-code", "3"], capture_output=True, text=True)
        if r.returncode == 3:
            problems.append("gitleaks found secrets:\n" + (r.stdout + r.stderr)[-2000:])
        elif r.returncode != 0:
            problems.append(f"gitleaks failed: {(r.stdout + r.stderr)[-500:]}")
    else:
        problems.append("gitleaks is missing (go install github.com/zricethezav/gitleaks/v8@v8.21.2)")
    return problems


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--list", action="store_true")
    g.add_argument("--export", metavar="DIR")
    g.add_argument("--check", action="store_true")
    a = ap.parse_args()
    if a.list:
        print("\n".join(public_files()))
        return 0
    if a.export:
        dest = Path(a.export)
        if dest.exists() and any(dest.iterdir()):
            sys.exit(f"{dest} is not empty")
        dest.mkdir(parents=True, exist_ok=True)
        print(f"{len(export(dest))} files written to {dest}")
        return 0
    with tempfile.TemporaryDirectory() as tmp:
        files = export(Path(tmp))
        problems = check(Path(tmp), files)
    for p in problems:
        print(p)
    print(f"{len(files)} files, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
