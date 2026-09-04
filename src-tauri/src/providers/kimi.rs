//! Kimi Code — one card like OpenCode: Session + Weekly from the
//! subscription, plus an API bar from the Moonshot pay-as-you-go wallet
//! when a key is saved in Settings.
//!
//! Quota comes from GET api.kimi.com/coding/v1/usages, authenticated with
//! the official Kimi Code CLI's OAuth login
//! (`%USERPROFILE%\.kimi-code\credentials\kimi-code.json`) or, when no CLI
//! login exists, a Kimi Coding API key pasted in Settings (env:
//! KIMI_CODING_API_KEY) — the endpoint accepts both. A saved key also
//! takes over when the CLI login dies (rotated refresh token), so the
//! card recovers without waiting for a terminal `kimi login`. OAuth
//! tokens refresh
//! against auth.kimi.com and are written back beside the CLI's file so the
//! CLI stays signed in — same write-back as Claude/Codex.
//!
//! Without a CLI login, a Kimi For Coding plan key pasted in Settings
//! (`%APPDATA%\Pane\kimi.json`) is sent as Bearer to the same endpoint —
//! what cc-switch and the plan's Anthropic-compatible endpoint accept
//! (issue #173). Login wins when both exist; the key is a fallback only.

use super::{http, stored_api_key, Metric, Snapshot};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ID: &str = "kimi";
const NAME: &str = "Kimi Code";

/// Shown when the CLI login is beyond refresh and no API key is saved.
const ROTATED_HINT: &str = "Kimi Code sign-in was rotated — run `kimi login` in a terminal once and Pane recovers automatically";
/// Public OAuth client id the official CLI (and OpenUsage) uses.
const CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
const USAGES_URL: &str = "https://api.kimi.com/coding/v1/usages";
const TOKEN_URL: &str = "https://auth.kimi.com/api/oauth/token";
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
const MAX_CRED_BYTES: u64 = 64 * 1024;
const MAX_USAGES_BYTES: usize = 256 * 1024;
const MAX_TOKEN_BYTES: usize = 16 * 1024;

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a pasted Kimi Coding API key, without saving it
/// (Customize "Test"). The same usages path a saved key rides.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_api_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live API-key query for one extra account. Kimi's Moonshot wallet is a
/// single global credential source, so account cards include only the Kimi
/// Coding plan rows and keep their own identity.
pub async fn snapshot_with_key_as(key: &str, card_id: &str, card_name: &str) -> Snapshot {
    match fetch_api_key_without_wallet(key).await {
        Ok(snap) => with_card_identity(snap, card_id, card_name),
        Err(error) => Snapshot::error(card_id, card_name, error),
    }
}

pub fn has_login() -> bool {
    cred_path().is_some()
}

/// Kimi Coding plan key from Pane Settings or its provider-specific env var.
/// Do not read `ANTHROPIC_AUTH_TOKEN`: that is commonly shared with a router
/// and would grab whichever vendor the router currently points at.
fn plan_key() -> Option<String> {
    stored_api_key("kimi", &["KIMI_CODING_API_KEY"])
}

pub fn has_plan_key() -> bool {
    plan_key().is_some()
}

pub fn preferred_source(has_key: bool, has_oauth: bool) -> Option<&'static str> {
    if has_key {
        Some("api_key")
    } else if has_oauth {
        Some("oauth")
    } else {
        None
    }
}

/// Anything that can produce the plan card: CLI login or pasted plan key.
/// Spend routing and the Moonshot fold key off this, not `has_login`.
pub fn has_credentials() -> bool {
    has_login() || has_plan_key()
}

/// Pure local probe for the Customize gear panel (no network): the Kimi
/// Code CLI's OAuth credential file exists on this machine.
pub fn local_credential_hint() -> Option<String> {
    cred_path().map(|_| "Kimi Code CLI OAuth sign-in".to_string())
}

/// Official CLI home. `KIMI_CODE_HOME` is honored only when it is an
/// absolute path — a relative value would resolve against Pane's cwd and
/// could point the spend walk or credential write at the wrong tree.
pub fn code_home() -> PathBuf {
    if let Some(p) = env_code_home() {
        return p;
    }
    dirs::home_dir().unwrap_or_default().join(".kimi-code")
}

fn env_code_home() -> Option<PathBuf> {
    let raw = std::env::var("KIMI_CODE_HOME").ok()?;
    absolute_home(&raw)
}

