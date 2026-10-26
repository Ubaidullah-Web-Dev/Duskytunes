//! The daemon's socket protocol. One JSON object per line in both directions.
pub mod wire;

pub use wire::{ErrorBody, Event, Incoming, Outgoing, Request, Response, PROTOCOL_VERSION};

use std::path::PathBuf;

/// `$XDG_RUNTIME_DIR/duskytunes/duskytunesd.sock`, with fallback to `ryotunesd.sock`.
pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| {
        std::env::temp_dir().join(format!("duskytunes-{}", unsafe { libc_uid() }))
    });
    let dusky = dir.join("duskytunes").join("duskytunesd.sock");
    if dusky.exists() {
        return dusky;
    }
    let ryo = dir.join("ryotunes").join("ryotunesd.sock");
    if ryo.exists() {
        return ryo;
    }
    dusky
}

// No libc dependency for one call: read the uid from the kernel.
unsafe fn libc_uid() -> u32 {
    std::fs::read_to_string("/proc/self/loginuid")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}
