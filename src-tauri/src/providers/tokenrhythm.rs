//! 基元律动 (tokenrhythm.studio) — web-session provider.
//!
//! Why a web session and not the API key: the `sk_…` key authenticates only
//! the `/v1` model namespace. Every console endpoint that reports account
//! money (`/api/wallet/summary`, `/api/api-keys/usage`, `/api/call-logs`)
//! lives under `/api` and answers 401 to the key — verified 2026-10-19
//! against the live site with a real key. The console authenticates with a
//! login session (phone + SMS, or the account password; production is
//! cookie-only, the Bearer path in its bundle is mock-only), so Pane hosts
//! the login itself.
//!
//! Each added account signs in through an isolated temporary WebView2
//! window. The cookie manager supplies httpOnly cookies after /api/auth/me
//! confirms the stable user id. Sessions are kept by login_accounts under
//! per-account OS-vault slots, with the standard file fallback when the
//! vault is unavailable; neither credentials nor response bodies are logged.
//! The pre-multi-account web/tokenrhythm slot migrates once, save-first.
//!
//! Response shapes of `/api/wallet/summary` were captured from the live
//! site on 2026-10-19 (owner's first login): the envelope is
//! `{code, message, data}` and the balance rides as the decimal string
//! `data.availableBalanceCny`. The parser still fails honestly with
//! `待确认` on an unrecognized shape instead of inventing numbers.

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

fn login_account(doc: &Value, cookie: String, csrf: String) -> Result<crate::login_accounts::LoginAccount, String> {
    if !signed_in_response(doc) { return Err("登录响应缺少账号身份".into()); }
    let data = &doc["data"];
    Ok(crate::login_accounts::LoginAccount {
        account_id: data["id"].as_str().unwrap().trim().to_string(),
        email: data["name"].as_str().unwrap_or_default().to_string(),
        label: String::new(),
        access_token: cookie,
        refresh_token: csrf,
        id_token: String::new(),
        expires_at: String::new(),
        added_at: chrono::Utc::now().timestamp(),
        extra: Value::Null,
    })
}

async fn record_session(cookie: String, csrf: String) -> Result<(), String> {
    let doc = call_console(ME_PATH, &cookie, &csrf).await
        .map_err(|_| "无法验证基元律动账号，请重试".to_string())?;
    let account = login_account(&doc, cookie, csrf)?;
    save_verified_account(account)
}

fn save_verified_account(account: crate::login_accounts::LoginAccount) -> Result<(), String> {
    let _mutation = crate::account_lifecycle::mutation_lock().lock().unwrap_or_else(|e| e.into_inner());
    crate::login_accounts::record_login(FAMILY_ID, account)?;
    // Save first, then retire the single-session source so deletion cannot
    // resurrect it. No browser profile is imported automatically afterwards.
    clear()
}

pub(crate) async fn migrate_legacy_session() -> Result<(), String> {
    static MIGRATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = MIGRATION.lock().await;
    let store = load_store();
    if store.cookie.trim().is_empty() { return Ok(()); }
    match call_console(ME_PATH, &store.cookie, &store.csrf).await {
        Ok(doc) => {
            let account = login_account(&doc, store.cookie, store.csrf)?;
            save_verified_account(account)
        }
        Err(CallErr::Auth) => clear(),
        Err(CallErr::Other(_)) => Err("原基元律动会话迁移暂时失败，请联网后重试".into()),
    }
}

pub fn status() -> LoginStatus {
    let store = load_store();
    LoginStatus {
        signed_in: !crate::login_accounts::load(FAMILY_ID).is_empty() || !store.cookie.trim().is_empty(),
        saved_at: store.saved_at,
    }
}

pub fn clear() -> Result<(), String> {
    crate::secretstore::clear_session(VAULT_SCOPE, &store_path())
}

// ---------------------------------------------------------------------------
// Reading the session out of Pane's own WebView2 profile
// ---------------------------------------------------------------------------

/// Only the isolated login window owns this login attempt's cookie jar.
#[cfg(not(feature = "harness"))]
fn session_window(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    use tauri::Manager as _;
    app.get_webview_window(LOGIN_WINDOW)
}

