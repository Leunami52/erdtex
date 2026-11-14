#!/usr/bin/env bash
# The LaTeX packages: every test document (latex_packages/tests/*.tex) and both manuals compile without
# an error. Built in a temporary copy, so the tracked PDFs stay as they are.
#
#   tools/check_packages.sh
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cp -r "$root/latex_packages" "$tmp/"
mkdir -p "$tmp/desktop_app_rust"                     # the manuals show the program's sample diagrams
cp -r "$root/desktop_app_rust/samples" "$tmp/desktop_app_rust/"
pk="$tmp/latex_packages"
status=0
build() {  # dir file runs
  local dir="$1" file="$2" runs="$3" n
  ( cd "$dir" || exit 1
    for _ in $(seq "$runs"); do
      TEXINPUTS="$pk:" pdflatex -interaction=nonstopmode "$file" > /dev/null 2>&1
    done )
  n=$(awk '/^!/' "$dir/${file%.tex}.log" 2>/dev/null | wc -l)
  if [ "$n" = 0 ] && [ -s "$dir/${file%.tex}.pdf" ]; then
    printf '%-50s ok\n' "${dir#"$tmp"/}/$file"
  else
    printf '%-50s ERRORS: %s\n' "${dir#"$tmp"/}/$file" "$n"
    awk '/^!/{c=3} c&&c--' "$dir/${file%.tex}.log" | head -12
    status=1
  fi
}
# a test needing a LaTeX package that isn't installed is skipped (figchild, for one, is fairly new)
missing() {  # file.tex -> the packages it loads that TeX can't find
  sed -n 's/^[^%]*\\usepackage\(\[[^]]*\]\)\{0,1\}{\([^}]*\)}.*/\2/p' "$1" | tr ',' '\n' | while read -r p; do
    p=${p// /}
    case $p in erdiagram|gsmdiagram|"") ;; *) kpsewhich "$p.sty" > /dev/null || echo "$p" ;; esac
  done
}
for t in "$pk"/tests/*.tex; do
  m=$(missing "$t" | tr '\n' ' ')
  if [ -n "$m" ]; then printf '%-50s skipped (not installed: %s)\n' "latex_packages/tests/$(basename "$t")" "$m"; continue; fi
  build "$pk/tests" "$(basename "$t")" 1
done
build "$pk/manuals" erdiagram-manual.tex 3
build "$pk/manuals" gsmdiagram-manual.tex 3
exit $status
