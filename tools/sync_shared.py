#!/usr/bin/env python3
"""Copy the blocks marked  %% <shared:NAME> ... %% </shared:NAME>  from
erdiagram.sty into gsmdiagram.sty, renaming the internal prefixes.

Both packages must stay self-contained single files (users only copy the
.sty), so the shared machinery is duplicated; this script keeps the two
copies identical.  Edit the shared blocks in erdiagram.sty only.
"""
import re
import sys
from pathlib import Path

PKG = Path(__file__).resolve().parent.parent / "latex_packages"
BLOCK = re.compile(r"(%% <shared:(\w+)>[^\n]*\n)(.*?)(%% </shared:\2>\n)", re.S)


def er_to_gsm(text: str) -> str:
    text = text.replace(r"\inputerdiagram", r"\inputgsmdiagram")
    text = text.replace(r"\ersetup", r"\gsmsetup")
    text = text.replace("erdiagram", "gsmdiagram")
    text = text.replace("ER rule", "GSM rule")
    text = text.replace("er@", "gsm@")          # macros and internal TikZ names
    text = re.sub(r"/er\b", "/gsm", text)       # pgfkeys paths
    text = re.sub(r"\ber (note|edge)\b", r"gsm \1", text)  # TikZ style names
    return text


def main() -> int:
    er = (PKG / "erdiagram.sty").read_text()
    gsm_path = PKG / "gsmdiagram.sty"
    gsm = gsm_path.read_text()
    blocks = {m.group(2): m.group(3) for m in BLOCK.finditer(er)}
    found = set()

    def repl(m):
        found.add(m.group(2))
        return m.group(1) + er_to_gsm(blocks[m.group(2)]) + m.group(4)

    new = BLOCK.sub(repl, gsm)
    missing = set(blocks) - found
    if missing:
        print("gsmdiagram.sty lacks shared blocks:", ", ".join(sorted(missing)))
        return 1
    if new != gsm:
        gsm_path.write_text(new)
        print("gsmdiagram.sty updated:", ", ".join(sorted(found)))
    else:
        print("gsmdiagram.sty already in sync")
    return 0


if __name__ == "__main__":
    sys.exit(main())
