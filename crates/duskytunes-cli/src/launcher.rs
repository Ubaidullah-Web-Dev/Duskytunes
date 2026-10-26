//! `duskytunes` native launcher for Dusky OS.
//!
//! Linux launcher for Duskytunes: communicates with `duskytunesd` over unix domain socket,
//! ensures the systemd socket or daemon process is started if not already running,
//! and requests the native Quickshell UI to open.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn main() {
    let sock = duskytunes_protocol::socket_path();

    // Fast path: a live daemon (or an idle systemd socket) answers `show` straight away.
    if show(&sock).is_ok() {
        return;
    }

    // Nothing is listening: bring the daemon up via systemd or direct process spawn.
    if let Err(e) = ensure_daemon() {
        eprintln!("duskytunes: could not start duskytunesd: {e}");
        // Fallback directly to launching the QML client if daemon fails to spawn
        let _ = spawn_fallback_qml();
        std::process::exit(1);
    }

    match show_until_ready(&sock) {
        Ok(()) => (),
        Err(e) => {
            eprintln!(
                "duskytunes: could not reach the Duskytunes daemon (duskytunesd): {e}\n\
                 Start it with `systemctl --user start duskytunesd.socket`, or run `duskytunesd`."
            );
            // Even if socket handshake times out, try opening the client directly
            let _ = spawn_fallback_qml();
            std::process::exit(1);
        }
    }
}

/// Send `show` over the daemon socket.
fn show(sock: &Path) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(sock)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(b"{\"id\":1,\"method\":\"show\"}\n")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let response: duskytunes_protocol::Response = serde_json::from_str(&line)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if response.id != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected response ID",
        ));
    }
    if let Some(error) = response.error {
        return Err(std::io::Error::other(error.message));
    }
    Ok(())
}

/// Retry `show` while the daemon binds its socket.
fn show_until_ready(sock: &Path) -> std::io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let error = match show(sock) {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };
        if Instant::now() >= deadline {
            return Err(error);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Start systemd socket unit or fallback to spawning daemon directly.
fn ensure_daemon() -> std::io::Result<()> {
    if start_socket_unit() {
        return Ok(());
    }
    spawn_daemon()
}

fn start_socket_unit() -> bool {
    Command::new("systemctl")
        .args(["--user", "--no-block", "start", "duskytunesd.socket"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn spawn_daemon() -> std::io::Result<()> {
    Command::new("duskytunesd")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

fn spawn_fallback_qml() -> std::io::Result<()> {
    Command::new("duskytunes-qml")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}
