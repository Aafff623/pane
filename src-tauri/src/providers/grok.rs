use super::{http, Metric, Snapshot};
use chrono::{DateTime, Duration, Utc};
use prost::Message;
use serde_json::Value;
use std::path::PathBuf;

const ID: &str = "grok";
const NAME: &str = "Grok";
const REMAINING_RESETS_URL: &str =
    "https://grok.com/prod_mc_billing.ConsumerUiSvc/GetRemainingResets";
const EMPTY_GRPC_WEB_MESSAGE: [u8; 5] = [0, 0, 0, 0, 0];
const MAX_RESET_CREDITS_BODY: usize = 64 * 1024;
const PROTO_TIMESTAMP_MIN_SECONDS: i64 = -62_135_596_800;
const PROTO_TIMESTAMP_MAX_SECONDS: i64 = 253_402_300_799;

fn auth_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".grok").join("auth.json")
}

/// Pure local probe for the Customize gear panel (no network): the Grok
/// CLI's auth.json exists on this machine.
pub fn local_credential_hint() -> Option<String> {
    auth_path()
        .exists()
        .then(|| "Grok CLI sign-in (~/.grok/auth.json)".to_string())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Reads the Grok CLI's auth.json and returns a usable access token,
/// refreshing (and writing the rotated pair back to the CLI's own file)
/// when the stored one has expired.
async fn cli_access_token(path: &std::path::Path) -> Result<String, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read auth.json: {e}"))?;
    let mut doc: Value = serde_json::from_str(&raw).map_err(|e| format!("parse auth.json: {e}"))?;

    // auth.json maps "<issuer>::<account-uuid>" to the account entry.
    let entry_key = doc
        .as_object()
        .and_then(|m| m.keys().next().cloned())
        .ok_or("auth.json is empty")?;
    let entry = doc.get(&entry_key).cloned().unwrap_or(Value::Null);

    let mut token = entry
        .get("key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let refresh_token = entry
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let issuer = entry
        .get("oidc_issuer")
        .and_then(Value::as_str)
        .unwrap_or("https://auth.x.ai")
        .trim_end_matches('/')
        .to_string();
    let client_id = entry
        .get("oidc_client_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let expired = entry
        .get("expires_at")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc) <= Utc::now() + Duration::seconds(60))
        .unwrap_or(false);

    if token.is_empty() || expired {
        if refresh_token.is_empty() || client_id.is_empty() {
            return Err("Grok token expired — run the Grok CLI once to sign in again".into());
        }
        let form = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", client_id.as_str()),
        ];
        let mut resp = http()
            .post(format!("{issuer}/oauth2/token"))
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("token refresh: {e}"))?;
        if resp.status().as_u16() == 404 {
            resp = http()
                .post(format!("{issuer}/oauth/token"))
                .form(&form)
                .send()
                .await
                .map_err(|e| format!("token refresh: {e}"))?;
        }
        if !resp.status().is_success() {
            return Err(format!(
                "token refresh failed (HTTP {}) — run the Grok CLI once to sign in again",
                resp.status()
            ));
        }
        let tok: Value = resp.json().await.map_err(|e| format!("token refresh parse: {e}"))?;
        let new_access = tok
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or("refresh response missing access_token")?
            .to_string();
        let expires_in = tok.get("expires_in").and_then(Value::as_i64).unwrap_or(3600);

        token = new_access.clone();

        // Refresh tokens rotate — write the new pair back so the Grok CLI
        // itself stays signed in.
        if let Some(e) = doc.get_mut(&entry_key).filter(|v| v.is_object()) {
            e["key"] = Value::from(new_access);
            if let Some(r) = tok.get("refresh_token").and_then(Value::as_str) {
                e["refresh_token"] = Value::from(r);
            }
            e["expires_at"] = Value::from((Utc::now() + Duration::seconds(expires_in)).to_rfc3339());
            // Keep a copy of the CLI's own file before touching it, so a bad
            // write can never cost the user their login.
            let _ = std::fs::copy(path, path.with_extension("json.pane-bak"));
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_string_pretty(&doc).unwrap_or(raw))
                .and_then(|_| std::fs::rename(&tmp, path))
                .map_err(|e| format!("write refreshed auth.json: {e}"))?;
        }
    }

    Ok(token)
}

