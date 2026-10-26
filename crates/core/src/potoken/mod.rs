//! PoToken / BotGuard generator (PoToken flow, webview runtime). Ports Metrolist's `PoTokenWebView` +
//! `PoTokenGenerator`, using the flow validated on WebKitGTK.
//!
//! Native Rust does all HTTP (`/Create`, `/GenerateIT`); the hidden webview only runs Google's
//! BotGuard JS (which cannot be reimplemented). Two tokens per session/video (PoToken flow): a
//! session token (minted from visitorData, cached in Rust with its TTL — `get_session_po_token`)
//! → the `/player` request body; a per-video streaming token (minted from videoId, lazy —
//! `get_streaming_po_token`) → `&pot=` on the stream URL. Everything is wrapped so a timeout /
//! broken webview returns `None` and the orchestrator falls through to the non-PoToken clients
//! (graceful degradation — stream selection §5).

mod jsutil;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::db::{now_secs, Db};
use crate::host::{JsBridge, JsError, JsSession};
use crate::http::WEB_UA;

const GOOGLE_API_KEY: &str = "AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw";
const REQUEST_KEY: &str = "O43z0dpjhgX20SCx4KAo";
const CREATE: &str = "https://www.youtube.com/api/jnn/v1/Create";
const GENERATE_IT: &str = "https://www.youtube.com/api/jnn/v1/GenerateIT";
const POTOKEN_LABEL: &str = "duskytunes-potoken";
/// Per-BotGuard-call budget — the webview's sandbox can be culled and hang forever (PoToken flow).
const CALL_TIMEOUT: Duration = Duration::from_secs(8);
/// Overall budget for a full mint (webview build + page-load + BotGuard bootstrap + token). Loose
/// backstop above the sum of the per-step timeouts; the per-call `CALL_TIMEOUT`s do the real work.
const MINT_BUDGET: Duration = Duration::from_secs(35);
/// Safety margin before the integrity token's TTL (PoToken flow §GenerateIT).
const EXPIRY_MARGIN: Duration = Duration::from_secs(10 * 60);

const HARNESS: &str = include_str!("../../po_token.html");

/// Glue defining `__lm.{run,mintInit,mint}` around the harness's BotGuard fns. `webPoSignalOutput`
/// holds a live JS function that can't cross to Rust, so it's parked in a JS global between calls
/// (webview runtime). One minter serves both session and per-video tokens.
const GLUE: &str = r#"window.__lm={
  wpso:null,
  async run(cd){var r=await runBotGuard(cd);window.__lm.wpso=r.webPoSignalOutput;return r.botguardResponse;},
  async mintInit(intTok){await createPoTokenMinter(window.__lm.wpso,new Uint8Array(intTok));return true;},
  async mint(ident){var t=await obtainPoToken(new Uint8Array(ident));return Array.from(t);}
};"#;

struct Minter {
    session_id: String,
    expires_at: Instant,
    bridge: Box<dyn JsSession>,
    last_used: Instant,
}

impl Minter {
    fn valid_for(&self, session_id: &str) -> bool {
        self.session_id == session_id && Instant::now() < self.expires_at && self.bridge.exists()
    }
}

/// Where the session token is persisted between runs. Internal: not in `UI_SETTINGS`, so the
/// webview can neither read nor write it.
const SESSION_TOKEN_KEY: &str = "potoken_session";

/// Cached session token (PoToken flow: minted from visitorData, ~12h TTL). Lives OUTSIDE the
/// webview `Minter` so the mint-and-destroy idle teardown doesn't force a full BotGuard
/// bootstrap on the next track start just to re-learn a string we already had.
///
/// Also survives the process, in `settings`. Google's own `/GenerateIT` says how long it is good
/// for (43200s, ~12h), and re-minting means standing up a whole hidden web process and running
/// BotGuard: measured at 88 MiB PSS and ~1.6s, at every launch, to re-learn a string we were told
/// stays valid until tomorrow. `expires_at` is therefore wall-clock, not an `Instant`, which
/// cannot outlive the process.
#[derive(serde::Serialize, serde::Deserialize)]
struct SessionToken {
    session_id: String,
    token: String,
    expires_at: i64,
}

impl SessionToken {
    fn valid_for(&self, session_id: &str, now: i64) -> bool {
        self.session_id == session_id && now < self.expires_at
    }
}

