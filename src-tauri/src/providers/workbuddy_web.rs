//! WorkBuddy's web-session fallback — the third route (ticket 06).
//!
//! Route order on the card: (1) the desktop app's own auth store
//! (`workbuddy_local`, no ceremony); (2) the Pane sign-in from the auth
//! center (the card's 去登录 jump); (3) THIS — a `www.workbuddy.cn` web
//! session the user explicitly imported. The channel never runs unless
//! the user enabled it and a cookie is on file: "未启用时该通道完全不发
//! 请求" (ticket 06 acceptance).
//!
//! The site binds the session to the browser's User-Agent — any other UA,
//! including another Chrome major, answers 401 (spec D4). So the request
//! UA is rebuilt from the local Chrome major, and a rejection retries
//! once with the previous major before the card reports an expired
//! session.
//!
//! Cookie import is an explicit action: either read Chrome's own cookie
//! store (Chromium `Cookies` SQLite → `v10` AES-256-GCM, the os_crypt key
//! unwrapped through the platform seam — the same chain Doubao uses), or
//! accept a pasted `name=value; …` header. Nothing here scans browsers on
//! its own.
//!
//! Endpoint shape (spec D4, magpie's wbResourceSummary): POST
//! `/billing/meter/get-user-resource-summary` (root path, NOT under
//! `/v2`) with an empty JSON body; the envelope is `{code, msg, data}`
//! and the credits live in `data.Packages[]` with the same field names
//! the token route aggregates (`CycleTotalCapacity` / `CycleRemainCapacity`
//! / `CycleUsedCapacity`, decimal strings). Reset-time enrichment via the
//! packages-list endpoint is deliberately not wired yet — when no cycle
//! end is present the balance is kept, which is exactly the accepted
//! degrade.

use super::codebuddy::{credits_face_from_items, credits_face_metrics};
use super::{config_dir, error_chain, http, read_small_text, Metric, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

const SUMMARY_URL: &str = "https://www.workbuddy.cn/billing/meter/get-user-resource-summary";
const SITE_ORIGIN: &str = "https://www.workbuddy.cn";
const STORE_FILE: &str = "workbuddy_web.json";
const MAX_STORE_BYTES: u64 = 256 * 1024;
const MAX_BODY_BYTES: usize = 256 * 1024;
const MAX_LOCAL_STATE_BYTES: u64 = 256 * 1024;
const CHROME_USER_DATA: &str = "Google/Chrome/User Data";
const CHROME_COOKIES_REL: &str = "Default/Network/Cookies";
const CHROME_VERSION_REL: &str = "Last Version";
/// Well-formed UA when no local Chrome is detectable (pasted cookie on a
/// machine without Chrome): a mismatched binding then surfaces as the
/// honest session-expired message rather than a fabricated success.
const FALLBACK_CHROME_MAJOR: u32 = 131;

const EXPIRED_HINT: &str = "WorkBuddy web session expired — import a fresh cookie in Customize, or sign in from the auth center.";

#[derive(Serialize, Deserialize, Clone, Default)]
struct Store {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    cookie: String,
    #[serde(default)]
    chrome_major: Option<u32>,
    #[serde(default)]
    saved_at: Option<String>,
}

/// What the Customize panel shows — never the cookie itself.
#[derive(Serialize)]
pub struct WebStatus {
    pub enabled: bool,
    pub has_cookie: bool,
    pub chrome_major: Option<u32>,
    pub saved_at: Option<String>,
}

fn store_path() -> PathBuf {
    config_dir().join(STORE_FILE)
}

fn load_store() -> Store {
    let Ok(raw) = read_small_text(&store_path(), MAX_STORE_BYTES, "workbuddy web session") else {
        return Store::default();
    };
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

fn save_store(store: &Store) -> Result<(), String> {
    std::fs::create_dir_all(config_dir()).map_err(|e| format!("create config dir: {e}"))?;
    let raw = serde_json::to_string_pretty(store).map_err(|e| format!("serialize session: {e}"))?;
    std::fs::write(store_path(), raw).map_err(|e| format!("save session: {e}"))
}

pub fn status() -> WebStatus {
    let store = load_store();
    WebStatus {
        enabled: store.enabled,
        has_cookie: !store.cookie.trim().is_empty(),
        chrome_major: store.chrome_major,
        saved_at: store.saved_at.clone(),
    }
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let mut store = load_store();
    store.enabled = enabled;
    save_store(&store)
}

pub fn clear() -> Result<(), String> {
    let _ = std::fs::remove_file(store_path());
    Ok(())
}

/// Stores a pasted `name=value; …` header. The Chrome major is re-detected
/// so the UA matches what the browser would send.
pub fn save_cookie(cookie: String) -> Result<(), String> {
    let cookie = cookie.trim().to_string();
    if cookie.is_empty() {
        return Err("the cookie is empty".into());
    }
    if cookie.contains('\n') || cookie.contains('\r') {
        return Err("the cookie must be a single header line".into());
    }
    let mut store = load_store();
    store.cookie = cookie;
    store.chrome_major = chrome_major();
    store.saved_at = Some(chrono::Utc::now().to_rfc3339());
    save_store(&store)
}

/// Explicit Chrome import: reads Chrome's own cookie store for
/// `*.workbuddy.cn` and saves the joined header. Chrome may keep the DB
/// locked while running — the backup API usually still reads it.
pub fn import_from_chrome() -> Result<u32, String> {
    let header = extract_chrome_cookie()?;
    let major = chrome_major().unwrap_or(FALLBACK_CHROME_MAJOR);
    let mut store = load_store();
    store.cookie = header;
    store.chrome_major = Some(major);
    store.saved_at = Some(chrono::Utc::now().to_rfc3339());
    save_store(&store)?;
    Ok(major)
}

// ---------------------------------------------------------------------------
// Chrome cookie extraction (explicit user action only)
// ---------------------------------------------------------------------------

fn chrome_user_data() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join(CHROME_USER_DATA))
}

