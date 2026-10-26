//! The visible Google sign-in window (authentication flow Path A), ported from
//! `src-tauri/src/login_webview.rs` onto the GTK thread. It opens a real WebKitGTK window with a
//! spoofed desktop UA, refuses any navigation off Google/YouTube, watches for the redirect back to
//! music.youtube.com, harvests the youtube cookies from the shared jar and hands them to the core.
//!
//! Persistent (the default `WebContext`) on purpose: the webview keeps its own Google session, so a
//! later re-login is one click with no password/paste.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use tokio::sync::oneshot;

use duskytunes_core::host::{LoginError, LoginFlow, LoginResult};
use duskytunes_core::session::{allowed_login_navigation, LOGIN_URL};

use crate::gtk_thread::Gtk;

/// WebKitGTK is a WebKit engine, so a macOS Safari UA is the most internally-consistent spoof and
/// the least likely to trip Google's "this browser may not be secure" block. **Tune here** if
/// Google rejects it — this is the fragile part (authentication flow Path A).
const LOGIN_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
                        (KHTML, like Gecko) Version/17.4.1 Safari/605.1.15";

/// The cookie jar is read for this origin — the same one YouTube Music serves from.
const MUSIC_ORIGIN: &str = "https://music.youtube.com";

/// Success or a reason the sign-in ended, funneled through the oneshot the window resolves.
type Outcome = Result<LoginResult, LoginError>;

/// The host's [`LoginFlow`]: one visible Google sign-in window per call, on the GTK thread.
pub struct GtkLogin {
    gtk: Gtk,
}

impl GtkLogin {
    pub fn new(gtk: Gtk) -> Self {
        GtkLogin { gtk }
    }
}

#[async_trait::async_trait]
impl LoginFlow for GtkLogin {
    async fn sign_in(&self) -> Result<LoginResult, LoginError> {
        let (tx, rx) = oneshot::channel::<Outcome>();
        let tx = Arc::new(Mutex::new(Some(tx)));
        self.gtk.call(move || build_login_window(tx)).await;
        // The sender is dropped when the window is torn down without resolving → cancelled.
        rx.await.unwrap_or(Err(LoginError::Cancelled))
    }
}

/// Build the sign-in window on the GTK thread and wire its two exits: a finished load whose jar
/// carries the auth cookies resolves `Ok`; the user closing the window resolves `Err(Cancelled)`.
fn build_login_window(tx: Arc<Mutex<Option<oneshot::Sender<Outcome>>>>) {
    use gtk::prelude::*;
    use webkit2gtk::{
        gio, CookieManagerExt, LoadEvent, NavigationPolicyDecision, NavigationPolicyDecisionExt,
        PolicyDecisionExt, PolicyDecisionType, SettingsExt, URIRequestExt, WebContext,
        WebContextExt, WebView, WebViewExt,
    };

    let view = WebView::new();
    // See `gtk_thread::install_clean_exit`: register our exit shim right after WebKit's own
    // aborting teardown registers (on the first view build), in case login opens the first view.
    crate::gtk_thread::install_clean_exit();
    if let Some(settings) = WebViewExt::settings(&view) {
        settings.set_user_agent(Some(LOGIN_UA));
    }

    // Allow-list: only https on Google/YouTube. A phishing redirect can never keep the window.
    view.connect_decide_policy(|_view, decision, kind| {
        if kind != PolicyDecisionType::NavigationAction
            && kind != PolicyDecisionType::NewWindowAction
        {
            return false; // not a navigation — let WebKit apply its default
        }
        let uri = decision
            .downcast_ref::<NavigationPolicyDecision>()
            .and_then(|d| d.navigation_action())
            .and_then(|a| a.request())
            .and_then(|r| r.uri());
        let allowed = uri
            .as_deref()
            .and_then(|u| url::Url::parse(u).ok())
            .map(|u| allowed_login_navigation(&u))
            .unwrap_or(false);
        if allowed {
            decision.use_();
        } else {
            decision.ignore();
        }
        true // we made the decision
    });

    // On every finished load, read the youtube jar; once SAPISID is present sign-in is complete.
    let tx_load = tx.clone();
    view.connect_load_changed(move |view, event| {
        if event != LoadEvent::Finished {
            return;
        }
        let Some(cm) = WebContext::default().and_then(|c| c.cookie_manager()) else {
            return;
        };
        let tx = tx_load.clone();
        let view = view.clone();
        cm.cookies(MUSIC_ORIGIN, None::<&gio::Cancellable>, move |res| {
            let Ok(cookies) = res else {
                return;
            };
            // `soup::Cookie`'s accessors are inherent (`&mut self`) so no `soup` dependency is
            // needed to lift each cookie into a plain (name, value, domain) triple.
            let triples: Vec<(String, String, String)> = cookies
                .into_iter()
                .map(|mut c| {
                    (
                        c.name().map(|g| g.to_string()).unwrap_or_default(),
                        c.value().map(|g| g.to_string()).unwrap_or_default(),
                        c.domain().map(|g| g.to_string()).unwrap_or_default(),
                    )
                })
                .collect();
            let pairs = youtube_cookie_pairs(triples);
            if innertube::cookie_sapisid(&cookie_header(&pairs)).is_some() {
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(Ok(LoginResult { cookies: pairs, authuser: 0 }));
                }
                if let Some(w) = view.toplevel() {
                    unsafe { w.destroy() };
                }
            }
        });
    });

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Sign in to YouTube Music");
    window.set_default_size(480, 720);
    window.add(&view);

    // Closing the window before the cookies land is a cancel.
    let tx_cancel = tx;
    window.connect_delete_event(move |_, _| {
        if let Some(tx) = tx_cancel.lock().unwrap().take() {
            let _ = tx.send(Err(LoginError::Cancelled));
        }
        glib::Propagation::Proceed
    });

    window.show_all();
    view.load_uri(LOGIN_URL);
}

