//! Doubao personal subscription (豆包桌面版 / doubao.com membership) quota.
//! Web-session provider: the commerce endpoints authenticate with the
//! desktop app's own cookies — no API key exists. The quota windows come
//! straight from the server: a 5-hour and a 7-day rolling window
//! (window_type 1/2, used_percent, end_time) plus the subscription cycle
//! itself and the user's quota-reset-card balance.
//!
//! Cookie chain (verified live 2026-09-12, non-standard Chromium):
//! `Cookies` SQLite `encrypted_value` strips the `v10` prefix → AES-256-GCM
//! with the Local State os_crypt key (same DPAPI unwrap as Qoder CN) → the
//! GCM plaintext carries a 32-byte random header before the real value.
//! Doubao holds an exclusive lock on the Cookies DB while running, so the
//! extract only succeeds while Doubao is fully quit; the working header is
//! cached in Pane's config dir (sessionid lives ~30 days) and refreshed on
//! any successful extraction.

use super::{config_dir, http, json_body, read_small_text, Metric, Snapshot};
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

const ID: &str = "doubao";
const NAME: &str = "Doubao";

const USER_DATA: &str = "Doubao/User Data";
const COOKIES_REL: &str = "Default/Network/Cookies";
const LOCAL_STATE_REL: &str = "Local State";
const QUOTA_URL: &str = "https://www.doubao.com/alice/commerce/sale/subscription/quota/summary";
const CARDS_URL: &str = "https://www.doubao.com/alice/commerce/marketing/card/balance";

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
const MAX_LOCAL_STATE_BYTES: u64 = 256 * 1024;
const MAX_QUOTA_BYTES: usize = 256 * 1024;
const MAX_CARD_BYTES: usize = 32 * 1024;
/// GCM plaintext = 32 random bytes + the real cookie value (live-verified
/// across every doubao cookie; offset 32 is always printable).
const PLAIN_HEADER_LEN: usize = 32;

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Pure local probe for the Customize gear panel (no network): a cached
/// session or an extractable Doubao sign-in exists on this machine.
pub fn local_credential_hint() -> Option<String> {
    if cookie_cache_path().is_file() {
        return Some("Doubao session cache".to_string());
    }
    cookies_path()
        .filter(|p| p.is_file())
        .map(|_| "Doubao desktop sign-in (quit Doubao once so Pane can read it)".to_string())
}

fn user_data_dir() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join(USER_DATA))
}

fn cookies_path() -> Option<PathBuf> {
    Some(user_data_dir()?.join(COOKIES_REL))
}

fn cookie_cache_path() -> PathBuf {
    config_dir().join("doubao_cookies.json")
}

/// Active cookie header: the cached one first, then a fresh extraction
/// (which also refreshes the cache).
fn cookie_header() -> Option<String> {
    if let Ok(raw) = std::fs::read_to_string(cookie_cache_path()) {
        if let Ok(doc) = serde_json::from_str::<Value>(raw.trim_start_matches('\u{feff}')) {
            if let Some(cookie) = doc.get("cookie").and_then(Value::as_str) {
                if cookie.contains("sessionid=") {
                    return Some(cookie.to_string());
                }
            }
        }
    }
    let header = extract_cookie_header()?;
    let _ = std::fs::write(
        cookie_cache_path(),
        serde_json::to_string_pretty(&serde_json::json!({
            "cookie": header,
            "savedAt": chrono::Utc::now().to_rfc3339(),
        }))
        .unwrap_or_default(),
    );
    Some(header)
}

