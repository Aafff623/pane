//! BibiGPT membership credits (bibigpt.cn).
//!
//! BibiGPT is a web console rather than a desktop CLI, so the session comes
//! from Pane's own incognito login window (same seam as `xfyun_speech`) and
//! the read is one authenticated GET.
//!
//! Endpoint facts verified 2026-11-20 without a session:
//! - `GET /api/v1/me` answers **401 UNAUTHORIZED** (not 404), and the body's
//!   internal route name is `agent.me` — the endpoint exists and is the
//!   account/credit read. `bibigpt.co` is a 302 shell; the API host is
//!   `bibigpt.cn`.
//! - Responses use one envelope: `{ message, code, data: { code, httpStatus,
//!   zodError, creditsShortfall } }`.
//!
//! What is NOT yet verified: the shape of the credit fields themselves. The
//! console shows three things — a monthly allowance (`5,824 / 6,000`), a
//! separate top-up balance that stacks, and the date the allowance refills.
//! The parser below therefore reads them by *candidate key names* and reports
//! anything it cannot find as unavailable rather than as zero — the same rule
//! `qoder_sessions` follows after its 2026-11 upstream change. Once one real
//! response has been captured, pin the exact path and delete the candidates.

use super::{json_body, Metric, Snapshot};
use serde_json::Value;
use std::time::Duration;

const ID: &str = "bibigpt";
const NAME: &str = "BibiGPT";
const SITE_ORIGIN: &str = "https://bibigpt.cn";
const ME_PATH: &str = "/api/v1/me";
const MAX_BODY_BYTES: usize = 64 * 1024;

pub fn local_credential_hint() -> Option<String> {
    matches!(
        crate::secretstore::get(TOKEN_TARGET),
        crate::secretstore::Secret::Found(_)
    )
    .then(|| "Pane BibiGPT 登录会话".to_string())
}

pub async fn snapshot() -> Snapshot {
    let crate::secretstore::Secret::Found(jar) = crate::secretstore::get(TOKEN_TARGET) else {
        return Snapshot::no_credentials(ID, NAME, "请先在 Pane 里登录 BibiGPT。");
    };
    match fetch(&jar).await {
        Ok(doc) => parse(&doc),
        Err(error) => Snapshot::error(ID, NAME, error),
    }
}

async fn fetch(jar: &str) -> Result<Value, String> {
    let resp = super::http()
        .get(format!("{SITE_ORIGIN}{ME_PATH}"))
        .header("Cookie", jar)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| super::error_chain(&e))?;
    let status = resp.status().as_u16();
    if status == 401 || status == 403 {
        return Err("BibiGPT 拒绝了当前登录态，请在 Pane 里重新登录。".into());
    }
    json_body(resp, MAX_BODY_BYTES, "BibiGPT 账户")
        .await
        .map_err(|e| format!("{e} (HTTP {status})"))
}

/// Depth-first search for the first value of any candidate key. The credit
/// response nests under `data`, but the exact nesting is unverified, so this
/// reads by key name instead of by path.
fn find<'a>(root: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match root {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key) {
                    return Some(value);
                }
            }
            map.values().find_map(|v| find(v, keys))
        }
        Value::Array(items) => items.iter().find_map(|v| find(v, keys)),
        _ => None,
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse().ok())
        .filter(|n| n.is_finite() && *n >= 0.0)
}

