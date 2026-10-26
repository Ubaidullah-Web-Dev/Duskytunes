#!/usr/bin/env bash
# ==============================================================================
#  Duskytunes — The Modern Desktop Music Player for Dusky & Arch Linux
#  Maintained by Ubaid (https://github.com/Ubaidullah-Web-Dev/Duskytunes)
#
#  Usage:
#    curl -sSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Duskytunes/main/install.sh | bash
#    or:
#    ./install.sh
# ==============================================================================
set -euo pipefail

# Text styling
BOLD="$(tput bold 2>/dev/null || true)"
GREEN="$(tput setaf 2 2>/dev/null || true)"
CYAN="$(tput setaf 6 2>/dev/null || true)"
YELLOW="$(tput setaf 3 2>/dev/null || true)"
RED="$(tput setaf 1 2>/dev/null || true)"
RESET="$(tput sgr0 2>/dev/null || true)"

log_info()  { printf "${CYAN}==>${RESET} ${BOLD}%s${RESET}\n" "$1"; }
log_ok()    { printf "${GREEN}  ✓${RESET} %s\n" "$1"; }
log_warn()  { printf "${YELLOW}  !${RESET} %s\n" "$1"; }
log_err()   { printf "${RED}  ✗${RESET} %s\n" "$1" >&2; }

printf "\n"
printf "${CYAN}${BOLD}"
cat << 'EOF'
     ____             __          __                         
    / __ \__  _______/ /____  __/ /___  ______  ___  _____   
   / / / / / / / ___/ //_/ / / / __/ / / / __ \/ _ \/ ___/   
  / /_/ / /_/ (__  ) ,< / /_/ / /_/ /_/ / / / /  __(__  )    
 /_____/\__,_/____/_/|_|\__, /\__/\__,_/_/ /_/\___/____/     
                       /____/                                
EOF
printf "${RESET}"
printf "   ${BOLD}Duskytunes Installer for Dusky OS & Arch Linux${RESET}\n"
printf "   ${CYAN}Created & Maintained by Ubaid${RESET}\n\n"

# Verify system is Arch/pacman-based
if ! command -v pacman >/dev/null 2>&1; then
  log_err "This installer requires an Arch-based Linux distribution (with pacman)."
  exit 1
fi

if [[ ${EUID:-$(id -u)} -eq 0 ]]; then
  log_err "Please run this script as your regular user (not root). It will invoke sudo when needed."
  exit 1
fi

# Locate or fetch source tree
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || true)"
TEMP_DIR=""

if [[ -f "$SCRIPT_DIR/client/App.qml" && -d "$SCRIPT_DIR/runtime" ]]; then
  SRC_ROOT="$SCRIPT_DIR"
else
  log_info "Downloading Duskytunes source files..."
  TEMP_DIR="$(mktemp -d /tmp/duskytunes-install-XXXXXX)"
  trap 'rm -rf "$TEMP_DIR"' EXIT
  git clone --depth 1 https://github.com/Ubaidullah-Web-Dev/Duskytunes.git "$TEMP_DIR"
  SRC_ROOT="$TEMP_DIR"
fi

# Step 1: Install system dependencies
log_info "Installing runtime dependencies via pacman..."
sudo pacman -S --needed --noconfirm \
  quickshell \
  libappindicator \
  mpv \
  yt-dlp \
  ffmpeg \
  openssl \
  librsvg \
  desktop-file-utils \
  hicolor-icon-theme \
  xdg-utils \
  polkit
log_ok "All system dependencies installed."

# Step 2: Install QML design runtime (Ryoku.Ui / Singletons)
log_info "Installing QML design runtime (Tokens & Singletons)..."
if [[ -d "$SRC_ROOT/runtime/qml/Ryoku/Ui" ]]; then
  mkdir -p "$HOME/.local/lib/qt6/qml/Ryoku"
  rm -rf "$HOME/.local/lib/qt6/qml/Ryoku/Ui"
  cp -r "$SRC_ROOT/runtime/qml/Ryoku/Ui" "$HOME/.local/lib/qt6/qml/Ryoku/"

  sudo mkdir -p "/usr/lib/qt6/qml/Ryoku"
  sudo rm -rf "/usr/lib/qt6/qml/Ryoku/Ui"
  sudo cp -r "$SRC_ROOT/runtime/qml/Ryoku/Ui" "/usr/lib/qt6/qml/Ryoku/"
  log_ok "QML design runtime registered."
else
  log_warn "QML runtime directory not found in source tree; using existing system runtime."
fi

# Step 3: Install Binaries
log_info "Installing Duskytunes binaries..."
BIN_DIR="$SRC_ROOT/bin/x86_64"

if [[ -f "$BIN_DIR/ryotunesd" ]]; then
  sudo install -Dm755 "$BIN_DIR/ryotunes" "/usr/bin/duskytunes"
  sudo ln -sf duskytunes "/usr/bin/ryotunes"

  sudo install -Dm755 "$BIN_DIR/ryotunesd" "/usr/bin/duskytunesd"
  sudo ln -sf duskytunesd "/usr/bin/ryotunesd"

  sudo install -Dm755 "$BIN_DIR/ryotunes-cli" "/usr/bin/duskytunes-cli"
  sudo ln -sf duskytunes-cli "/usr/bin/ryotunes-cli"

  if [[ -f "$BIN_DIR/ryotunes-sync" ]]; then
    sudo install -Dm755 "$BIN_DIR/ryotunes-sync" "/usr/bin/duskytunes-sync"
    sudo ln -sf duskytunes-sync "/usr/bin/ryotunes-sync"
  fi
