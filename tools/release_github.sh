#!/usr/bin/env bash
# A release on GitHub (Leunami52/erdtex): the deb, the rpm and the Windows installer from dist/ (built and
# tested with tools/build_rust_release.sh and tools/test_rust_release.sh), and the LaTeX packages as a
# zip, with the notes of tools/release_notes.md. The version is the program's (tauri.conf.json), the tag
# vX.Y.Z; the release's text comes first from NEWS.md (what's new, in Markdown).
#
#   tools/release_github.sh NEWS.md [--draft]       the program and the packages, tag vX.Y.Z
#   tools/release_github.sh NEWS.md --packages      only the packages, tag packages-vA.B.C
#
# The tag must be pushed already for a published release (Actions builds the website from it); a draft
# may name a tag that doesn't exist yet. Website plan, W18.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
repo=Leunami52/erdtex
news="${1:?usage: tools/release_github.sh NEWS.md [--draft|--packages]}"
mode="${2:-}"
[ -f "$news" ] || { echo "$news: not found" >&2; exit 1; }
version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' \
  "$root/app/src-tauri/tauri.conf.json")
packages=$(sed -n 's/.*\\ProvidesPackage{erdiagram}\[[^]]* v\([0-9.]*\) .*/\1/p' "$root/latex_packages/erdiagram.sty")
[ -n "$packages" ] || { echo "the packages' version wasn't found in erdiagram.sty" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
zip="$tmp/latex-packages-$packages.zip"
python3 - "$root/latex_packages" "$zip" <<'EOF'
import sys, zipfile
from pathlib import Path
src, out = Path(sys.argv[1]), sys.argv[2]
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for name in ["erdiagram.sty", "gsmdiagram.sty", "README.md", "LICENSE",
                 "manuals/erdiagram-manual.pdf", "manuals/gsmdiagram-manual.pdf"]:
        z.write(src / name, Path(name).name)
EOF

if [ "$mode" = --packages ]; then
  tag="packages-v$packages"
  title="LaTeX packages $packages"
  { cat "$news"; printf '\nThe LaTeX packages `erdiagram` and `gsmdiagram` %s with their manuals: keep the `.sty`\nfiles next to your document. The program is unchanged.\n' "$packages"; } > "$tmp/notes.md"
  files=("$zip")
else
  tag="v$version"
  title="ERDTeX $version"
  files=("$root/dist/ERDTeX-$version-amd64.deb" "$root/dist/ERDTeX-$version-x86_64.rpm"
         "$root/dist/ERDTeX-$version-setup.exe" "$zip")
  for f in "${files[@]}"; do [ -f "$f" ] || { echo "$f: missing (tools/build_rust_release.sh)" >&2; exit 1; }; done
  python3 - "$root/tools/release_notes.md" "$news" "$version" "$packages" > "$tmp/notes.md" <<'EOF'
import sys
from pathlib import Path
template, news, version, packages = sys.argv[1:]
print(Path(template).read_text().replace("{{NEWS}}", Path(news).read_text().strip())
      .replace("{{VERSION}}", version).replace("{{PACKAGES}}", packages), end="")
EOF
fi

# a draft names the tag it will create on main when published; a release needs the pushed tag
if [ "$mode" = --draft ]; then
  where=(--draft --target main)
else
  where=(--verify-tag)
fi
gh release create "$tag" "${files[@]}" --repo "$repo" --title "$title" --notes-file "$tmp/notes.md" "${where[@]}"
gh release view "$tag" --repo "$repo" --json url,isDraft,assets --jq '.url, .isDraft, (.assets[] | .name)'
