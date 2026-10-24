//! Tencent Cloud OrcaTerm — package quota card.
//!
//! OrcaTerm has no credits; the desktop client keeps a plaintext `data.bin`
//! under `%APPDATA%\com.orcaterm-desktop.app` with an OAuth access token and
//! its expiry. Pane reads that file, never writes it. An expired token is
//! refreshed through OrcaTerm's own `OAuthRefreshToken` action and the new
//! short-lived token lives only in Pane's memory.
//!
//! The billing response is one `usageStats` array:
//! `hunyuan_ai` (monthly AI requests), `space` (GB), `secret` (count) and
//! `session_share` (`-1` = unlimited). Limits and `roleType` are shown as
//! reported — never mapped to an assumed Pro plan. A missing/expired session
//! is `no_credentials`, not a card full of zeros.

use super::{json_body, read_small_text, send_with_direct_fallback, Metric, Snapshot};
use crate::usage_units::UsageUnit;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const FAMILY_ID: &str = "orcaterm";
const DISPLAY_NAME: &str = "腾讯云 OrcaTerm";
const API: &str = "https://api.orcaterm.com/cgi/api";
const PRODUCT: &str = "orcaterm";
const MAX_BODY_BYTES: usize = 256 * 1024;
const MAX_DATA_BYTES: u64 = 256 * 1024;
const REFRESH_MARGIN_MS: i64 = 60_000;

const NO_SESSION_HINT: &str = "Open the OrcaTerm desktop app and sign in once, then refresh.";
const EXPIRED_HINT: &str =
    "The OrcaTerm session expired and could not be refreshed — open OrcaTerm and sign in again.";
const BAD_DATA_HINT: &str =
    "OrcaTerm's local session file is unreadable — open OrcaTerm and sign in again.";

#[derive(Clone, Default, PartialEq)]
struct LocalSession {
    token: String,
    expires_at_ms: Option<i64>,
    nickname: Option<String>,
}

#[derive(Clone, Default, PartialEq)]
struct CachedToken {
    source_token: String,
    token: String,
    expires_at_ms: i64,
}

static CACHED: Mutex<Option<CachedToken>> = Mutex::new(None);
static FETCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Debug, PartialEq)]
struct UsageStat {
    feature: String,
    used: f64,
    limit: f64,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Billing {
    role: String,
    stats: Vec<UsageStat>,
    deadline_ms: Option<i64>,
}

pub fn family_id() -> &'static str {
    FAMILY_ID
}

pub fn local_credential_hint() -> Option<String> {
    read_small_text(&data_path(), MAX_DATA_BYTES, "OrcaTerm data.bin")
        .ok()
        .and_then(|raw| parse_local_session(&raw))
        .map(|_| "OrcaTerm app sign-in".to_string())
}

pub async fn snapshot() -> Snapshot {
    let _fetch = FETCH_LOCK.lock().await;
    let path = data_path();
    let raw = match read_small_text(&path, MAX_DATA_BYTES, "OrcaTerm data.bin") {
        Ok(raw) => raw,
        Err(_) => return Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, NO_SESSION_HINT),
    };
    let Some(session) = parse_local_session(&raw) else {
        return Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, BAD_DATA_HINT);
    };
    let now = now_ms();
    let mut token = usable_token(&session, now);
    if token.is_none() {
        match refresh_token(&session.token).await {
            Ok(fresh) => {
                *CACHED.lock().unwrap_or_else(|e| e.into_inner()) = Some(fresh.clone());
                token = Some(fresh.token);
            }
            Err(RefreshErr::Auth) => {
                return Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, EXPIRED_HINT)
            }
            Err(RefreshErr::Transient(message)) => {
                return Snapshot::error(FAMILY_ID, DISPLAY_NAME, message)
            }
        }
    }
    let Some(token) = token else {
        return Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, NO_SESSION_HINT);
    };
    match fetch_billing(&token).await {
        Ok(BillingResult::Ok(billing)) => {
            snapshot_from_billing(&billing, session.nickname.as_deref())
        }
        Ok(BillingResult::Auth) => {
            *CACHED.lock().unwrap_or_else(|e| e.into_inner()) = None;
            Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, EXPIRED_HINT)
        }
        Ok(BillingResult::Error(message)) => Snapshot::error(FAMILY_ID, DISPLAY_NAME, message),
        Err(message) => Snapshot::error(FAMILY_ID, DISPLAY_NAME, message),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn data_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_default()
        .join("com.orcaterm-desktop.app")
        .join("data.bin")
}

