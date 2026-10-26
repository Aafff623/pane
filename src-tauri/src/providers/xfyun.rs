//! 讯飞星火 MaaS — web-session provider.
//!
//! The API key only authenticates the inference gateway; every console quota
//! endpoint lives behind the signed-in web session. Pane hosts the login in
//! its own isolated WebView2 window, verifies the session against
//! `/api/v1/gpt-finetune/user`, and stores the cookie jar in the same
//! login-account/vault path as the other Pane-managed logins.
//!
//! The free quota is a one-off grant: `data.claim` may be null, and the
//! quota fields are read from `data.claim.freeQuota`. The parser accepts
//! JSON numbers and numeric strings, never invents a missing total, and
//! reports a structure we do not recognize honestly. The real response is
//! calibrated on the first owner login; see ticket 03.

use super::{error_chain, http, response_shape, Metric, Snapshot};
use crate::login_accounts::LoginAccount;
use crate::usage_units::UsageUnit;
use serde_json::Value;
use std::time::Duration;

const FAMILY_ID: &str = "xfyun";
const DISPLAY_NAME: &str = "讯飞星火";
const SITE_ORIGIN: &str = "https://maas.xfyun.cn";
const LOGIN_URL: &str = "https://maas.xfyun.cn/";
const USER_PATH: &str = "/api/v1/gpt-finetune/user";
const QUOTA_PATH: &str =
    "/api/v1/gpt-finetune/api-services/free-quota?channel=spark-x2.6";
const USAGE_PATH: &str = "/api/v1/gpt-finetune/api-services/usage";
const LOGIN_WINDOW: &str = "xfyun-login";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);

/// Quota-semantic keys recorded verbatim when `PANE_CONSOLE_PROBE=1` (#38
/// calibration). Identity keys (`uid`, `name`, `mobile`) are deliberately
/// absent, so the shape log cannot carry PII.
const PROBE_ALLOW: &[&str] = &[
    "code", "flag", "status", "quotaStatus", "state", "channel", "unit", "unitType", "total",
    "used", "left", "trial", "remaining", "surplus", "totalQuota", "expiresAt", "expireTime",
    "beginTime", "endTime", "claim",
];

fn probe_log_path() -> std::path::PathBuf {
    super::config_dir().join("xfyun_probe_log.json")
}
const LOGIN_POLL: Duration = Duration::from_secs(2);
const MAX_BODY_BYTES: usize = 256 * 1024;

const SIGNED_OUT_HINT: &str =
    "Spark web session expired — sign in again from the auth center.";
const NO_CLAIM_HINT: &str =
    "No Spark-X2.6 free quota claimed yet — claim it in the iFlytek console, then refresh.";

#[derive(Debug, PartialEq)]
enum Quota {
    NotClaimed,
    Claimed {
        total: f64,
        used: f64,
        remaining: f64,
        status: String,
        expires: Option<String>,
        app_id: Option<String>,
    },
}

enum CallErr {
    Auth,
    Other(String),
}

pub fn login_card_name(login: &LoginAccount) -> String {
    let label = if login.label.trim().is_empty() { &login.email } else { &login.label };
    if label.trim().is_empty() { DISPLAY_NAME.into() } else { format!("{DISPLAY_NAME} — {label}") }
}

pub fn status() -> crate::providers::tokenrhythm::LoginStatus {
    let accounts = crate::login_accounts::load(FAMILY_ID);
    crate::providers::tokenrhythm::LoginStatus {
        signed_in: !accounts.is_empty(),
        saved_at: accounts
            .first()
            .filter(|row| row.added_at > 0)
            .and_then(|row| chrono::DateTime::from_timestamp(row.added_at, 0))
            .map(|at| at.to_rfc3339()),
    }
}

/// The web session is per account; account deletion is the clearing path.
/// There is no legacy single-session slot for xfyun, so this is a no-op.
pub fn clear() -> Result<(), String> {
    Ok(())
}

