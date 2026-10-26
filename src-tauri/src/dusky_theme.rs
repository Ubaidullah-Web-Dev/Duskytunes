//! Event-driven Dusky palette bridge.
//!
//! Dusky's own QML apps resolve colour from three live files:
//! - ~/.config/dusky/theme.json       (followWallpaper)
//! - ~/.config/dusky/shell.json       (named themePalette + motion)
//! - ~/.cache/dusky/colors.json       (wallpaper Material roles)
//!
//! Keep exactly the same precedence here: named scheme -> wallpaper palette while enabled ->
//! signature defaults. Linux watches the parent directories with inotify, so theme changes are
//! pushed to WebKit immediately without a permanent polling clock.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

use duskytunes_core::host::EventSink;

fn base_dir(env_key: &str, fallback: &str) -> PathBuf {
    if let Some(v) = std::env::var_os(env_key) {
        return PathBuf::from(v);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(fallback)
}

fn paths() -> (PathBuf, PathBuf) {
    (
        base_dir("XDG_CONFIG_HOME", ".config").join("dusky"),
        base_dir("XDG_CACHE_HOME", ".cache").join("dusky"),
    )
}

fn read_json(path: &std::path::Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

fn usable(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned)
}

fn hex_luminance(s: &str) -> Option<f64> {
    let s = s.trim().strip_prefix('#')?;
    let (r, g, b) = match s.len() {
        6 | 8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
        ),
        3 | 4 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            (r, g, b)
        }
        _ => return None,
    };
    Some((0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)) / 255.0)
}

/// Resolve the same Material-role chain used by Dusky.Ui.Singletons.Tokens.
pub fn tokens() -> Value {
    let (config, cache) = paths();
    let theme_path = config.join("theme.json");
    let shell_path = config.join("shell.json");
    let colors_path = cache.join("colors.json");

    let detected = theme_path.exists() || shell_path.exists() || colors_path.exists();
    let theme = read_json(&theme_path);
    let shell = read_json(&shell_path);
    let wall = read_json(&colors_path);

    let follow = theme.get("followWallpaper").and_then(Value::as_bool).unwrap_or(true);
    let named = shell.get("themePalette").filter(|v| v.is_object());
    let role = |key: &str, fallback: &str| -> String {
        usable(named.and_then(|v| v.get(key)))
            .or_else(|| if follow { usable(wall.get(key)) } else { None })
            .unwrap_or_else(|| fallback.to_string())
    };

    let paper = role("surface", "#000000");
    let paper_lift = role("surfaceContainerLow", "#0a0a0a");
    let panel = role("surfaceContainer", &paper_lift);
    let card = role("surfaceContainerHigh", &panel);
    let sidebar = role("surfaceContainerLow", &paper_lift);
    let player = role("surfaceContainer", &panel);

    let motion = shell.get("theme").and_then(|v| v.get("motion"));
    let scale = motion
        .and_then(|v| v.get("scale"))
        .and_then(Value::as_f64)
        .filter(|v| *v > 0.0)
        .unwrap_or(1.0);
    let reduce = motion.and_then(|v| v.get("reduce")).and_then(Value::as_bool).unwrap_or(false);

    let light = hex_luminance(&paper).is_some_and(|v| v > 0.5);
    let source = if named.is_some() {
        "named"
    } else if follow && wall.is_object() {
        "wallpaper"
    } else {
        "signature"
    };

    json!({
        "detected": detected,
        "paper": paper,
        "paperLift": paper_lift,
        "panel": panel,
        "card": card,
        "sidebar": sidebar,
        "player": player,
        "ink": role("onSurface", "#cdc4ba"),
        "inkDim": role("onSurfaceVariant", "#b0a9a0"),
        "bone": role("inverseSurface", "#cdc4ba"),
        "inkOnBone": role("inverseOnSurface", "#000000"),
        "primary": role("primary", "#e2342a"),
        "onPrimary": role("onPrimary", "#ffffff"),
        "primaryContainer": role("primaryContainer", "#3b1a18"),
        "onPrimaryContainer": role("onPrimaryContainer", "#ffdad6"),
        "secondary": role("secondary", "#8d6e68"),
        "onSecondary": role("onSecondary", "#ffffff"),
        "secondaryContainer": role("secondaryContainer", "#ffdad3"),
        "onSecondaryContainer": role("onSecondaryContainer", "#2c1512"),
        "tertiary": role("tertiary", "#7c5635"),
        "onTertiary": role("onTertiary", "#ffffff"),
        "tertiaryContainer": role("tertiaryContainer", "#ffdcc1"),
        "onTertiaryContainer": role("onTertiaryContainer", "#2e1500"),
        "outline": role("outline", "#958f87"),
        "outlineVariant": role("outlineVariant", "#4a4540"),
        "light": light,
        "motionScale": scale,
        "reduceMotion": reduce,
        "source": source
    })
}

#[cfg(target_os = "linux")]
pub fn spawn_watcher(sink: Arc<dyn EventSink>) {
    let (config, cache) = paths();
    std::thread::Builder::new()
        .name("dusky-theme-watch".into())
        .spawn(move || unsafe {
            let fd = libc::inotify_init1(libc::IN_CLOEXEC);
            if fd < 0 {
                tracing::warn!("dusky theme watcher: inotify_init1 failed");
                return;
            }

            let mask = libc::IN_CLOSE_WRITE
                | libc::IN_MOVED_TO
                | libc::IN_CREATE
                | libc::IN_DELETE
                | libc::IN_ATTRIB;

            let mut watched = 0;
            for dir in [&config, &cache] {
                if !dir.exists() {
                    continue;
                }
                use std::os::unix::ffi::OsStrExt;
                let Ok(path) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else {
                    continue;
                };
                if libc::inotify_add_watch(fd, path.as_ptr(), mask) >= 0 {
                    watched += 1;
                }
            }

            if watched == 0 {
                libc::close(fd);
                return;
            }

            let mut buf = [0u8; 4096];
            loop {
                let n = libc::read(fd, buf.as_mut_ptr().cast(), buf.len());
                if n <= 0 {
                    break;
                }
                let n = n as usize;
                let mut offset = 0usize;
                let header = std::mem::size_of::<libc::inotify_event>();
                let mut relevant = false;

                while offset + header <= n {
                    let ev = std::ptr::read_unaligned(
                        buf.as_ptr().add(offset).cast::<libc::inotify_event>(),
                    );
                    let name_len = ev.len as usize;
                    if offset + header + name_len > n {
                        break;
                    }
                    if name_len > 0 {
                        let raw = &buf[offset + header..offset + header + name_len];
                        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
                        if let Ok(name) = std::str::from_utf8(&raw[..end]) {
                            relevant |= matches!(name, "theme.json" | "shell.json" | "colors.json");
                        }
                    }
                    offset += header + name_len;
                }

                if relevant {
                    // Directory events are CLOSE_WRITE/MOVED_TO/CREATE, so the producer has
                    // finished the write before we read. The payload itself is the fresh token set;
                    // WebKit does not need a second timer or filesystem round-trip.
                    sink.emit("dusky-theme-changed", tokens());
                }
            }

            libc::close(fd);
        })
        .ok();
}

#[cfg(not(target_os = "linux"))]
pub fn spawn_watcher(_sink: Arc<dyn EventSink>) {}