fn parse_local_session(raw: &str) -> Option<LocalSession> {
    let doc: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok()?;
    let token = doc.get("oauth_access_token")?.as_str()?.trim().to_string();
    if token.is_empty() {
        return None;
    }
    let identity = doc.get("auth_identity").and_then(|value| match value {
        Value::String(text) => serde_json::from_str::<Value>(text).ok(),
        Value::Object(_) => Some(value.clone()),
        _ => None,
    });
    let string_at = |key: &str| {
        identity
            .as_ref()
            .and_then(|value| value.get(key))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string)
    };
    Some(LocalSession {
        token,
        expires_at_ms: doc.get("oauth_expires_at").and_then(Value::as_i64),
        nickname: string_at("nickName"),
    })
}

fn usable_token(session: &LocalSession, now_ms: i64) -> Option<String> {
    let mut cached = CACHED.lock().unwrap_or_else(|e| e.into_inner());
    select_token(session, &mut cached, now_ms)
}

fn select_token(
    session: &LocalSession,
    cached: &mut Option<CachedToken>,
    now_ms: i64,
) -> Option<String> {
    if cached
        .as_ref()
        .is_some_and(|cached| cached.source_token != session.token)
    {
        *cached = None;
    }
    if let Some(cached) = cached.as_ref() {
        if cached.expires_at_ms - REFRESH_MARGIN_MS > now_ms {
            return Some(cached.token.clone());
        }
    }
    if session
        .expires_at_ms
        .is_some_and(|expires| expires - REFRESH_MARGIN_MS > now_ms)
        && !session.token.is_empty()
    {
        return Some(session.token.clone());
    }
    None
}

enum RefreshErr {
    Auth,
    Transient(String),
}

async fn refresh_token(token: &str) -> Result<CachedToken, RefreshErr> {
    let body = json!({"action": "OAuthRefreshToken", "data": {"accessToken": token}});
    let response = send_with_direct_fallback(|client| {
        client
            .post(format!("{API}?action=OAuthRefreshToken"))
            .header("Authorization", format!("Bearer {token}"))
            .header("X-Product", PRODUCT)
            .header("Content-Type", "application/json")
            .json(&body)
    })
    .await
    .map_err(RefreshErr::Transient)?;
    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Err(RefreshErr::Auth);
    }
    if !response.status().is_success() {
        return Err(RefreshErr::Transient(format!(
            "OrcaTerm token refresh returned HTTP {}",
            response.status().as_u16()
        )));
    }
    let doc = json_body(response, MAX_BODY_BYTES, "orcaterm refresh")
        .await
        .map_err(RefreshErr::Transient)?;
    parse_refresh(&doc, token, now_ms())
}

fn parse_refresh(doc: &Value, source_token: &str, now_ms: i64) -> Result<CachedToken, RefreshErr> {
    let response = doc.pointer("/data/Response").ok_or_else(|| {
        RefreshErr::Transient("OrcaTerm token refresh returned no response body".into())
    })?;
    if response.get("Error").is_some() {
        return Err(RefreshErr::Transient(
            "OrcaTerm token refresh was rejected".into(),
        ));
    }
    if let Some(code) = response.get("Code").and_then(Value::as_i64) {
        if code != 0 {
            return Err(RefreshErr::Transient(
                response
                    .get("Message")
                    .and_then(Value::as_str)
                    .unwrap_or("OrcaTerm token refresh failed")
                    .to_string(),
            ));
        }
    }
    let fresh = response
        .get("AccessToken")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| RefreshErr::Transient("OrcaTerm token refresh returned no token".into()))?;
    let expires_in = response
        .get("ExpiresIn")
        .and_then(Value::as_i64)
        .filter(|seconds| *seconds > 0)
        .unwrap_or(7200);
    Ok(CachedToken {
        source_token: source_token.to_string(),
        token: fresh.to_string(),
        expires_at_ms: now_ms.saturating_add(expires_in.saturating_mul(1000)),
    })
}