pub async fn migrate_legacy_session() -> Result<(), String> {
    Ok(())
}

pub async fn snapshot() -> Snapshot {
    match crate::login_accounts::load(FAMILY_ID).into_iter().next() {
        Some(login) => snapshot_with_login_as(login, FAMILY_ID).await,
        None => Snapshot::no_credentials(
            FAMILY_ID,
            DISPLAY_NAME,
            "Sign in to the Spark web console from the auth center to see its free quota.".into(),
        ),
    }
}

pub async fn snapshot_with_login(login: LoginAccount) -> Snapshot {
    let id = crate::login_accounts::card_id_for_account(FAMILY_ID, &login);
    snapshot_with_login_as(login, &id).await
}

async fn snapshot_with_login_as(login: LoginAccount, id: &str) -> Snapshot {
    let name = login_card_name(&login);
    match call_console(QUOTA_PATH, &login.access_token).await {
        Ok(doc) => match parse_quota(&doc) {
            Ok(Quota::NotClaimed) => Snapshot::needs_setup(
                id,
                &name,
                Some("Spark-X2.6".into()),
                vec![],
                NO_CLAIM_HINT,
            ),
            Ok(Quota::Claimed { total, used, remaining, status, expires, app_id }) => {
                let active = status.eq_ignore_ascii_case("ACTIVE");
                let mut metrics = Vec::new();
                if active && total > 0.0 {
                    let percent = (used / total) * 100.0;
                    metrics.push(
                        Metric::progress(
                            "Free quota",
                            percent,
                            Some(format!("{} of {} tokens used", fmt_tokens(used), fmt_tokens(total))),
                        )
                        .with_usage(UsageUnit::Tokens, used, Some(total)),
                    );
                } else {
                    // Exhausted, expired and unknown statuses must not look
                    // like a healthy percentage. Keep the raw reading visible
                    // and let the status row carry the meaning.
                    metrics.push(Metric::text(
                        "Free quota",
                        format!("{} / {} tokens", fmt_tokens(used), fmt_tokens(total)),
                    ));
                }
                metrics.push(Metric::text("Remaining", fmt_tokens(remaining)));
                match expires {
                    Some(expires) => metrics.push(Metric::text("Expires", expires)),
                    None => metrics.push(Metric::text("Expires", "Unconfirmed".into())),
                }
                if !active {
                    metrics.push(Metric::text("Status", status));
                }
                if let Some((usage_total, rows)) = fetch_usage_7d(&login.access_token).await {
                    metrics.push(Metric::text(
                        "Usage (7d)",
                        format!("{} tokens · {rows} rows", fmt_tokens(usage_total)),
                    ));
                }
                let plan = app_id
                    .filter(|app| !app.trim().is_empty())
                    .map(|_| "Spark-X2.6".to_string())
                    .or_else(|| Some("Spark-X2.6".into()));
                Snapshot::ok(id, &name, plan, metrics)
            }
            Err(reason) => Snapshot::error(id, &name, reason),
        },
        Err(CallErr::Auth) => Snapshot::no_credentials(id, &name, SIGNED_OUT_HINT.into()),
        Err(CallErr::Other(reason)) => {
            Snapshot::error(id, &name, format!("Spark console: {reason}"))
        }
    }
}

/// A cookie jar from Pane's own login window. The window's incognito
/// profile is the only source; Pane never reads a browser profile's
/// SQLite/Cookie database.
#[cfg(not(feature = "harness"))]
async fn extract_cookie(app: &tauri::AppHandle) -> Option<String> {
    use tauri::Manager as _;
    let window = app.get_webview_window(LOGIN_WINDOW)?;
    let cookies = crate::platform::read_webview_cookies(&window, SITE_ORIGIN).await;
    let jar = cookies
        .into_iter()
        .filter(|(name, value)| !name.trim().is_empty() && !value.is_empty())
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>();
    (!jar.is_empty()).then(|| jar.join("; "))
}

#[cfg(feature = "harness")]
async fn extract_cookie(_app: &tauri::AppHandle) -> Option<String> {
    None
}

