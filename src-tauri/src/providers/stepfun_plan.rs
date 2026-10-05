//! StepFun Step Plan subscription quota — covers BOTH billing shapes the
//! platform has run since the 2026-06-18 upgrade: the legacy Coding Plan
//! (rolling 5h/weekly windows, prompt-counted) and the Token Plan (monthly
//! Credit pool + 30-day top-up packs). Classification is by payload shape,
//! NOT by `plan_family` — CodexBar issue #2491 shows the family id lies
//! during migration.
//!
//! Credentials live in `%APPDATA%/Pane/stepfun-plan.json` (same file the
//! Settings paste-box has always written; a legacy `{apiKey}` blob is read
//! as `token`). The stored token is an Oasis-Token pair
//! "<access JWT>...<refresh JWT>" — the refresh half carries the device_id
//! the platform demands in Oasis-Webid, a bare access JWT gets rejected as
//! "embezzled".
//!
//! Autonomy ladder (no timers — renew on use, same shape as sensenova):
//!   L0  cached access JWT whose exp is still ahead → just fetch.
//!   L1  expired/401 → POST RefreshToken with the stored pair (single-flight
//!       + backoff); a rotated pair is persisted atomically.
//!   L2  refresh dead (invalid grant) but username+password are on file →
//!       the full 3-step login (INGRESSCOOKIE → RegisterDevice →
//!       SignInByPassword) — every password mistake backs off 30s → 5m →
//!       30m, then stops until the next process start.
//!   L3  nothing on file → no_credentials: fill the account in Settings.
//! HTTP 429 is throttling, never touches the ladder.

