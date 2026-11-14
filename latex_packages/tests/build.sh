#!/usr/bin/env bash
# usage: build.sh file.tex [dpi]
# Compiles a test against the packages in .., prints errors and warnings and
# renders PNGs to $IMG (default: build/test_png in the repository root).
set -u
f="${1%.tex}"; dpi="${2:-80}"
cd "$(dirname "$0")"
IMG="${IMG:-$(cd ../.. && pwd)/build/test_png}"
mkdir -p "$IMG"
TEXINPUTS=..: pdflatex -interaction=nonstopmode -halt-on-error "$f.tex" > /dev/null
grep -E '^!|Warning: ER|Warning: GSM|erdiagram Warning|gsmdiagram Warning|Overfull' -A2 "$f.log" | head -40
rm -f "$IMG/$f"-*.png
pdftoppm -png -r "$dpi" "$f.pdf" "$IMG/$f" 2>/dev/null
ls "$IMG" | grep "^$f-" | head
