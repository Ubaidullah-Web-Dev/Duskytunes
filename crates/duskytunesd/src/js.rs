//! `JsBridge` over an offscreen WebKitGTK view per label. Same contract as the Tauri host's
//! `webview.rs`: sync evals round-trip once; async work parks its result in `window.__slots`.
//!
//! Unlike the Tauri port, a `webkit2gtk::WebView` is `!Send`, so it can never be stored in the
//! `Arc<Mutex<..>>` the plan sketched (that would make `GtkJs` `!Send` and fail `JsBridge`'s
//! `Send + Sync` bound). The live views therefore live in a GTK-thread-local registry and never
//! leave that thread; every operation is marshalled onto it through `Gtk::call`. A separate,
//! `Send` set of live labels backs the synchronous `exists()`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use javascriptcore::ValueExt;
use duskytunes_core::host::{JsBridge, JsError, JsSession};
use serde_json::Value;
use webkit2gtk::{gio, WebView, WebViewExt};

use crate::gtk_thread::Gtk;

thread_local! {
    /// Every live offscreen view, owned entirely by the GTK thread.
    static VIEWS: RefCell<HashMap<String, WebView>> = RefCell::new(HashMap::new());
}

/// What an eval funnels back through the oneshot: the JSON of the value, the view being gone, or an
/// engine error — so the async side maps each to the right `JsError`.
enum EvalOut {
    Json(String),
    Gone,
    Failed(String),
}

pub struct GtkJs {
    gtk: Gtk,
    /// Labels whose view is currently registered on the GTK thread. Mirrors `VIEWS` so `exists()`
    /// can answer without hopping threads.
    labels: Arc<Mutex<HashSet<String>>>,
}

impl GtkJs {
    pub fn new(gtk: Gtk) -> Self {
        GtkJs { gtk, labels: Arc::new(Mutex::new(HashSet::new())) }
    }
}

#[derive(Clone)]
pub struct Session {
    gtk: Gtk,
    label: String,
    labels: Arc<Mutex<HashSet<String>>>,
}

#[async_trait::async_trait]
impl JsBridge for GtkJs {
    async fn create(
        &self,
        label: &str,
        harness_html: &str,
        init_script: &str,
    ) -> Result<Box<dyn JsSession>, JsError> {
        self.reclaim(label).await;
        let init = format!(
            "window.__jserr=null;window.__slots={{}};\
             window.addEventListener('error',function(e){{window.__jserr=String((e&&e.message)||e);}});\
             window.onunhandledrejection=function(e){{window.__jserr=String((e.reason&&e.reason.message)||e.reason);}};\n{init_script}"
        );
        let data_url = format!("data:text/html,{}", urlencoding::encode(harness_html));
        let label_s = label.to_owned();
        let labels = self.labels.clone();
        self.gtk
            .call(move || {
                use gtk::prelude::*;
                use webkit2gtk::{
                    UserContentInjectedFrames, UserContentManager, UserContentManagerExt,
                    UserScript, UserScriptInjectionTime,
                };
                let ucm = UserContentManager::new();
                ucm.add_script(&UserScript::new(
                    &init,
                    UserContentInjectedFrames::TopFrame,
                    UserScriptInjectionTime::Start,
                    &[],
                    &[],
                ));
                let view = WebView::builder().user_content_manager(&ucm).build();
                // WebKit's aborting exit-teardown registers when the first view is built; register
                // our clean-exit shim now so it runs before it.
                crate::gtk_thread::install_clean_exit();
                let win = gtk::OffscreenWindow::new();
                win.add(&view);
                win.show_all();
                view.load_uri(&data_url);
                VIEWS.with(|v| v.borrow_mut().insert(label_s.clone(), view));
                labels.lock().unwrap().insert(label_s);
            })
            .await;
        let session =
            Session { gtk: self.gtk.clone(), label: label.to_owned(), labels: self.labels.clone() };
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            if matches!(
                session
                    .eval_json("location.protocol==='data:'".into(), Duration::from_millis(800))
                    .await,
                Ok(Value::Bool(true))
            ) {
                tracing::info!(label, "js bridge OK");
                return Ok(Box::new(session));
            }
            if Instant::now() >= deadline {
                session.destroy();
                return Err(JsError::Timeout(Duration::from_secs(12)));
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }

    async fn reclaim(&self, label: &str) {
        let label = label.to_owned();
        let labels = self.labels.clone();
        self.gtk
            .call(move || {
                if let Some(view) = VIEWS.with(|v| v.borrow_mut().remove(&label)) {
                    use gtk::prelude::*;
                    if let Some(w) = view.toplevel() {
                        unsafe { w.destroy() };
                    }
                }
                labels.lock().unwrap().remove(&label);
            })
            .await;
    }
}

#[async_trait::async_trait]
impl JsSession for Session {
    fn eval(&self, js: &str) -> Result<(), JsError> {
        if !self.labels.lock().unwrap().contains(&self.label) {
            return Err(JsError::Gone(self.label.clone()));
        }
        let js = js.to_owned();
        let label = self.label.clone();
        // Fire and forget on the GTK thread; the result is discarded like Tauri's `eval`.
        let _ = self.gtk.call(move || {
            VIEWS.with(|v| {
                if let Some(view) = v.borrow().get(&label) {
                    view.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
                }
            });
        });
        Ok(())
    }

