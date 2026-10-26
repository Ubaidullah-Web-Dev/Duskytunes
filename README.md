<div align="center">

# 🎵 Duskytunes

**A native desktop music instrument for Dusky OS & Arch Linux with native audio playback, live dynamic theming, and an interface shaped for Wayland & Hyprland.**

<br />

<a href="https://github.com/Ubaidullah-Web-Dev/Duskytunes"><img alt="License" src="https://img.shields.io/badge/license-GPL--3.0-blue?style=flat-square"></a>
<img alt="Linux x86_64" src="https://img.shields.io/badge/platform-Arch%20%7C%20Dusky%20Linux-8996a8?style=flat-square&logo=arch-linux&logoColor=white">
<img alt="Rust" src="https://img.shields.io/badge/core-Rust-2d3136?style=flat-square&logo=rust&logoColor=white">
<img alt="Quickshell / QML" src="https://img.shields.io/badge/UI-Quickshell%20%2F%20QML-2d3136?style=flat-square">
<img alt="libmpv" src="https://img.shields.io/badge/audio-libmpv-2d3136?style=flat-square">

<br /><br />

### 🚀 One-Line Installation

Run this single command in your terminal to automatically install Duskytunes and all its dependencies:

```bash
curl -sSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Duskytunes/main/install.sh | bash
```

</div>

<br />

---

## ⚡ What is Duskytunes?

**Duskytunes** is a Linux-first desktop music application shaped specifically for **Dusky + Hyprland**.

The primary interface is written in pure **Quickshell / QML**, connected to a dedicated background Rust daemon (`duskytunesd`). Audio playback, queue management, and provider integrations run natively in **libmpv** and Rust independently of the visible client window.

- **Native playback**: Audio playback stays in native `libmpv`, outside web engine overhead. Supports MPRIS, hardware media keys, system tray, and gapless playback.
- **Dynamic Theming**: Automatically consumes live wallpaper and palette roles. Changes in wallpaper or theme retint the main player and mini-player instantly.
- **Multi-Provider Support**: Seamlessly browse and play music from YouTube Music, Spotify, and SoundCloud.
- **One-Click Downloads**: Download music in original format, MP3, Opus, or FLAC with automatically embedded cover art and synced `.lrc` lyrics.
- **Synced Lyrics**: Interactive synced lyrics with click-to-seek support.
- **Mini-Player**: Compact floating companion player with Hyprland window rules.

---

## 📦 Installation & Usage

### 1. Automatic One-Line Install (Recommended)

```bash
curl -sSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Duskytunes/main/install.sh | bash
```

The installer handles everything automatically:
- Installs necessary system packages (`quickshell`, `libappindicator`, `mpv`, `yt-dlp`, `ffmpeg`, etc.)
- Registers the QML design system runtime (`Dusky.Ui.Singletons`)
- Installs the application binaries and desktop launchers
- Enables and starts the background daemon socket via systemd

### 2. Manual Installation from Source

```bash
git clone https://github.com/Ubaidullah-Web-Dev/Duskytunes.git
cd Duskytunes
./install.sh
```

Or build an official pacman package from the repository:

```bash
cd packaging/arch
makepkg -si
```

---

## 🎧 Launching Duskytunes

- **Application Menu**: Search for **Duskytunes** in Rofi, Wofi, or your Dusky application launcher.
- **Terminal**: Run `duskytunes`
- **Mini Player**: Launch from within the app bar or via `duskytunes-cli show`

---

## 🔄 Updating Duskytunes

- **In-App Updater**: Open **Settings** (gear icon) → **About** → **Check for new version**, then click **Update**.
- **Or re-run the installer**:
  ```bash
  curl -sSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Duskytunes/main/install.sh | bash
  ```

---

## 🛠️ CLI Controls

Duskytunes provides `duskytunes-cli` for scriptable IPC controls with the background daemon:

```bash
duskytunes-cli hello                           # Check daemon status & protocol
duskytunes-cli show                            # Raise or open the native UI
duskytunes-cli search '{"query":"daft punk"}'  # Search music provider
duskytunes-cli events                          # Stream real-time playback events
```

---

## 👤 Author & Credits

- **Maintained & Packaged by**: [Ubaid](https://github.com/Ubaidullah-Web-Dev)
- **GitHub**: [https://github.com/Ubaidullah-Web-Dev/Duskytunes](https://github.com/Ubaidullah-Web-Dev/Duskytunes)
- Based on the Duskytunes and LiMusic open-source foundations under the GNU General Public License v3.0 or later.