elif [[ -f "/usr/bin/ryotunesd" ]]; then
  sudo cp -p /usr/bin/ryotunes /usr/bin/duskytunes 2>/dev/null || true
  sudo cp -p /usr/bin/ryotunesd /usr/bin/duskytunesd 2>/dev/null || true
  sudo cp -p /usr/bin/ryotunes-cli /usr/bin/duskytunes-cli 2>/dev/null || true
  sudo cp -p /usr/bin/ryotunes-sync /usr/bin/duskytunes-sync 2>/dev/null || true
fi

# Launcher script and diagnostics
sudo install -Dm755 "$SRC_ROOT/packaging/linux/duskytunes-qml" "/usr/bin/duskytunes-qml"
sudo ln -sf duskytunes-qml "/usr/bin/ryotunes-qml"

sudo install -Dm755 "$SRC_ROOT/scripts/diagnostics.sh" "/usr/bin/duskytunes-diagnostics"
sudo ln -sf duskytunes-diagnostics "/usr/bin/ryotunes-diagnostics"
log_ok "Binaries installed to /usr/bin."

# Step 4: Install Client UI and Skins
log_info "Installing QML client and skins..."
sudo rm -rf "/usr/share/duskytunes"
sudo install -dm755 "/usr/share/duskytunes"
sudo cp -r "$SRC_ROOT/client" "/usr/share/duskytunes/client"
sudo rm -rf "/usr/share/duskytunes/client/tests"

sudo install -dm755 "/usr/share/duskytunes/skins"
sudo cp -r "$SRC_ROOT/skins/." "/usr/share/duskytunes/skins/"
sudo install -Dm644 "$SRC_ROOT/matugen/ryotunes.json" "/usr/share/duskytunes/matugen/duskytunes.json"

sudo rm -rf "/usr/share/ryotunes"
sudo ln -sf /usr/share/duskytunes "/usr/share/ryotunes"
log_ok "Client UI installed to /usr/share/duskytunes."

# Step 5: Install Desktop Entry and Icons
log_info "Installing desktop entry and icons..."
sudo install -Dm644 "$SRC_ROOT/packaging/linux/duskytunes.desktop" "/usr/share/applications/duskytunes.desktop"
sudo ln -sf duskytunes.desktop "/usr/share/applications/ryotunes.desktop"

for s in 32x32 64x64 128x128; do
  sudo install -Dm644 "$SRC_ROOT/src-tauri/icons/${s}.png" "/usr/share/icons/hicolor/${s}/apps/duskytunes.png"
  sudo ln -sf duskytunes.png "/usr/share/icons/hicolor/${s}/apps/ryotunes.png"
done
sudo install -Dm644 "$SRC_ROOT/src-tauri/icons/128x128@2x.png" "/usr/share/icons/hicolor/256x256/apps/duskytunes.png"
sudo ln -sf duskytunes.png "/usr/share/icons/hicolor/256x256/apps/ryotunes.png"
sudo install -Dm644 "$SRC_ROOT/src-tauri/icons/icon.png" "/usr/share/icons/hicolor/512x512/apps/duskytunes.png"
sudo ln -sf duskytunes.png "/usr/share/icons/hicolor/512x512/apps/ryotunes.png"

sudo gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true
sudo update-desktop-database -q /usr/share/applications 2>/dev/null || true
log_ok "Desktop entry and icons registered."

# Step 6: Configure and Start Systemd User Service
log_info "Configuring background daemon socket..."
sudo install -Dm644 "$SRC_ROOT/packaging/linux/duskytunesd.socket" "/usr/lib/systemd/user/duskytunesd.socket"
sudo install -Dm644 "$SRC_ROOT/packaging/linux/duskytunesd.service" "/usr/lib/systemd/user/duskytunesd.service"
sudo install -Dm644 "$SRC_ROOT/packaging/linux/90-duskytunes.preset" "/usr/lib/systemd/user-preset/90-duskytunes.preset"

systemctl --user daemon-reload 2>/dev/null || true
systemctl --user stop duskytunesd.socket duskytunesd.service ryotunesd.socket ryotunesd.service 2>/dev/null || true
systemctl --user enable --now duskytunesd.socket 2>/dev/null || true

# Symlink legacy socket path to duskytunesd.sock for 100% backward compatibility
USER_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
if [[ -d "$USER_RUNTIME" ]]; then
  mkdir -p "$USER_RUNTIME/ryotunes"
  ln -sf "../duskytunes/duskytunesd.sock" "$USER_RUNTIME/ryotunes/ryotunesd.sock" 2>/dev/null || true
fi
log_ok "Background daemon socket active."

printf "\n"
printf "${GREEN}${BOLD}======================================================${RESET}\n"
printf "${GREEN}${BOLD}  Duskytunes has been successfully installed!${RESET}\n"
printf "${GREEN}${BOLD}======================================================${RESET}\n\n"
printf "Launch it by running:  ${CYAN}${BOLD}duskytunes${RESET}\n"
printf "Or find ${BOLD}Duskytunes${RESET} in your application launcher.\n\n"
