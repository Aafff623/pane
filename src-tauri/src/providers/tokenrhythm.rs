//! 基元律动 (tokenrhythm.studio) — web-session provider.
//!
//! Why a web session and not the API key: the `sk_…` key authenticates only
//! the `/v1` model namespace. Every console endpoint that reports account
//! money (`/api/wallet/summary`, `/api/api-keys/usage`, `/api/call-logs`)
//! lives under `/api` and answers 401 to the key — verified 2026-10-19
//! against the live site with a real key. The console authenticates with a
//! login session (phone + SMS; production is cookie-only, the Bearer path
//! in its bundle is mock-only), so Pane hosts the login itself.
//!
//! Flow: the user signs in inside a Pane-hosted webview (the captcha and
//! the SMS code are theirs to clear); the session then lives in Pane's own
//! WebView2 profile (`EBWebView`), which persists across restarts like a
//! browser staying signed in. Refresh reads the session cookie straight
//! out of that profile (the same Chromium cookie chain Doubao uses), caches
//! the header in the OS vault, and calls the console API with reqwest. No
//! cookie or token is ever written to Pane's JSON files or logs.
//!
//! Response shapes of `/api/wallet/summary` are UNVERIFIED (needs the
//! owner's first login) — the parser tries the plausible field names and
//! otherwise fails honestly with `待确认` instead of inventing numbers.

