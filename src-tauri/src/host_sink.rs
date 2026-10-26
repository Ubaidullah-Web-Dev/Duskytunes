//! `EventSink` for the Tauri host: every core event becomes a Tauri event on every window.

use duskytunes_core::host::EventSink;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

pub struct TauriSink(pub AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, event: &'static str, payload: Value) {
        // The Windows taskbar thumbnail toolbar isn't a webview event, and the core no longer holds
        // an AppHandle to poke it. Drive its play/pause glyph off the same playback-state the UI
        // gets. cfg-gated so non-Windows builds stay byte-identical.
        #[cfg(target_os = "windows")]
        if event == "playback-state" {
            if let Some(state) = payload.as_str() {
                crate::taskbar::set_playing(&self.0, state == "playing");
            }
        }
        if let Err(e) = self.0.emit(event, payload) {
            tracing::debug!(event, error = %e, "event emit failed (no window?)");
        }
    }
}
