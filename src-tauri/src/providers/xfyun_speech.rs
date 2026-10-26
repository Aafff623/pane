//! 讯飞开放平台（console.xfyun.cn）语音资源包 — web-session provider (GitHub #47).
//!
//! There is no official quota API: the MaaS gateway that #38 uses answers 404
//! on every usage/quota/balance route, so the only reading of the five speech
//! packs is the console's own dashboard endpoints behind the signed-in web
//! session.
//!
//! Two things make this family different from #38, and both are traps if they
//! get conflated:
//!
//! 1. **Login state lives only in the body.** Every one of these endpoints
//!    answers HTTP 200; an expired session returns `flag:false` with
//!    `code:80000` ("请重新登录"). #38's envelope signals the same thing with
//!    `code:4001`. Neither predicate may be reused for the other family — a
//!    silent miss here renders a *logged-out* account as a card full of zeros.
//! 2. **Three unit and time encodings.** Interaction counts
//!    (`meter/api-use-count`: 次, `expireDate` is a naive string),录音转写
//!    (`lfasr/getlfasrInfo`: an array of hour packs, `dueDate` string) and
//!    实时转写 (`get-rtasr-app-auth`: `leftCount` in *seconds*, `expireTime`
//!    as unix seconds, plus `maxConnection` in 路). They never sum together.
//!
//! A pack's trial pool (`threshold`/`remainCount`) and its purchased balance
//! (`left`) are two separate facts: adding them into one number would claim a
//! 500-call trial makes a 10,000-call pack bigger.
//!
//! Forbidden calls: `/dashboard/webapi/getInfo/v1` and
//! `/dashboard/app/getHttpSign` both return the plaintext APIKey/APISecret.
//! This provider needs neither and must never call them.

use super::{Metric, Snapshot};
use crate::login_accounts::LoginAccount;
use crate::usage_units::UsageUnit;
use serde_json::Value;
use std::time::Duration;

const FAMILY_ID: &str = "xfyun-speech";
const DISPLAY_NAME: &str = "讯飞开放平台语音";
const SITE_ORIGIN: &str = "https://console.xfyun.cn";
const LOGIN_URL: &str = "https://console.xfyun.cn/";
const LOGIN_WINDOW: &str = "xfyun-speech-login";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
const LOGIN_POLL: Duration = Duration::from_secs(2);
const MAX_BODY_BYTES: usize = 256 * 1024;

const PROBE_PATH: &str = "/dashboard/user/profile/get";
const APP_LIST_PATH: &str = "/dashboard/app/list?currPage=1&keyword=&type=";
const METER_PATH: &str = "/dashboard/meter/api-use-count/get";
const LFASR_PATH: &str = "/dashboard/lfasr/getlfasrInfo";
const RTASR_PATH: &str = "/dashboard/lfasr/get-rtasr-app-auth";

const SIGNED_OUT_HINT: &str =
    "iFlytek console session expired — sign in again from the auth center.";

/// Quota-semantic keys recorded verbatim under `PANE_CONSOLE_PROBE=1` (#47
/// calibration). The profile endpoint also carries name / ID-card / mobile, so
/// none of those keys are allowlisted — they land in the log as a type only.
const PROBE_ALLOW: &[&str] = &[
    "code", "flag", "status", "state", "unit", "unitType", "total", "used", "left", "trial",
    "remaining", "surplus", "serviceType", "expiresAt", "expireTime", "beginTime", "endTime",
    "apiUseCount", "totalCount", "usedCount", "leftCount",
];

fn probe_log_path() -> std::path::PathBuf {
    super::config_dir().join("xfyun_speech_probe_log.json")
}

/// The business ids that actually answer the interaction-count endpoint.
/// Measured 2026-10-23: `tts` / `iat` / `ise` return a body; `lfasr`, `rta` and
/// friends return `flag:true, data:null` (a normal empty state, not an error).
/// Never extrapolate this list — unconfirmed channels stay out until measured.
const COUNT_CHANNELS: [&str; 3] = ["tts", "iat", "ise"];
const CHANNEL_NAMES: [(&str, &str); 3] = [
    ("tts", "Online TTS"),
    ("iat", "Dictation (IAT)"),
    ("ise", "Speech evaluation"),
];

#[derive(Debug, PartialEq)]
enum CallErr {
    Auth,
    Other(String),
}