enum BillingResult {
    Ok(Billing),
    Auth,
    Error(String),
}

async fn fetch_billing(token: &str) -> Result<BillingResult, String> {
    let body = json!({"action": "DescribeUserBillingStatus", "data": {}});
    let response = send_with_direct_fallback(|client| {
        client
            .post(format!("{API}?action=DescribeUserBillingStatus"))
            .header("Authorization", format!("Bearer {token}"))
            .header("X-Product", PRODUCT)
            .header("Content-Type", "application/json")
            .json(&body)
    })
    .await?;
    let status = response.status().as_u16();
    if status == 401 || status == 403 {
        return Ok(BillingResult::Auth);
    }
    if !(200..300).contains(&status) {
        return Err(format!("OrcaTerm billing returned HTTP {status}"));
    }
    let doc = json_body(response, MAX_BODY_BYTES, "orcaterm billing").await?;
    let response = match doc.pointer("/data/Response") {
        Some(value) => value,
        None => {
            return Ok(BillingResult::Error(
                "OrcaTerm billing returned no response body".into(),
            ))
        }
    };
    if let Some(error) = response.get("Error") {
        let message = error
            .get("Message")
            .or_else(|| error.get("Code"))
            .and_then(Value::as_str)
            .unwrap_or("OrcaTerm billing error");
        return Ok(BillingResult::Error(message.to_string()));
    }
    if let Some(code) = response.get("Code").and_then(Value::as_i64) {
        if code != 0 {
            return Ok(BillingResult::Error(
                response
                    .get("Message")
                    .and_then(Value::as_str)
                    .unwrap_or("OrcaTerm billing error")
                    .to_string(),
            ));
        }
    }
    match parse_billing(response) {
        Some(billing) => Ok(BillingResult::Ok(billing)),
        None => Ok(BillingResult::Error(
            "OrcaTerm billing response has no usable quota fields".into(),
        )),
    }
}

fn parse_billing(doc: &Value) -> Option<Billing> {
    let stats = doc
        .get("usageStats")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(parse_usage_stat)
        .collect();
    Some(Billing {
        role: doc
            .get("roleType")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or("unknown")
            .to_string(),
        stats,
        deadline_ms: parse_deadline(doc.get("curDeadline"))
            .or_else(|| parse_deadline(doc.get("dayofDeadline"))),
    })
}

fn parse_usage_stat(row: &Value) -> Option<UsageStat> {
    let limit = row.get("quotaLimit").and_then(Value::as_f64)?;
    Some(UsageStat {
        feature: row.get("feature")?.as_str()?.trim().to_string(),
        used: row.get("quotaUsed").and_then(Value::as_f64)?.max(0.0),
        limit,
        start_ms: parse_date_ms(row.get("quotaStartDate")),
        end_ms: parse_date_ms(row.get("quotaEndDate")),
    })
}

fn parse_deadline(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    if let Some(ms) = value.as_i64() {
        return (ms > 0).then(|| {
            if ms < 1_000_000_000_000 {
                ms * 1000
            } else {
                ms
            }
        });
    }
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    text.parse::<i64>()
        .ok()
        .map(|ms| {
            if ms < 1_000_000_000_000 {
                ms * 1000
            } else {
                ms
            }
        })
        .or_else(|| {
            chrono::DateTime::parse_from_rfc3339(text)
                .ok()
                .map(|at| at.timestamp_millis())
        })
}

