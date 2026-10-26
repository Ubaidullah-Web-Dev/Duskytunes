#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-$ROOT/dist}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

VERSION="$(sed -n 's/^version = "\([0-9.]*\)"/\1/p' "$(dirname "$0")/../Cargo.toml" | head -1)"
PKGREL="1"
PKGNAME="duskytunes-v2.4"
BINNAME="duskytunes-v2.4"
SYNCNAME="duskytunes-v2.4-sync"
ICONNAME="duskytunes-v2.4"
LIBDIR="/usr/lib/duskytunes-v2.4"
SHAREDIR="/usr/share/duskytunes-v2.4"
HOOKNAME="99-duskytunes-v2.4-replacement.hook"
INSTALLSCRIPT="duskytunes-v2.4.install"

RYO="$ROOT/target/release/duskytunes"
SYNC="$ROOT/target/release/duskytunes-sync"
[[ -x "$RYO" ]] || { echo "missing release binary: $RYO" >&2; exit 1; }
[[ -x "$SYNC" ]] || { echo "missing sync binary: $SYNC" >&2; exit 1; }
for cmd in makepkg bsdtar zstd zip sha256sum; do
  command -v "$cmd" >/dev/null || { echo "missing command: $cmd" >&2; exit 1; }
done

WORK="$(mktemp -d -t duskytunes-v2.4-package-XXXXXX)"
trap 'rm -rf "$WORK"' EXIT
PKGWORK="$WORK/package"
mkdir -p "$PKGWORK"

install -Dm755 "$RYO" "$PKGWORK/duskytunes"
cat > "$PKGWORK/$BINNAME" <<'WRAP'
#!/usr/bin/env bash
set -e
exec /usr/lib/duskytunes-v2.4/duskytunes "$@"
WRAP
chmod 755 "$PKGWORK/$BINNAME"
install -Dm755 "$SYNC" "$PKGWORK/$SYNCNAME"

cp "$ROOT/packaging/linux/duskytunes.desktop" "$PKGWORK/duskytunes.desktop"
sed -i -e 's|^Exec=.*|Exec=duskytunes|' -e "s|^Icon=.*|Icon=$ICONNAME|" "$PKGWORK/duskytunes.desktop"
grep -q '^X-Duskytunes-Replacement=true$' "$PKGWORK/duskytunes.desktop" || printf '%s
' 'X-Duskytunes-Replacement=true' >> "$PKGWORK/duskytunes.desktop"

cp "$ROOT/LICENSE" "$PKGWORK/LICENSE"
cp "$ROOT/README.md" "$PKGWORK/README.md"
cp "$ROOT/RELEASE_NOTES.md" "$PKGWORK/RELEASE_NOTES.md"
cp "$ROOT/UPSTREAM.md" "$PKGWORK/UPSTREAM.md"
cp "$ROOT/integrations/quickshell/DuskytunesBarWidget.qml" "$PKGWORK/DuskytunesBarWidget.qml"
cp "$ROOT/integrations/quickshell/README.md" "$PKGWORK/QUICKSHELL_README.md"
cp "$ROOT/packaging/dusky/duskytunes-window-rule.lua" "$PKGWORK/duskytunes-window-rule.lua"
cp "$ROOT/src-tauri/icons/32x32.png" "$PKGWORK/icon32.png"
cp "$ROOT/src-tauri/icons/64x64.png" "$PKGWORK/icon64.png"
cp "$ROOT/src-tauri/icons/128x128.png" "$PKGWORK/icon128.png"
cp "$ROOT/src-tauri/icons/128x128@2x.png" "$PKGWORK/icon256.png"
cp "$ROOT/src-tauri/icons/icon.png" "$PKGWORK/icon512.png"

cat > "$PKGWORK/activate-replacement" <<'ACTIVATE'
#!/usr/bin/env bash
set -euo pipefail
LIBDIR=/usr/lib/duskytunes-v2.4
REALBIN="$LIBDIR/duskytunes"
TEMPLATE=/usr/share/duskytunes-v2.4/duskytunes.desktop
TARGET_BIN=/usr/bin/duskytunes
TARGET_DESKTOP=/usr/share/applications/duskytunes.desktop
STATE=/var/lib/duskytunes-v2.4
BACKUP="$STATE/stock"

[[ -x "$REALBIN" ]] || { echo "Duskytunes replacement binary is missing: $REALBIN" >&2; exit 1; }
[[ -f "$TEMPLATE" ]] || { echo "Duskytunes desktop template is missing: $TEMPLATE" >&2; exit 1; }
mkdir -p "$BACKUP"

