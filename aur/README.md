# duskytunes-bin AUR packaging

This directory is the AUR recipe for Duskytunes' precompiled x86_64 Linux release.

The AUR package downloads the versioned binary payload from the matching GitHub Release and repackages it for pacman. It does **not** build the Tauri/Rust/Svelte application locally.

Before publishing or updating the AUR package:

1. Confirm the matching GitHub Release asset exists.
2. Verify its SHA-256 against `PKGBUILD`.
3. Run `makepkg --printsrcinfo > .SRCINFO` on an Arch-based system.
4. Test with `makepkg -si`.
5. Commit `PKGBUILD`, `.SRCINFO`, `duskytunes-bin.install`, and `LICENSE` to the AUR package repository.

The packaged application remains GPL-3.0-or-later. The AUR packaging files in this directory are offered under 0BSD.
