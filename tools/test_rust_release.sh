#!/usr/bin/env bash
# Each release build of the Rust version installed in a clean system, then its self-test
# (`erdtex --self-test`, with the window on a virtual display): the deb on Ubuntu 22.04
# and Debian 12, the rpm on Fedora; the Windows installer under Wine (installed silently, then the
# self-test without the window: Wine has no WebView2, so the window is tested on real Windows).
#
#   tools/test_rust_release.sh [deb|rpm|windows]     (all if nothing is said)
set -uo pipefail
cd "$(dirname "$0")/.."
VERSION=$(python3 -c 'import json; print(json.load(open("desktop_app_rust/src-tauri/tauri.conf.json"))["version"])')
what=${1:-all}
case $what in all|deb|rpm|windows) ;; *) echo "usage: $0 [deb|rpm|windows]" >&2; exit 2 ;; esac
failed=0

# what each system needs besides the package: a virtual display, and to see the file types
check_inside='
  set -e
  erdtex --self-test --no-window
  xvfb-run -a erdtex --self-test
  grep -q "\*\.erd" /usr/share/mime/globs2 && grep -q "\*\.gsmd" /usr/share/mime/globs2 && echo "ok    the file types are known"
  test -f /usr/share/applications/erdtex.desktop && ! grep -q NoDisplay /usr/share/applications/erdtex.desktop && echo "ok    the menu entry, named after the app id of the window"
'

run() {   # image, install commands
  echo "== $1"
  if podman run --rm -v "$PWD/dist":/dist:Z,ro "$1" bash -c "$2 && $check_inside"; then
    echo "-- $1: passed"
  else
    echo "-- $1: FAILED"; failed=1
  fi
}

if [[ $what == all || $what == deb ]]; then
  deb=/dist/ERDTeX-$VERSION-amd64.deb
  for img in docker.io/library/ubuntu:22.04 docker.io/library/debian:12; do
    run "$img" "export DEBIAN_FRONTEND=noninteractive; apt-get update -qq >/dev/null && apt-get install -y -qq $deb xvfb xauth shared-mime-info >/dev/null"
  done
fi

if [[ $what == all || $what == rpm ]]; then
  rpm=/dist/ERDTeX-$VERSION-x86_64.rpm
  run registry.fedoraproject.org/fedora:latest "dnf install -y -q $rpm xorg-x11-server-Xvfb xauth shared-mime-info >/dev/null"
fi

if [[ $what == all || $what == windows ]]; then
  # in a container with Wine (docker.io/tobix/wine:stable), as the Python release used one: a
  # Wine folder set up there takes seconds, on this computer Wine's first setup can hang
  echo "== Windows installer under Wine"
  if podman run --rm -v "$PWD/dist":/dist:Z,ro -e VERSION="$VERSION" docker.io/tobix/wine:stable bash -c '
    set -e
    export WINEDEBUG=-all WINEDLLOVERRIDES="mscoree=;mshtml="
    wine wineboot -i >/dev/null 2>&1; wineserver -w
    # Wine has no WebView2 and the installer would try to download it: marked as present here
    # (on real Windows the installer fetches it where it is missing)
    wine reg add "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" \
      /v pv /t REG_SZ /d 130.0.0.0 /f >/dev/null 2>&1
    timeout 300 wine "Z:\\dist\\ERDTeX-$VERSION-setup.exe" /S >/dev/null 2>&1; wineserver -w
    exe=$(find ~/.wine/drive_c -iname erdtex.exe | head -1)
    test -n "$exe" && echo "ok    installed at ${exe#*/drive_c/}"
    timeout 120 wine "$exe" --self-test --no-window
    wine reg query "HKCU\\Software\\Classes\\.erd" 2>/dev/null | grep -qi . && echo "ok    .erd is associated"
    wine reg query "HKCU\\Software\\Classes\\.gsmd" 2>/dev/null | grep -qi . && echo "ok    .gsmd is associated"
    lnk=$(find ~/.wine/drive_c/users -path "*Start Menu/Programs/ERDTeX/*.lnk" | head -1)
    test -n "$lnk" && echo "ok    the Start menu entry, in a folder of its own: ${lnk##*Programs/}"
    unins=$(find "$(dirname "$exe")" -iname "uninstall.exe" | head -1)
    timeout 120 wine "$unins" /S >/dev/null 2>&1; wineserver -w; sleep 2
    test ! -e "$exe" && echo "ok    uninstalled"
  '; then
    echo "-- Windows: passed"
  else
    echo "-- Windows: FAILED"; failed=1
  fi
fi

exit $failed