fn parse_date_ms(value: Option<&Value>) -> Option<i64> {
    let text = value?.as_str()?.trim();
    if text.is_empty() || text.starts_with("1970-01-01") {
        return None;
    }
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.timestamp_millis())
}

fn snapshot_from_billing(billing: &Billing, nickname: Option<&str>) -> Snapshot {
    let plan = Some(plan_label(&billing.role));
    let mut metrics = Vec::new();
    let mut name = DISPLAY_NAME.to_string();
    if let Some(nickname) = nickname.filter(|text| !text.trim().is_empty()) {
        name = format!("{DISPLAY_NAME} — {}", nickname.trim());
    }
    if let Some(deadline) = billing.deadline_ms {
        metrics.push(Metric::text("Expires", fmt_date_ms(deadline)));
    }
    for stat in &billing.stats {
        match stat.feature.as_str() {
            "hunyuan_ai" => metrics.push(quota_row(
                "AI requests",
                stat,
                "requests",
                stat.end_ms.map(|end| {
                    (
                        end,
                        stat.start_ms
                            .filter(|start| end > *start)
                            .map(|start| end - start),
                    )
                }),
            )),
            "space" => metrics.push(quota_row("Storage", stat, "GB", None)),
            "secret" => metrics.push(quota_row("Credentials", stat, "", None)),
            "session_share" => metrics.push(if stat.limit < 0.0 {
                Metric::text("Shared sessions", "Unlimited".into())
            } else {
                quota_row("Shared sessions", stat, "", None)
            }),
            other => metrics.push(quota_row(other, stat, "", None)),
        }
    }
    if metrics.is_empty() {
        return Snapshot::error(
            FAMILY_ID,
            name.as_str(),
            "OrcaTerm billing returned no quota rows".into(),
        );
    }
    Snapshot::ok(FAMILY_ID, &name, plan, metrics)
}

fn quota_row(
    label: &str,
    stat: &UsageStat,
    unit: &str,
    reset: Option<(i64, Option<i64>)>,
) -> Metric {
    if stat.limit < 0.0 {
        return Metric::text(label, "Unlimited".into());
    }
    let detail = match unit {
        "" => format!("{} / {}", fmt_number(stat.used), fmt_number(stat.limit)),
        "GB" => format!("{} / {} GB", fmt_number(stat.used), fmt_number(stat.limit)),
        _ => format!(
            "{} / {} {}",
            fmt_number(stat.used),
            fmt_number(stat.limit),
            unit
        ),
    };
    let percent = if stat.limit > 0.0 {
        stat.used / stat.limit * 100.0
    } else {
        0.0
    };
    let mut row = Metric::progress(label, percent, Some(detail));
    if matches!(stat.feature.as_str(), "hunyuan_ai") && stat.limit > 0.0 {
        row = row.with_usage(UsageUnit::Requests, stat.used, Some(stat.limit));
    }
    if let Some((end, period)) = reset {
        row = row.with_reset(Some(end), period);
    }
    row
}

fn plan_label(role: &str) -> String {
    match role {
        "basic" => "Basic".into(),
        "professional" => "Professional".into(),
        other => other.to_string(),
    }
}

fn fmt_number(value: f64) -> String {
    if !value.is_finite() {
        return "—".into();
    }
    if (value - value.round()).abs() < 0.000_001 {
        let rounded = value.round() as i64;
        let text = rounded.abs().to_string();
        let mut grouped = String::new();
        for (index, ch) in text.chars().rev().enumerate() {
            if index > 0 && index % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(ch);
        }
        let grouped: String = grouped.chars().rev().collect();
        return if rounded < 0 {
            format!("-{grouped}")
        } else {
            grouped
        };
    }
    format!("{value:.2}")
}