/// One interaction-count row (`meter/api-use-count`), kept as separate facts.
#[derive(Debug, PartialEq)]
struct CountMeter {
    /// Purchased pack balance in interactions.
    left: Option<f64>,
    /// Trial pool total (`threshold`) and remaining (`remainCount`).
    trial_total: Option<f64>,
    trial_remaining: Option<f64>,
    used: Option<f64>,
    /// Naive `YYYY-MM-DD HH:[:SS]` string, no timezone in the payload.
    expires: Option<String>,
}

#[derive(Debug, PartialEq)]
struct HourPack {
    total_hours: Option<f64>,
    left_hours: Option<f64>,
    due: Option<String>,
    /// `status` / `convertStatus` meanings are unconfirmed (待确认).
    status: Option<f64>,
}

#[derive(Debug, PartialEq)]
struct RealtimeAuth {
    /// `leftCount` is measured in **seconds**.
    left_seconds: Option<f64>,
    used_seconds: Option<f64>,
    /// Concurrent lanes (路) — a capacity fact, never a usage number.
    max_connection: Option<f64>,
    /// unix seconds.
    expire_at: Option<i64>,
}

pub fn family_id() -> &'static str {
    FAMILY_ID
}

pub fn login_card_name(login: &LoginAccount) -> String {
    let label = if login.label.trim().is_empty() { &login.email } else { &login.label };
    if label.trim().is_empty() {
        DISPLAY_NAME.into()
    } else {
        format!("{DISPLAY_NAME} — {label}")
    }
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

pub fn clear() -> Result<(), String> {
    Ok(())
}

pub async fn migrate_legacy_session() -> Result<(), String> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Transport and envelope
// ---------------------------------------------------------------------------

/// GET a console endpoint with only the stored cookie. Measured: `Referer`,
/// `Origin` and `User-Agent` do not change the answer, so nothing beyond the
/// cookie and `Accept` is sent.
#[cfg(not(test))]
async fn call_get(path: &str, cookie: &str) -> Result<Value, CallErr> {
    let resp = super::http()
        .get(format!("{SITE_ORIGIN}{path}"))
        .header("Cookie", cookie)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| CallErr::Other(super::error_chain(&e)))?;
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
    // Calibration capture (#47): opt-in, local, shape-only. The signed-out
    // branch is recorded because this family's envelope answers HTTP 200 with
    // `flag:false` — the mapping is exactly what needs confirming.
    if super::response_shape::enabled() {
        let outcome = match envelope_error(&doc) {
            Some(CallErr::Auth) => "signed_out",
            Some(_) => "rejected",
            None => "ok",
        };
        super::response_shape::note(
            &probe_log_path(),
            serde_json::json!({
                "at": chrono::Utc::now().timestamp_millis(),
                "path": path,
                "status": status,
                "outcome": outcome,
                "shape": super::response_shape::skeleton(&doc, PROBE_ALLOW, 0),
            }),
        );
    }
    if envelope_error(&doc).is_some() {
        return Err(envelope_error(&doc).unwrap());
    }
    Ok(doc)
}

#[cfg(test)]
async fn call_get(_path: &str, _cookie: &str) -> Result<Value, CallErr> {
    Err(CallErr::Other("network is never used in tests".into()))
}

/// The HTTP status is never the signal here: an unauthenticated call answers
/// 200 with `flag:false`. Splitting the envelope out keeps the rule visible in
/// one place and is why this family cannot borrow #38's `code == 4001` test.
fn envelope_error(doc: &Value) -> Option<CallErr> {
    let flag = doc.get("flag").and_then(Value::as_bool);
    let code = doc.get("code").and_then(Value::as_i64);
    match (flag, code) {
        (Some(false), Some(80000)) => Some(CallErr::Auth),
        (Some(false), Some(20003)) => Some(CallErr::Other(
            "console rejected the request parameters (code 20003)".into(),
        )),
        (Some(false), other) => Some(CallErr::Other(format!(
            "console envelope failure (code {:?})",
            other
        ))),
        _ => None,
    }
}

/// `flag:true, data:null` is a legitimate empty state ("this channel has no
/// such meter"), not an error and not a zero.
fn data<'a>(doc: &'a Value) -> Option<&'a Value> {
    doc.get("data").filter(|value| !value.is_null())
}