/// Reads Doubao's Cookies DB (only possible while Doubao is fully quit),
/// decrypts every doubao.com cookie and joins them into one header.
/// Same-host duplicates prefer the bare `.doubao.com` entry (site-wide).
fn extract_cookie_header() -> Option<String> {
    let src = cookies_path()?;
    let dst = config_dir().join("doubao_cookies_snapshot.db");
    std::fs::create_dir_all(config_dir()).ok()?;
    // Devin's locked-DB pattern: try the backup API first (works when the
    // writer allows readers); fall back to a plain copy.
    let snapshot_ok = (|| -> Result<(), String> {
        let src_conn = rusqlite::Connection::open_with_flags(
            &src,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;
        let mut dst_conn = rusqlite::Connection::open(&dst).map_err(|e| e.to_string())?;
        let backup = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)
            .map_err(|e| e.to_string())?;
        backup
            .run_to_completion(5, Duration::from_millis(20), None)
            .map_err(|e| e.to_string())?;
        Ok(())
    })()
    .is_ok();
    let snapshot_ok = snapshot_ok || std::fs::copy(&src, &dst).map(|_| ()).is_ok();
    if !snapshot_ok {
        return None;
    }

    let dir = user_data_dir()?;
    let key = os_crypt_key(&dir).ok()?;
    let conn = rusqlite::Connection::open_with_flags(
        &dst,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    let mut stmt =
        conn.prepare("SELECT host_key, name, encrypted_value FROM cookies WHERE host_key LIKE '%doubao.com'")
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
    let _ = std::fs::remove_file(&dst);

    let mut jar: Vec<(bool, String, String)> = Vec::new(); // (site-wide, name, value)
    for (host, name, blob) in rows {
        if blob.len() <= PLAIN_HEADER_LEN + 3 + 12 + 16 {
            continue;
        }
        let Some(value) = decrypt_cookie(&blob, &key) else {
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
    if !jar.iter().any(|(_, n, _)| n == "sessionid") {
        return None;
    }
    Some(
        jar.into_iter()
            .map(|(_, n, v)| format!("{n}={v}"))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Doubao's v10 blob: AES-256-GCM like Chromium os_crypt, but the GCM
/// plaintext carries a 32-byte random header before the value.
fn decrypt_cookie(blob: &[u8], key: &[u8; 32]) -> Option<String> {
    let plain = decode_os_crypt(blob, key).ok()?;
    let text = String::from_utf8(plain.get(PLAIN_HEADER_LEN..)?.to_vec()).ok()?;
    looks_like_text(&text).then_some(text)
}

/// The peel sanity bar: cookie values are printable text, so a wrong key
/// or wrong header length (binary junk) is rejected here.
fn looks_like_text(s: &str) -> bool {
    let printable = s.chars().filter(|c| !c.is_control()).count();
    printable * 2 > s.len()
}

/// `Local State` → `os_crypt.encrypted_key` → DPAPI → 32-byte AES key.
fn os_crypt_key(dir: &std::path::Path) -> Result<[u8; 32], String> {
    let raw =
        read_small_text(&dir.join(LOCAL_STATE_REL), MAX_LOCAL_STATE_BYTES, "Local State")?;
    use base64::Engine;
    let doc: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("parse Local State: {e}"))?;
    let encoded = doc
        .pointer("/os_crypt/encrypted_key")
        .and_then(Value::as_str)
        .ok_or("Local State has no os_crypt.encrypted_key")?;
    let wrapped = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("decode encrypted_key: {e}"))?;
    let wrapped = wrapped
        .strip_prefix(b"DPAPI")
        .ok_or("encrypted_key lacks the DPAPI prefix")?;
    let key = crate::platform::dpapi_unprotect(wrapped)
        .ok_or("DPAPI unwrap of the Doubao key failed")?;
    key.try_into().map_err(|_| "os_crypt key is not 32 bytes".to_string())
}

/// Chromium os_crypt v10: nonce = bytes 3..15, ciphertext+tag = 15...
/// (Same scheme as Qoder CN's auth.v1.dat.)
fn decode_os_crypt(raw: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if raw.len() < 3 + 12 + 16 || &raw[..3] != b"v10" {
        return Err("not a v10 os_crypt blob".into());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&raw[3..15]), &raw[15..])
        .map_err(|_| "cookie decryption failed (key mismatch?)".into())
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(cookie) = cookie_header() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "No Doubao session found. Quit the Doubao desktop app completely and hit refresh — Pane reads its sign-in cookies while it is not running.",
        ));
    };
    fetch_with_cookie(&cookie).await
}

