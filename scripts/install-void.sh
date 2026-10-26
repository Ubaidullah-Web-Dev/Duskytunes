#!/usr/bin/env bash
# Install or update Duskytunes on Void Linux from the latest GitHub release.
# Releases only ship an Arch package, so this repacks it as an .xbps and installs that.
# Once installed, Settings > About > Update does the same thing from inside the app.
set -euo pipefail

repo=Ubaidullah-Web-Dev/Duskytunes
here=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

command -v xbps-create >/dev/null || { echo 'This script is for Void Linux.' >&2; exit 2; }
command -v bsdtar >/dev/null || sudo xbps-install -Sy bsdtar

depends=$(grep -v '^#' "$here/packaging/void/depends" | sed '/^$/d; s/$/>=0/' | paste -sd' ')
tag=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" |
  sed -n 's/.*"tag_name": *"\(v[0-9.]*\)".*/\1/p' | head -n1)
[[ -n $tag ]] || { echo 'Could not find the latest release.' >&2; exit 1; }
ver=${tag#v}
pkg=duskytunes-$ver-1-x86_64.pkg.tar.zst

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
curl -fsSLO "https://github.com/$repo/releases/download/$tag/$pkg"
curl -fsSLO "https://github.com/$repo/releases/download/$tag/$pkg.sha256"
sha256sum -c "$pkg.sha256"

mkdir root repo
bsdtar -xf "$pkg" -C root usr
rm -rf root/usr/lib/systemd # no user systemd on Void; `duskytunes` starts the daemon itself
libs=$(ldd root/usr/bin/* 2>&1 || true)
if grep -q 'version `GLIBC_.*not found' <<<"$libs"; then
  echo "$tag needs a newer glibc than this system has." >&2
  exit 1
fi

(cd repo && xbps-create -q -A x86_64 -n "duskytunes-${ver}_1" -l GPL-3.0-or-later \
  -H "https://github.com/$repo" -s 'Dusky music client, playback daemon, and sync server' \
  -D "$depends" ../root)
xbps-rindex -a repo/*.xbps

up=()
xbps-query duskytunes >/dev/null 2>&1 && up=(-u)
sudo xbps-install -Sy "${up[@]}" --repository="$work/repo" duskytunes
echo "Installed Duskytunes $ver. Quit Duskytunes (duskytunes-cli quit) and open it again."