use super::chromium;
use super::{config_dir, error_chain, http, Metric, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

const SITE_ORIGIN: &str = "https://tokenrhythm.studio";
const LOGIN_URL: &str = "https://tokenrhythm.studio/login";
const ME_PATH: &str = "/api/auth/me";
const WALLET_PATH: &str = "/api/wallet/summary";
const VAULT_SCOPE: &str = "web/tokenrhythm";
const STORE_FILE: &str = "tokenrhythm_web.json";
const MAX_STORE_BYTES: u64 = 64 * 1024;
const MAX_BODY_BYTES: usize = 256 * 1024;
/// Pane's own WebView2 profile (the app's single browser profile).
const EBWEBVIEW_DIR: &str = "EBWebView";
const COOKIES_REL: &str = "Default/Network/Cookies";
const LOCAL_STATE_REL: &str = "Local State";
/// The console's CSRF cookie — its value rides as `X-CSRF-Token` on
/// mutations and is harmless on the GETs Pane makes.
const CSRF_COOKIE: &str = "tr_csrf";
const LOGIN_WINDOW: &str = "tr-login";
/// How long the login window waits for the user before giving up.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
const LOGIN_POLL: Duration = Duration::from_secs(2);

const SIGNED_OUT_HINT: &str =
    "基元律动会话已过期——在设置里重新网页登录。";
const FAMILY_ID: &str = "tokenrhythm";
const DISPLAY_NAME: &str = "基元律动";

#[derive(Serialize, Deserialize, Clone, Default)]
struct Store {
    #[serde(default)]
    cookie: String,
    #[serde(default)]
    csrf: String,
    #[serde(default)]
    saved_at: Option<String>,
}

/// What Settings shows — never the cookie itself.
#[derive(Serialize)]
pub struct LoginStatus {
    pub signed_in: bool,
    pub saved_at: Option<String>,
}

fn store_path() -> PathBuf {
    config_dir().join(STORE_FILE)
}

fn load_store() -> Store {
    crate::secretstore::load_session(VAULT_SCOPE, &store_path(), MAX_STORE_BYTES)
        .and_then(|doc| serde_json::from_value(doc).ok())
        .unwrap_or_default()
}

fn save_store(store: &Store) -> Result<(), String> {
    std::fs::create_dir_all(config_dir()).map_err(|e| format!("create config dir: {e}"))?;
    let doc = serde_json::to_value(store).map_err(|e| format!("serialize session: {e}"))?;
    crate::secretstore::save_session(VAULT_SCOPE, &store_path(), &doc)
}

pub fn status() -> LoginStatus {
    let store = load_store();
    LoginStatus {
        signed_in: !store.cookie.trim().is_empty(),
        saved_at: store.saved_at,
    }
}

pub fn clear() -> Result<(), String> {
    crate::secretstore::clear_session(VAULT_SCOPE, &store_path())
}

// ---------------------------------------------------------------------------
// Reading the session cookie out of Pane's own WebView2 profile
// ---------------------------------------------------------------------------

fn profile_dir() -> Option<PathBuf> {
    dirs::data_local_dir()
        .map(|d| d.join("com.jazii.pane"))
        .map(|d| d.join(EBWEBVIEW_DIR))
        .filter(|d| d.is_dir())
}

/// Deletes the copied Cookies DB on every exit path — a half-read copy of
/// every site's encrypted cookies must not linger in Pane's config dir.
struct SnapshotGuard(PathBuf);
impl Drop for SnapshotGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// All `tokenrhythm.studio` cookies from Pane's own profile, joined into a
/// request header, plus the CSRF value. `None` when the profile holds no
/// session for the site (the user never signed in, or the DB is locked).
fn extract_profile_cookie() -> Option<(String, String)> {
    let dir = profile_dir()?;
    let src = dir.join(COOKIES_REL);
    if !src.is_file() {
        return None;
    }
    let dst = config_dir().join("tokenrhythm_web_snapshot.db");
    std::fs::create_dir_all(config_dir()).ok();
    let _guard = SnapshotGuard(dst.clone());
    let snapshot_ok = (|| -> Result<(), String> {
        let src_conn =
            rusqlite::Connection::open_with_flags(&src, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| e.to_string())?;
        let mut dst_conn = rusqlite::Connection::open(&dst).map_err(|e| e.to_string())?;
        let backup =
            rusqlite::backup::Backup::new(&src_conn, &mut dst_conn).map_err(|e| e.to_string())?;
        backup
            .run_to_completion(5, Duration::from_millis(20), None)
            .map_err(|e| e.to_string())
    })()
    .is_ok();
    let snapshot_ok = snapshot_ok || std::fs::copy(&src, &dst).map(|_| ()).is_ok();
    if !snapshot_ok {
        return None;
    }
    let key = chromium::os_crypt_key(&dir).ok()?;
    let conn =
        rusqlite::Connection::open_with_flags(&dst, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT host_key, name, encrypted_value FROM cookies WHERE host_key LIKE '%tokenrhythm.studio'",
        )
        .ok()?;
    let rows: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .ok()?
        .flatten()
        .collect();
    let mut jar: Vec<String> = Vec::new();
    let mut csrf = String::new();
    for (_, name, blob) in rows {
        let Some(value) = chromium::decode_os_crypt(&blob, &key)
            .ok()
            .and_then(|plain| String::from_utf8(plain).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        if name == CSRF_COOKIE {
            csrf = value.clone();
        }
        jar.push(format!("{name}={value}"));
    }
    if jar.is_empty() {
        return None;
    }
    Some((jar.join("; "), csrf))
}

/// Caches whatever the profile currently holds — called after the login
/// window reports success and on demand when a refresh finds the cache
/// empty.
fn cache_profile_session() -> bool {
    let Some((cookie, csrf)) = extract_profile_cookie() else {
        return false;
    };
    let store = Store {
        cookie,
        csrf,
        saved_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    save_store(&store).is_ok()
}

// ---------------------------------------------------------------------------
// The console request
// ---------------------------------------------------------------------------

enum CallErr {
    /// The site refused the session (expired / signed out).
    Auth,
    Other(String),
}

async fn call_console(path: &str, cookie: &str, csrf: &str) -> Result<Value, CallErr> {
    let mut req = http()
        .get(format!("{SITE_ORIGIN}{path}"))
        .header("Cookie", cookie)
        .header("Referer", format!("{SITE_ORIGIN}/account"))
        .header("Origin", SITE_ORIGIN)
        .header("Accept", "application/json");
    if !csrf.is_empty() {
        req = req.header("X-CSRF-Token", csrf);
    }
    let resp = req.send().await.map_err(|e| CallErr::Other(error_chain(&e)))?;
    let status = resp.status().as_u16();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| CallErr::Other(format!("read response: {e}")))?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err(CallErr::Other("response too large".into()));
    }
    if status == 401 || status == 403 {
        return Err(CallErr::Auth);
    }
    if !(200..300).contains(&status) {
        return Err(CallErr::Other(format!("HTTP {status}")));
    }
    serde_json::from_slice(&bytes).map_err(|e| CallErr::Other(format!("response parse: {e}")))
}

// ---------------------------------------------------------------------------
// Wallet parsing (shape unverified — fail honestly, never invent)
// ---------------------------------------------------------------------------

/// Numbers a wallet body may carry, found by key name at any depth. Kept
/// pure for the tests.
fn wallet_numbers(doc: &Value) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    let mut stack: Vec<&Value> = vec![doc];
    while let Some(v) = stack.pop() {
        match v {
            Value::Object(map) => {
                for (k, child) in map {
                    if let Some(n) = child.as_f64() {
                        out.push((k.clone(), n));
                    } else if let Some(s) = child.as_str() {
                        // Balances often ride as decimal strings.
                        if let Ok(n) = s.trim().parse::<f64>() {
                            out.push((k.clone(), n));
                        }
                    }
                    stack.push(child);
                }
            }
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    out
}

/// The balance rows for a wallet body. `None` when nothing recognizable
/// is there — the caller reports 待确认 rather than showing invented rows.
fn metrics_from_wallet(doc: &Value) -> Option<Vec<Metric>> {
    let numbers = wallet_numbers(doc);
    if numbers.is_empty() {
        return None;
    }
    let avail = numbers
        .iter()
        .find(|(k, _)| {
            let k = k.to_ascii_lowercase();
            (k.contains("avail") || k.contains("balance") || k.contains("remain"))
                && !k.contains("frozen")
                && !k.contains("locked")
        })
        .map(|(_, n)| *n)?;
    let mut metrics = vec![Metric::text("可用额度", format!("¥{avail:.2}"))];
    if let Some((_, frozen)) = numbers.iter().find(|(k, _)| {
        let k = k.to_ascii_lowercase();
        k.contains("frozen") || k.contains("locked")
    }) {
        metrics.push(Metric::text("冻结额度", format!("¥{frozen:.2}")));
    }
    Some(metrics)
}

/// Keys the body actually carried — the 待确认 report names them so the
/// owner's first login can calibrate the parser.
fn wallet_key_digest(doc: &Value) -> String {
    wallet_numbers(doc)
        .into_iter()
        .map(|(k, _)| k)
        .take(8)
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/// The card's snapshot — the doubao-style no-arg shape the provider
/// dispatch table expects (family id and display name are this module's
/// own constants).
pub async fn snapshot() -> Snapshot {
    snapshot_for(FAMILY_ID, DISPLAY_NAME).await
}

/// The card's snapshot. Never `None` — the channel always reports
/// something (the signed-out hint when there is no session).
pub async fn snapshot_for(family_id: &str, display_name: &str) -> Snapshot {
    let mut store = load_store();
    if store.cookie.trim().is_empty() {
        // The profile may hold a session Pane has not cached yet (first run
        // after a web login in a previous app version, or a cache clear).
        if cache_profile_session() {
            store = load_store();
        }
    }
    if store.cookie.trim().is_empty() {
        return Snapshot::no_credentials(
            family_id,
            display_name,
            "尚未登录基元律动——在设置里完成网页登录。".into(),
        );
    }
    match call_console(WALLET_PATH, &store.cookie, &store.csrf).await {
        Ok(doc) => match metrics_from_wallet(&doc) {
            Some(metrics) => Snapshot::ok(family_id, display_name, None, metrics),
            None => Snapshot::error(
                family_id,
                display_name,
                format!(
                    "基元律动钱包响应结构未识别（待确认）：{}",
                    wallet_key_digest(&doc)
                ),
            ),
        },
        Err(CallErr::Auth) => {
            // The cached session may merely be stale — try the profile once
            // before declaring the session dead.
            if cache_profile_session() {
                let fresh = load_store();
                if fresh.cookie != store.cookie {
                    return match call_console(WALLET_PATH, &fresh.cookie, &fresh.csrf).await {
                        Ok(doc) => match metrics_from_wallet(&doc) {
                            Some(metrics) => {
                                Snapshot::ok(family_id, display_name, None, metrics)
                            }
                            None => Snapshot::error(
                                family_id,
                                display_name,
                                format!(
                                    "基元律动钱包响应结构未识别（待确认）：{}",
                                    wallet_key_digest(&doc)
                                ),
                            ),
                        },
                        Err(_) => Snapshot::no_credentials(
                            family_id,
                            display_name,
                            SIGNED_OUT_HINT.into(),
                        ),
                    };
                }
            }
            Snapshot::no_credentials(family_id, display_name, SIGNED_OUT_HINT.into())
        }
        Err(CallErr::Other(reason)) => Snapshot::error(
            family_id,
            display_name,
            format!("基元律动控制台: {reason}"),
        ),
    }
}

// ---------------------------------------------------------------------------
// Login window (async command surface — see lib.rs for the wrappers)
// ---------------------------------------------------------------------------

/// Set by the login window's in-page probe (server truth: `/api/auth/me`
/// answered 200). Watched by `login` alongside the cookie fallback.
#[allow(dead_code)]
static LOGIN_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Called from the login window's initialization script once the console
/// reports a session. Rust closes the window from the `login` loop.
#[allow(dead_code)]
pub fn login_ready() {
    LOGIN_READY.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Cookie NAMES the profile holds for the site — a new name appearing is
/// the fallback signal when the in-page invoke channel is unavailable.
fn profile_cookie_names() -> Vec<String> {
    let Some(dir) = profile_dir() else {
        return Vec::new();
    };
    let src = dir.join(COOKIES_REL);
    if !src.is_file() {
        return Vec::new();
    }
    let dst = config_dir().join("tokenrhythm_names_snapshot.db");
    std::fs::create_dir_all(config_dir()).ok();
    let _guard = SnapshotGuard(dst.clone());
    let ok = (|| -> Result<(), String> {
        let src_conn =
            rusqlite::Connection::open_with_flags(&src, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| e.to_string())?;
        let mut dst_conn = rusqlite::Connection::open(&dst).map_err(|e| e.to_string())?;
        let backup =
            rusqlite::backup::Backup::new(&src_conn, &mut dst_conn).map_err(|e| e.to_string())?;
        backup
            .run_to_completion(5, Duration::from_millis(20), None)
            .map_err(|e| e.to_string())
    })()
    .is_ok();
    let ok = ok || std::fs::copy(&src, &dst).map(|_| ()).is_ok();
    if !ok {
        return Vec::new();
    }
    let Ok(conn) =
        rusqlite::Connection::open_with_flags(&dst, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return Vec::new();
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT name FROM cookies WHERE host_key LIKE '%tokenrhythm.studio'",
    ) else {
        return Vec::new();
    };
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// Opens the sign-in window and waits until the console reports a session.
/// The user clears the captcha and the SMS code inside the window; the
/// session then lives in Pane's own WebView2 profile.
///
/// App-only: the parse-tests harness's `tauri` stub has no window builder,
/// and the login window is meaningless there.
#[cfg(not(feature = "harness"))]
pub async fn login(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager as _;
    if load_store().cookie.trim().is_empty() {
        cache_profile_session();
    }
    if !load_store().cookie.trim().is_empty() {
        return Ok(()); // already signed in
    }
    if let Some(w) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = w.set_focus();
        return Ok(());
    }
    let url = tauri::Url::parse(LOGIN_URL).map_err(|e| format!("login url: {e}"))?;
    LOGIN_READY.store(false, std::sync::atomic::Ordering::SeqCst);
    tauri::WebviewWindowBuilder::new(app, LOGIN_WINDOW, tauri::WebviewUrl::External(url))
        .title("登录基元律动")
        .inner_size(480.0, 720.0)
        .center()
        .initialization_script(LOGIN_PROBE_JS)
        .build()
        .map_err(|e| e.to_string())?;
    // Let the login page settle so its own first-party cookies land before
    // the fallback baseline is taken.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let baseline: std::collections::HashSet<String> =
        profile_cookie_names().into_iter().collect();
    let deadline = std::time::Instant::now() + LOGIN_TIMEOUT;
    loop {
        if LOGIN_READY.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        // Fallback: a cookie name the page did not have before.
        if profile_cookie_names()
            .into_iter()
            .any(|n| !baseline.contains(&n))
        {
            break;
        }
        if app.get_webview_window(LOGIN_WINDOW).is_none() {
            break; // the user closed it
        }
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(LOGIN_POLL).await;
    }
    if let Some(w) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = w.close();
    }
    // The webview has released the cookie DB — read and cache the session.
    if !cache_profile_session() {
        return Err("登录未完成（窗口已关闭）".into());
    }
    Ok(())
}

/// Injected into the login window: as soon as the console answers
/// `/api/auth/me` with 200 the page tells Rust to close the window. Falls
/// back to silence when the invoke channel is unavailable — the Rust-side
/// cookie poll then decides.
const LOGIN_PROBE_JS: &str = r#"
(() => {
  const done = () => {
    try { window.__TAURI_INTERNALS__.invoke('tokenrhythm_login_ready', {}); } catch (e) {}
  };
  const tick = async () => {
    try {
      const r = await fetch('/api/auth/me', { credentials: 'include' });
      if (r.ok) { done(); return; }
    } catch (e) {}
    setTimeout(tick, 1500);
  };
  tick();
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn wallet_rows_come_from_the_plausible_field_names() {
        let doc = json!({
            "code": 0,
            "data": {
                "availableBalance": "128.50",
                "frozenBalance": 20
            }
        });
        let metrics = metrics_from_wallet(&doc).expect("rows");
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].label, "可用额度");
        assert_eq!(metrics[0].value.as_deref(), Some("¥128.50"));
        assert_eq!(metrics[1].label, "冻结额度");
        assert_eq!(metrics[1].value.as_deref(), Some("¥20.00"));
    }

    #[test]
    fn an_unrecognized_shape_yields_no_rows_rather_than_invented_ones() {
        let doc = json!({ "code": 0, "data": { "userQuotaWidget": 7, "creditsLeft": 3 } });
        // "creditsLeft" contains neither avail/balance/remain → nothing.
        assert!(metrics_from_wallet(&doc).is_none());
        // The digest names the keys that WERE there, for calibration.
        let digest = wallet_key_digest(&doc);
        assert!(digest.contains("creditsLeft"), "{digest}");
    }

    #[test]
    fn a_body_without_numbers_yields_nothing() {
        assert!(metrics_from_wallet(&json!({"data":{}})).is_none());
    }
}