async fn fetch_with_cookie(cookie: &str) -> Result<Snapshot, String> {
    let (quota_resp, cards_resp) = tokio::join!(post_json(QUOTA_URL, cookie, "quota"), post_json(CARDS_URL, cookie, "cards"));

    let quota_resp = quota_resp?;
    if quota_resp.status().as_u16() == 401 || quota_resp.status().as_u16() == 403 {
        return Err(
            "Doubao session was rejected — sign in to doubao.com or the desktop app, quit it, and refresh"
                .into(),
        );
    }
    if !quota_resp.status().is_success() {
        return Err(format!("quota endpoint: HTTP {}", quota_resp.status()));
    }
    let quota: Value = json_body(quota_resp, MAX_QUOTA_BYTES, "quota").await?;
    if quota.get("code").and_then(Value::as_i64).unwrap_or(0) != 0 {
        return Err(
            "Doubao session was rejected — sign in to doubao.com or the desktop app, quit it, and refresh"
                .into(),
        );
    }
    let cards: Option<Value> = match cards_resp {
        Ok(resp) if resp.status().is_success() => json_body(resp, MAX_CARD_BYTES, "cards").await.ok(),
        _ => None,
    };

    let (plan, metrics) = metrics_from_docs(&quota, cards.as_ref())?;
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

async fn post_json(url: &str, cookie: &str, what: &str) -> Result<reqwest::Response, String> {
    http()
        .post(url)
        .header("Cookie", cookie)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0.0.0 Safari/537.36",
        )
        .json(&serde_json::json!({}))
        .timeout(Duration::from_secs(12))
        .send()
        .await
        .map_err(|e| format!("{what} request: {e}"))
}