fn number(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        // Several console fields arrive as numeric strings.
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Parsers (pure, exercised by synthetic payloads only)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct ConsoleApp {
    app_id: String,
    name: String,
}

fn parse_apps(doc: &Value) -> Vec<ConsoleApp> {
    let Some(rows) = data(doc).and_then(|d| d.get("currentPageData")).and_then(Value::as_array)
    else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let app_id = row.get("appId").and_then(Value::as_str).unwrap_or_default();
            if app_id.trim().is_empty() {
                return None;
            }
            // `serviceList` is null in this response, so which services an app
            // has enabled cannot be read here — every channel is probed.
            Some(ConsoleApp {
                app_id: app_id.to_string(),
                name: row
                    .get("appName")
                    .and_then(Value::as_str)
                    .unwrap_or(app_id)
                    .to_string(),
            })
        })
        .collect()
}

fn parse_count_meter(doc: &Value) -> Option<CountMeter> {
    let body = data(doc)?;
    let meter = CountMeter {
        left: number(body.get("left")),
        trial_total: number(body.get("threshold")),
        trial_remaining: number(body.get("remainCount")),
        used: number(body.get("usedCount")),
        expires: body
            .get("expireDate")
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    };
    // An all-empty object is the "no such meter" shape, not a zero balance.
    (meter.left.is_some()
        || meter.trial_total.is_some()
        || meter.used.is_some()
        || meter.expires.is_some())
    .then_some(meter)
}

fn parse_hour_packs(doc: &Value) -> Vec<HourPack> {
    let Some(rows) = data(doc).and_then(Value::as_array) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let pack = HourPack {
                total_hours: number(row.get("count")),
                left_hours: number(row.get("leftCount")),
                due: row
                    .get("dueDate")
                    .and_then(Value::as_str)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                status: number(row.get("status")),
            };
            (pack.total_hours.is_some() || pack.left_hours.is_some() || pack.due.is_some())
                .then_some(pack)
        })
        // Packs expire independently: oldest due date first so the next one to
        // run out is the row a reader sees on top.
        .collect::<Vec<_>>()
}

fn parse_realtime(doc: &Value) -> Option<RealtimeAuth> {
    let body = data(doc)?;
    let row = RealtimeAuth {
        left_seconds: number(body.get("leftCount")),
        used_seconds: number(body.get("usedCount")),
        max_connection: number(body.get("maxConnection")),
        expire_at: number(body.get("expireTime")).map(|value| value.round() as i64),
    };
    (row.left_seconds.is_some()
        || row.used_seconds.is_some()
        || row.max_connection.is_some()
        || row.expire_at.is_some())
    .then_some(row)
}

/// Console naive timestamps (`YYYY-MM-DD HH:MM` and `HH:MM:SS`), local clock.
/// A timezone is never assumed beyond the local one the app already uses for
/// every other day bucket; an unparseable value yields None, never a guess.
fn parse_console_time(raw: Option<&str>) -> Option<i64> {
    let text = raw?.trim();
    if text.is_empty() {
        return None;
    }
    for format in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%d"] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(text, format) {
            use chrono::{Local, TimeZone};
            return Local
                .from_local_datetime(&naive)
                .single()
                .map(|at| at.timestamp_millis());
        }
        if format.ends_with("%d") {
            if let Ok(day) = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d") {
                use chrono::{Local, TimeZone};
                let at = day.and_hms_opt(23, 59, 59)?;
                return Local.from_local_datetime(&at).single().map(|v| v.timestamp_millis());
            }
        }
    }
    None
}

fn minutes_from_hours(hours: f64) -> f64 {
    (hours * 60.0).round().max(0.0)
}

fn minutes_from_seconds(seconds: f64) -> f64 {
    (seconds / 60.0).round().max(0.0)
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/// The bare card (no account suffix) reads the first stored account, like
/// every other web-session family. The per-account cards go through
/// `snapshot_with_login`.
pub async fn snapshot() -> Snapshot {
    if let Err(reason) = migrate_legacy_session().await {
        return Snapshot::error(FAMILY_ID, DISPLAY_NAME, reason);
    }
    match crate::login_accounts::load(FAMILY_ID).into_iter().next() {
        Some(login) => snapshot_with_login_as(login, FAMILY_ID.to_string()).await,
        None => Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, SIGNED_OUT_HINT),
    }
}