    async fn eval_json(&self, js: String, timeout: Duration) -> Result<Value, JsError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<EvalOut>();
        let label = self.label.clone();
        self.gtk
            .call(move || {
                VIEWS.with(|v| match v.borrow().get(&label) {
                    None => {
                        let _ = tx.send(EvalOut::Gone);
                    }
                    Some(view) => {
                        view.evaluate_javascript(
                            &js,
                            None,
                            None,
                            None::<&gio::Cancellable>,
                            move |res| {
                                let out = match res {
                                    Ok(v) => EvalOut::Json(
                                        v.to_json(0)
                                            .map(|g| g.to_string())
                                            .unwrap_or_else(|| "null".into()),
                                    ),
                                    Err(e) => EvalOut::Failed(e.to_string()),
                                };
                                let _ = tx.send(out);
                            },
                        );
                    }
                });
            })
            .await;
        let out = tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| JsError::Timeout(timeout))?
            .map_err(|_| JsError::Eval("callback dropped".into()))?;
        let raw = match out {
            EvalOut::Gone => return Err(JsError::Gone(self.label.clone())),
            EvalOut::Failed(e) => return Err(JsError::Eval(e)),
            EvalOut::Json(raw) => raw,
        };
        if raw.is_empty() || raw == "null" || raw == "undefined" {
            return Ok(Value::Null);
        }
        serde_json::from_str(&raw).map_err(|e| JsError::Eval(format!("bad json {raw:?}: {e}")))
    }

    async fn call_async(&self, expr: &str, timeout: Duration) -> Result<Value, JsError> {
        // Verbatim port of `webview.rs` `call_async` + `poll_json` on top of `eval`/`eval_json`.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.eval(&format!(
            "(async()=>{{try{{window.__slots['{id}']={{ok:1,v:await ({expr})}};}}\
             catch(e){{window.__slots['{id}']={{ok:0,e:String((e&&e.message)||e)}};}}}})();"
        ))?;
        let deadline = Instant::now() + timeout;
        let slot = loop {
            match self
                .eval_json(format!("(window.__slots['{id}']||null)"), Duration::from_secs(3))
                .await
            {
                Ok(v) if !v.is_null() => break v,
                Ok(_) => {}
                Err(JsError::Gone(l)) => return Err(JsError::Gone(l)),
                Err(_) => {}
            }
            if let Ok(Value::String(msg)) =
                self.eval_json("window.__jserr||null".into(), Duration::from_secs(3)).await
            {
                return Err(JsError::BadEnvironment(msg));
            }
            if Instant::now() >= deadline {
                return Err(JsError::Timeout(timeout));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        let _ = self.eval(&format!("delete window.__slots['{id}'];"));
        if slot.get("ok").and_then(Value::as_i64) == Some(1) {
            Ok(slot.get("v").cloned().unwrap_or(Value::Null))
        } else {
            Err(JsError::Eval(
                slot.get("e").and_then(Value::as_str).unwrap_or("async call failed").to_owned(),
            ))
        }
    }

    fn exists(&self) -> bool {
        self.labels.lock().unwrap().contains(&self.label)
    }

    fn destroy(&self) {
        let label = self.label.clone();
        let labels = self.labels.clone();
        let _ = self.gtk.call(move || {
            if let Some(view) = VIEWS.with(|v| v.borrow_mut().remove(&label)) {
                use gtk::prelude::*;
                if let Some(w) = view.toplevel() {
                    unsafe { w.destroy() };
                }
            }
            labels.lock().unwrap().remove(&label);
        });
    }

    fn clone_session(&self) -> Box<dyn JsSession> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a Wayland/X11 display: `cargo test -p duskytunesd -- --ignored js_round_trip`.
    #[tokio::test]
    #[ignore]
    async fn js_round_trip() {
        let js = GtkJs::new(Gtk::start());
        let s = js
            .create("t", "<!doctype html><html><body></body></html>", "window.x=41;")
            .await
            .unwrap();
        assert_eq!(
            s.eval_json("window.x+1".into(), Duration::from_secs(3)).await.unwrap(),
            Value::from(42)
        );
        assert_eq!(
            s.call_async("Promise.resolve('ok')", Duration::from_secs(3)).await.unwrap(),
            Value::from("ok")
        );
        assert!(s.exists());
        s.destroy();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!s.exists());
    }
}