/// Chrome's own major version, from the `Last Version` file it keeps.
fn chrome_major() -> Option<u32> {
    let raw = std::fs::read_to_string(chrome_user_data()?.join(CHROME_VERSION_REL)).ok()?;
    raw.trim().split('.').next()?.parse::<u32>().ok()
}

/// Deletes the copied Cookies DB on every exit path — a half-read copy of
/// every site's encrypted cookies must not linger in Pane's config dir.
struct SnapshotGuard(PathBuf);
impl Drop for SnapshotGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn extract_chrome_cookie() -> Result<String, String> {
    let dir = chrome_user_data().ok_or("Chrome's user-data folder was not found")?;
    let src = dir.join(CHROME_COOKIES_REL);
    if !src.is_file() {
        return Err("Chrome has no cookie database for its default profile".into());
    }
    let dst = config_dir().join("workbuddy_web_snapshot.db");
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
        return Err("Chrome's cookie database is locked — close Chrome and try again".into());
    }
    let key = super::chromium::os_crypt_key(&dir)?;
    let conn =
        rusqlite::Connection::open_with_flags(&dst, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("open cookie snapshot: {e}"))?;
    let mut stmt = conn
        .prepare("SELECT host_key, name, encrypted_value FROM cookies WHERE host_key LIKE '%workbuddy.cn'")
        .map_err(|e| format!("query cookies: {e}"))?;
    let rows: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(|e| format!("read cookies: {e}"))?
        .flatten()
        .collect();

    let mut jar: Vec<(bool, String, String)> = Vec::new(); // (site-wide, name, value)
    for (host, name, blob) in rows {
        let Some(value) = decrypt_chrome_cookie(&blob, &key) else {
            continue;
        };
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        let site_wide = host.starts_with('.');
        if let Some(slot) = jar.iter_mut().find(|(_, n, _)| n == &name) {
            if site_wide && !slot.0 {
                *slot = (site_wide, name, value);
            }
        } else {
            jar.push((site_wide, name, value));
        }
    }
    if jar.is_empty() {
        return Err(
            "Chrome has no www.workbuddy.cn cookies — sign in on the site in Chrome first".into(),
        );
    }
    Ok(jar
        .into_iter()
        .map(|(_, n, v)| format!("{n}={v}"))
        .collect::<Vec<_>>()
        .join("; "))
}

/// Standard Chromium os_crypt: `v10` + nonce + GCM ciphertext; the
/// plaintext is the cookie value as-is (the 32-byte header is Doubao's
/// own quirk, not Chrome's). App-bound `v20` blobs fail the decode and are
/// skipped — the paste path remains.
fn decrypt_chrome_cookie(blob: &[u8], key: &[u8; 32]) -> Option<String> {
    let plain = super::chromium::decode_os_crypt(blob, key).ok()?;
    let text = String::from_utf8(plain).ok()?;
    super::chromium::looks_like_text(&text).then_some(text)
}

