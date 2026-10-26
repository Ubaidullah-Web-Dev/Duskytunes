#!/usr/bin/env bash
set -euo pipefail
# Safe example only: install into a dedicated folder and never edit the user's shell.qml.
dest="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/duskytunes"
mkdir -p "$dest"
cp "$(dirname "$0")/DuskytunesBarWidget.qml" "$dest/"
printf 'Installed optional widget to %s\nImport it from your Dusky/Quickshell bar config as DuskytunesBarWidget {}.\n' "$dest"