fn absolute_home(raw: &str) -> Option<PathBuf> {
    let p = PathBuf::from(raw.trim());
    if p.as_os_str().is_empty() || !p.is_absolute() {
        return None;
    }
    #[cfg(windows)]
    {
        use std::path::Component;
        // `\Windows\...` is "absolute" on Windows but drive-relative.
        // Only honor a real drive/UNC prefix so a planted env can't aim
        // credential writes at the current-drive root.
        if !matches!(p.components().next(), Some(Component::Prefix(_))) {
            return None;
        }
    }
    Some(p)
}

fn cred_candidates(kimi_home: Option<PathBuf>, user_home: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(home) = kimi_home {
        candidates.push(home.join("credentials").join("kimi-code.json"));
    }
    if let Some(home) = user_home {
        candidates.push(home.join(".kimi-code").join("credentials").join("kimi-code.json"));
        // OpenUsage/Mac spelling — keep as a fallback.
        candidates.push(home.join(".kimi").join("credentials").join("kimi-code.json"));
    }
    candidates
}

fn cred_path() -> Option<PathBuf> {
    cred_candidates(env_code_home(), dirs::home_dir())
        .into_iter()
        .find(|p| is_regular_file(p))
}

/// Which credential feeds this refresh: an explicitly configured Kimi
/// Coding API key first, otherwise the CLI's OAuth credential. This prevents
/// an old OAuth file from silently taking over a user-supplied key.
enum Credential {
    ApiKey(String),
    OAuth(PathBuf),
}

fn pick_credential(cred: Option<PathBuf>, key: Option<String>) -> Option<Credential> {
    if let Some(key) = key {
        return Some(Credential::ApiKey(key));
    }
    cred.map(Credential::OAuth)
}

/// Why the CLI's OAuth login produced no snapshot. `Rotated` means both
/// the stored token and a forced refresh were rejected — the login stays
/// dead until the next `kimi login`; anything else is transient and the
/// next refresh cycle retries it.
enum OAuthFailure {
    Rotated,
    Other(String),
}

fn classify_oauth_error(e: String) -> OAuthFailure {
    if e == ROTATED_HINT {
        OAuthFailure::Rotated
    } else {
        OAuthFailure::Other(e)
    }
}

/// After a dead OAuth login the saved key takes over; without one the
/// original sign-in guidance stands.
fn rotated_fallback(key: Option<String>) -> Result<String, String> {
    key.ok_or_else(|| ROTATED_HINT.to_string())
}

async fn fetch() -> Result<Snapshot, String> {
    let key = plan_key();
    let credential = pick_credential(cred_path(), key.clone());
    let Some(credential) = credential else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Kimi Code sign-in not found. Run `kimi login` in a terminal, or paste your Kimi For Coding key in Settings (gear icon).",
        ));
    };
    match credential {
        Credential::ApiKey(key) => fetch_api_key(&key).await,
        Credential::OAuth(path) => match fetch_oauth(&path).await {
            Ok(snap) => Ok(snap),
            // The CLI login died (rotated refresh token): a pasted key
            // takes over instead of blanking the card until `kimi login`.
            Err(OAuthFailure::Rotated) => match rotated_fallback(key) {
                Ok(key) => fetch_api_key(&key).await,
                Err(e) => Err(e),
            },
            Err(OAuthFailure::Other(e)) => Err(e),
        },
    }
}

/// Pasted-key path: the same usages endpoint accepts an API key directly.
/// Unlike OAuth there is no refresh token, so a 401 is final.
async fn fetch_api_key(key: &str) -> Result<Snapshot, String> {
    fetch_api_key_internal(key, true).await
}

async fn fetch_api_key_without_wallet(key: &str) -> Result<Snapshot, String> {
    fetch_api_key_internal(key, false).await
}

async fn fetch_api_key_internal(key: &str, include_wallet: bool) -> Result<Snapshot, String> {
    let (usages, api) = if include_wallet {
        fetch_usages_and_wallet(key).await
    } else {
        (fetch_usages(key).await, Ok(Vec::new()))
    };
    let mut snap = match usages {
        Ok(doc) => parse_snapshot(&doc)?,
        Err(e) => return Err(key_usages_error(e)),
    };
    if include_wallet {
        merge_wallet_rows(&mut snap, api);
    }
    Ok(snap)
}

fn with_card_identity(mut snap: Snapshot, card_id: &str, card_name: &str) -> Snapshot {
    snap.id = card_id.to_string();
    snap.name = card_name.to_string();
    snap
}