// ---------------------------------------------------------------------------
// The web request
// ---------------------------------------------------------------------------

/// The UA the browser that owns the cookie would send.
fn chrome_ua(major: u32) -> String {
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{major}.0.0.0 Safari/537.36"
    )
}

/// Majors to try: the detected one, then the previous (the binding is
/// exact-major, so an updated Chrome on the same profile answers 401).
fn ua_major_ladder(stored: Option<u32>) -> Vec<u32> {
    match stored {
        Some(major) if major > 1 => vec![major, major - 1],
        Some(major) => vec![major],
        None => vec![FALLBACK_CHROME_MAJOR],
    }
}

enum SummaryErr {
    /// The site refused the session (UA/session mismatch) — try the
    /// previous major, then report expired.
    Auth,
    Other(String),
}

async fn fetch_summary(cookie: &str, ua: &str) -> Result<Value, SummaryErr> {
    let resp = http()
        .post(SUMMARY_URL)
        .header("User-Agent", ua)
        .header("Cookie", cookie)
        .header("Referer", format!("{SITE_ORIGIN}/"))
        .header("Origin", SITE_ORIGIN)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body("{}")
        .send()
        .await
        .map_err(|e| SummaryErr::Other(error_chain(&e)))?;
    let status = resp.status().as_u16();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| SummaryErr::Other(format!("read response: {e}")))?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err(SummaryErr::Other("response too large".into()));
    }
    classify_web_response(status, &bytes)
}

/// Status/body classification, kept pure for the tests: 401/403 is the
/// session verdict, a non-zero business code is a plain failure.
fn classify_web_response(status: u16, bytes: &[u8]) -> Result<Value, SummaryErr> {
    if status == 401 || status == 403 {
        return Err(SummaryErr::Auth);
    }
    if !(200..300).contains(&status) {
        return Err(SummaryErr::Other(format!("HTTP {status}")));
    }
    let doc: Value = serde_json::from_slice(bytes)
        .map_err(|e| SummaryErr::Other(format!("response parse: {e}")))?;
    match doc.get("code").and_then(Value::as_i64) {
        Some(0) => Ok(doc),
        Some(code) => Err(SummaryErr::Other(format!(
            "code {code}{}",
            doc.get("msg")
                .and_then(Value::as_str)
                .filter(|m| !m.is_empty())
                .map(|m| format!(": {m}"))
                .unwrap_or_default()
        ))),
        None => Err(SummaryErr::Other("response has no code".into())),
    }
}

/// The credits packages of a summary body, wherever the gateway nested
/// them.
fn web_packs(data: &Value) -> Vec<&Value> {
    for path in ["/Packages", "/packages", "/data/Packages", "/data/packages"] {
        if let Some(items) = data.pointer(path).and_then(Value::as_array) {
            return items.iter().collect();
        }
    }
    Vec::new()
}

/// The card row set for a web summary — same basis as the token route's
/// credits face (spec D6): meter, Unlimited / Left text degrade, Reserved.
fn metrics_from_summary(data: &Value, now_ms: i64) -> (Vec<Metric>, Option<String>) {
    let packs = web_packs(data);
    let mut metrics = Vec::new();
    let mut plan = None;
    if let Some(face) = credits_face_from_items(&packs, now_ms) {
        metrics.extend(credits_face_metrics(&face));
        plan = face.plan.clone();
    }
    if plan.is_none() {
        plan = ["SubscriptionPackageName", "SubscriptionPackageCode"]
            .iter()
            .find_map(|k| {
                data.get(*k)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            });
    }
    (metrics, plan)
}