pub async fn snapshot_with_login(login: LoginAccount) -> Snapshot {
    let id = crate::login_accounts::card_id_for_account(FAMILY_ID, &login);
    snapshot_with_login_as(login, id).await
}

async fn snapshot_with_login_as(login: LoginAccount, id: String) -> Snapshot {
    let cookie = login.access_token.clone();
    // Login state is decided by the console itself, never by "a cookie exists".
    match call_get(PROBE_PATH, &cookie).await {
        Ok(doc) if envelope_error(&doc).is_none() => {}
        Ok(_) | Err(CallErr::Auth) => {
            return Snapshot::no_credentials(&id, &login_card_name(&login), SIGNED_OUT_HINT);
        }
        Err(CallErr::Other(error)) => return Snapshot::error(&id, &login_card_name(&login), error),
    }

    let mut metrics = Vec::new();
    let apps = match call_get(APP_LIST_PATH, &cookie).await {
        Ok(doc) => parse_apps(&doc),
        Err(CallErr::Auth) => {
            return Snapshot::no_credentials(&id, &login_card_name(&login), SIGNED_OUT_HINT);
        }
        Err(CallErr::Other(error)) => return Snapshot::error(&id, &login_card_name(&login), error),
    };
    if apps.is_empty() {
        return Snapshot::needs_setup(
            &id,
            &login_card_name(&login),
            None,
            vec![],
            "No application under this account yet — create one in the console, then refresh.",
        );
    }

    for app in &apps {
        for channel in COUNT_CHANNELS {
            let path = format!("{METER_PATH}?businessId={channel}&appId={}", app.app_id);
            match call_get(&path, &cookie).await {
                Ok(doc) => {
                    if let Some(meter) = parse_count_meter(&doc) {
                        metrics.extend(count_metrics(&app.name, channel, &meter));
                    }
                }
                Err(CallErr::Auth) => {
                    return Snapshot::no_credentials(
                        &id,
                        &login_card_name(&login),
                        SIGNED_OUT_HINT,
                    );
                }
                // A channel that does not carry this meter answers data:null and
                // is already filtered out above; a real failure stays silent per
                // row so one bad channel cannot blank the whole card.
                Err(CallErr::Other(_)) => {}
            }
        }

        match call_get(&format!("{LFASR_PATH}?appId={}", app.app_id), &cookie).await {
            Ok(doc) => {
                for pack in parse_hour_packs(&doc) {
                    metrics.push(hour_pack_metric(&app.name, &pack));
                }
            }
            Err(CallErr::Auth) => {
                return Snapshot::no_credentials(
                    &id,
                    &login_card_name(&login),
                    SIGNED_OUT_HINT,
                );
            }
            Err(CallErr::Other(_)) => {}
        }

        match call_get(&format!("{RTASR_PATH}?appId={}", app.app_id), &cookie).await {
            Ok(doc) => {
                if let Some(row) = parse_realtime(&doc) {
                    metrics.extend(realtime_metrics(&app.name, &row));
                }
            }
            Err(CallErr::Auth) => {
                return Snapshot::no_credentials(
                    &id,
                    &login_card_name(&login),
                    SIGNED_OUT_HINT,
                );
            }
            Err(CallErr::Other(_)) => {}
        }
    }

    if metrics.is_empty() {
        return Snapshot::needs_setup(
            &id,
            &login_card_name(&login),
            None,
            vec![],
            "No speech resource pack under this account yet — claim one in the console, then refresh.",
        );
    }
    Snapshot::ok(&id, &login_card_name(&login), None, metrics)
}