# When v2.4 is installed on top of an older custom package, the live /usr/bin and desktop entry
# are custom too, so they must never be mistaken for stock. Adopt the genuine stock backup that
# the previous replacement package already captured before that package is removed.
adopt_previous_backup() {
  local prev
  for prev in     /var/lib/duskytunes-v2.3/stock     /var/lib/duskytunes-v2.2/stock     /var/lib/duskytunes-v2.1/stock     /var/lib/duskytunes-v2.0/stock     /var/lib/duskytunes-v1.9/stock     /var/lib/duskytunes-v1.8/stock     /var/lib/duskytunes-v1.7/stock     /var/lib/duskytunes-v1.6/stock     /var/lib/duskytunes-v1.5/stock     /var/lib/duskytunes-v1.4/stock; do
    if [[ ! -e "$BACKUP/duskytunes" && ! -L "$BACKUP/duskytunes" ]]        && [[ -e "$prev/duskytunes" || -L "$prev/duskytunes" ]]; then
      cp -a "$prev/duskytunes" "$BACKUP/duskytunes"
    fi
    if [[ ! -f "$BACKUP/duskytunes.desktop" && -f "$prev/duskytunes.desktop" ]]; then
      cp -a "$prev/duskytunes.desktop" "$BACKUP/duskytunes.desktop"
    fi
    if [[ ( -e "$BACKUP/duskytunes" || -L "$BACKUP/duskytunes" )        && -f "$BACKUP/duskytunes.desktop" ]]; then
      break
    fi
  done
}
adopt_previous_backup

is_known_custom_bin() {
  local resolved
  resolved="$(readlink -f "$TARGET_BIN" 2>/dev/null || true)"
  [[ "$resolved" == "$REALBIN" ]] || [[ "$resolved" =~ ^/usr/lib/duskytunes-v(1\.[4-9]|2\.[0-3]|2[0-4])/duskytunes$ ]]
}
is_custom_desktop() {
  [[ -f "$TARGET_DESKTOP" ]] && grep -q '^X-Duskytunes-Replacement=true$' "$TARGET_DESKTOP"
}

if [[ -e "$TARGET_BIN" || -L "$TARGET_BIN" ]] && ! is_known_custom_bin; then
  rm -f "$BACKUP/duskytunes"
  cp -a "$TARGET_BIN" "$BACKUP/duskytunes"
fi
if [[ -f "$TARGET_DESKTOP" ]] && ! is_custom_desktop; then
  cp -a --remove-destination "$TARGET_DESKTOP" "$BACKUP/duskytunes.desktop"
fi

rm -f "$TARGET_BIN"
ln -s "$REALBIN" "$TARGET_BIN"
install -Dm644 "$TEMPLATE" "$TARGET_DESKTOP"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
ACTIVATE
chmod 755 "$PKGWORK/activate-replacement"

cat > "$PKGWORK/deactivate-replacement" <<'DEACTIVATE'
#!/usr/bin/env bash
set -euo pipefail
REALBIN=/usr/lib/duskytunes-v2.4/duskytunes
TARGET_BIN=/usr/bin/duskytunes
TARGET_DESKTOP=/usr/share/applications/duskytunes.desktop
BACKUP=/var/lib/duskytunes-v2.4/stock

is_our_bin() {
  [[ -L "$TARGET_BIN" ]] && [[ "$(readlink -f "$TARGET_BIN" 2>/dev/null || true)" == "$REALBIN" ]]
}
is_our_desktop() {
  [[ -f "$TARGET_DESKTOP" ]]     && grep -q '^X-Duskytunes-Replacement=true$' "$TARGET_DESKTOP"     && grep -q '^Icon=duskytunes-v2.4$' "$TARGET_DESKTOP"
}

if is_our_bin; then
  rm -f "$TARGET_BIN"
  [[ -e "$BACKUP/duskytunes" || -L "$BACKUP/duskytunes" ]] && cp -a "$BACKUP/duskytunes" "$TARGET_BIN"
elif [[ ! -e "$TARGET_BIN" && ! -L "$TARGET_BIN" && ( -e "$BACKUP/duskytunes" || -L "$BACKUP/duskytunes" ) ]]; then
  cp -a "$BACKUP/duskytunes" "$TARGET_BIN"
fi

if is_our_desktop; then
  rm -f "$TARGET_DESKTOP"
  [[ -f "$BACKUP/duskytunes.desktop" ]] && cp -a "$BACKUP/duskytunes.desktop" "$TARGET_DESKTOP"
elif [[ ! -f "$TARGET_DESKTOP" && -f "$BACKUP/duskytunes.desktop" ]]; then
  cp -a "$BACKUP/duskytunes.desktop" "$TARGET_DESKTOP"
fi
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
DEACTIVATE
chmod 755 "$PKGWORK/deactivate-replacement"

cat > "$PKGWORK/$HOOKNAME" <<'HOOK'
[Trigger]
Operation = Install
Operation = Upgrade
Type = Path
Target = usr/bin/duskytunes
Target = usr/share/applications/duskytunes.desktop