pub struct PoTokenGenerator {
    js: Arc<dyn JsBridge>,
    db: Arc<Db>,
    minter: Mutex<Option<Minter>>,
    /// Session token cache (PoToken flow: minted from visitorData, ~12h TTL). Lives OUTSIDE the
    /// webview minter so the mint-and-destroy idle teardown doesn't force a full BotGuard
    /// bootstrap on the next track start just to re-learn a string we already had.
    session_token: Mutex<Option<SessionToken>>,
    /// Latched once the system webview proves unusable (uncaught JS error) — thereafter always
    /// degrade to non-PoToken clients (PoToken flow §BadWebViewException).
    webview_bad: AtomicBool,
}

impl PoTokenGenerator {
    pub fn new(js: Arc<dyn JsBridge>, db: Arc<Db>) -> Self {
        // A token stored by a previous run is as good as one minted now, right up to its expiry.
        // A wrong-session or expired one is simply never returned by `cached_session_token`, so it
        // costs nothing to load it optimistically and let the normal validity check reject it.
        let stored: Option<SessionToken> =
            db.get_setting(SESSION_TOKEN_KEY).and_then(|raw| serde_json::from_str(&raw).ok());
        if let Some(t) = &stored {
            tracing::debug!(
                expires_in = t.expires_at - now_secs(),
                "loaded stored PoToken session"
            );
        }
        PoTokenGenerator {
            js,
            db,
            minter: Mutex::new(None),
            session_token: Mutex::new(stored),
            webview_bad: AtomicBool::new(false),
        }
    }

    /// Forget the session token, in memory and on disk.
    ///
    /// Called when a web-client stream is rejected (the orchestrator's off-hot-path self-heal).
    /// Without this a token that Google stopped honouring early would be replayed for the rest of
    /// its nominal 12 hours instead of only until the next launch.
    pub async fn invalidate_session_token(&self) {
        *self.session_token.lock().await = None;
        self.db.delete_setting(SESSION_TOKEN_KEY);
    }