fn signed_in_user(doc: &Value) -> Option<(String, String)> {
    if doc.get("code").and_then(Value::as_i64) != Some(0) {
        return None;
    }
    let data = doc.get("data")?;
    let uid = match data.get("uid")? {
        Value::String(uid) => uid.trim().to_string(),
        Value::Number(uid) => uid.to_string(),
        _ => return None,
    };
    if uid.is_empty() {
        return None;
    }
    let label = data
        .get("nickName")
        .and_then(Value::as_str)
        .or_else(|| data.get("login").and_then(Value::as_str))
        .or_else(|| data.get("mobile").and_then(Value::as_str))
        .unwrap_or_default()
        .trim()
        .to_string();
    Some((uid, label))
}

fn usage_total(doc: &Value) -> Option<(f64, usize)> {
    let rows = doc.get("data")?.get("rows")?.as_array()?;
    let mut total = 0.0;
    for row in rows {
        for key in ["inputTokens", "cacheTokens", "outputTokens"] {
            total += parse_num(row.get(key)).unwrap_or(0.0);
        }
    }
    Some((total, rows.len()))
}

async fn fetch_usage_7d(cookie: &str) -> Option<(f64, usize)> {
    let end = chrono::Local::now().date_naive();
    let start = end - chrono::Duration::days(6);
    let mut total = 0.0;
    let mut count = 0;
    for page in 1..=5 {
        let path = format!(
            "{USAGE_PATH}?page={page}&size=100&startDate={}&endDate={}",
            start.format("%Y-%m-%d"),
            end.format("%Y-%m-%d")
        );
        let doc = call_console(&path, cookie).await.ok()?;
        let (page_total, page_rows) = usage_total(&doc)?;
        total += page_total;
        count += page_rows;
        let reported = doc
            .pointer("/data/total")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        if page_rows == 0 || (reported > 0 && count >= reported) {
            break;
        }
    }
    Some((total, count))
}

async fn call_console(path: &str, cookie: &str) -> Result<Value, CallErr> {    let resp = http()
        .get(format!("{SITE_ORIGIN}{path}"))
        .header("Cookie", cookie)
        .header("Referer", format!("{SITE_ORIGIN}/"))
        .header("Origin", SITE_ORIGIN)
        .header("x-auth-source", "")
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| CallErr::Other(error_chain(&e)))?;
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
    let doc: Value = serde_json::from_slice(&bytes)
        .map_err(|e| CallErr::Other(format!("response parse: {e}")))?;
    // Calibration capture (#38): opt-in, local, shape-only. Recording the
    // 4001 branch matters because "cookie-only is not enough" is one of the
    // open questions this log exists to answer.
    if response_shape::enabled() {
        let rejected = doc.get("code").and_then(Value::as_i64) == Some(4001);
        response_shape::note(
            &probe_log_path(),
            serde_json::json!({
                "at": chrono::Utc::now().timestamp_millis(),
                "path": path,
                "status": status,
                "outcome": if rejected { "auth4001" } else { "ok" },
                "shape": response_shape::skeleton(&doc, PROBE_ALLOW, 0),
            }),
        );
    }
    if doc.get("code").and_then(Value::as_i64) == Some(4001) {
        return Err(CallErr::Auth);
    }
    Ok(doc)
}