/// Count channels: the console renders ONE number, "剩余服务量", computed as
/// the purchased pack's remaining (`left`) plus the trial pool's remaining
/// (`remainCount`). Verified against the live console on 2026-10-23: left
/// 10000 + remainCount 500 showed 10500, and the "购买服务量" slot was empty —
/// no total is published anywhere in these payloads. So the card shows the same
/// sum the owner sees on the web, names the trial share, and never invents a
/// percentage from a guessed total.
fn count_metrics(app: &str, channel: &str, meter: &CountMeter) -> Vec<Metric> {
    let name = CHANNEL_NAMES
        .iter()
        .find(|(id, _)| *id == channel)
        .map(|(_, label)| *label)
        .unwrap_or(channel);
    let mut out = Vec::new();
    let pack_left = meter.left.unwrap_or_default().max(0.0);
    // Only the reported trial balance is shown. Deriving it from
    // `threshold − usedCount` when `remainCount` is missing invented a trial
    // the payload never mentioned (and inflated the console's total).
    let trial_left = meter.trial_remaining.unwrap_or_default().max(0.0);
    if meter.left.is_some() || meter.trial_remaining.is_some() {
        let value = if trial_left > 0.0 {
            format!(
                "{} left · {} trial",
                fmt_count(pack_left + trial_left),
                fmt_count(trial_left)
            )
        } else {
            format!("{} left", fmt_count(pack_left))
        };
        out.push(Metric::text(&format!("{name} ({app})"), value));
    }
    if let Some(raw) = meter.expires.as_deref() {
        let value = match parse_console_time(Some(raw)) {
            Some(ms) => format!("{raw} ({})", fmt_expiry(ms)),
            None => raw.to_string(),
        };
        out.push(Metric::text("Expires", value));
    }
    out
}