/// A rejected pasted key is a deterministic failure — unlike the OAuth
/// path there is no refresh token to retry with.
fn key_usages_error(e: UsagesError) -> String {
    match e {
        UsagesError::Unauthorized => {
            "Kimi Coding API key was rejected — check it in Settings".into()
        }
        UsagesError::Other(e) => e,
    }
}

async fn fetch_oauth(path: &Path) -> Result<Snapshot, OAuthFailure> {
    let access = load_access(path, false).await.map_err(classify_oauth_error)?;
    let (usages, api) = fetch_usages_and_wallet(&access).await;
    let mut snap = match usages {
        Ok(doc) => parse_snapshot(&doc).map_err(OAuthFailure::Other)?,
        Err(UsagesError::Unauthorized) => {
            let access = load_access(path, true).await.map_err(classify_oauth_error)?;
            match fetch_usages(&access).await {
                Ok(doc) => parse_snapshot(&doc).map_err(OAuthFailure::Other)?,
                Err(UsagesError::Unauthorized) => return Err(OAuthFailure::Rotated),
                Err(UsagesError::Other(e)) => return Err(OAuthFailure::Other(e)),
            }
        }
        Err(UsagesError::Other(e)) => return Err(OAuthFailure::Other(e)),
    };
    merge_wallet_rows(&mut snap, api);
    Ok(snap)
}

/// Plan usages plus, when wanted, the Moonshot wallet rows in parallel.
/// No Moonshot key, or Moonshot switched off → Session + Weekly only.
/// Disabled Moonshot must not be contacted through this folded card.
async fn fetch_usages_and_wallet(
    access: &str,
) -> (Result<Value, UsagesError>, Result<Vec<Metric>, String>) {
    if super::moonshot::wallet_wanted() {
        tokio::join!(fetch_usages(access), super::moonshot::api_rows())
    } else {
        (fetch_usages(access).await, Ok(Vec::new()))
    }
}

fn merge_wallet_rows(snap: &mut Snapshot, api: Result<Vec<Metric>, String>) {
    match api {
        Ok(rows) => snap.metrics.extend(rows),
        Err(_) => {
            snap.warning =
                Some("Moonshot API wallet couldn't refresh — retrying next cycle".into());
        }
    }
}

enum UsagesError {
    Unauthorized,
    Other(String),
}


async fn fetch_usages(access: &str) -> Result<Value, UsagesError> {
    let resp = http()
        .get(USAGES_URL)
        .bearer_auth(access)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| UsagesError::Other(format!("usage request: {e}")))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(UsagesError::Unauthorized);
    }
    if !status.is_success() {
        return Err(UsagesError::Other(format!("usage endpoint: HTTP {status}")));
    }
    super::json_body(resp, MAX_USAGES_BYTES, "usage")
        .await
        .map_err(UsagesError::Other)
}

/// Load a usable access token, refreshing when expired (or when `force`).
/// Refresh tokens rotate — write the new pair back so the CLI stays signed in.
async fn load_access(path: &Path, force: bool) -> Result<String, String> {
    let raw = super::read_small_text(path, MAX_CRED_BYTES, "credentials")?;
    let mut doc: Value = serde_json::from_str(&raw).map_err(|e| format!("parse credentials: {e}"))?;

    let mut access = doc
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let refresh = doc
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let expires_ms = expires_at_ms(doc.get("expires_at"));
    let now_ms = Utc::now().timestamp_millis();
    // 5-minute buffer matches OpenUsage; Kimi access tokens are short-lived.
    let stale = access.is_empty() || expires_ms <= now_ms + 5 * 60_000;
    if !force && !stale {
        return Ok(access);
    }
    if refresh.is_empty() {
        return Err(
            "token expired and no refresh token present — run `kimi login` in a terminal".into(),
        );
    }
    // Refresh rotates the CLI's refresh token. If we can't write it back
    // (a planted symlink), don't call the token endpoint — that would
    // sign the CLI out from under the user.
    if !can_write_creds(path) {
        if !access.is_empty() && !force {
            return Ok(access);
        }
        return Err(
            "Kimi Code credentials are not a regular file — run `kimi login` in a terminal".into(),
        );
    }

    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh.as_str()),
        ("client_id", CLIENT_ID),
    ];
    let resp = http()
        .post(TOKEN_URL)
        .header("Accept", "application/json")
        .form(&form)
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("token refresh: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = bounded_text(resp, MAX_TOKEN_BYTES).await;
        if status.as_u16() == 401 || status.as_u16() == 403 || body.contains("invalid_grant") {
            return Err(ROTATED_HINT.into());
        }
        return Err(format!("token refresh failed: HTTP {status}"));
    }
    let tok = super::json_body(resp, MAX_TOKEN_BYTES, "token refresh").await?;
    let new_access = tok
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or("refresh response missing access_token")?
        .to_string();
    let new_refresh = tok
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or(&refresh)
        .to_string();
    let expires_in = json_f64(tok.get("expires_in")).unwrap_or(3600.0) as i64;

    access = new_access.clone();

    if doc.is_object() {
        doc["access_token"] = Value::from(new_access);
        doc["refresh_token"] = Value::from(new_refresh);
        doc["expires_in"] = Value::from(expires_in);
        // The CLI stores unix seconds (sometimes with a fractional part).
        doc["expires_at"] = Value::from((now_ms / 1000) + expires_in);
        let _ = std::fs::copy(path, path.with_extension("json.pane-bak"));
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&doc).unwrap_or(raw))
            .and_then(|_| std::fs::rename(&tmp, path))
            .map_err(|e| format!("write refreshed credentials: {e}"))?;
    }
    Ok(access)
}