pub(crate) fn parse(doc: &Value) -> Snapshot {
    // A 200 with an error code inside the envelope is not a reading.
    let code = doc.get("code").and_then(Value::as_str).unwrap_or_default();
    if code.eq_ignore_ascii_case("UNAUTHORIZED") || code.eq_ignore_ascii_case("FORBIDDEN") {
        return Snapshot::error(ID, NAME, "BibiGPT 需要重新登录。".into());
    }

    let used = number(find(doc, &["creditsUsed", "usedCredits", "credits_used", "used"]));
    let total = number(find(doc, &["creditsTotal", "totalCredits", "credits_total", "total", "quota"]));
    let topup = number(find(doc, &["topupBalance", "topup", "rechargeBalance", "extraCredits", "bonusCredits"]));
    let plan = find(doc, &["planName", "plan", "membership", "tier"])
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let renews_at = find(doc, &["currentPeriodEnd", "periodEnd", "expiresAt", "renewAt", "resetAt"]);

    let mut metrics: Vec<Metric> = Vec::new();
    let mut headline = plan.map(str::to_string);

    // The allowance row is the card's main number when the console published
    // both halves. A used-total pair that is missing either half stays a text
    // row saying so — never a 0% bar.
    match (used, total) {
        (Some(used), Some(total)) if total > 0.0 => {
            let percent = (used / total * 100.0).clamp(0.0, 100.0);
            let label = plan.map(|p| format!("{p} 月度额度")).unwrap_or_else(|| "月度额度".into());
            let detail = Some(format!("{} / {}", fmt_int(used), fmt_int(total)));
            let mut row = Metric::progress(&label, percent, detail);
            if let Some(at) = renews_at.and_then(epoch_ms) {
                row.resets_at = Some(at);
            }
            metrics.push(row);
            headline = headline.or_else(|| Some(format!("{} / {}", fmt_int(used), fmt_int(total))));
        }
        _ => {
            // No usable pair. Say which half is missing instead of inventing 0%.
            let missing = match (used.is_some(), total.is_some()) {
                (false, false) => "额度数据暂不可读",
                (true, false) => "只有已用、没有总量",
                _ => "只有总量、没有已用",
            };
            metrics.push(Metric::text("月度额度", missing.into()));
        }
    }

    // Top-up credits stack on top of the allowance and have no denominator, so
    // this is a count with no bar (same treatment as the 讯飞 trial pool).
    if let Some(topup) = topup {
        metrics.push(Metric::text("充值余额", fmt_int(topup)));
    }

    Snapshot::ok(ID, NAME, headline, metrics)
}

fn fmt_int(n: f64) -> String {
    if (n.fract()).abs() < f64::EPSILON {
        format!("{n:.0}")
    } else {
        format!("{n:.2}")
    }
}

/// Accepts epoch milliseconds, epoch seconds, or an RFC 3339 string.
fn epoch_ms(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(if n > 1_000_000_000_000 { n } else { n * 1000 });
    }
    let raw = value.as_str()?;
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|t| t.timestamp_millis())
}

const TOKEN_TARGET: &str = "Pane:bibigpt:session";
const FAMILY_ID: &str = "bibigpt";
const LOGIN_URL: &str = "https://bibigpt.cn/user/login";
const LOGIN_WINDOW: &str = "bibigpt-login";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
const LOGIN_POLL: Duration = Duration::from_secs(2);

/// Where one captured `/api/v1/me` body is written so the exact credit field
/// names can be pinned without a hand-built request. Local only: the console
/// log directory, never the repo and never the network.
fn diagnostic_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("PaneAafff623").join("bibigpt-response.json"))
}

/// Drops the raw body next to the app's own state. A short breadcrumb in the
/// log names the file; the body itself stays out of the log so account data
/// does not end up in a shared console capture.
fn save_probe(doc: &Value) {
    let Some(path) = diagnostic_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if serde_json::to_string_pretty(doc)
        .ok()
        .and_then(|body| std::fs::write(&path, body).ok())
        .is_some()
    {
        eprintln!("[pane] bibigpt: captured one /api/v1/me response -> {}", path.display());
    }
}