use super::{http, json_body, Metric, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

const ID: &str = "stepfun-plan";
const NAME: &str = "StepFun Step Plan";
const PLATFORM: &str = "https://platform.stepfun.com";
const RATE_URL: &str =
    "https://platform.stepfun.com/api/step.openapi.devcenter.Dashboard/QueryStepPlanRateLimit";
const STATUS_URL: &str =
    "https://platform.stepfun.com/api/step.openapi.devcenter.Dashboard/GetStepPlanStatus";
const REGISTER_DEVICE_URL: &str =
    "https://platform.stepfun.com/passport/proto.api.passport.v1.PassportService/RegisterDevice";
const SIGN_IN_URL: &str =
    "https://platform.stepfun.com/passport/proto.api.passport.v1.PassportService/SignInByPassword";
const REFRESH_URL: &str =
    "https://platform.stepfun.com/passport/proto.api.passport.v1.PassportService/RefreshToken";
const APP_ID: &str = "10300";
/// The web frontend's pre-registration webid — the passport endpoints
/// 4xx without it, and it doubles as the fallback when a token pair carries
/// no device_id claim (derivation cases the platform rejects by name).
const DEFAULT_WEBID: &str = "c8a1002d2c457e758785a9979832217c7c0b884c";
/// The passport layer fingerprint-checks the UA; a non-browser agent string
/// gets a 403 on some edges.
const BROWSER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36";
const MAX_BODY_BYTES: usize = 64 * 1024;

const BACKOFFS_MS: [i64; 3] = [30_000, 5 * 60_000, 30 * 60_000];

/// Stored credential triple. `apiKey` is read as `token` so the file the
/// Settings paste-box always wrote keeps working untouched.
#[derive(Clone, Default, Serialize, Deserialize)]
struct Creds {
    #[serde(default, alias = "apiKey")]
    token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    username: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    password: String,
}

fn creds_path() -> PathBuf {
    super::config_dir().join("stepfun-plan.json")
}

fn load_creds() -> Creds {
    std::fs::read_to_string(creds_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Atomic persist: sibling temp file + rename, so a crash mid-write can
/// never leave a half credential file.
fn save_creds(creds: &Creds) {
    let path = creds_path();
    let tmp = path.with_extension("json.tmp");
    let ok = serde_json::to_string_pretty(creds)
        .ok()
        .and_then(|s| std::fs::write(&tmp, s).ok())
        .is_some_and(|()| std::fs::rename(&tmp, &path).is_ok());
    if !ok {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Renewal bookkeeping: single-flight so a concurrent refresh cycle can't
/// pile duplicate logins onto PassportService, plus backoff after repeated
/// failures (a wrong password should be heard once, not every 30s).
#[derive(Default)]
struct Renew {
    in_flight: bool,
    failures: u32,
    next_attempt_ms: i64,
}

static RENEW: Mutex<Renew> = Mutex::new(Renew {
    in_flight: false,
    failures: 0,
    next_attempt_ms: 0,
});

/// Why a login/refresh failed — decides backoff and user messaging.
#[derive(Debug)]
enum AuthErr {
    /// Credential rejected (401/invalid grant/bad password): final for this
    /// attempt, back off, never silently retry the same secret.
    Invalid(String),
    /// Network/5xx/parse trouble: retryable with backoff.
    Transient(String),
}

type AuthResult<T> = Result<T, AuthErr>;

pub fn local_credential_hint() -> Option<String> {
    let c = load_creds();
    (!c.token.is_empty() || (!c.username.is_empty() && !c.password.is_empty()))
        .then(|| "Pane StepFun Step Plan session".into())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of pasted credentials (Customize "Test"); never saved. A pasted
/// token is used as-is; account+password runs a throwaway login.
pub async fn snapshot_with_creds(token: &str, username: &str, password: &str) -> Snapshot {
    let token = token.trim();
    if !token.is_empty() {
        return match fetch_with(token).await {
            Ok(s) => s,
            Err(e) => Snapshot::error(ID, NAME, describe_fetch(e)),
        };
    }
    match full_login(username.trim(), password).await {
        Ok(tok) => match fetch_with(&tok).await {
            Ok(s) => s,
            Err(e) => Snapshot::error(ID, NAME, describe_fetch(e)),
        },
        Err(AuthErr::Invalid(m)) => {
            Snapshot::error(ID, NAME, format!("Step Plan sign-in failed: {m}"))
        }
        Err(AuthErr::Transient(m)) => {
            Snapshot::error(ID, NAME, format!("Step Plan sign-in unavailable: {m}"))
        }
    }
}

fn describe_fetch(e: FetchErr) -> String {
    match e {
        FetchErr::Auth => "Step Plan session was rejected".into(),
        FetchErr::Other(m) => m,
    }
}

/// Settings "Save": persist the triple. Account+password alone triggers one
/// real login so the stored token pair is born here (and any typo in the
/// password is caught at save time, not at the next refresh).
pub async fn save_creds_interactive(
    token: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let mut creds = load_creds();
    // Non-empty fields overwrite; an empty field keeps whatever is stored —
    // filling only the account must not wipe the token pair (and vice versa).
    if !token.trim().is_empty() {
        creds.token = token.trim().to_string();
    }
    if !username.trim().is_empty() {
        creds.username = username.trim().to_string();
    }
    if !password.is_empty() {
        creds.password = password.to_string();
    }
    if creds.token.is_empty() && (!creds.username.is_empty() && !creds.password.is_empty()) {
        let tok = full_login(&creds.username, &creds.password)
            .await
            .map_err(|e| match e {
                AuthErr::Invalid(m) => format!("sign-in failed: {m}"),
                AuthErr::Transient(m) => format!("sign-in unavailable: {m}"),
            })?;
        creds.token = tok;
    }
    save_creds(&creds);
    Ok(())
}

pub fn clear_creds() {
    let _ = std::fs::remove_file(creds_path());
}

/// The stored Oasis-Webid for a pair: the refresh half's `device_id` claim
/// (reverse order — that's where the platform puts it), else the frontend's
/// default. The server cross-checks this value against the token's device
/// and rejects a mismatch as "oasis-token is embezzled".
fn webid_for_token(token: &str) -> String {
    for half in token.rsplit("...") {
        if let Some(id) = device_id_of_jwt(half).filter(|s| !s.is_empty()) {
            return id;
        }
    }
    DEFAULT_WEBID.to_string()
}

fn device_id_of_jwt(jwt: &str) -> Option<String> {
    use base64::Engine;
    let payload_b64 = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64))
        .ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v.get("device_id")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Access-segment JWT exp as ms-from-now (60s skew floor); None when the
/// token is unparseable or the pair has no access half.
fn access_expires_in_ms(token: &str) -> Option<i64> {
    use base64::Engine;
    let jwt = token.split("...").next()?;
    let payload_b64 = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64))
        .ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    let exp = v.get("exp")?.as_i64()?;
    Some(exp * 1000 - chrono::Utc::now().timestamp_millis())
}

async fn fetch() -> Result<Snapshot, String> {
    let mut creds = load_creds();
    if creds.token.is_empty() && creds.username.is_empty() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Sign in to StepFun Step Plan in Settings (account + password keeps you signed in).",
        ));
    }

    // L0: locally-valid access JWT → straight to the quota endpoint. A 401
    // despite a valid local exp (revoked, clock skew) falls through.
    if access_expires_in_ms(&creds.token).is_some_and(|ms| ms > 60_000) {
        match fetch_with(&creds.token).await {
            Ok(s) => return Ok(s),
            Err(FetchErr::Auth) => {}
            Err(e) => return Err(describe_fetch(e)),
        }
    }

    // L1: rotate the pair via RefreshToken (single-flight + backoff).
    let refreshed = if !creds.token.is_empty() {
        renew(&mut creds).await
    } else {
        None
    };
    let token = match refreshed {
        Some(t) => t,
        None => {
            // L2: refresh impossible/dead — full login if the account is on
            // file (its own single-flight + backoff inside).
            if !creds.username.is_empty() && !creds.password.is_empty() {
                match full_login(&creds.username, &creds.password).await {
                    Ok(t) => {
                        creds.token = t.clone();
                        save_creds(&creds);
                        t
                    }
                    Err(e) => return Err(describe_login(&e)),
                }
            } else {
                return Err(
                    "Step Plan session expired — sign in again in Settings (account + password)"
                        .to_string(),
                );
            }
        }
    };
    match fetch_with(&token).await {
        Ok(s) => Ok(s),
        Err(FetchErr::Auth) => Err(
            "Step Plan session was rejected — check your account in Settings".to_string(),
        ),
        Err(FetchErr::Other(m)) => Err(m),
    }
}

