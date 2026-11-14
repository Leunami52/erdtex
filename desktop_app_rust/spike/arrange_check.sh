#!/usr/bin/env bash
# The check harness for arranging (the root README, "Arranging a diagram automatically"): every
# diagram of the repository arranged as the app does it (the core's graph, the page's layout in
# Node.js, the core choosing the best candidate by its figures), the result compiled by LaTeX, and the
# figures before and after.
#
#     spike/arrange_check.sh [anew]      -> target/arrange/<name>.{erd,png}, a table on stdout
set -uo pipefail
cd "$(dirname "$0")/.."
REPO=$(cd .. && pwd)
MODE=${1:-tidy}
OUT=target/arrange/$MODE
mkdir -p "$OUT"
source "$HOME/.cargo/env"
cargo build -q -p diagram-core --features json --example arrange_check
H=target/debug/examples/arrange_check
printf "%-34s %-26s %s\n" diagram chosen "overlaps / close / through / crossings / length / size: before -> after"
for f in "$REPO"/desktop_app_rust/test_diagrams/*.erd "$REPO"/desktop_app_rust/test_diagrams/*.gsmd; do
  [ -f "$f" ] || continue
  name=$(basename "$(dirname "$f")")_$(basename "$f"); name=${name/test_diagrams_/}
  ext=${f##*.}
  $H graph "$f" > "$OUT/$name.graph.json"
  node --experimental-strip-types --no-warnings spike/arrange_check.mjs "$OUT/$name.graph.json" "$MODE" > "$OUT/$name.cands.json" 2> "$OUT/$name.node.log"
  $H best "$f" "$OUT/$name.cands.json" "$OUT/$name" > "$OUT/$name.figures.json" || { echo "$name: best failed"; continue; }
  python3 - "$OUT/$name.figures.json" "$name" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
rows = {r["name"]: r["figures"] for r in d["rows"]}
o, b = rows["original"], rows[d["chosen"]]
fmt = lambda f: f"{f['overlaps']}/{f['close']}/{f['through']}/{f['crossings']}/{f['length']:.0f}/{f['width']:.0f}x{f['height']:.0f}" + (f" out {f['moved_out']}" if f.get('moved_out') else "") + (f" boxes {f['box_overlaps']}" if f.get('box_overlaps') else "")
print(f"{sys.argv[2]:34} {d['chosen']:26} {fmt(o):24} -> {fmt(b)}")
PY
  # the result compiles in LaTeX (with the app's default preamble), as the original does
  if [ "$ext" = erd ]; then pkg=erdiagram; cmd=inputerdiagram; else pkg=gsmdiagram; cmd=inputgsmdiagram; fi
  compile() {  # compile WRAPPER DIAGRAM
    printf '\\documentclass[border=6pt]{standalone}\n\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n\\usepackage{%s}\n\\begin{document}\n\\%s{%s}\n\\end{document}\n' "$pkg" "$cmd" "$2" > "$OUT/$1.tex"
    # the current directory first: latex_packages/tests/v1/ holds files of the same names
    (cd "$OUT" && TEXINPUTS=".:$REPO/latex_packages//:" pdflatex -interaction=batchmode -halt-on-error "$1.tex" > /dev/null 2>&1)
  }
  # (not "$name.tex": TeX would read that one for \input{$name})
  if compile "latex-$name" "$(cd "$OUT" && pwd)/$name"; then
    pdftocairo -png -singlefile -r 60 "$OUT/latex-$name.pdf" "$OUT/$name.result"
  elif compile "original-$name" "$f"; then
    echo "   LaTeX failed: $name"
  else
    echo "   (LaTeX fails for the original as well: $name)"
  fi
done