async fn fetch() -> Result<Snapshot, String> {
    let path = auth_path();
    // Credential order: the Grok CLI's own auth.json first, then Pane's
    // own OAuth login from the gear panel (%APPDATA%\Pane\oauth\grok.json
    // — refreshed and written back there, the CLI's file untouched).
    let token = if path.exists() {
        cli_access_token(&path).await?
    } else {
        match crate::oauth::valid_tokens(ID).await {
            Ok(Some(t)) => t.access_token,
            Ok(None) => {
                return Ok(Snapshot::no_credentials(
                    ID,
                    NAME,
                    "Grok CLI sign-in not found (~\\.grok\\auth.json).",
                ));
            }
            Err(e) => return Err(e),
        }
    };

    let billing_req = http()
        .get("https://cli-chat-proxy.grok.com/v1/billing?format=credits")
        .bearer_auth(&token)
        .send();

    let settings_req = http()
        .get("https://cli-chat-proxy.grok.com/v1/settings")
        .bearer_auth(&token)
        .send();
    let user_req = http()
        .get("https://cli-chat-proxy.grok.com/v1/user?include=subscription")
        .bearer_auth(&token)
        .send();
    let resets_req = remaining_reset_metrics(&token);
    let (billing_resp, settings_resp, user_resp, resets_result) =
        tokio::join!(billing_req, settings_req, user_req, resets_req);

    let billing_resp = billing_resp.map_err(|e| format!("billing request: {e}"))?;
    if billing_resp.status().as_u16() == 401 || billing_resp.status().as_u16() == 403 {
        return Err("Grok session expired — run the Grok CLI once to refresh it".into());
    }
    if !billing_resp.status().is_success() {
        return Err(format!("billing endpoint: HTTP {}", billing_resp.status()));
    }
    let billing: Value = billing_resp.json().await.map_err(|e| format!("billing parse: {e}"))?;

    let mut metrics = Vec::new();
    if let Some(used) = credit_usage_percent(&billing) {
        let (resets_at, period_ms) = current_period_window(&billing);
        metrics.push(Metric::progress("Usage", used, None).with_reset(resets_at, period_ms));
    }
    collect_billing_metrics(&billing, "", &mut metrics);
    if metrics.is_empty() {
        // Log field names (never values) so unknown shapes are debuggable.
        if let Some(map) = billing.as_object() {
            eprintln!(
                "[pane] grok billing keys: {:?}",
                map.keys().collect::<Vec<_>>()
            );
        }
        return Err("unexpected billing response shape".into());
    }
    metrics.truncate(4);

    // Pay-as-you-go cap badge. proto3-as-JSON omits zero fields, so a
    // missing onDemandCap simply means overage is disabled.
    let cap = billing
        .pointer("/config/onDemandCap/val")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    metrics.push(Metric::text(
        "Extra usage",
        if cap > 0.0 { format!("{cap:.0} cap") } else { "Disabled".to_string() },
    ));
    match resets_result {
        Ok(credits) => metrics.extend(credits),
        Err(err) => eprintln!("[pane] grok reset credits: {}", err.category()),
    }

    let settings = match settings_resp {
        Ok(resp) if resp.status().is_success() => resp.json::<Value>().await.ok(),
        _ => None,
    };
    let user = match user_resp {
        Ok(resp) if resp.status().is_success() => resp.json::<Value>().await.ok(),
        _ => None,
    };

    let plan = resolve_subscription_plan(settings.as_ref(), user.as_ref());

    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

fn resolve_subscription_plan(settings: Option<&Value>, user: Option<&Value>) -> Option<String> {
    settings
        .and_then(|doc| {
            doc.get("subscription_tier_display")
                .or_else(|| doc.get("subscriptionTierDisplay"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .or_else(|| {
            user.and_then(|doc| {
                doc.get("subscriptionTier")
                    .or_else(|| doc.get("subscription_tier"))
                    .and_then(Value::as_str)
                    .and_then(display_from_subscription_code)
            })
        })
}

fn display_from_subscription_code(code: &str) -> Option<String> {
    let code = code.trim();
    if code.is_empty() {
        return None;
    }

    let normalized = code.to_ascii_lowercase().replace(['_', ' ', '-'], "");
    match normalized.as_str() {
        "supergrokpro" | "supergrokheavy" | "heavy" => Some("SuperGrok Heavy".into()),
        "supergrok" | "supergroklite" | "grokpro" => Some("SuperGrok".into()),
        "xpremiumplus" | "premiumplus" => Some("X Premium+".into()),
        "xpremium" | "premium" => Some("X Premium".into()),
        "free" | "basic" | "none" | "null" | "anonymous" => None,
        _ => Some(code.into()),
    }
}

/// Usage percent for the current billing window. proto3-as-JSON omits
/// zero-valued fields, so a fresh window reports 0% used by *omitting*
/// creditUsagePercent entirely — with a currentPeriod present, that absence
/// is an explicit "nothing used yet", not an unknown response shape.
fn credit_usage_percent(billing: &Value) -> Option<f64> {
    billing
        .pointer("/config/creditUsagePercent")
        .and_then(Value::as_f64)
        .or_else(|| billing.pointer("/config/currentPeriod").map(|_| 0.0))
}

/// The aggregate quota resets at the end of the provider-reported current
/// period. A missing/invalid start only disables pacing; it does not hide the
/// explicit reset time.
fn current_period_window(billing: &Value) -> (Option<i64>, Option<i64>) {
    let period = billing.pointer("/config/currentPeriod");
    let parse = |field: &str| {
        period
            .and_then(|p| p.get(field))
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|t| t.timestamp_millis())
    };
    let start = parse("start");
    let end = parse("end");
    let period_ms = match (start, end) {
        (Some(start), Some(end)) if end > start => Some(end - start),
        _ => None,
    };
    (end, period_ms)
}

/// Undocumented endpoint — collect anything that looks like a usage percent
/// or a credit balance.
fn collect_billing_metrics(node: &Value, parent_key: &str, metrics: &mut Vec<Metric>) {
    match node {
        Value::Array(items) => {
            for item in items {
                collect_billing_metrics(item, parent_key, metrics);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                let lower = key.to_lowercase();
                if let Some(n) = value.as_f64() {
                    if lower.contains("percent") {
                        let label = if lower.contains("week") || parent_key.to_lowercase().contains("week") {
                            "Weekly"
                        } else {
                            "Usage"
                        };
                        // The undocumented payload repeats the same percent
                        // under several keys/nestings — one row per label.
                        if !metrics.iter().any(|m| m.label == label) {
                            metrics.push(Metric::progress(label, n, None));
                        }
                    } else if lower.contains("credit") || lower.contains("balance") {
                        metrics.push(Metric::text(key, format!("{n:.2}")));
                    }
                } else {
                    collect_billing_metrics(value, key, metrics);
                }
            }
        }
        _ => {}
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResetCreditError {
    Transport,
    HttpStatus,
    Oversized,
    Framing,
    TrailerStatus,
    Decode,
}

impl ResetCreditError {
    fn category(self) -> &'static str {
        match self {
            Self::Transport => "transport",
            Self::HttpStatus => "HTTP status",
            Self::Oversized => "oversized body",
            Self::Framing => "framing",
            Self::TrailerStatus => "trailer status",
            Self::Decode => "protobuf decode",
        }
    }
}

#[derive(Clone, PartialEq, Message)]
struct RemainingResets {
    #[prost(message, repeated, tag = "10")]
    tokens: Vec<ResetToken>,
}

#[derive(Clone, PartialEq, Message)]
struct ResetToken {
    #[prost(message, optional, tag = "30")]
    validity_end: Option<ProtoTimestamp>,
}

#[derive(Clone, PartialEq, Message)]
struct ProtoTimestamp {
    #[prost(int64, tag = "1")]
    seconds: i64,
    #[prost(int32, tag = "2")]
    nanos: i32,
}

async fn remaining_reset_metrics(token: &str) -> Result<Vec<Metric>, ResetCreditError> {
    let resp = http()
        .post(REMAINING_RESETS_URL)
        .bearer_auth(token)
        .header("Content-Type", "application/grpc-web+proto")
        .header("X-Grpc-Web", "1")
        .header("Origin", "https://grok.com")
        .body(EMPTY_GRPC_WEB_MESSAGE.to_vec())
        .send()
        .await
        .map_err(|_| ResetCreditError::Transport)?;
    if !resp.status().is_success() {
        return Err(ResetCreditError::HttpStatus);
    }
    let body = limited_response_body(resp).await?;
    reset_credit_metrics(&body)
}

/// Cap the read itself — Content-Length alone does not bound a chunked
/// or lying response (same approach as the pricing catalog fetch).
async fn limited_response_body(mut resp: reqwest::Response) -> Result<Vec<u8>, ResetCreditError> {
    if let Some(content_length) = resp.content_length() {
        ensure_reset_credit_body_size(content_length)?;
    }
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|_| ResetCreditError::Transport)? {
        accept_reset_credit_chunk(&mut body, &chunk)?;
    }
    Ok(body)
}

fn accept_reset_credit_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), ResetCreditError> {
    if body.len().saturating_add(chunk.len()) > MAX_RESET_CREDITS_BODY {
        return Err(ResetCreditError::Oversized);
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn ensure_reset_credit_body_size(size: u64) -> Result<(), ResetCreditError> {
    if size > MAX_RESET_CREDITS_BODY as u64 {
        return Err(ResetCreditError::Oversized);
    }
    Ok(())
}

fn reset_credit_metrics(body: &[u8]) -> Result<Vec<Metric>, ResetCreditError> {
    ensure_reset_credit_body_size(body.len() as u64)?;
    let payload = parse_unary_grpc_web(body)?;
    let decoded = RemainingResets::decode(payload).map_err(|_| ResetCreditError::Decode)?;
    Ok(metrics_from_tokens(decoded.tokens))
}

fn parse_unary_grpc_web(body: &[u8]) -> Result<&[u8], ResetCreditError> {
    let mut offset = 0;
    let mut data: Option<&[u8]> = None;
    let mut saw_trailer = false;
    while offset < body.len() {
        if saw_trailer {
            return Err(ResetCreditError::Framing);
        }
        if body.len() - offset < 5 {
            return Err(ResetCreditError::Framing);
        }
        let flags = body[offset];
        let frame_len = u32::from_be_bytes([
            body[offset + 1],
            body[offset + 2],
            body[offset + 3],
            body[offset + 4],
        ]) as usize;
        offset += 5;
        if body.len() - offset < frame_len {
            return Err(ResetCreditError::Framing);
        }
        let payload = &body[offset..offset + frame_len];
        offset += frame_len;

        const COMPRESSED: u8 = 0x01;
        const TRAILER: u8 = 0x80;
        if flags & !(COMPRESSED | TRAILER) != 0 || flags & COMPRESSED != 0 {
            return Err(ResetCreditError::Framing);
        }
        if flags & TRAILER != 0 {
            if data.is_none() {
                return Err(ResetCreditError::Framing);
            }
            if grpc_trailer_status(payload)? != 0 {
                return Err(ResetCreditError::TrailerStatus);
            }
            saw_trailer = true;
        } else if data.is_some() {
            return Err(ResetCreditError::Framing);
        } else {
            data = Some(payload);
        }
    }
    data.ok_or(ResetCreditError::Framing)
}

fn grpc_trailer_status(payload: &[u8]) -> Result<i32, ResetCreditError> {
    let text = std::str::from_utf8(payload).map_err(|_| ResetCreditError::Framing)?;
    let mut status = None;
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ResetCreditError::Framing);
        };
        if name.eq_ignore_ascii_case("grpc-status") {
            if status.is_some() {
                return Err(ResetCreditError::TrailerStatus);
            }
            status = Some(
                value
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| ResetCreditError::TrailerStatus)?,
            );
        }
    }
    status.ok_or(ResetCreditError::TrailerStatus)
}

fn proto_timestamp_millis(ts: &ProtoTimestamp) -> Option<i64> {
    if !(PROTO_TIMESTAMP_MIN_SECONDS..=PROTO_TIMESTAMP_MAX_SECONDS).contains(&ts.seconds)
        || !(0..1_000_000_000).contains(&ts.nanos)
    {
        return None;
    }
    let nanos_ms = i64::from(ts.nanos / 1_000_000);
    ts.seconds.checked_mul(1000)?.checked_add(nanos_ms)
}

fn metrics_from_tokens(tokens: Vec<ResetToken>) -> Vec<Metric> {
    let mut credits: Vec<Option<i64>> = tokens
        .into_iter()
        .map(|token| token.validity_end.as_ref().and_then(proto_timestamp_millis))
        .collect();
    credits.sort_by_key(|expiry| expiry.unwrap_or(i64::MAX));
    let many = credits.len() > 1;
    credits
        .into_iter()
        .enumerate()
        .map(|(i, resets_at)| Metric {
            label: if many {
                format!("Reset credit {}", i + 1)
            } else {
                "Reset credit".into()
            },
            kind: "action".into(),
            used_percent: None,
            detail: None,
            value: Some("Available".into()),
            resets_at,
            period_ms: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact shape xAI returns right after a weekly rollover (captured
    /// live 2026-07-19): zero usage means creditUsagePercent is omitted.
    fn rollover_billing() -> Value {
        serde_json::json!({
            "config": {
                "currentPeriod": {
                    "type": "USAGE_PERIOD_TYPE_WEEKLY",
                    "start": "2026-07-19T11:17:21.044357+00:00",
                    "end": "2026-07-26T11:17:21.044357+00:00"
                },
                "onDemandCap": { "val": 0 },
                "onDemandUsed": { "val": 0 },
                "isUnifiedBillingUser": true,
                "prepaidBalance": { "val": 0 },
                "billingPeriodStart": "2026-07-19T11:17:21.044357+00:00",
                "billingPeriodEnd": "2026-07-26T11:17:21.044357+00:00"
            }
        })
    }

    #[test]
    fn omitted_percent_with_period_is_zero_usage() {
        assert_eq!(credit_usage_percent(&rollover_billing()), Some(0.0));
    }

    #[test]
    fn explicit_percent_wins() {
        let mut billing = rollover_billing();
        billing["config"]["creditUsagePercent"] = Value::from(37.5);
        assert_eq!(credit_usage_percent(&billing), Some(37.5));
    }

    #[test]
    fn no_config_stays_unknown_shape() {
        assert_eq!(credit_usage_percent(&serde_json::json!({})), None);
        assert_eq!(credit_usage_percent(&serde_json::json!({ "config": {} })), None);
    }

    #[test]
    fn rollover_still_reports_reset_window() {
        let (resets_at, period_ms) = current_period_window(&rollover_billing());
        assert!(resets_at.is_some());
        assert_eq!(period_ms, Some(7 * 24 * 3_600_000));
    }

    #[test]
    fn settings_display_name_takes_priority_over_user_tier() {
        let settings = serde_json::json!({
            "subscription_tier_display": "SuperGrok Heavy"
        });
        let user = serde_json::json!({ "subscriptionTier": "SuperGrok" });

        assert_eq!(
            resolve_subscription_plan(Some(&settings), Some(&user)).as_deref(),
            Some("SuperGrok Heavy")
        );
    }

    #[test]
    fn user_tier_is_mapped_when_settings_has_no_display_name() {
        let settings = serde_json::json!({});
        let user = serde_json::json!({ "subscriptionTier": "SuperGrokPro" });

        assert_eq!(
            resolve_subscription_plan(Some(&settings), Some(&user)).as_deref(),
            Some("SuperGrok Heavy")
        );
    }

    #[test]
    fn free_tier_is_hidden_and_unknown_tier_is_preserved() {
        let free = serde_json::json!({ "subscriptionTier": "free" });
        let unknown = serde_json::json!({ "subscriptionTier": "SuperGrokUltra" });

        assert_eq!(resolve_subscription_plan(None, Some(&free)), None);
        assert_eq!(
            resolve_subscription_plan(None, Some(&unknown)).as_deref(),
            Some("SuperGrokUltra")
        );
    }

    #[test]
    fn missing_subscription_fields_returns_no_plan() {
        let settings = serde_json::json!({});
        let user = serde_json::json!({});

        assert_eq!(
            resolve_subscription_plan(Some(&settings), Some(&user)),
            None
        );
    }

    const TOKEN_ID: &str = "secret-reset-token-xyz";
    const EARLIER_SECS: i64 = 1_777_000_000;
    const FUTURE_SECS: i64 = 1_777_086_400;

    fn varint(mut n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        while n >= 0x80 {
            out.push((n as u8) | 0x80);
            n >>= 7;
        }
        out.push(n as u8);
        out
    }

    fn key(field: u32, wire: u32) -> Vec<u8> {
        varint(((field << 3) | wire) as u64)
    }

    fn delimited(field: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = key(field, 2);
        out.extend(varint(payload.len() as u64));
        out.extend_from_slice(payload);
        out
    }

    fn timestamp(seconds: i64, nanos: i32) -> Vec<u8> {
        let mut out = key(1, 0);
        out.extend(varint(seconds as u64));
        if nanos != 0 {
            out.extend(key(2, 0));
            out.extend(varint(nanos as u64));
        }
        out
    }

    fn reset_token(id: &str, end: Option<(i64, i32)>) -> Vec<u8> {
        let mut tok = delimited(10, id.as_bytes());
        if let Some((seconds, nanos)) = end {
            tok.extend(delimited(30, &timestamp(seconds, nanos)));
        }
        tok
    }

    fn remaining_resets_proto(tokens: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        for token in tokens {
            out.extend(delimited(10, token));
        }
        out
    }

    fn grpc_web_frame(flags: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + payload.len());
        out.push(flags);
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn grpc_web_unary(payload: &[u8], trailer_status: Option<i32>) -> Vec<u8> {
        let mut out = grpc_web_frame(0x00, payload);
        if let Some(status) = trailer_status {
            let trailers = format!("grpc-status: {status}\r\n");
            out.extend(grpc_web_frame(0x80, trailers.as_bytes()));
        }
        out
    }

    #[test]
    fn one_future_token_becomes_unnumbered_reset_credit() {
        let proto = remaining_resets_proto(&[reset_token(TOKEN_ID, Some((FUTURE_SECS, 5_000_000)))]);
        let body = grpc_web_unary(&proto, Some(0));

        let metrics = reset_credit_metrics(&body).expect("one future credit");
        assert_eq!(metrics.len(), 1);
        let metric = &metrics[0];
        assert_eq!(metric.label, "Reset credit");
        assert_eq!(metric.kind, "action");
        assert_eq!(metric.detail, None);
        assert_eq!(metric.value.as_deref(), Some("Available"));
        assert_eq!(metric.resets_at, Some(FUTURE_SECS * 1000 + 5));
        assert_eq!(metric.period_ms, None);

        let serialized = serde_json::to_string(metric).unwrap();
        assert!(!serialized.contains(TOKEN_ID), "token id leaked: {serialized}");
        assert!(
            serialized.contains("\"detail\":null"),
            "redeem detail must stay empty: {serialized}"
        );
    }

    fn assert_no_token_ids(metrics: &[Metric], ids: &[&str]) {
        let serialized = serde_json::to_string(metrics).unwrap();
        for id in ids {
            assert!(!serialized.contains(id), "token id leaked: {serialized}");
        }
        for metric in metrics {
            assert_eq!(metric.kind, "action");
            assert_eq!(metric.detail, None);
            assert!(!metric.label.contains("Usage"));
        }
    }

    #[test]
    fn multiple_future_tokens_are_numbered_earliest_first() {
        let proto = remaining_resets_proto(&[
            reset_token("tok-late", Some((FUTURE_SECS + 86_400, 0))),
            reset_token("tok-soon", Some((FUTURE_SECS, 0))),
        ]);
        let metrics = reset_credit_metrics(&grpc_web_unary(&proto, Some(0)))
            .expect("two future credits");
        assert_eq!(
            metrics
                .iter()
                .map(|m| (m.label.as_str(), m.resets_at))
                .collect::<Vec<_>>(),
            vec![
                ("Reset credit 1", Some(FUTURE_SECS * 1000)),
                ("Reset credit 2", Some((FUTURE_SECS + 86_400) * 1000)),
            ]
        );
        assert_no_token_ids(&metrics, &["tok-late", "tok-soon"]);
    }

    #[test]
    fn endpoint_tokens_are_not_filtered_by_local_expiry() {
        let proto = remaining_resets_proto(&[
            reset_token("tok-past", Some((EARLIER_SECS - 1, 0))),
            reset_token("tok-earlier", Some((EARLIER_SECS, 0))),
            reset_token("tok-future", Some((FUTURE_SECS, 0))),
        ]);
        let metrics = reset_credit_metrics(&grpc_web_unary(&proto, Some(0)))
            .expect("trust endpoint collection");
        assert_eq!(
            metrics
                .iter()
                .map(|m| (m.label.as_str(), m.resets_at))
                .collect::<Vec<_>>(),
            vec![
                ("Reset credit 1", Some((EARLIER_SECS - 1) * 1000)),
                ("Reset credit 2", Some(EARLIER_SECS * 1000)),
                ("Reset credit 3", Some(FUTURE_SECS * 1000)),
            ]
        );
        assert_no_token_ids(&metrics, &["tok-past", "tok-earlier", "tok-future"]);
    }

    #[test]
    fn empty_collection_emits_no_metrics_but_past_dated_token_remains() {
        let empty = reset_credit_metrics(&grpc_web_unary(
            &remaining_resets_proto(&[]),
            Some(0),
        ))
        .expect("empty");
        assert!(empty.is_empty());

        let past_dated =
            remaining_resets_proto(&[reset_token("tok-old", Some((EARLIER_SECS, 0)))]);
        let metrics = reset_credit_metrics(&grpc_web_unary(&past_dated, Some(0)))
            .expect("past-dated token");
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].label, "Reset credit");
        assert_eq!(metrics[0].resets_at, Some(EARLIER_SECS * 1000));
        let serialized = serde_json::to_string(&metrics).unwrap();
        assert!(!serialized.contains("tok-old"));
        assert!(!serialized.contains("Reset credits"));
    }

    #[test]
    fn undated_and_invalid_expiry_stay_available_after_dated() {
        let proto = remaining_resets_proto(&[
            reset_token("tok-undated", None),
            reset_token("tok-overflow", Some((i64::MAX, 0))),
            reset_token("tok-negative-nanos", Some((FUTURE_SECS, -1))),
            reset_token("tok-large-nanos", Some((FUTURE_SECS, 1_000_000_000))),
            reset_token("tok-before-min", Some((-62_135_596_801, 0))),
            reset_token("tok-after-max", Some((253_402_300_800, 0))),
            reset_token("tok-dated", Some((FUTURE_SECS, 0))),
            reset_token("tok-undated-2", None),
        ]);
        let metrics = reset_credit_metrics(&grpc_web_unary(&proto, Some(0)))
            .expect("undated credits");
        assert_eq!(metrics.len(), 8);
        assert_eq!(metrics[0].label, "Reset credit 1");
        assert_eq!(metrics[0].resets_at, Some(FUTURE_SECS * 1000));
        for (index, metric) in metrics.iter().enumerate().skip(1) {
            assert_eq!(metric.label, format!("Reset credit {}", index + 1));
            assert_eq!(metric.resets_at, None);
            assert_eq!(metric.value.as_deref(), Some("Available"));
        }
        assert_no_token_ids(
            &metrics,
            &[
                "tok-undated",
                "tok-overflow",
                "tok-negative-nanos",
                "tok-large-nanos",
                "tok-before-min",
                "tok-after-max",
                "tok-dated",
                "tok-undated-2",
            ],
        );
    }

    #[test]
    fn equal_expiry_preserves_protobuf_order() {
        let proto = remaining_resets_proto(&[
            reset_token("tok-a", Some((FUTURE_SECS, 0))),
            reset_token("tok-b", Some((FUTURE_SECS, 0))),
        ]);
        let metrics = reset_credit_metrics(&grpc_web_unary(&proto, Some(0)))
            .expect("equal expiry");
        assert_eq!(metrics[0].label, "Reset credit 1");
        assert_eq!(metrics[1].label, "Reset credit 2");
        assert_eq!(metrics[0].resets_at, metrics[1].resets_at);
        assert_no_token_ids(&metrics, &["tok-a", "tok-b"]);
    }

    fn future_proto() -> Vec<u8> {
        remaining_resets_proto(&[reset_token(TOKEN_ID, Some((FUTURE_SECS, 0)))])
    }

    fn assert_category(body: &[u8], want: ResetCreditError) {
        match reset_credit_metrics(body) {
            Err(err) => {
                assert_eq!(err, want);
                assert_eq!(err.category(), want.category());
                assert!(!err.category().contains(TOKEN_ID));
                assert!(!err.category().contains("Bearer"));
            }
            Ok(_) => panic!("expected {want:?}, got metrics"),
        }
    }

    #[test]
    fn unary_parser_accepts_data_with_or_without_successful_trailer() {
        let proto = future_proto();
        let with_trailer = reset_credit_metrics(&grpc_web_unary(&proto, Some(0)))
            .expect("data + trailer");
        let without = reset_credit_metrics(&grpc_web_unary(&proto, None))
            .expect("data only");
        assert_eq!(with_trailer.len(), 1);
        assert_eq!(without.len(), 1);
        assert_eq!(with_trailer[0].resets_at, without[0].resets_at);
    }

    #[test]
    fn unknown_protobuf_fields_do_not_block_known_credits() {
        let mut token = reset_token(TOKEN_ID, Some((FUTURE_SECS, 0)));
        token.extend(delimited(11, b"unknown-token-field"));
        let mut proto = delimited(1, b"unknown-envelope-field");
        proto.extend(delimited(10, &token));
        let metrics = reset_credit_metrics(&grpc_web_unary(&proto, Some(0)))
            .expect("unknown fields");
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].label, "Reset credit");
        assert_eq!(metrics[0].resets_at, Some(FUTURE_SECS * 1000));
        assert_no_token_ids(&metrics, &[TOKEN_ID, "unknown-token-field"]);
    }

    #[test]
    fn truncated_header_is_framing() {
        assert_category(&[0, 0, 0, 0], ResetCreditError::Framing);
        let trailers = b"grpc-status: 0\r\n";
        assert_category(&grpc_web_frame(0x80, trailers), ResetCreditError::Framing);
    }

    #[test]
    fn mismatched_frame_length_is_framing() {
        let mut body = vec![0, 0, 0, 0, 20];
        body.extend_from_slice(&[1, 2, 3]);
        assert_category(&body, ResetCreditError::Framing);
    }

    #[test]
    fn unsupported_flags_are_framing() {
        let proto = future_proto();
        assert_category(&grpc_web_frame(0x01, &proto), ResetCreditError::Framing);
        assert_category(&grpc_web_frame(0x02, &proto), ResetCreditError::Framing);
        assert_category(&grpc_web_frame(0x81, &proto), ResetCreditError::Framing);
    }

    #[test]
    fn duplicate_data_frames_are_framing() {
        let frame = grpc_web_frame(0x00, &future_proto());
        let mut body = frame.clone();
        body.extend_from_slice(&frame);
        assert_category(&body, ResetCreditError::Framing);
    }

    #[test]
    fn extra_bytes_after_trailer_are_framing() {
        let mut body = grpc_web_unary(&future_proto(), Some(0));
        body.push(0);
        assert_category(&body, ResetCreditError::Framing);
    }

    #[test]
    fn non_zero_trailer_status_is_rejected() {
        assert_category(
            &grpc_web_unary(&future_proto(), Some(2)),
            ResetCreditError::TrailerStatus,
        );
    }

    #[test]
    fn trailer_requires_exactly_one_grpc_status() {
        let mut missing = grpc_web_frame(0x00, &future_proto());
        missing.extend(grpc_web_frame(0x80, b"grpc-message: ok\r\n"));
        assert_category(&missing, ResetCreditError::TrailerStatus);

        let mut duplicate = grpc_web_frame(0x00, &future_proto());
        duplicate.extend(grpc_web_frame(
            0x80,
            b"grpc-status: 2\r\ngrpc-status: 0\r\n",
        ));
        assert_category(&duplicate, ResetCreditError::TrailerStatus);
    }

    #[test]
    fn malformed_protobuf_payload_is_decode() {
        let mut bad = key(10, 2);
        bad.extend(varint(10));
        bad.push(0x01);
        assert_category(&grpc_web_unary(&bad, Some(0)), ResetCreditError::Decode);
    }

    #[test]
    fn body_over_64kib_is_oversized() {
        assert_category(
            &vec![0; MAX_RESET_CREDITS_BODY + 1],
            ResetCreditError::Oversized,
        );
        assert_eq!(
            ensure_reset_credit_body_size((MAX_RESET_CREDITS_BODY as u64) + 1),
            Err(ResetCreditError::Oversized)
        );
        let proto = future_proto();
        let body = grpc_web_unary(&proto, Some(0));
        assert!(body.len() <= MAX_RESET_CREDITS_BODY);
        ensure_reset_credit_body_size(MAX_RESET_CREDITS_BODY as u64).expect("at-limit length");
        reset_credit_metrics(&body).expect("at-limit body");
    }

    #[test]
    fn streamed_chunks_stop_at_64kib() {
        let mut body = vec![0; MAX_RESET_CREDITS_BODY];
        assert_eq!(
            accept_reset_credit_chunk(&mut body, &[0]),
            Err(ResetCreditError::Oversized)
        );
        assert_eq!(body.len(), MAX_RESET_CREDITS_BODY);

        let mut under = vec![0; MAX_RESET_CREDITS_BODY - 1];
        accept_reset_credit_chunk(&mut under, &[0]).expect("exact cap");
        assert_eq!(under.len(), MAX_RESET_CREDITS_BODY);
    }

    #[test]
    fn failure_categories_never_include_sensitive_values() {
        for err in [
            ResetCreditError::Transport,
            ResetCreditError::HttpStatus,
            ResetCreditError::Oversized,
            ResetCreditError::Framing,
            ResetCreditError::TrailerStatus,
            ResetCreditError::Decode,
        ] {
            let category = err.category();
            assert!(!category.is_empty());
            assert!(!category.contains(TOKEN_ID));
            assert!(!category.contains("Bearer"));
            assert!(!category.contains("authorization"));
            assert!(!category.contains("grpc-status"));
        }
        assert_eq!(ResetCreditError::Transport.category(), "transport");
        assert_eq!(ResetCreditError::HttpStatus.category(), "HTTP status");
        assert_eq!(ResetCreditError::Oversized.category(), "oversized body");
        assert_eq!(ResetCreditError::Framing.category(), "framing");
        assert_eq!(ResetCreditError::TrailerStatus.category(), "trailer status");
        assert_eq!(ResetCreditError::Decode.category(), "protobuf decode");
    }
}