/// The parse pipeline fetch() runs, kept pure for the unit tests below.
fn metrics_from_docs(
    quota: &Value,
    cards: Option<&Value>,
) -> Result<(Option<String>, Vec<Metric>), String> {
    let data = quota.get("data").ok_or("unexpected quota response shape")?;
    let sub = data.pointer("/current_subscription");
    let plan = sub
        .and_then(|s| {
            s.pointer("/display/short_name")
                .or_else(|| s.pointer("/display/product_name"))
        })
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut metrics = Vec::new();
    let mut rows: Vec<(i64, f64, i64, i64)> = Vec::new(); // (window_type, used_percent, start, end)
    for limit in data
        .pointer("/window_limit_section/window_limit_groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|g| g.get("window_limits").and_then(Value::as_array))
        .flatten()
    {
        let wt = limit.get("window_type").and_then(Value::as_i64).unwrap_or(0);
        let used = limit
            .get("used_percent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let start = limit.get("start_time").and_then(Value::as_i64).unwrap_or(0);
        let end = limit.get("end_time").and_then(Value::as_i64).unwrap_or(0);
        rows.push((wt, used, start, end));
    }
    // window_type 1 = 5h session, 2 = 7d weekly (live-verified). An idle
    // session window reports end_time 0 — "starts counting on first use".
    for (wt, label, fallback_period) in
        [(1i64, "Session", 5 * HOUR_MS), (2, "Weekly", 7 * DAY_MS)]
    {
        let Some(row) = rows.iter().find(|r| r.0 == wt) else {
            continue;
        };
        let (_, used, start, end) = *row;
        let period = if end > start && start > 0 { end - start } else { fallback_period };
        metrics.push(
            Metric::progress(label, used.clamp(0.0, 100.0), None)
                .with_reset((end > 0).then_some(end), Some(period)),
        );
    }
    if metrics.is_empty() {
        return Err("no usage windows in quota response".into());
    }

    // Subscription cycle: the closest thing to a monthly row. Text, never
    // a guessed meter.
    if let Some(end) = sub.and_then(|s| s.get("end_time")).and_then(Value::as_i64) {
        if end > 0 {
            let day = end / 1000;
            let date = chrono::DateTime::from_timestamp(day, 0)
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            metrics.push(Metric::text("Subscription", format!("renews {date}")).with_reset(Some(end), Some(30 * DAY_MS)));
        }
    }

    // Quota reset cards: bypass a drained window on the desktop app.
    // Surface the earliest expiry — a card past it doesn't come back, so
    // a bare count would overstate what's usable.
    if let Some(data) = cards.and_then(|c| c.get("data")) {
        let count = data.get("available_count").and_then(Value::as_i64).unwrap_or(0);
        if count > 0 {
            let expiry = data
                .get("earliest_expire_time")
                .and_then(Value::as_i64)
                .filter(|ms| *ms > 0)
                .and_then(|ms| chrono::DateTime::from_timestamp(ms / 1000, 0))
                .map(|t| t.format("%Y-%m-%d").to_string());
            let value = match expiry {
                Some(d) => format!("{count} available · earliest expires {d}"),
                None => format!("{count} available"),
            };
            metrics.push(Metric::text("Reset cards", value));
        }
    }

    Ok((plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn quota_doc() -> Value {
        json!({
            "data": {
                "current_subscription": {
                    "sku_key": "doubao_personal_std",
                    "display": {"product_name": "个人订阅", "short_name": "标准套餐"},
                    "is_gift": true,
                    "start_time": 1788274127908i64,
                    "end_time": 1790866127908i64,
                    "status": 3
                },
                "window_limit_section": {
                    "usage_exhausted": false,
                    "window_limit_groups": [{
                        "feature_group": "general",
                        "window_limits": [
                            {"window_type": 1, "used_percent": 0, "start_time": 0, "end_time": 0},
                            {"window_type": 2, "used_percent": 43, "start_time": 1788927964162i64, "end_time": 1789532764162i64}
                        ]
                    }]
                }
            },
            "code": 0
        })
    }

    fn labels(metrics: &[Metric]) -> Vec<&str> {
        metrics.iter().map(|m| m.label.as_str()).collect()
    }

    #[test]
    fn live_quota_shape_renders_windows_and_cycle() {
        let (plan, metrics) = metrics_from_docs(&quota_doc(), None).unwrap();
        assert_eq!(plan.as_deref(), Some("标准套餐"));
        assert_eq!(labels(&metrics), ["Session", "Weekly", "Subscription"]);

        // Idle 5h window: 0 used, no reset countdown, 5h period.
        assert_eq!(metrics[0].used_percent, Some(0.0));
        assert_eq!(metrics[0].resets_at, None);
        assert_eq!(metrics[0].period_ms, Some(5 * HOUR_MS));

        // 7d window: 43% used, server-supplied reset, observed length kept.
        assert_eq!(metrics[1].used_percent, Some(43.0));
        assert_eq!(metrics[1].resets_at, Some(1789532764162));
        assert_eq!(metrics[1].period_ms, Some(1789532764162 - 1788927964162));

        // Subscription cycle = the "monthly" row, as text + reset.
        assert_eq!(metrics[2].kind, "text");
        assert!(metrics[2].value.as_deref().unwrap().contains("2026-"));
        assert_eq!(metrics[2].resets_at, Some(1790866127908));
    }

    #[test]
    fn reset_cards_row_appears_only_when_available() {
        let none = metrics_from_docs(&quota_doc(), None).unwrap().1;
        assert!(!labels(&none).contains(&"Reset cards"));

        let cards = json!({"data": {"available_count": 7,
            "available_count_by_card_key": {"pc_quota_reset_card": 6, "quota_reset_card": 1},
            "earliest_expire_time": 1790866212579i64,
            "has_available_card": true}, "code": 0});
        let with = metrics_from_docs(&quota_doc(), Some(&cards)).unwrap().1;
        assert_eq!(labels(&with), ["Session", "Weekly", "Subscription", "Reset cards"]);
        assert_eq!(
            with[3].value.as_deref(),
            Some("7 available · earliest expires 2026-10-01")
        );
    }

    #[test]
    fn zero_card_balance_is_hidden() {
        let cards = json!({"data": {"available_count": 0}, "code": 0});
        let metrics = metrics_from_docs(&quota_doc(), Some(&cards)).unwrap().1;
        assert!(!labels(&metrics).contains(&"Reset cards"));
    }

    #[test]
    fn missing_windows_is_an_error() {
        let empty = json!({"data": {}, "code": 0});
        assert!(metrics_from_docs(&empty, None).is_err());
        let no_data = json!({"code": 0});
        assert!(metrics_from_docs(&no_data, None).is_err());
    }

    #[test]
    fn cookie_peel_sanity_bar() {
        // Real cookie values are printable text — accepted.
        assert!(looks_like_text("session-token-value"));
        assert!(looks_like_text("HMhZcH1Y6btk9rUPo2utrcN8M5Ety2gAIcYgtlzo9eOQ"));
        // Binary junk (wrong key / wrong header length) — rejected.
        assert!(!looks_like_text("\u{1}\u{2}\u{3}"));
        assert!(!looks_like_text("ab\u{0}\u{1}\u{2}\u{3}cd"));
    }
}