fn hour_pack_metric(app: &str, pack: &HourPack) -> Metric {
    let left = pack.left_hours.unwrap_or_default();
    let total = pack.total_hours.unwrap_or_default().max(left);
    let percent = if total > 0.0 {
        ((total - left) / total * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    Metric::progress(
        &format!("Transcription hours ({app})"),
        percent,
        Some(format!(
            "{} min left of {} min pack",
            fmt_count(minutes_from_hours(left)),
            fmt_count(minutes_from_hours(total))
        )),
    )
    .with_usage(UsageUnit::Minutes, minutes_from_hours(left), Some(minutes_from_hours(total)))
}

fn realtime_metrics(app: &str, row: &RealtimeAuth) -> Vec<Metric> {
    let mut out = Vec::new();
    if let Some(left) = row.left_seconds {
        // Seconds are the vendor unit and the purchase size is not published,
        // so this is a remaining figure, not a bar over an invented total.
        out.push(Metric::text(
            &format!("Realtime transcription ({app})"),
            format!(
                "{} min left ({} s)",
                fmt_count(minutes_from_seconds(left)),
                fmt_count(left)
            ),
        ));
    }
    if let Some(lanes) = row.max_connection {
        // Capacity, not consumption: never folded into a usage number.
        out.push(Metric::text("Max connections", format!("{:.0} lanes", lanes)));
    }
    if let Some(unix_secs) = row.expire_at.filter(|value| *value > 0) {
        out.push(Metric::text("Expires", fmt_expiry(unix_secs * 1000)));
    }
    out
}

fn fmt_count(value: f64) -> String {
    if !value.is_finite() {
        return "—".into();
    }
    format!("{:.*}", if value.fract() == 0.0 { 0 } else { 1 }, value)
}

fn fmt_expiry(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// The stable account id: the console's `account_id` cookie *is* the numeric
/// uid. `/dashboard/user/certification/get` also carries `data.uid`, but the
/// same body returns name / ID-card / mobile, so that endpoint is never
/// requested for identity here.
fn uid_from_cookies(cookies: &[(String, String)]) -> Option<String> {
    cookies
        .iter()
        .find(|(name, _)| name == "account_id")
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(not(feature = "harness"))]
async fn extract_cookies(app: &tauri::AppHandle) -> Vec<(String, String)> {
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
async fn extract_cookies(_app: &tauri::AppHandle) -> Vec<(String, String)> {
    Vec::new()
}

/// Pane hosts the console login in its own incognito WebView2 window; the owner
/// types their own credentials and solves the captcha. Nothing here touches the
/// browser profile stores, and the session is verified by asking the console
/// rather than by "a cookie exists".
#[cfg(not(feature = "harness"))]
pub async fn login(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager as _;
    static LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOGIN_LOCK
        .try_lock()
        .map_err(|_| "An iFlytek console sign-in is already in progress".to_string())?;
    if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = window.set_focus();
        return Err("The iFlytek console sign-in window is already open".into());
    }
    let url = tauri::Url::parse(LOGIN_URL).map_err(|e| format!("login url: {e}"))?;
    tauri::WebviewWindowBuilder::new(app, LOGIN_WINDOW, tauri::WebviewUrl::External(url))
        .title("讯飞开放平台登录")
        .incognito(true)
        .inner_size(520.0, 760.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    let deadline = std::time::Instant::now() + LOGIN_TIMEOUT;
    let mut verified: Option<(String, String)> = None;
    while verified.is_none() {
        let cookies = extract_cookies(app).await;
        if let (Some(uid), Some(jar)) = (
            uid_from_cookies(&cookies),
            (!cookies.is_empty()).then(|| {
                cookies
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            }),
        ) {
            // The console answers 200 even when logged out, so the body decides.
            if matches!(call_get(PROBE_PATH, &jar).await, Ok(ref doc) if envelope_error(doc).is_none()) {
                verified = Some((jar, uid));
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
    let Some((cookie, uid)) = verified else {
        return Err("Sign-in did not finish (window closed)".into());
    };
    let account = LoginAccount {
        account_id: uid.clone(),
        email: String::new(),
        // The uid is the only identity the console gives without also leaking
        // name / ID-card / mobile, so it doubles as the card label.
        label: uid,
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
    Err("iFlytek console sign-in is unavailable in the test harness".into())
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn the_account_id_cookie_is_the_stable_uid_and_nothing_else_qualifies() {
        let cookies = vec![
            ("ssoSessionId".to_string(), "opaque-session-value".to_string()),
            ("account_id".to_string(), " 1234567 ".to_string()),
            ("utm_source".to_string(), "noise".to_string()),
        ];
        assert_eq!(uid_from_cookies(&cookies).as_deref(), Some("1234567"));
        // A missing or non-numeric account_id is not an identity: the console
        // must not end up keying cards on a marketing cookie or a PII value.
        assert_eq!(uid_from_cookies(&[]), None);
        assert_eq!(uid_from_cookies(&[("account_id".into(), "wx_abc".into())]), None);
        assert_eq!(uid_from_cookies(&[("account_id".into(), "".into())]), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn envelope_separates_signed_out_from_a_real_zero() {
        // Measured: HTTP stays 200 for all four shapes; only the body tells.
        assert!(matches!(
            envelope_error(&json!({"flag": false, "code": 80000, "message": "登录异常，请重新登录"})),
            Some(CallErr::Auth)
        ));
        assert!(matches!(
            envelope_error(&json!({"flag": false, "code": 20003})),
            Some(CallErr::Other(_))
        ));
        assert!(envelope_error(&json!({"flag": true, "code": 0, "data": null})).is_none());
        assert!(envelope_error(&json!({"flag": true, "code": 0, "data": {"left": 100}})).is_none());
        // #38's signal must not be read as logged-in here, and vice versa.
        assert!(envelope_error(&json!({"code": 4001, "message": "用户未登录"})).is_none());
        assert!(data(&json!({"flag": true, "code": 0, "data": null})).is_none());
        // An all-null meter body is "no such channel", not a zero balance.
        assert_eq!(parse_count_meter(&json!({"flag": true, "data": null})), None);
        assert_eq!(parse_count_meter(&json!({"flag": true, "data": {}})), None);
    }

    #[test]
    fn console_remaining_is_pack_left_plus_trial_and_never_invents_a_total() {
        let doc = json!({"flag": true, "code": 0, "data": {
            "usedCount": 9, "threshold": 500, "remainCount": 491,
            "left": 10000, "expireDate": "2026-11-03 23:59:59", "conc": 5}});
        let meter = parse_count_meter(&doc).expect("meter");
        assert_eq!(meter.left, Some(10000.0));
        assert_eq!(meter.trial_total, Some(500.0));
        assert_eq!(meter.trial_remaining, Some(491.0));
        assert_eq!(meter.used, Some(9.0));

        // The console's single number is left + trial remaining: 10000 + 491
        // here. Live check 2026-10-23 confirmed that sum (10500 at 500/500)
        // and that no total is published.
        let rows = count_metrics("localdev", "tts", &meter);
        assert_eq!(rows.len(), 2, "one remaining row plus expiry — no fabricated total");
        assert_eq!(rows[0].kind, "text");
        assert_eq!(rows[0].value.as_deref(), Some("10491 left · 491 trial"));
        assert!(rows[0].usage.is_none(), "no invented used/total pair");
        assert_eq!(rows[1].label, "Expires");
        assert!(rows[1].value.as_deref().unwrap_or_default().starts_with("2026-11-03 23:59:59"));
    }

    #[test]
    fn hour_packs_are_read_as_hours_and_sorted_by_their_own_expiry() {
        let doc = json!({"flag": true, "code": 0, "data": [
            {"count": 5, "leftCount": 3, "dueDate": "2027-01-01 00:00:00", "status": 1},
            {"count": 5, "leftCount": 5, "dueDate": "2026-12-01 00:00:00", "status": 1},
        ]});
        let packs = parse_hour_packs(&doc);
        assert_eq!(packs.len(), 2);
        assert_eq!(packs[0].left_hours, Some(3.0));
        // 5 小时 pack 用 Minutes 表达：300 分钟，不是 0.05 之类的怪数。
        let metric = hour_pack_metric("localdev", &packs[0]);
        assert_eq!(metric.usage.as_ref().unwrap().used, 180.0);
        assert_eq!(metric.usage.as_ref().unwrap().total, Some(300.0));
        assert_eq!(metric.used_percent, Some(40.0));
        // Empty and null bodies are normal, never an error row.
        assert!(parse_hour_packs(&json!({"flag": true, "data": []})).is_empty());
        assert!(parse_hour_packs(&json!({"flag": true, "data": null})).is_empty());
    }

    #[test]
    fn realtime_counts_seconds_but_reports_minutes_and_keeps_lanes_separate() {
        let doc = json!({"flag": true, "code": 0, "data": {
            "leftCount": 18000, "usedCount": 600, "maxConnection": 1,
            "expireTime": 1790000000, "connectType": 1, "timeType": 2, "isDel": 0}});
        let row = parse_realtime(&doc).expect("realtime");
        assert_eq!(row.left_seconds, Some(18000.0));
        let rows = realtime_metrics("localdev", &row);
        assert_eq!(rows[0].kind, "text");
        assert_eq!(rows[0].value.as_deref(), Some("300 min left (18000 s)"));
        assert!(rows[0].usage.is_none(), "purchase size is unpublished — no bar over a guess");
        let lanes = rows.iter().find(|m| m.label == "Max connections").unwrap();
        assert_eq!(lanes.value.as_deref(), Some("1 lanes"), "capacity stays its own fact");
        assert!(rows.iter().any(|m| m.label == "Expires" && !m.value.as_deref().unwrap_or_default().is_empty()));
    }

    #[test]
    fn broken_or_unparsable_times_never_become_a_fake_date_or_a_negative_usage() {
        assert_eq!(parse_console_time(None), None);
        assert_eq!(parse_console_time(Some("")), None);
        assert_eq!(parse_console_time(Some("not-a-date")), None);
        assert_eq!(parse_console_time(Some("2026/11/03")), None);
        // A real naive console timestamp resolves in the local zone.
        assert!(parse_console_time(Some("2026-11-03 23:59")).is_some());
        assert!(parse_console_time(Some("2026-11-03 23:59:59")).is_some());

        assert_eq!(minutes_from_seconds(-50.0), 0.0);
        assert_eq!(minutes_from_hours(f64::NAN), 0.0);
        assert_eq!(fmt_count(f64::NAN), "—");
        // An invalid expireTime yields no expiry row instead of a guessed day.
        let rows = realtime_metrics("app", &RealtimeAuth {
            left_seconds: Some(60.0), used_seconds: Some(0.0),
            max_connection: None, expire_at: Some(0),
        });
        assert!(rows.iter().all(|m| m.label != "Expires"));
    }

    #[test]
    fn app_listing_keeps_every_app_and_survives_the_null_service_list() {
        // /dashboard/app/list requires currPage/keyword/type; adding pageSize
        // makes the console answer code 20003. serviceList comes back null, so
        // enabled services cannot be read here.
        let doc = json!({"flag": true, "code": 0, "data": {"totalCount": 2, "currentPageData": [
            {"appId": "APPIDONE", "appName": "localdev", "serviceList": null},
            {"appId": "APPIDTWO", "appName": "second", "serviceList": null},
            {"appId": "", "appName": "junk"},
        ]}});
        let apps = parse_apps(&doc);
        assert_eq!(apps.len(), 2, "every app under the account must be probed");
        assert_eq!(apps[1].name, "second");
        assert!(parse_apps(&json!({"flag": true, "data": null})).is_empty());
    }
}