[Action]
Description = Re-activating Duskytunes v2 after a Dusky package update
When = PostTransaction
Exec = /usr/lib/duskytunes-v2.4/activate-replacement
HOOK

cat > "$PKGWORK/$INSTALLSCRIPT" <<'INSTALL'
post_install() {
  /usr/lib/duskytunes-v2.4/activate-replacement || return 1
}
post_upgrade() {
  /usr/lib/duskytunes-v2.4/activate-replacement || return 1
}
pre_remove() {
  /usr/lib/duskytunes-v2.4/deactivate-replacement || true
}
post_remove() {
  rm -rf /var/lib/duskytunes-v2.4
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
}
INSTALL

cat > "$PKGWORK/PKGBUILD" <<'PKG'
pkgname=duskytunes-v2.4
pkgver=$VERSION
pkgrel=1
pkgdesc='Duskytunes v2 - Dusky replacement Linux desktop music client'
arch=('x86_64')
url='https://github.com/Ubaidullah-Web-Dev/Duskytunes'
license=('GPL-3.0-or-later')
depends=('webkit2gtk-4.1' 'libappindicator-gtk3' 'mpv' 'openssl' 'librsvg' 'desktop-file-utils' 'hicolor-icon-theme' 'xdg-utils')
provides=('duskytunes=$VERSION')
conflicts=('duskytunes-v2.3' 'duskytunes-v2.2' 'duskytunes-v2.1' 'duskytunes-v2.0')
install='duskytunes-v2.4.install'
source=('duskytunes-v2.4' 'duskytunes' 'duskytunes-v2.4-sync' 'duskytunes.desktop' 'activate-replacement' 'deactivate-replacement' '99-duskytunes-v2.4-replacement.hook' 'duskytunes-v2.4.install' 'LICENSE' 'README.md' 'RELEASE_NOTES.md' 'UPSTREAM.md' 'DuskytunesBarWidget.qml' 'QUICKSHELL_README.md' 'icon32.png' 'icon64.png' 'icon128.png' 'icon256.png' 'icon512.png' 'duskytunes-window-rule.lua')
sha256sums=('SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP')
package() {
  install -Dm755 duskytunes-v2.4 "$pkgdir/usr/bin/duskytunes-v2.4"
  install -Dm755 duskytunes "$pkgdir/usr/lib/duskytunes-v2.4/duskytunes"
  install -Dm755 duskytunes-v2.4-sync "$pkgdir/usr/bin/duskytunes-v2.4-sync"
  install -Dm755 activate-replacement "$pkgdir/usr/lib/duskytunes-v2.4/activate-replacement"
  install -Dm755 deactivate-replacement "$pkgdir/usr/lib/duskytunes-v2.4/deactivate-replacement"
  install -Dm644 duskytunes.desktop "$pkgdir/usr/share/duskytunes-v2.4/duskytunes.desktop"
  install -Dm644 99-duskytunes-v2.4-replacement.hook "$pkgdir/usr/share/libalpm/hooks/99-duskytunes-v2.4-replacement.hook"
  install -Dm644 DuskytunesBarWidget.qml "$pkgdir/usr/share/duskytunes-v2.4/quickshell/DuskytunesBarWidget.qml"
  install -Dm644 QUICKSHELL_README.md "$pkgdir/usr/share/doc/duskytunes-v2.4/QUICKSHELL.md"
  install -Dm644 duskytunes-window-rule.lua "$pkgdir/usr/share/duskytunes-v2.4/duskytunes-window-rule.lua"
  install -Dm644 icon32.png "$pkgdir/usr/share/icons/hicolor/32x32/apps/duskytunes-v2.4.png"
  install -Dm644 icon64.png "$pkgdir/usr/share/icons/hicolor/64x64/apps/duskytunes-v2.4.png"
  install -Dm644 icon128.png "$pkgdir/usr/share/icons/hicolor/128x128/apps/duskytunes-v2.4.png"
  install -Dm644 icon256.png "$pkgdir/usr/share/icons/hicolor/256x256/apps/duskytunes-v2.4.png"
  install -Dm644 icon512.png "$pkgdir/usr/share/icons/hicolor/512x512/apps/duskytunes-v2.4.png"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/duskytunes-v2.4/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/duskytunes-v2.4/README.md"
  install -Dm644 RELEASE_NOTES.md "$pkgdir/usr/share/doc/duskytunes-v2.4/RELEASE_NOTES.md"
  install -Dm644 UPSTREAM.md "$pkgdir/usr/share/doc/duskytunes-v2.4/UPSTREAM.md"
}
PKG
(cd "$PKGWORK" && makepkg --clean --cleanbuild --noconfirm)
PKGFILE="$(find "$PKGWORK" -maxdepth 1 -name "$PKGNAME-$VERSION-$PKGREL-*.pkg.tar.zst" -print -quit)"
[[ -n "$PKGFILE" ]] || { echo "package creation failed" >&2; exit 1; }