    /// Session token for the `/player` request body (PoToken flow). Cheap when cached; otherwise
    /// performs the full bootstrap (and leaves the minter warm for streaming-token mints).
    pub async fn get_session_po_token(&self, visitor_data: &str) -> Option<String> {
        if self.webview_bad.load(Ordering::SeqCst) {
            return None;
        }
        if let Some(token) = self.cached_session_token(visitor_data).await {
            return Some(token);
        }
        match timeout(MINT_BUDGET, self.ensure_minter(visitor_data)).await {
            Ok(Ok(_guard)) => self.cached_session_token(visitor_data).await,
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "PoToken session mint failed — degrading");
                if matches!(e, MintError::Webview(JsError::BadEnvironment(_))) {
                    self.webview_bad.store(true, Ordering::SeqCst);
                }
                self.teardown().await;
                None
            }
            Err(_) => {
                tracing::warn!("PoToken session mint timed out — degrading");
                self.teardown().await;
                None
            }
        }
    }

    /// Per-video streaming token for the `&pot=` URL param (PoToken flow). Builds/reuses the
    /// minter; call ONLY when a web-client stream URL actually resolved (post-decipher).
    pub async fn get_streaming_po_token(
        &self,
        video_id: &str,
        visitor_data: &str,
    ) -> Option<String> {
        if self.webview_bad.load(Ordering::SeqCst) {
            return None;
        }
        match timeout(MINT_BUDGET, self.mint_streaming(video_id, visitor_data)).await {
            Ok(Ok(pot)) => Some(pot),
            Ok(Err(e)) => {
                tracing::warn!(video_id, error = %e, "PoToken streaming mint failed — degrading");
                if matches!(e, MintError::Webview(JsError::BadEnvironment(_))) {
                    self.webview_bad.store(true, Ordering::SeqCst);
                }
                self.teardown().await;
                None
            }
            Err(_) => {
                tracing::warn!(video_id, "PoToken streaming mint timed out — degrading");
                self.teardown().await;
                None
            }
        }
    }

    async fn cached_session_token(&self, visitor_data: &str) -> Option<String> {
        let now = now_secs();
        self.session_token
            .lock()
            .await
            .as_ref()
            .filter(|t| t.valid_for(visitor_data, now))
            .map(|t| t.token.clone())
    }

    /// Ensure `self.minter` holds a valid minter for `visitor_data`, (re)building it via
    /// `create_minter` (which also refreshes the cached session token through `bootstrap_minter`)
    /// if needed. Returns the locked guard so callers needing the minter's bridge (streaming
    /// mint) can keep using it without a re-lock race.
    async fn ensure_minter<'a>(
        &'a self,
        visitor_data: &str,
    ) -> Result<tokio::sync::MutexGuard<'a, Option<Minter>>, MintError> {
        let mut guard = self.minter.lock().await;
        if !guard.as_ref().is_some_and(|m| m.valid_for(visitor_data)) {
            if let Some(old) = guard.take() {
                old.bridge.destroy();
            }
            *guard = Some(self.create_minter(visitor_data).await?);
        }
        Ok(guard)
    }

    /// Per-video token (identifier = videoId). One retry with a fresh minter on failure.
    async fn mint_streaming(
        &self,
        video_id: &str,
        visitor_data: &str,
    ) -> Result<String, MintError> {
        let mut guard = self.ensure_minter(visitor_data).await?;
        let minter = guard.as_mut().expect("minter present");
        minter.last_used = Instant::now();
        let bridge = minter.bridge.clone_session();
        match mint_token(&*bridge, video_id.as_bytes()).await {
            Ok(pot) => Ok(pot),
            Err(e) => {
                tracing::debug!(error = %e, "per-video mint failed, rebuilding minter once");
                bridge.destroy();
                let fresh = self.create_minter(visitor_data).await?;
                let pot = mint_token(&*fresh.bridge, video_id.as_bytes()).await?;
                *guard = Some(fresh);
                Ok(pot)
            }
        }
    }

    /// Full BotGuard bootstrap: Create → runBotGuard → GenerateIT → createMinter → session token.
    async fn create_minter(&self, session_id: &str) -> Result<Minter, MintError> {
        let bridge = self.js.create(POTOKEN_LABEL, HARNESS, GLUE).await?;
        match self.bootstrap_minter(&*bridge, session_id).await {
            Ok(m) => Ok(m),
            Err(e) => {
                bridge.destroy(); // don't orphan the hidden window on a failed bootstrap
                Err(e)
            }
        }
    }

    /// The fallible BotGuard steps, run against an already-built `bridge`. Split out so
    /// `create_minter` can destroy the webview on any error path.
    async fn bootstrap_minter(
        &self,
        bridge: &dyn JsSession,
        session_id: &str,
    ) -> Result<Minter, MintError> {
        // 1. /Create → descrambled challengeData for runBotGuard.
        let scrambled = self.create_challenge().await?;
        let challenge = jsutil::parse_challenge_data(&scrambled).map_err(MintError::Parse)?;

        // 2. [webview] runBotGuard(challenge) → botguardResponse.
        let botguard_response = bridge
            .call_async(&format!("__lm.run({challenge})"), CALL_TIMEOUT)
            .await?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| MintError::Parse("runBotGuard returned non-string".into()))?;

        // 3. /GenerateIT → integrity token + ttl.
        let (int_token, ttl) = self.generate_it(&botguard_response).await?;

        // 4. [webview] createPoTokenMinter(integrityToken).
        bridge
            .call_async(
                &format!("__lm.mintInit({})", jsutil::js_byte_array(&int_token)),
                CALL_TIMEOUT,
            )
            .await?;

        // 5. [webview] session token (identifier = visitorData). Minted exactly once.
        let session_pot = mint_token(bridge, session_id.as_bytes()).await?;
        // One lifetime, two clocks: the minter never outlives this process, so its expiry stays an
        // `Instant`, while the session token is written to disk and needs wall-clock.
        let good_for = Duration::from_secs(ttl).saturating_sub(EXPIRY_MARGIN);

        // Cache the session token outside the minter (see `SessionToken` doc) so it survives both
        // the webview's mint-and-destroy idle teardown and the process itself.
        let token = SessionToken {
            session_id: session_id.to_owned(),
            token: session_pot,
            expires_at: now_secs() + good_for.as_secs() as i64,
        };
        if let Ok(json) = serde_json::to_string(&token) {
            self.db.set_setting(SESSION_TOKEN_KEY, &json);
        }
        *self.session_token.lock().await = Some(token);

        tracing::info!(ttl, "PoToken minter ready");
        Ok(Minter {
            session_id: session_id.to_owned(),
            expires_at: Instant::now() + good_for,
            bridge: bridge.clone_session(),
            last_used: Instant::now(),
        })
    }

    /// POST `/Create` and return the scrambled challenge blob (`create[1]`). Native HTTP.
    async fn create_challenge(&self) -> Result<String, MintError> {
        let resp = crate::http::client()
            .post(CREATE)
            .header("User-Agent", WEB_UA)
            .header("Content-Type", "application/json+protobuf")
            .header("x-goog-api-key", GOOGLE_API_KEY)
            .header("x-user-agent", "grpc-web-javascript/0.1")
            .body(format!("[\"{REQUEST_KEY}\"]"))
            .send()
            .await?
            .text()
            .await?;
        let raw: Value =
            serde_json::from_str(&resp).map_err(|e| MintError::Parse(e.to_string()))?;
        raw.get(1)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| MintError::Parse("create[1] missing".into()))
    }

    /// POST `/GenerateIT(botguardResponse)` → `(integrityToken bytes, ttlSeconds)`. Native HTTP.
    async fn generate_it(&self, botguard_response: &str) -> Result<(Vec<u8>, u64), MintError> {
        let body = serde_json::json!([REQUEST_KEY, botguard_response]).to_string();
        let resp = crate::http::client()
            .post(GENERATE_IT)
            .header("User-Agent", WEB_UA)
            .header("Content-Type", "application/json+protobuf")
            .header("x-goog-api-key", GOOGLE_API_KEY)
            .header("x-user-agent", "grpc-web-javascript/0.1")
            .body(body)
            .send()
            .await?
            .text()
            .await?;
        jsutil::parse_integrity_token_data(&resp).map_err(MintError::Parse)
    }

    /// Warm the PoToken webview for `visitor_data` (PoToken flow §startup). Non-fatal.
    pub async fn teardown_if_idle(&self, idle: Duration) {
        let mut guard = self.minter.lock().await;
        if let Some(m) = guard.as_ref() {
            if m.last_used.elapsed() >= idle {
                m.bridge.destroy();
                *guard = None;
                tracing::debug!("PoToken webview torn down (idle)");
            }
        }
    }

    async fn teardown(&self) {
        if let Some(m) = self.minter.lock().await.take() {
            m.bridge.destroy();
        }
        // A failed/cancelled create_minter (or a mint timeout that cancelled us mid-bootstrap)
        // leaves an untracked window on our label — reclaim it so no hidden webview is orphaned.
        self.js.reclaim(POTOKEN_LABEL).await;
    }
}

