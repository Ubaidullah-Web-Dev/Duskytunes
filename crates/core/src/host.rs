//! The seams between the core and whoever hosts it.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

/// Server-push channel: the host fans an event out to every listening UI.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: &'static str, payload: Value);
}

#[derive(Debug, thiserror::Error)]
pub enum JsError {
    #[error("js session '{0}' does not exist")]
    Gone(String),
    #[error("js eval failed: {0}")]
    Eval(String),
    #[error("timed out after {0:?}")]
    Timeout(Duration),
    #[error("js environment reported an error: {0}")]
    BadEnvironment(String),
    #[error("js environment build failed: {0}")]
    Build(String),
}

/// A JavaScript environment able to run YouTube's player.js and BotGuard harnesses.
#[async_trait::async_trait]
pub trait JsBridge: Send + Sync + 'static {
    async fn create(
        &self,
        label: &str,
        harness_html: &str,
        init_script: &str,
    ) -> Result<Box<dyn JsSession>, JsError>;

    /// Destroy any environment left behind under `label` by a cancelled `create` and wait until the
    /// label is free again. Same semantics as the host's `destroy_and_wait`.
    async fn reclaim(&self, label: &str);
}

/// One live environment. Mirrors `src-tauri/src/webview.rs` `Bridge` one to one.
#[async_trait::async_trait]
pub trait JsSession: Send + Sync {
    fn eval(&self, js: &str) -> Result<(), JsError>;
    async fn eval_json(&self, js: String, timeout: Duration) -> Result<Value, JsError>;
    async fn call_async(&self, expr: &str, timeout: Duration) -> Result<Value, JsError>;
    fn exists(&self) -> bool;
    fn destroy(&self);
    /// A second handle to the same environment (the PoToken minter keeps one per session).
    fn clone_session(&self) -> Box<dyn JsSession>;
}

#[derive(Debug, Clone)]
pub struct LoginResult {
    /// `name=value` pairs for the `.youtube.com` domain, as the cookie jar hands them out.
    pub cookies: Vec<(String, String)>,
    /// Google account index the user picked (`authuser`).
    pub authuser: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("sign-in was cancelled")]
    Cancelled,
    #[error("sign-in failed: {0}")]
    Failed(String),
}

/// The interactive Google sign-in, owned by the host because it needs a visible browser.
#[async_trait::async_trait]
pub trait LoginFlow: Send + Sync + 'static {
    async fn sign_in(&self) -> Result<LoginResult, LoginError>;
}

/// Where the core keeps its files. The host resolves them (Tauri's `app_data_dir`, or XDG).
#[derive(Debug, Clone)]
pub struct Paths {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl Paths {
    pub fn covers_dir(&self) -> PathBuf {
        self.data_dir.join("covers")
    }
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("duskytunes.db")
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::Mutex;

    /// An `EventSink` that records what was emitted, for unit tests of the core.
    #[derive(Default)]
    pub struct RecordingSink {
        pub events: Mutex<Vec<(&'static str, Value)>>,
    }

    impl EventSink for RecordingSink {
        fn emit(&self, event: &'static str, payload: Value) {
            self.events.lock().unwrap().push((event, payload));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::RecordingSink;
    use super::*;

    #[test]
    fn recording_sink_keeps_order() {
        let sink = RecordingSink::default();
        sink.emit("playback-state", Value::String("playing".into()));
        sink.emit("position", serde_json::json!({ "position": 1.5 }));
        let events = sink.events.lock().unwrap();
        assert_eq!(events[0].0, "playback-state");
        assert_eq!(events[1].1["position"], 1.5);
    }

    #[test]
    fn paths_derive_children() {
        let p = Paths { data_dir: "/d".into(), cache_dir: "/c".into() };
        assert_eq!(p.covers_dir(), PathBuf::from("/d/covers"));
        assert_eq!(p.db_path(), PathBuf::from("/d/duskytunes.db"));
    }
}