LIST="$WORK/package-contents.txt"
bsdtar -tf "$PKGFILE" > "$LIST"
for path in   'usr/bin/duskytunes-v2.4'   'usr/lib/duskytunes-v2.4/duskytunes'   'usr/share/duskytunes-v2.4/duskytunes.desktop'   'usr/share/duskytunes-v2.4/duskytunes-window-rule.lua'   'usr/share/libalpm/hooks/99-duskytunes-v2.4-replacement.hook'; do
  grep -qx "$path" "$LIST" || { echo "missing package path: $path" >&2; exit 1; }
done
if grep -Eq '^usr/bin/duskytunes$|^usr/share/applications/duskytunes(-v[^/]*)?\.desktop$' "$LIST"; then
  echo "replacement package claims a Dusky-owned launcher path" >&2
  exit 1
fi

cp "$PKGFILE" "$OUT/"
FINAL_PKG="$OUT/$(basename "$PKGFILE")"

# AUR/prebuilt payload: package-owned usr/ tree only, no pacman metadata.
PAYLOAD_DIR="$WORK/payload"
mkdir -p "$PAYLOAD_DIR"
bsdtar -xf "$FINAL_PKG" -C "$PAYLOAD_DIR" usr
tar --zstd -C "$PAYLOAD_DIR" -cf "$OUT/duskytunes-v$VERSION-linux-x86_64.tar.zst" usr

# One-click-ish end-user folder. install.sh keeps old custom packages out of the way and reasserts
# v2.4 after their uninstall hooks restore stock.
BUNDLE="$WORK/Duskytunes-v2.4-Dusky-x86_64"
mkdir -p "$BUNDLE"
cp "$FINAL_PKG" "$BUNDLE/"
cp "$ROOT/scripts/dusky-window-rule.sh" "$BUNDLE/dusky-window-rule.sh"
cat > "$BUNDLE/install.sh" <<'BINSTALL'
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PKG="$(find "$HERE" -maxdepth 1 -name 'duskytunes-v2.4-$VERSION-1-x86_64.pkg.tar.zst' -print -quit)"
[[ -n "$PKG" ]] || { echo "Duskytunes v2 package not found beside install.sh" >&2; exit 1; }

sudo pacman -U --needed "$PKG"
for old in duskytunes-v2.3 duskytunes-v2.2 duskytunes-v2.1 duskytunes-v2.0 duskytunes-v20 duskytunes-v21 duskytunes-v22 duskytunes-v23 duskytunes-v24 duskytunes-v1.4 duskytunes-v1.5 duskytunes-v1.6 duskytunes-v1.7 duskytunes-v1.8 duskytunes-v1.9; do
  if pacman -Q "$old" >/dev/null 2>&1; then
    sudo pacman -R --noconfirm "$old"
  fi
done
sudo /usr/lib/duskytunes-v2.4/activate-replacement
"$HERE/dusky-window-rule.sh" install
command -v update-desktop-database >/dev/null 2>&1 && sudo update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
echo "Duskytunes v2 installed. Launch it with: duskytunes"
BINSTALL
chmod 755 "$BUNDLE/install.sh" "$BUNDLE/dusky-window-rule.sh"

cat > "$BUNDLE/uninstall.sh" <<'BUNINSTALL'
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
"$HERE/dusky-window-rule.sh" remove || true
sudo pacman -Rns duskytunes-v2.4
BUNINSTALL
chmod 755 "$BUNDLE/uninstall.sh"

cat > "$BUNDLE/README.txt" <<'BREADME'
Duskytunes v2 for Dusky / CachyOS / Arch x86_64

Recommended:
  ./install.sh

Manual package install:
  sudo pacman -U ./duskytunes-v2.4-$VERSION-1-x86_64.pkg.tar.zst

The installer keeps dusky-desktop installed and replaces only the stock Duskytunes entry points.
BREADME

(
  cd "$BUNDLE"
  sha256sum "$(basename "$FINAL_PKG")" > SHA256SUMS
)
(cd "$WORK" && zip -qr "$OUT/Duskytunes-v2.4-Dusky-x86_64.zip" "$(basename "$BUNDLE")")

(
  cd "$OUT"
  sha256sum     "$(basename "$FINAL_PKG")"     duskytunes-v$VERSION-linux-x86_64.tar.zst     Duskytunes-v2.4-Dusky-x86_64.zip     > SHA256SUMS-v$VERSION.txt
)

echo "release assets written to $OUT"
cat "$OUT/SHA256SUMS-v$VERSION.txt"