/// [webview] obtain one PoToken for `identifier` (raw UTF-8 bytes) → URL-safe base64.
async fn mint_token(bridge: &dyn JsSession, identifier: &[u8]) -> Result<String, MintError> {
    let arr = bridge
        .call_async(&format!("__lm.mint({})", jsutil::js_byte_array(identifier)), CALL_TIMEOUT)
        .await?;
    let bytes: Vec<u8> = arr
        .as_array()
        .ok_or_else(|| MintError::Parse("mint returned non-array".into()))?
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n as u8))
        .collect();
    if bytes.is_empty() {
        return Err(MintError::Parse("empty PoToken".into()));
    }
    Ok(jsutil::b64url_encode_no_pad(&bytes))
}

#[derive(Debug, thiserror::Error)]
enum MintError {
    #[error("webview: {0}")]
    Webview(#[from] JsError),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("parse: {0}")]
    Parse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(session_id: &str, expires_at: i64) -> SessionToken {
        SessionToken { session_id: session_id.to_owned(), token: "tok".to_owned(), expires_at }
    }

    #[test]
    fn session_token_valid_for_matching_id_and_future_expiry() {
        assert!(token("vd123", 1_000).valid_for("vd123", 900));
    }

    #[test]
    fn session_token_invalid_for_wrong_id() {
        // Signing in or out changes visitorData, so a token minted for the old one must not be
        // replayed against the new one.
        assert!(!token("vd123", 1_000).valid_for("other", 900));
    }

    #[test]
    fn session_token_invalid_when_expired() {
        assert!(!token("vd123", 1_000).valid_for("vd123", 1_000), "expiry is exclusive");
        assert!(!token("vd123", 1_000).valid_for("vd123", 1_001));
    }

    #[test]
    fn session_token_survives_a_round_trip_through_settings() {
        // What is stored between runs. A shape change here silently turns every launch back into
        // a full BotGuard bootstrap (the load is a best-effort `ok()`), so pin it.
        let json = serde_json::to_string(&token("vd123", 4_000)).unwrap();
        let back: SessionToken = serde_json::from_str(&json).unwrap();
        assert!(back.valid_for("vd123", 3_999));
        assert_eq!(back.token, "tok");

        // Anything unreadable must read as "no token" rather than panic the constructor.
        assert!(serde_json::from_str::<SessionToken>("{}").is_err());
        assert!(serde_json::from_str::<SessionToken>("not json").is_err());
    }
}