#[cfg(not(feature = "harness"))]
pub async fn login(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager as _;
    static LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOGIN_LOCK
        .try_lock()
        .map_err(|_| "An iFlytek Spark sign-in is already in progress".to_string())?;
    if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = window.set_focus();
        return Err("The iFlytek Spark sign-in window is already open".into());
    }
    let url = tauri::Url::parse(LOGIN_URL).map_err(|e| format!("login url: {e}"))?;
    tauri::WebviewWindowBuilder::new(app, LOGIN_WINDOW, tauri::WebviewUrl::External(url))
        .title("iFlytek Spark sign-in")
        .incognito(true)
        .inner_size(520.0, 760.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    let deadline = std::time::Instant::now() + LOGIN_TIMEOUT;
    let mut verified: Option<(String, String, String)> = None;
    while verified.is_none() {
        if let Some(cookie) = extract_cookie(app).await {
            if let Ok(doc) = call_console(USER_PATH, &cookie).await {
                if let Some((uid, label)) = signed_in_user(&doc) {
                    verified = Some((cookie, uid, label));
                    break;
                }
            }
        }
        if app.get_webview_window(LOGIN_WINDOW).is_none() {
            return Err("Sign-in did not finish (window closed)".into());
        }
        if std::time::Instant::now() > deadline {
            if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
                let _ = window.close();
            }
            return Err("Sign-in timed out (window closed)".into());
        }
        tokio::time::sleep(LOGIN_POLL).await;
    }
    if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = window.close();
    }
    let Some((cookie, uid, label)) = verified else {
        return Err("Sign-in did not finish (window closed)".into());
    };
    let account = LoginAccount {
        account_id: uid,
        email: String::new(),
        label,
        access_token: cookie,
        refresh_token: String::new(),
        id_token: String::new(),
        expires_at: String::new(),
        added_at: chrono::Utc::now().timestamp(),
        extra: serde_json::json!({ "sourceKind": "web_session" }),
    };
    let _mutation = crate::account_lifecycle::mutation_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    crate::login_accounts::record_login(FAMILY_ID, account)
}

#[cfg(feature = "harness")]
pub async fn login(_app: &tauri::AppHandle) -> Result<(), String> {
    Err("iFlytek Spark sign-in is unavailable in the test harness".into())
}

fn parse_num(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    let number = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(text) => text.trim().parse::<f64>().ok()?,
        _ => return None,
    };
    (number.is_finite() && number >= 0.0).then_some(number)
}

fn parse_expiry(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(number) = value.as_i64() {
        let millis = if number > 1_000_000_000_000 { number } else { number.saturating_mul(1000) };
        return chrono::DateTime::from_timestamp_millis(millis)
            .map(|at| at.format("%Y-%m-%d").to_string());
    }
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        if let Ok(number) = text.parse::<i64>() {
            let millis = if number > 1_000_000_000_000 { number } else { number.saturating_mul(1000) };
            return chrono::DateTime::from_timestamp_millis(millis)
                .map(|at| at.format("%Y-%m-%d").to_string());
        }
        return chrono::DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|at| at.format("%Y-%m-%d").to_string());
    }
    None
}

fn parse_quota(doc: &Value) -> Result<Quota, String> {
    let data = doc.get("data").ok_or("free-quota response has no data")?;
    let claim = data.get("claim").ok_or("free-quota response has no data.claim")?;
    if claim.is_null() {
        return Ok(Quota::NotClaimed);
    }
    let quota = claim
        .get("freeQuota")
        .ok_or("free-quota response has no data.claim.freeQuota")?;
    let total = parse_num(quota.get("totalTokens"))
        .ok_or("free-quota response has no totalTokens")?;
    if total <= 0.0 {
        return Err("free-quota totalTokens is not positive".into());
    }
    let mut used = parse_num(quota.get("usedTokens"));
    let mut remaining = parse_num(quota.get("remainingTokens"));
    if used.is_none() && remaining.is_none() {
        return Err("free-quota response has neither usedTokens nor remainingTokens".into());
    }
    if used.is_none() {
        used = remaining.map(|remaining| (total - remaining).max(0.0));
    }
    if remaining.is_none() {
        remaining = used.map(|used| (total - used).max(0.0));
    }
    Ok(Quota::Claimed {
        total,
        used: used.unwrap_or(0.0),
        remaining: remaining.unwrap_or(0.0),
        status: quota
            .get("quotaStatus")
            .and_then(Value::as_str)
            .unwrap_or("UNKNOWN")
            .to_string(),
        expires: parse_expiry(quota.get("expiresAt")),
        app_id: claim.get("appId").and_then(Value::as_str).map(str::to_string),
    })
}