/// The web route's snapshot, or `None` when the channel is off / has no
/// cookie — in which case the caller falls back to the sign-in hint and
/// nothing was requested.
pub async fn snapshot(family_id: &str, display_name: &str) -> Option<Snapshot> {
    let store = load_store();
    if !store.enabled || store.cookie.trim().is_empty() {
        return None;
    }
    let majors = ua_major_ladder(store.chrome_major);
    for (index, major) in majors.iter().enumerate() {
        match fetch_summary(&store.cookie, &chrome_ua(*major)).await {
            Ok(doc) => {
                let data = doc.get("data").cloned().unwrap_or(Value::Null);
                let now_ms = chrono::Utc::now().timestamp_millis();
                let (metrics, plan) = metrics_from_summary(&data, now_ms);
                if metrics.is_empty() {
                    return Some(Snapshot::error(
                        family_id,
                        display_name,
                        "WorkBuddy web session returned no quota data".into(),
                    ));
                }
                return Some(Snapshot::ok(family_id, display_name, plan, metrics));
            }
            Err(SummaryErr::Auth) => {
                // Exact-major binding: one retry with the previous major.
                if index + 1 < majors.len() {
                    continue;
                }
                return Some(Snapshot::no_credentials(family_id, display_name, EXPIRED_HINT));
            }
            Err(SummaryErr::Other(reason)) => {
                return Some(Snapshot::error(
                    family_id,
                    display_name,
                    format!("WorkBuddy web session: {reason}"),
                ));
            }
        }
    }
    Some(Snapshot::no_credentials(family_id, display_name, EXPIRED_HINT))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ua_carries_the_exact_major() {
        assert!(chrome_ua(131).contains("Chrome/131.0.0.0"));
        assert!(chrome_ua(131).starts_with("Mozilla/5.0"));
    }

    #[test]
    fn the_ladder_tries_the_detected_major_then_the_previous() {
        assert_eq!(ua_major_ladder(Some(131)), vec![131, 130]);
        assert_eq!(ua_major_ladder(Some(1)), vec![1]);
        assert_eq!(ua_major_ladder(None), vec![FALLBACK_CHROME_MAJOR]);
    }

    #[test]
    fn session_rejection_and_business_errors_classify_apart() {
        assert!(matches!(
            classify_web_response(401, b"{}"),
            Err(SummaryErr::Auth)
        ));
        assert!(matches!(
            classify_web_response(403, b"{}"),
            Err(SummaryErr::Auth)
        ));
        assert!(matches!(
            classify_web_response(500, b"oops"),
            Err(SummaryErr::Other(_))
        ));
        let code_err = classify_web_response(200, br#"{"code":10001,"msg":"parameter error"}"#);
        assert!(matches!(code_err, Err(SummaryErr::Other(ref m)) if m.contains("10001")));
        let ok = classify_web_response(200, br#"{"code":0,"data":{"Packages":[]}}"#);
        assert!(ok.is_ok());
    }

    #[test]
    fn the_web_summary_builds_the_same_credits_rows_as_the_token_route() {
        let now = 1_760_100_000_000i64;
        let data = json!({
            "Packages": [
                { "PackageCode": "p_tcaca", "CycleTotalCapacity": "500",
                  "CycleRemainCapacity": "50.5", "CycleUsedCapacity": "449.5" }
            ],
            "IsPaidUser": true,
            "SubscriptionPackageName": "体验版"
        });
        let (metrics, plan) = metrics_from_summary(&data, now);
        assert_eq!(plan.as_deref(), Some("体验版"));
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].label, "Credits");
        assert_eq!(metrics[0].kind, "progress");
        // used = total - remain = 449.5 → 89.9%.
        let pct = metrics[0].used_percent.unwrap();
        assert!((pct - 89.9).abs() < 0.05, "{pct}");
        assert_eq!(metrics[0].detail.as_deref(), Some("450 of 500 credits used"));
        // No cycle end in the summary: balance kept, no reset — the
        // accepted degrade.
        assert_eq!(metrics[0].resets_at, None);
    }

    #[test]
    fn unlimited_and_zero_total_degrade_to_text_rows() {
        let now = 1_760_100_000_000i64;
        let unlimited = json!({
            "Packages": [ { "PackageCode": "p", "CycleTotalCapacity": "-1", "Unlimited": true } ]
        });
        let (metrics, _) = metrics_from_summary(&unlimited, now);
        assert_eq!(metrics[0].kind, "text");
        assert_eq!(metrics[0].value.as_deref(), Some("Unlimited"));

        let zero = json!({
            "Packages": [ { "PackageCode": "p", "CycleTotalCapacity": "0", "CycleRemainCapacity": "0" } ]
        });
        let (metrics, _) = metrics_from_summary(&zero, now);
        assert_eq!(metrics[0].kind, "text");
        assert_eq!(metrics[0].value.as_deref(), Some("0 credits left"));
    }

    #[test]
    fn frozen_capacity_rides_as_the_reserved_row() {
        let now = 1_760_100_000_000i64;
        let data = json!({
            "Packages": [ { "PackageCode": "p", "CycleTotalCapacity": "500",
                            "CycleRemainCapacity": "480", "CycleFrozenCapacity": "20" } ]
        });
        let (metrics, _) = metrics_from_summary(&data, now);
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[1].label, "Reserved");
        assert_eq!(metrics[1].value.as_deref(), Some("20 credits reserved"));
    }
}