/// The site's cookies as a request header, plus the CSRF value, read through
/// the browser's own cookie manager. `None` when no webview holds a session
/// for the site yet (the user never signed in, or the manager is
/// unavailable).
///
/// The profile's cookie DB is NOT readable here: the browser process keeps
/// it locked with no share flags for as long as the app runs, and the
/// session value is httpOnly, so page script cannot see it either. The
/// cookie manager is the only channel that reports it.
#[cfg(not(feature = "harness"))]
async fn extract_session(app: &tauri::AppHandle) -> Option<(String, String)> {
    let window = session_window(app)?;
    let cookies = crate::platform::read_webview_cookies(&window, SITE_ORIGIN).await;
    if cookies.is_empty() {
        return None;
    }
    let mut jar: Vec<String> = Vec::new();
    let mut csrf = String::new();
    for (name, value) in cookies {
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
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

#[cfg(feature = "harness")]
async fn extract_session(_app: &tauri::AppHandle) -> Option<(String, String)> {
    // No webviews in the harness — the login window is meaningless there.
    None
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
    if let Err(reason) = migrate_legacy_session().await {
        return Snapshot::error(FAMILY_ID, DISPLAY_NAME, reason);
    }
    if let Some(login) = crate::login_accounts::load(FAMILY_ID).into_iter().next() {
        snapshot_with_login_as(login, FAMILY_ID).await
    } else {
        Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME,
            "尚未登录基元律动——在设置里添加账号并完成网页登录。".into())
    }
}

pub fn login_card_name(login: &crate::login_accounts::LoginAccount) -> String {
    let label = if login.label.trim().is_empty() { &login.email } else { &login.label };
    if label.trim().is_empty() { DISPLAY_NAME.into() } else { format!("{DISPLAY_NAME} — {label}") }
}

pub async fn snapshot_with_login(login: crate::login_accounts::LoginAccount) -> Snapshot {
    let id = crate::login_accounts::card_id_for_account(FAMILY_ID, &login);
    snapshot_with_login_as(login, &id).await
}

pub async fn snapshot_with_login_as(login: crate::login_accounts::LoginAccount, id: &str) -> Snapshot {
    let name = login_card_name(&login);
    match call_console(WALLET_PATH, &login.access_token, &login.refresh_token).await {
        Ok(doc) => match metrics_from_wallet(&doc) {
            Some(metrics) => Snapshot::ok(id, &name, None, metrics),
            None => Snapshot::error(id, &name, format!(
                "基元律动钱包响应结构未识别（待确认）：{}", wallet_key_digest(&doc))),
        },
        // Retain the account identity on expiry: the user can re-login or
        // delete it explicitly. Never clear another account's session.
        Err(CallErr::Auth) => Snapshot::no_credentials(id, &name, SIGNED_OUT_HINT.into()),
        Err(CallErr::Other(reason)) => Snapshot::error(id, &name, format!("基元律动控制台: {reason}")),
    }
}

// ---------------------------------------------------------------------------
// Login window (async command surface — see lib.rs for the wrappers)
// ---------------------------------------------------------------------------

/// True when `/api/auth/me` answers with a real account: the envelope's
/// `code` is 0 and `data.id` names a user. A guest or expired session
/// answers 401 or a non-zero code.
fn signed_in_response(doc: &Value) -> bool {
    doc.get("code").and_then(Value::as_i64) == Some(0)
        && doc
            .get("data")
            .and_then(|d| d.get("id"))
            .and_then(Value::as_str)
            .is_some_and(|id| !id.trim().is_empty())
}

/// Whether a cookie jar carries a live session. Read straight from the
/// site — the same truth the card's own refresh uses — so detection cannot
/// be fooled by cookie names or timing.
async fn session_live(cookie: &str, csrf: &str) -> bool {
    match call_console(ME_PATH, cookie, csrf).await {
        Ok(doc) => signed_in_response(&doc),
        Err(_) => false,
    }
}

/// Opens an isolated sign-in window for adding or reauthenticating an
/// account. A verified server id upserts only that account; closing or
/// timing out leaves every saved account intact. Completion is polled on
/// the Rust side because remote content cannot invoke app commands.
///
/// App-only: the parse-tests harness's `tauri` stub has no window builder,
/// and the login window is meaningless there.
#[cfg(not(feature = "harness"))]
pub async fn login(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager as _;
    static LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOGIN_LOCK.try_lock().map_err(|_| "基元律动登录正在进行，请先完成或关闭登录窗口".to_string())?;
    migrate_legacy_session().await?;
    if let Some(w) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = w.set_focus();
        return Err("基元律动登录窗口已打开，请先完成或关闭该窗口".into());
    }
    let url = tauri::Url::parse(LOGIN_URL).map_err(|e| format!("login url: {e}"))?;
    tauri::WebviewWindowBuilder::new(app, LOGIN_WINDOW, tauri::WebviewUrl::External(url))
        .title("登录基元律动")
        .incognito(true)
        .inner_size(480.0, 720.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    // Let the login page settle so its own first-party cookies land before
    // the first check.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let deadline = std::time::Instant::now() + LOGIN_TIMEOUT;
    // The jar that answered with a live session — cached after the window
    // closes, so no second read is needed.
    let mut session: Option<(String, String)> = None;
    while session.is_none() {
        if let Some((cookie, csrf)) = extract_session(app).await {
            if session_live(&cookie, &csrf).await {
                session = Some((cookie, csrf));
                break;
            }
        }
        if app.get_webview_window(LOGIN_WINDOW).is_none() {
            return Err("登录未完成（窗口已关闭）".into()); // the user closed it
        }
        if std::time::Instant::now() > deadline {
            if let Some(w) = app.get_webview_window(LOGIN_WINDOW) {
                let _ = w.close();
            }
            return Err("登录超时（窗口已关闭）".into());
        }
        tokio::time::sleep(LOGIN_POLL).await;
    }
    if let Some(w) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = w.close();
    }
    let Some((cookie, csrf)) = session else {
        return Err("登录未完成（窗口已关闭）".into());
    };
    record_session(cookie, csrf).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn multiple_web_logins_use_server_identity_and_relogin_keeps_label() {
        let me = json!({"code":0,"data":{"id":"synthetic-web-user","name":"User A"}});
        let mut a = login_account(&me, "old-cookie".into(), "csrf-a".into()).unwrap();
        a.label = "My account".into();
        let id = crate::login_accounts::card_id_for_account(FAMILY_ID, &a);
        let refreshed = login_account(&me, "rotated-cookie".into(), "csrf-b".into()).unwrap();
        assert_eq!(id, crate::login_accounts::card_id_for_account(FAMILY_ID, &refreshed));
        let mut rows = vec![a];
        crate::login_accounts::upsert(&mut rows, refreshed);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "My account");
        assert_eq!(rows[0].access_token, "rotated-cookie");
        let other = login_account(&json!({"code":0,"data":{"id":"synthetic-web-other"}}),
            "other-cookie".into(), "".into()).unwrap();
        crate::login_accounts::upsert(&mut rows, other);
        assert_eq!(rows.len(), 2);
        assert_ne!(id, crate::login_accounts::card_id_for_account(FAMILY_ID, &rows[1]));
        let public = serde_json::to_string(&crate::login_accounts::account_rows(FAMILY_ID, &rows)).unwrap();
        assert!(!public.contains("rotated-cookie"));
        assert!(!public.contains("other-cookie"));
        assert!(login_account(&json!({"code":0,"data":{}}), "cookie".into(), "".into()).is_err());
    }

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

    #[test]
    fn the_real_me_body_reads_as_signed_in() {
        // Shape captured from the live site on 2026-10-19 (first login).
        let doc = json!({
            "code": 0,
            "message": "ok",
            "data": {
                "id": "bcacd231-3fd9-4a5b-93c9-73f8256d6cfd",
                "name": "17836372725",
                "status": "active"
            }
        });
        assert!(signed_in_response(&doc));
    }

    #[test]
    fn a_guest_or_error_body_is_not_signed_in() {
        assert!(!signed_in_response(&json!({"code": 401, "message": "unauthorized"})));
        assert!(!signed_in_response(&json!({"code": 0, "data": null})));
        assert!(!signed_in_response(&json!({"code": 0, "data": {"id": ""}})));
    }

    #[test]
    fn the_real_wallet_body_yields_the_available_balance() {
        // Shape captured from the live site on 2026-10-19 (first login).
        let doc = json!({
            "code": 0,
            "message": "ok",
            "data": {
                "currency": "CNY",
                "availableBalanceCny": "142.31105772",
                "giftAvailableCny": "142.31105772",
                "giftLockedCny": "0.00000000",
                "rechargeBalanceCny": "0.00000000",
                "debtBalanceCny": "0.00000000",
                "frozenBalanceCny": "0.00000000",
                "giftTotalCny": "142.31105772",
                "giftStatus": "active",
                "voidedGiftCny": "0.00000000",
                "asOf": "2026-10-19T07:23:02.155Z"
            }
        });
        let metrics = metrics_from_wallet(&doc).expect("rows");
        assert_eq!(metrics[0].label, "可用额度");
        assert_eq!(metrics[0].value.as_deref(), Some("¥142.31"));
    }
}