/// One RefreshToken attempt under the single-flight lock; returns the new
/// pair on success (creds already persisted with the rotated pair).
async fn renew(creds: &mut Creds) -> Option<String> {
    // Scoped so the guard provably dies before the await below (the guarded
    // future must stay Send for lib.rs's refresh pool).
    {
        let mut guard = RENEW.lock().ok()?;
        if guard.in_flight {
            return None;
        }
        let now = chrono::Utc::now().timestamp_millis();
        if guard.failures > 0 && now < guard.next_attempt_ms {
            return None;
        }
        guard.in_flight = true;
    }

    let result = refresh_pair(&creds.token).await;

    {
        let mut guard = RENEW.lock().ok()?;
        guard.in_flight = false;
        match result {
            Ok(pair) => {
                guard.failures = 0;
                guard.next_attempt_ms = 0;
                creds.token = pair.clone();
                save_creds(creds);
                Some(pair)
            }
            Err(_) => {
                let step = (guard.failures as usize).min(BACKOFFS_MS.len() - 1);
                guard.next_attempt_ms = chrono::Utc::now().timestamp_millis() + BACKOFFS_MS[step];
                guard.failures += 1;
                None
            }
        }
    }
}

fn describe_login(e: &AuthErr) -> String {
    match e {
        AuthErr::Invalid(m) => format!("Step Plan sign-in failed: {m}"),
        AuthErr::Transient(m) => format!("Step Plan sign-in unavailable: {m}"),
    }
}

/// Full 3-step login (no lock — callers hold the RENEW flight context or are
/// interactive one-shots): INGRESSCOOKIE → RegisterDevice → SignInByPassword.
async fn full_login(username: &str, password: &str) -> AuthResult<String> {
    let ingress = get_ingress_cookie().await?;
    let anon = register_device(&ingress).await?;
    let webid = webid_for_token(&anon);
    let resp = http()
        .post(SIGN_IN_URL)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", &webid)
        .header(
            "Cookie",
            format!("Oasis-Token={anon}; Oasis-Webid={webid}; INGRESSCOOKIE={ingress}"),
        )
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if resp.status().as_u16() == 429 {
        return Err(AuthErr::Transient("rate-limited".into()));
    }
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        // A wrong password answers 200-with-error or 4xx — both final.
        return Err(AuthErr::Invalid(format!(
            "HTTP {status} {}",
            body.chars().take(160).collect::<String>()
        )));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| AuthErr::Transient(format!("body: {e}")))?;
    combine_token(&doc, "login")
        .ok_or_else(|| AuthErr::Invalid("no access token in login response".into()))
}