/// Domain-match by hand rather than with `cookies_for_url` (matches
/// `src-tauri/src/login_webview.rs`). Anything outside youtube.com is dropped, google.com included:
/// this becomes a `Cookie` header sent to YouTube, and a cookie without a domain we recognise
/// doesn't belong in it.
///
/// Input is `(name, value, domain)` triples: the daemon lifts them from `soup::Cookie` (whose
/// domain may keep a leading dot), the tests build them directly.
fn youtube_cookie_pairs(cookies: Vec<(String, String, String)>) -> Vec<(String, String)> {
    // Normalise the domain the way Tauri's `Cookie::domain()` did (leading dot stripped), then sort
    // so the most specific domain is inserted last and wins a name collision, the way a browser
    // resolves one.
    let mut cookies: Vec<(String, String, String)> = cookies
        .into_iter()
        .map(|(name, value, domain)| {
            let domain = domain.strip_prefix('.').map(str::to_owned).unwrap_or(domain);
            (name, value, domain)
        })
        .collect();
    cookies.sort_by_key(|(_, _, domain)| domain.len());
    let mut jar = BTreeMap::new();
    for (name, value, domain) in cookies {
        if domain == "youtube.com" || domain.ends_with(".youtube.com") {
            jar.insert(name, value);
        }
    }
    jar.into_iter().collect()
}

/// The `Cookie` header a browser would send: `name=value` joined with `; `.
fn cookie_header(pairs: &[(String, String)]) -> String {
    pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `"SAPISID=abc; Domain=.youtube.com"` → `(name, value, domain)`, as the cookie jar hands them.
    fn cookie(s: &str) -> (String, String, String) {
        let (nv, domain) = s.split_once("; Domain=").unwrap_or((s, ""));
        let (name, value) = nv.split_once('=').unwrap();
        (name.to_string(), value.to_string(), domain.to_string())
    }

    #[test]
    fn keeps_the_youtube_jar_and_drops_everything_else() {
        // `.youtube.com` is where the auth cookies actually live.
        let header = cookie_header(&youtube_cookie_pairs(vec![
            cookie("SAPISID=abc; Domain=.youtube.com"),
            cookie("SID=def; Domain=.youtube.com"),
            cookie("VISITOR_INFO1_LIVE=xyz; Domain=music.youtube.com"),
            cookie("SAPISID=notthisone; Domain=.google.com"),
            cookie("nodomain=1"),
        ]));
        assert_eq!(header, "SAPISID=abc; SID=def; VISITOR_INFO1_LIVE=xyz");
        // The check the harvest gates on: no SAPISID means sign-in silently gives up.
        assert_eq!(innertube::cookie_sapisid(&header), Some("abc"));
    }

    #[test]
    fn the_most_specific_domain_wins_a_name_collision() {
        let header = cookie_header(&youtube_cookie_pairs(vec![
            cookie("PREF=broad; Domain=.youtube.com"),
            cookie("PREF=specific; Domain=music.youtube.com"),
        ]));
        assert_eq!(header, "PREF=specific");
    }
}