fn fmt_date_ms(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| {
            at.with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = r#"{
      "auth_identity": "{\"userId\":\"test-user\",\"nickName\":\"Tester\"}",
      "oauth_expires_at": 1792836158886,
      "oauth_access_token": "ot_session_test",
      "userId": "test-user"
    }"#;

    const BILLING: &str = r#"{
      "userId": "test-user",
      "roleType": "basic",
      "usageStats": [
        {"feature":"hunyuan_ai","quotaLimit":1000,"quotaUsed":120,
         "quotaStartDate":"2026-10-23T16:00:00.000Z","quotaEndDate":"2026-11-23T16:00:00.000Z"},
        {"feature":"space","quotaLimit":10,"quotaUsed":2.5,
         "quotaStartDate":"1970-01-01T00:00:00.000Z","quotaEndDate":"1970-01-01T00:00:00.000Z"},
        {"feature":"secret","quotaLimit":5,"quotaUsed":0},
        {"feature":"session_share","quotaLimit":-1,"quotaUsed":0}
      ],
      "curDeadline": null,
      "dayofDeadline": null
    }"#;

    #[test]
    fn parses_the_plaintext_local_session() {
        let session = parse_local_session(DATA).expect("session");
        assert_eq!(session.token, "ot_session_test");
        assert_eq!(session.expires_at_ms, Some(1792836158886));
        assert_eq!(session.nickname.as_deref(), Some("Tester"));
        assert!(parse_local_session("{}").is_none());
        assert!(parse_local_session("not json").is_none());
    }

    #[test]
    fn parses_usage_stats_and_unlimited() {
        let value: Value = serde_json::from_str(BILLING).unwrap();
        let billing = parse_billing(&value).expect("billing");
        assert_eq!(billing.role, "basic");
        assert_eq!(billing.stats.len(), 4);
        let ai = &billing.stats[0];
        assert_eq!(ai.feature, "hunyuan_ai");
        assert_eq!(ai.used, 120.0);
        assert_eq!(ai.limit, 1000.0);
        assert!(ai.end_ms.is_some());
        let space = &billing.stats[1];
        assert!(space.start_ms.is_none(), "1970 sentinel is not a window");
        assert!(space.end_ms.is_none());
        let share = &billing.stats[3];
        assert_eq!(share.limit, -1.0);
    }

    #[test]
    fn card_rows_show_reported_limits_without_guessing_pro() {
        let value: Value = serde_json::from_str(BILLING).unwrap();
        let billing = parse_billing(&value).unwrap();
        let snap = snapshot_from_billing(&billing, Some("Tester"));
        assert_eq!(snap.plan.as_deref(), Some("Basic"));
        assert_eq!(snap.status, "ok");
        let rows: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(
            rows,
            vec!["AI requests", "Storage", "Credentials", "Shared sessions"]
        );
        let ai = &snap.metrics[0];
        assert_eq!(ai.used_percent, Some(12.0));
        assert_eq!(ai.detail.as_deref(), Some("120 / 1,000 requests"));
        assert!(ai.resets_at.is_some());
        assert_eq!(ai.usage.as_ref().map(|u| u.unit), Some(UsageUnit::Requests));
        let storage = &snap.metrics[1];
        assert_eq!(storage.detail.as_deref(), Some("2.50 / 10 GB"));
        assert!(storage.resets_at.is_none());
        let share = &snap.metrics[3];
        assert_eq!(share.value.as_deref(), Some("Unlimited"));
    }

    #[test]
    fn zero_limit_is_zero_not_a_division_error() {
        let stat = UsageStat {
            feature: "hunyuan_ai".into(),
            used: 0.0,
            limit: 0.0,
            start_ms: None,
            end_ms: None,
        };
        let row = quota_row("AI requests", &stat, "requests", None);
        assert_eq!(row.used_percent, Some(0.0));
        assert_eq!(row.detail.as_deref(), Some("0 / 0 requests"));
    }

    #[test]
    fn unknown_role_is_shown_verbatim() {
        assert_eq!(plan_label("professional"), "Professional");
        assert_eq!(plan_label("team-edition"), "team-edition");
        assert_eq!(fmt_number(12345.0), "12,345");
        assert_eq!(fmt_number(2.5), "2.50");
    }

    #[test]
    fn refresh_response_needs_a_token() {
        let value: Value = serde_json::from_str(
            r#"{"data":{"Response":{"Code":0,"Message":"Success","AccessToken":"ot_session_new","ExpiresIn":7200}}}"#,
        )
        .unwrap();
        let token = parse_refresh(&value, "disk-token", 1000)
            .ok()
            .expect("valid refresh");
        assert_eq!(token.source_token, "disk-token");
        assert_eq!(token.token, "ot_session_new");
        assert_eq!(token.expires_at_ms, 7_201_000);
        assert!(
            parse_refresh(&json!({"data":{"Response":{"Code":0}}}), "disk-token", 1000).is_err()
        );
        assert!(parse_refresh(&json!({}), "disk-token", 1000).is_err());
        assert!(parse_refresh(
            &json!({"data":{"Response":{"Code":7,"AccessToken":"invalid"}}}),
            "disk-token",
            1000
        )
        .is_err());
        assert!(parse_refresh(
            &json!({"data":{"Response":{"Error":{},"AccessToken":"invalid"}}}),
            "disk-token",
            1000
        )
        .is_err());
    }

    #[test]
    fn switching_disk_token_discards_the_previous_account_cache() {
        let session = LocalSession {
            token: "account-b".into(),
            expires_at_ms: Some(200_000),
            nickname: None,
        };
        let mut cache = Some(CachedToken {
            source_token: "account-a".into(),
            token: "fresh-a".into(),
            expires_at_ms: 300_000,
        });
        assert_eq!(
            select_token(&session, &mut cache, 100_000).as_deref(),
            Some("account-b")
        );
        assert!(cache.is_none());
        let expired = LocalSession {
            expires_at_ms: Some(10_000),
            ..session
        };
        assert!(select_token(&expired, &mut cache, 100_000).is_none());
    }

    #[test]
    fn refresh_is_only_needed_within_sixty_seconds_of_expiry() {
        let mut session = LocalSession {
            token: "disk-token".into(),
            expires_at_ms: Some(200_000),
            nickname: None,
        };
        let mut cache = None;
        assert_eq!(
            select_token(&session, &mut cache, 139_999).as_deref(),
            Some("disk-token")
        );
        assert!(select_token(&session, &mut cache, 140_000).is_none());
        cache = Some(CachedToken {
            source_token: session.token.clone(),
            token: "fresh-token".into(),
            expires_at_ms: 400_000,
        });
        session.expires_at_ms = Some(10_000);
        assert_eq!(
            select_token(&session, &mut cache, 200_000).as_deref(),
            Some("fresh-token")
        );
        assert!(select_token(&session, &mut cache, 340_000).is_none());
    }

    #[test]
    fn reset_end_is_kept_without_a_start_date() {
        let doc = json!({"roleType":"basic","usageStats":[
            {"feature":"hunyuan_ai","quotaLimit":1000,"quotaUsed":0,"quotaEndDate":"2026-11-23T16:00:00Z"}
        ]});
        let snap = snapshot_from_billing(&parse_billing(&doc).unwrap(), None);
        assert!(snap.metrics[0].resets_at.is_some());
        assert!(snap.metrics[0].period_ms.is_none());
    }

    #[test]
    fn missing_usage_is_not_presented_as_zero_consumption() {
        assert!(parse_usage_stat(&json!({"feature":"hunyuan_ai","quotaLimit":1000})).is_none());
        assert!(
            parse_usage_stat(&json!({"feature":"space","quotaLimit":10,"quotaUsed":"bad"}))
                .is_none()
        );
        let billing = parse_billing(&json!({"usageStats":[]})).unwrap();
        assert_eq!(snapshot_from_billing(&billing, None).status, "error");
    }
}