fn can_write_creds(path: &Path) -> bool {
    is_regular_file(path)
}

fn is_regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .ok()
        .is_some_and(|m| m.is_file() && !m.file_type().is_symlink())
}

async fn bounded_text(resp: reqwest::Response, max_bytes: usize) -> String {
    match resp.bytes().await {
        Ok(b) if b.len() <= max_bytes => String::from_utf8_lossy(&b).into_owned(),
        _ => String::new(),
    }
}

fn parse_snapshot(doc: &Value) -> Result<Snapshot, String> {
    const SESSION_MS: i64 = 5 * HOUR_MS;
    const WEEK_MS: i64 = 7 * DAY_MS;
    let mut metrics = Vec::new();
    let mut weekly_limit = None;

    // Session = rolling 5-hour window (duration 300 TIME_UNIT_MINUTE).
    for entry in doc.get("limits").and_then(Value::as_array).unwrap_or(&vec![]) {
        if !is_session_window(entry) {
            continue;
        }
        let node = entry.get("detail").unwrap_or(entry);
        if let Some(m) = progress_from("Session", node, SESSION_MS) {
            metrics.push(m);
            break;
        }
    }

    if let Some(usage) = doc.get("usage") {
        weekly_limit = json_f64(usage.get("limit"));
        if let Some(m) = progress_from("Weekly", usage, WEEK_MS) {
            metrics.push(m);
        }
    }

    if metrics.is_empty() {
        return Err("usage response had no recognizable limit windows".into());
    }
    // The API-key path doesn't always carry user.membership.level — name
    // the product rather than leaving the plan blank.
    let plan = plan_from_doc(doc, weekly_limit).or_else(|| Some("Kimi Coding".into()));
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

fn progress_from(label: &str, node: &Value, period_ms: i64) -> Option<Metric> {
    let used = used_percent(node)?;
    let resets_at = parse_reset(node.get("resetTime").or_else(|| node.get("reset_time")));
    Some(Metric::progress(label, used, None).with_reset(resets_at, Some(period_ms)))
}

fn is_session_window(entry: &Value) -> bool {
    let duration = json_f64(entry.pointer("/window/duration"));
    let unit = entry
        .pointer("/window/timeUnit")
        .or_else(|| entry.pointer("/window/time_unit"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let unit = unit.to_ascii_uppercase();
    match (duration, unit.as_str()) {
        (Some(d), u) if (d - 300.0).abs() < 0.5 && u.contains("MINUTE") => true,
        (Some(d), u) if (d - 5.0).abs() < 0.5 && u.contains("HOUR") => true,
        _ => false,
    }
}

fn used_percent(node: &Value) -> Option<f64> {
    let limit = json_f64(node.get("limit"))?;
    if limit <= 0.0 {
        return None;
    }
    if let Some(used) = json_f64(node.get("used")) {
        return Some((used / limit * 100.0).clamp(0.0, 100.0));
    }
    let remaining = json_f64(node.get("remaining"))?;
    Some(((limit - remaining) / limit * 100.0).clamp(0.0, 100.0))
}

fn plan_from_doc(doc: &Value, weekly_limit: Option<f64>) -> Option<String> {
    if let Some(level) = doc
        .pointer("/user/membership/level")
        .and_then(Value::as_str)
        .and_then(map_membership_level)
    {
        return Some(level);
    }
    match weekly_limit.map(|n| n.round() as i64) {
        // Last resort when the API omits membership.level. These are the
        // old request-count caps; current responses usually send percent
        // (limit 100) plus a LEVEL_* code instead.
        Some(1024) => Some("Andante".into()),
        Some(2048) => Some("Moderato".into()),
        Some(7168) => Some("Allegretto".into()),
        _ => None,
    }
}

/// Display names match https://www.kimi.ai/membership/pricing
/// (Moderato / Allegretto / Allegro / Vivace). `LEVEL_*` codes shifted
/// when Allegro and Vivace launched (issue #156): INTERMEDIATE used to
/// print as Moderato. Older codes (Adagio / Andante) are mapped if the
/// API still sends them.
fn map_membership_level(raw: &str) -> Option<String> {
    let upper = raw.trim().to_ascii_uppercase();
    let key = upper.strip_prefix("LEVEL_").unwrap_or(&upper);
    Some(match key {
        "FREE" | "ADAGIO" | "BASIC" => "Adagio".into(),
        "ANDANTE" | "BEGINNER" | "INTRO" => "Andante".into(),
        "MODERATO" | "STANDARD" => "Moderato".into(),
        "INTERMEDIATE" | "ALLEGRETTO" | "ALLEGROTTO" | "PROFESSIONAL" | "PRO" => {
            "Allegretto".into()
        }
        "ADVANCED" | "ALLEGRO" => "Allegro".into(),
        "PREMIUM" | "VIVACE" => "Vivace".into(),
        other if other.is_empty() => return None,
        other => title_case(other),
    })
}

fn title_case(s: &str) -> String {
    let mut out = String::new();
    let mut cap = true;
    for c in s.chars() {
        if c == '_' || c == '-' {
            out.push(' ');
            cap = true;
            continue;
        }
        if cap {
            out.extend(c.to_uppercase());
            cap = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn json_f64(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn expires_at_ms(v: Option<&Value>) -> i64 {
    let Some(n) = json_f64(v) else { return 0 };
    if n.abs() >= 1e12 {
        n as i64
    } else {
        (n * 1000.0) as i64
    }
}

/// `resetTime` is ISO-8601; some responses carry extra fractional digits
/// that strict RFC3339 rejects, so trim the fraction to microseconds.
fn parse_reset(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::String(s) => parse_iso_ms(s),
        Value::Number(n) => {
            let n = n.as_f64()?;
            Some(if n.abs() < 1e10 { (n * 1000.0) as i64 } else { n as i64 })
        }
        _ => None,
    }
}

fn parse_iso_ms(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }
    let dot = s.find('.')?;
    let (head, rest) = s.split_at(dot);
    let rest = &rest[1..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let tz: String = rest.chars().skip_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let frac: String = digits.chars().chain(std::iter::repeat('0')).take(6).collect();
    DateTime::parse_from_rfc3339(&format!("{head}.{frac}{tz}"))
        .ok()
        .map(|dt| dt.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn labels(metrics: &[Metric]) -> Vec<&str> {
        metrics.iter().map(|m| m.label.as_str()).collect()
    }

    #[test]
    fn session_then_weekly_like_claude() {
        let doc = json!({
            "usage": {
                "limit": "2048",
                "used": "214",
                "remaining": "1834",
                "resetTime": "2026-01-09T15:23:13.716839Z"
            },
            "limits": [{
                "window": {"duration": 300, "timeUnit": "TIME_UNIT_MINUTE"},
                "detail": {
                    "limit": "200",
                    "used": "139",
                    "remaining": "61",
                    "resetTime": "2026-01-06T13:33:02.717479433Z"
                }
            }],
            "user": {"membership": {"level": "LEVEL_INTERMEDIATE"}}
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(snap.id, "kimi");
        assert_eq!(snap.name, "Kimi Code");
        assert_eq!(snap.plan.as_deref(), Some("Allegretto"));
        assert_eq!(labels(&snap.metrics), ["Session", "Weekly"]);
        let session = &snap.metrics[0];
        assert!((session.used_percent.unwrap() - 69.5).abs() < 0.01);
        assert_eq!(session.period_ms, Some(5 * HOUR_MS));
        assert!(session.resets_at.is_some());
        let weekly = &snap.metrics[1];
        assert!((weekly.used_percent.unwrap() - (214.0 / 2048.0 * 100.0)).abs() < 0.01);
        assert_eq!(weekly.period_ms, Some(7 * DAY_MS));
    }

    #[test]
    fn remaining_without_used_still_meters() {
        let doc = json!({
            "usage": {"limit": "100", "remaining": "74", "resetTime": "2026-02-11T17:32:50.757941Z"},
            "limits": [{
                "window": {"duration": 300, "timeUnit": "TIME_UNIT_MINUTE"},
                "detail": {"limit": "100", "remaining": "85", "resetTime": "2026-02-07T12:32:50.757941Z"}
            }]
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(labels(&snap.metrics), ["Session", "Weekly"]);
        assert!((snap.metrics[0].used_percent.unwrap() - 15.0).abs() < 0.01);
        assert!((snap.metrics[1].used_percent.unwrap() - 26.0).abs() < 0.01);
        // No membership.level in the payload → product-name fallback.
        assert_eq!(snap.plan.as_deref(), Some("Kimi Coding"));
    }

    #[test]
    fn weekly_limit_names_the_tier() {
        let andante = json!({"usage": {"limit": 1024, "used": 10, "remaining": 1014}});
        assert_eq!(parse_snapshot(&andante).unwrap().plan.as_deref(), Some("Andante"));
        let mid = json!({"usage": {"limit": 2048, "used": 0, "remaining": 2048}});
        assert_eq!(parse_snapshot(&mid).unwrap().plan.as_deref(), Some("Moderato"));
        let high = json!({"usage": {"limit": "7168", "used": "0", "remaining": "7168"}});
        assert_eq!(parse_snapshot(&high).unwrap().plan.as_deref(), Some("Allegretto"));
    }

    #[test]
    fn membership_level_matches_kimi_page_names() {
        let cases = [
            ("LEVEL_FREE", "Adagio"),
            ("LEVEL_BASIC", "Adagio"),
            ("ADAGIO", "Adagio"),
            ("LEVEL_ANDANTE", "Andante"),
            ("LEVEL_STANDARD", "Moderato"),
            ("LEVEL_MODERATO", "Moderato"),
            ("LEVEL_INTERMEDIATE", "Allegretto"),
            ("ALLEGRETTO", "Allegretto"),
            ("ALLEGROTTO", "Allegretto"),
            ("LEVEL_ADVANCED", "Allegro"),
            ("LEVEL_ALLEGRO", "Allegro"),
            ("LEVEL_PREMIUM", "Vivace"),
            ("VIVACE", "Vivace"),
            ("PRO", "Allegretto"),
        ];
        for (raw, name) in cases {
            assert_eq!(map_membership_level(raw).as_deref(), Some(name), "{raw}");
        }
    }

    #[test]
    fn ignores_non_session_windows() {
        let doc = json!({
            "usage": {"limit": "100", "used": "10", "remaining": "90"},
            "limits": [{
                "window": {"duration": 10080, "timeUnit": "TIME_UNIT_MINUTE"},
                "detail": {"limit": "100", "used": "10", "remaining": "90"}
            }]
        });
        let snap = parse_snapshot(&doc).expect("weekly-only is ok");
        assert_eq!(labels(&snap.metrics), ["Weekly"]);
    }

    #[test]
    fn empty_body_is_an_error() {
        assert!(parse_snapshot(&json!({})).is_err());
    }

    #[test]
    fn credential_picker_prefers_api_key_then_oauth() {
        let path = PathBuf::from(r"C:\Users\me\.kimi-code\credentials\kimi-code.json");
        // Both present → the explicitly configured API key is the active source.
        match pick_credential(Some(path.clone()), Some("sk-key".into())) {
            Some(Credential::ApiKey(key)) => assert_eq!(key, "sk-key"),
            _ => panic!("API key must win when both sources exist"),
        }
        // No CLI login → the API key remains the active source.
        match pick_credential(None, Some("sk-key".into())) {
            Some(Credential::ApiKey(k)) => assert_eq!(k, "sk-key"),
            _ => panic!("API key must be used when no OAuth credential exists"),
        }
        // No API key → a local OAuth credential is still supported.
        match pick_credential(Some(path.clone()), None) {
            Some(Credential::OAuth(p)) => assert_eq!(p, path),
            _ => panic!("OAuth must be used when no API key exists"),
        }
        assert!(pick_credential(None, None).is_none());
    }

    #[test]
    fn status_source_reports_only_the_active_kimi_credential() {
        assert_eq!(preferred_source(true, true), Some("api_key"));
        assert_eq!(preferred_source(true, false), Some("api_key"));
        assert_eq!(preferred_source(false, true), Some("oauth"));
        assert_eq!(preferred_source(false, false), None);
    }

    #[test]
    fn named_api_key_snapshot_uses_the_account_identity() {
        let snap = with_card_identity(
            Snapshot::ok(ID, NAME, None, vec![]),
            "kimi@account-two",
            "Kimi Code — Account Two",
        );
        assert_eq!(snap.id, "kimi@account-two");
        assert_eq!(snap.name, "Kimi Code — Account Two");
    }

    #[test]
    fn key_path_unauthorized_is_a_deterministic_rejection() {
        let msg = key_usages_error(UsagesError::Unauthorized);
        assert!(msg.contains("rejected"), "{msg}");
        // Network/HTTP failures pass through untouched.
        assert_eq!(key_usages_error(UsagesError::Other("boom".into())), "boom");
    }

    #[test]
    fn rotated_oauth_falls_back_to_saved_key() {
        // Dead CLI login + pasted key → the key takes over.
        assert_eq!(
            rotated_fallback(Some("sk-key".into())),
            Ok("sk-key".to_string())
        );
        // Dead CLI login and no key → the original sign-in guidance.
        assert_eq!(rotated_fallback(None), Err(ROTATED_HINT.to_string()));
        // Only the rotated hint classifies as terminal; network errors
        // stay transient and retry next cycle.
        assert!(matches!(
            classify_oauth_error(ROTATED_HINT.to_string()),
            OAuthFailure::Rotated
        ));
        assert!(matches!(
            classify_oauth_error("usage request: network down".into()),
            OAuthFailure::Other(_)
        ));
    }

    #[test]
    fn iso_with_extra_fractional_digits_parses() {
        let ms = parse_iso_ms("2026-01-06T13:33:02.717479433Z").expect("ns iso");
        let expected = DateTime::parse_from_rfc3339("2026-01-06T13:33:02.717479Z")
            .unwrap()
            .timestamp_millis();
        assert_eq!(ms, expected);
    }

    #[test]
    fn expires_at_seconds_and_millis() {
        assert_eq!(expires_at_ms(Some(&json!(1_700_000_000))), 1_700_000_000_000);
        assert_eq!(expires_at_ms(Some(&json!(1_700_000_000_000i64))), 1_700_000_000_000);
        assert_eq!(expires_at_ms(Some(&json!("1700000000"))), 1_700_000_000_000);
    }

    #[test]
    fn relative_kimi_code_home_is_ignored() {
        assert!(absolute_home("relative/path").is_none());
        assert!(absolute_home("").is_none());
        assert!(absolute_home("  ").is_none());
        #[cfg(windows)]
        {
            assert!(absolute_home(r"\Windows\System32").is_none());
            assert!(absolute_home(r"C:\Users\jazii\.kimi-code").is_some());
        }
        #[cfg(not(windows))]
        {
            assert!(absolute_home("/home/me/.kimi-code").is_some());
        }
    }

    #[test]
    fn cred_candidates_prefer_kimi_home_then_dot_dirs() {
        let kimi = PathBuf::from(r"D:\custom-kimi");
        let user = PathBuf::from(r"C:\Users\me");
        let paths = cred_candidates(Some(kimi), Some(user));
        assert_eq!(
            paths,
            [
                PathBuf::from(r"D:\custom-kimi\credentials\kimi-code.json"),
                PathBuf::from(r"C:\Users\me\.kimi-code\credentials\kimi-code.json"),
                PathBuf::from(r"C:\Users\me\.kimi\credentials\kimi-code.json"),
            ]
        );
    }

    #[test]
    fn oversized_credentials_are_rejected() {
        let dir = std::env::temp_dir().join(format!("pane-kimi-cred-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kimi-code.json");
        std::fs::write(&path, vec![b'x'; (MAX_CRED_BYTES as usize) + 1]).unwrap();
        let err = crate::providers::read_small_text(&path, MAX_CRED_BYTES, "credentials").unwrap_err();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
        assert!(err.contains("large"), "{err}");
    }
}