fn fmt_tokens(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn quota(total: Value, used: Value, remaining: Value, status: &str, expires: Value) -> Value {
        json!({
            "code": 0,
            "data": {
                "claim": {
                    "appId": "app-test",
                    "freeQuota": {
                        "totalTokens": total,
                        "usedTokens": used,
                        "remainingTokens": remaining,
                        "quotaStatus": status,
                        "expiresAt": expires
                    }
                }
            }
        })
    }

    #[test]
    fn claim_null_means_not_claimed() {
        assert_eq!(parse_quota(&json!({"code":0,"data":{"claim":null}})), Ok(Quota::NotClaimed));
    }

    #[test]
    fn accepts_numbers_and_numeric_strings() {
        let parsed = parse_quota(&quota(
            json!("1000000000"),
            json!(123),
            json!("999999877"),
            "ACTIVE",
            json!("2026-11-22T00:00:00Z"),
        )).unwrap();
        match parsed {
            Quota::Claimed { total, used, remaining, status, expires, app_id } => {
                assert_eq!(total, 1_000_000_000.0);
                assert_eq!(used, 123.0);
                assert_eq!(remaining, 999_999_877.0);
                assert_eq!(status, "ACTIVE");
                assert_eq!(expires.as_deref(), Some("2026-11-22"));
                assert_eq!(app_id.as_deref(), Some("app-test"));
            }
            _ => panic!("expected claimed quota"),
        }
    }

    #[test]
    fn missing_remaining_is_derived_from_used() {
        let parsed = parse_quota(&json!({
            "data": {"claim": {"freeQuota": {"totalTokens": 100, "usedTokens": 25, "quotaStatus": "ACTIVE"}}}
        })).unwrap();
        match parsed {
            Quota::Claimed { remaining, .. } => assert!((remaining - 75.0).abs() < f64::EPSILON),
            _ => panic!("expected claimed quota"),
        }
    }

    #[test]
    fn unparseable_expiry_does_not_invent_a_date() {
        let parsed = parse_quota(&quota(json!(100), json!(1), json!(99), "ACTIVE", json!("not-a-date"))).unwrap();
        assert!(matches!(parsed, Quota::Claimed { expires: None, .. }));
    }

    #[test]
    fn missing_total_and_both_counters_fail_honestly() {
        assert!(parse_quota(&json!({"data":{"claim":{"freeQuota":{"usedTokens":1}}}})).is_err());
        assert!(parse_quota(&json!({"data":{"claim":{"freeQuota":{"totalTokens":100}}}})).is_err());
        assert!(parse_quota(&json!({"data":{}})).is_err());
    }

    #[test]
    fn exhausted_and_expired_statuses_are_preserved() {
        for status in ["EXHAUSTED", "EXPIRED"] {
            let parsed = parse_quota(&quota(json!(100), json!(100), json!(0), status, json!(1790000000000_i64))).unwrap();
            assert!(matches!(parsed, Quota::Claimed { status: ref actual, .. } if actual == status));
        }
    }

    #[test]
    fn auth_code_4001_is_recognized_from_a_200_body() {
        let doc = json!({"code":4001,"message":"用户未登录"});
        assert!(!signed_in_user(&doc).is_some());
    }

    #[test]
    fn usage_rows_accept_numbers_and_strings() {
        let doc = json!({
            "code": 0,
            "data": {
                "rows": [
                    {"inputTokens": 10, "cacheTokens": "3", "outputTokens": 4},
                    {"inputTokens": "5", "cacheTokens": 0, "outputTokens": "6"}
                ],
                "total": 2
            }
        });
        assert_eq!(usage_total(&doc), Some((28.0, 2)));
    }

    #[test]
    fn malformed_usage_returns_none_instead_of_zero() {
        assert_eq!(usage_total(&json!({"data":{}})), None);
        assert_eq!(usage_total(&json!({"data":{"rows":null}})), None);
    }
}