/// Step 1: GET the platform homepage and harvest INGRESSCOOKIE from
/// Set-Cookie.
async fn get_ingress_cookie() -> AuthResult<String> {
    let resp = http()
        .get(PLATFORM)
        .header("User-Agent", BROWSER_UA)
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", DEFAULT_WEBID)
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if !resp.status().is_success() {
        return Err(AuthErr::Transient(format!("homepage HTTP {}", resp.status())));
    }
    for value in resp.headers().get_all("set-cookie") {
        let Ok(s) = value.to_str() else { continue };
        if let Some(rest) = s.split("INGRESSCOOKIE=").nth(1) {
            let cookie = rest.split(';').next().unwrap_or("").trim();
            if !cookie.is_empty() {
                return Ok(cookie.to_string());
            }
        }
    }
    Err(AuthErr::Transient("INGRESSCOOKIE not issued".into()))
}

/// Step 2: anonymous device registration → "access...refresh" pair.
async fn register_device(ingress: &str) -> AuthResult<String> {
    let resp = http()
        .post(REGISTER_DEVICE_URL)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", DEFAULT_WEBID)
        .header("Cookie", format!("INGRESSCOOKIE={ingress}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if !resp.status().is_success() {
        return Err(AuthErr::Transient(format!(
            "device registration HTTP {}",
            resp.status()
        )));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| AuthErr::Transient(format!("body: {e}")))?;
    combine_token(&doc, "device")
        .ok_or_else(|| AuthErr::Transient("no device token".into()))
}

/// Rotate the stored pair. A 4xx body ("token is expired"/"illegal") means
/// the pair is dead for good — that routes the caller to full login.
async fn refresh_pair(token: &str) -> AuthResult<String> {
    let webid = webid_for_token(token);
    let resp = http()
        .post(REFRESH_URL)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", &webid)
        .header("Oasis-Token", token)
        .header("Cookie", format!("Oasis-Token={token}; Oasis-Webid={webid}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if !resp.status().is_success() {
        return Err(AuthErr::Invalid(format!("refresh HTTP {}", resp.status())));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| AuthErr::Transient(format!("body: {e}")))?;
    combine_token(&doc, "refresh").ok_or_else(|| AuthErr::Invalid("no token in refresh".into()))
}

/// {accessToken:{raw}, refreshToken:{raw}} → "<access>...<refresh>" — the
/// pair form the platform's device_id check expects.
fn combine_token(doc: &Value, _what: &str) -> Option<String> {
    let access = doc
        .get("accessToken")
        .and_then(|t| t.get("raw"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())?;
    match doc
        .get("refreshToken")
        .and_then(|t| t.get("raw"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        Some(refresh) => Some(format!("{access}...{refresh}")),
        None => Some(access.to_string()),
    }
}

/// How a quota fetch failed — 401/403 sends the ladder running, 429 is
/// plain throttling that must not.
enum FetchErr {
    Auth,
    Other(String),
}

impl FetchErr {
    fn from_status(code: u16) -> Option<Self> {
        match code {
            401 | 403 => Some(FetchErr::Auth),
            429 => Some(FetchErr::Other(
                "Step Plan rate-limited — retrying next cycle".into(),
            )),
            _ => None,
        }
    }
}

async fn post_usage(token: &str, url: &str) -> Result<Value, FetchErr> {
    let webid = webid_for_token(token);
    let mut req = http()
        .post(url)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", &webid)
        .header("Cookie", format!("Oasis-Token={token}; Oasis-Webid={webid}"));
    if url != SIGN_IN_URL {
        req = req.header("Oasis-Token", token);
    }
    let r = req
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| FetchErr::Other(format!("Step Plan request: {e}")))?;
    if let Some(e) = FetchErr::from_status(r.status().as_u16()) {
        return Err(e);
    }
    if !r.status().is_success() {
        return Err(FetchErr::Other(format!(
            "Step Plan endpoint: HTTP {}",
            r.status()
        )));
    }
    json_body(r, MAX_BODY_BYTES, "Step Plan")
        .await
        .map_err(FetchErr::Other)
}

async fn fetch_with(token: &str) -> Result<Snapshot, FetchErr> {
    let rate = post_usage(token, RATE_URL).await?;
    let status = post_usage(token, STATUS_URL).await.ok();
    parse_snapshot(&rate, status.as_ref()).map_err(FetchErr::Other)
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(|x| x.as_f64().or_else(|| x.as_str()?.parse().ok()))
}

fn ts(v: Option<&Value>) -> Option<i64> {
    num(v).map(|s| {
        let secs = s as i64;
        if secs > 0 && secs < 2_000_000_000_000 {
            secs * 1000
        } else {
            secs
        }
    })
}

/// Shape decides the rows: a live rolling window means the legacy Coding
/// Plan (its reset_time is in the future); otherwise, if the credit pool is
/// present, this is the Token Plan. Both missing → no usable data.
fn parse_snapshot(rate: &Value, status: Option<&Value>) -> Result<Snapshot, String> {
    let mut m = Vec::new();
    let plan = status
        .and_then(|v| v.pointer("/subscription/name").and_then(Value::as_str))
        .map(str::to_owned)
        .or_else(|| Some("Step Plan".into()));

    let window_live = ts(rate.get("five_hour_usage_reset_time")).is_some_and(|ms| ms > 0)
        || ts(rate.get("weekly_usage_reset_time")).is_some_and(|ms| ms > 0);

    if window_live {
        if let Some(left) = num(rate.get("five_hour_usage_left_rate")) {
            m.push(
                Metric::progress("Session", (1.0 - left) * 100.0, None).with_reset(
                    ts(rate.get("five_hour_usage_reset_time")),
                    Some(5 * 60 * 60 * 1000),
                ),
            );
        }
        if let Some(left) = num(rate.get("weekly_usage_left_rate")) {
            m.push(
                Metric::progress("Weekly", (1.0 - left) * 100.0, None).with_reset(
                    ts(rate.get("weekly_usage_reset_time")),
                    Some(7 * 86_400_000),
                ),
            );
        }
    } else if let Some(c) = rate.get("plan_credit_rate_limit") {
        if let Some(left) = num(c.get("subscription_credit_left_rate")) {
            m.push(
                Metric::progress(
                    "Credit",
                    (1.0 - left) * 100.0,
                    Some(format!("{:.1}% remaining", left * 100.0)),
                )
                .with_reset(
                    ts(c.get("subscription_credit_reset_time")),
                    Some(30 * 86_400_000),
                ),
            );
        }
        if num(c.get("topup_credit_left_rate")).is_some_and(|r| r > 0.0) {
            let left = num(c.get("topup_credit_left_rate")).unwrap_or(0.0);
            m.push(
                Metric::progress(
                    "Top-up",
                    (1.0 - left) * 100.0,
                    Some(format!("{:.1}% remaining", left * 100.0)),
                )
                .with_reset(None, Some(30 * 86_400_000)),
            );
        }
    }

    if m.is_empty() {
        return Err("Step Plan returned no quota data".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim shape from a CodexBar fixture (2026-10) — string decimals,
    /// pool + one 30-day bucket. topup_credit_left_rate = 0 → no Top-up row.
    const CREDIT_SAMPLE: &str = r#"{"plan_credit_rate_limit":{
        "subscription_credit_left_rate":0.9641096,
        "subscription_credit_reset_time":"1786288293",
        "topup_credit_left_rate":0,
        "credit_buckets":[{"type":1,"credit_total":"400000000","credit_residual":"385643840","expire_at":"1789000000","next_reset_at":"0"}]
    }}"#;

    /// Legacy Coding Plan: live windows (reset timestamps in the future).
    const WINDOW_SAMPLE: &str = r#"{
        "plan_family":1,
        "five_hour_usage_left_rate":0.99781543,
        "five_hour_usage_reset_time":"1791012792",
        "weekly_usage_left_rate":0.85,
        "weekly_usage_reset_time":"1791181992"
    }"#;

    #[test]
    fn parses_credit_pool_shape() {
        let doc: Value = serde_json::from_str(CREDIT_SAMPLE).unwrap();
        let snap = parse_snapshot(&doc, None).expect("parse");
        assert_eq!(snap.status, "ok");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Credit"]);
        let c = &snap.metrics[0];
        // 0.9641096 left → ~3.59% used
        assert!((c.used_percent.unwrap() - 3.589).abs() < 0.01);
        assert_eq!(c.resets_at, Some(1786288293 * 1000));
        assert_eq!(c.period_ms, Some(30 * 86_400_000));
    }

    #[test]
    fn topup_row_appears_only_when_positive() {
        let doc = serde_json::json!({
            "plan_credit_rate_limit": {
                "subscription_credit_left_rate": 0.5,
                "subscription_credit_reset_time": "1786288293",
                "topup_credit_left_rate": 0.5
            }
        });
        let snap = parse_snapshot(&doc, None).expect("parse");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Credit", "Top-up"]);
    }

    #[test]
    fn live_window_beats_credit_pool_during_migration() {
        // Migration decks can carry both; a live reset_time wins.
        let doc = serde_json::json!({
            "plan_family": 2,
            "five_hour_usage_left_rate": 0.4,
            "five_hour_usage_reset_time": "1791012792",
            "weekly_usage_left_rate": 0.9,
            "weekly_usage_reset_time": "1791181992",
            "plan_credit_rate_limit": { "subscription_credit_left_rate": 0.9 }
        });
        let snap = parse_snapshot(&doc, None).expect("parse");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Session", "Weekly"]);
    }

    #[test]
    fn parses_window_shape() {
        let doc: Value = serde_json::from_str(WINDOW_SAMPLE).unwrap();
        let snap = parse_snapshot(&doc, None).expect("parse");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Session", "Weekly"]);
        // 0.99781543 left → 0.218% used
        assert!((snap.metrics[0].used_percent.unwrap() - 0.218).abs() < 0.01);
    }

    #[test]
    fn empty_payload_is_an_error() {
        assert!(parse_snapshot(&serde_json::json!({"plan_family": 2}), None).is_err());
    }

    #[test]
    fn webid_prefers_refresh_half_device_id_then_default() {
        use base64::Engine;
        let access = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"exp":9999999999}"#);
        let refresh = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"device_id":"abc123","exp":9999999999}"#);
        let pair = format!("h.{access}.s...h.{refresh}.s");
        assert_eq!(webid_for_token(&pair), "abc123");
        // No device claim anywhere → frontend default.
        assert_eq!(webid_for_token("h.payload.s"), DEFAULT_WEBID);
    }

    #[test]
    fn access_exp_prechecks_the_access_half() {
        use base64::Engine;
        let far_future =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"exp":9999999999}"#);
        let pair = format!("h.{far_future}.s...h.{far_future}.s");
        assert!(access_expires_in_ms(&pair).is_some_and(|ms| ms > 3_000_000_000_000));
        // A single bare JWT parses as its own access half.
        assert!(access_expires_in_ms(&format!("h.{far_future}.s")).is_some());
        assert_eq!(access_expires_in_ms("garbage"), None);
    }

    #[test]
    fn combine_token_builds_the_pair_form() {
        let doc = serde_json::json!({
            "accessToken": {"raw": "ACCESS"},
            "refreshToken": {"raw": "REFRESH"}
        });
        assert_eq!(combine_token(&doc, "login").as_deref(), Some("ACCESS...REFRESH"));
        let doc = serde_json::json!({"accessToken": {"raw": "ACCESS"}});
        assert_eq!(combine_token(&doc, "device").as_deref(), Some("ACCESS"));
        let doc = serde_json::json!({"refreshToken": {"raw": "X"}});
        assert_eq!(combine_token(&doc, "login"), None);
    }

    #[test]
    fn legacy_api_key_blob_reads_as_token() {
        let json = r#"{"apiKey":"HEADER.payload.sig"}"#;
        let creds: Creds = serde_json::from_str(json).unwrap();
        assert_eq!(creds.token, "HEADER.payload.sig");
        assert!(creds.username.is_empty() && creds.password.is_empty());
        // The new shape round-trips and omits empty secrets.
        let c = Creds {
            token: "t".into(),
            username: "u".into(),
            password: String::new(),
        };
        let s = serde_json::to_string(&c).unwrap();
        assert!(!s.contains("password"));
    }

    #[test]
    fn separate_id() {
        assert_eq!(ID, "stepfun-plan");
        assert_eq!(BACKOFFS_MS, [30_000, 300_000, 1_800_000]);
    }
}