#[cfg(not(feature = "harness"))]
pub async fn login(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager as _;
    static LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOGIN_LOCK
        .try_lock()
        .map_err(|_| "A BibiGPT sign-in is already in progress".to_string())?;
    if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = window.set_focus();
        return Err("The BibiGPT sign-in window is already open".into());
    }
    let url = tauri::Url::parse(LOGIN_URL).map_err(|e| format!("login url: {e}"))?;
    tauri::WebviewWindowBuilder::new(app, LOGIN_WINDOW, tauri::WebviewUrl::External(url))
        .title("BibiGPT 登录")
        .incognito(true)
        .inner_size(520.0, 760.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    let deadline = std::time::Instant::now() + LOGIN_TIMEOUT;
    let mut verified: Option<String> = None;
    while verified.is_none() {
        let cookies = cookies_from(app).await;
        if !cookies.is_empty() {
            let jar = cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; ");
            // Only a 200 with a real body proves the session: BibiGPT answers
            // 401 for an unauthenticated read, which is what we must not save.
            if let Ok(doc) = fetch(&jar).await {
                save_probe(&doc);
                verified = Some(jar);
                break;
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
    let Some(jar) = verified else {
        return Err("Sign-in did not finish (window closed)".into());
    };
    crate::secretstore::put(TOKEN_TARGET, &jar)
}

#[cfg(not(feature = "harness"))]
async fn cookies_from(app: &tauri::AppHandle) -> Vec<(String, String)> {
    use tauri::Manager as _;
    let Some(window) = app.get_webview_window(LOGIN_WINDOW) else {
        return Vec::new();
    };
    crate::platform::read_webview_cookies(&window, SITE_ORIGIN)
        .await
        .into_iter()
        .filter(|(name, value)| !name.trim().is_empty() && !value.is_empty())
        .collect()
}

#[cfg(feature = "harness")]
pub async fn login(_app: &tauri::AppHandle) -> Result<(), String> {
    Err("BibiGPT sign-in is unavailable in the test harness".into())
}

pub fn status() -> crate::providers::tokenrhythm::LoginStatus {
    crate::providers::tokenrhythm::LoginStatus {
        signed_in: matches!(
            crate::secretstore::get(TOKEN_TARGET),
            crate::secretstore::Secret::Found(_)
        ),
        saved_at: None,
    }
}

pub fn clear() -> Result<(), String> {
    crate::secretstore::remove_checked(TOKEN_TARGET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_used_total_pair_becomes_a_progress_row() {
        let doc = serde_json::json!({
            "message": "OK",
            "data": {
                "plan": { "name": "Plus", "creditsUsed": 5824, "creditsTotal": 6000 }
            }
        });
        let snap = parse(&doc);
        assert_eq!(snap.status, "ok");
        let row = &snap.metrics[0];
        assert_eq!(row.kind, "progress");
        assert!((row.used_percent.unwrap() - 97.066).abs() < 0.01);
        assert_eq!(row.detail.as_deref(), Some("5824 / 6000"));
    }

    #[test]
    fn a_missing_total_never_becomes_a_zero_percent_bar() {
        let doc = serde_json::json!({ "data": { "creditsUsed": 120 } });
        let snap = parse(&doc);
        let row = &snap.metrics[0];
        assert_eq!(row.kind, "text");
        assert_eq!(row.value.as_deref(), Some("只有已用、没有总量"));
        assert!(row.used_percent.is_none());
    }

    #[test]
    fn topup_credits_render_as_a_count_without_a_bar() {
        let doc = serde_json::json!({
            "data": { "creditsUsed": 10, "creditsTotal": 100, "topupBalance": 1 }
        });
        let snap = parse(&doc);
        let topup = snap.metrics.iter().find(|m| m.label == "充值余额").expect("topup row");
        assert_eq!(topup.kind, "text");
        assert_eq!(topup.value.as_deref(), Some("1"));
    }

    #[test]
    fn an_unauthorized_envelope_is_an_error_not_a_zero() {
        let doc = serde_json::json!({
            "message": "UNAUTHORIZED",
            "code": "UNAUTHORIZED",
            "data": { "code": "UNAUTHORIZED", "httpStatus": 401 }
        });
        let snap = parse(&doc);
        assert_eq!(snap.status, "error");
        assert!(snap.metrics.is_empty());
    }

    #[test]
    fn rfc3339_period_end_maps_to_the_reset_countdown() {
        let doc = serde_json::json!({
            "data": {
                "creditsUsed": 1, "creditsTotal": 10,
                "currentPeriodEnd": "2026-12-07T00:00:00Z"
            }
        });
        let snap = parse(&doc);
        assert_eq!(snap.metrics[0].resets_at, Some(1796601600000));
    }
}
