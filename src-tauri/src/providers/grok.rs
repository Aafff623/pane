use super::{http, Metric, Snapshot};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::path::PathBuf;

const ID: &str = "grok";
const NAME: &str = "Grok";

fn auth_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".grok").join("auth.json")
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let path = auth_path();
    if !path.exists() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Grok CLI sign-in not found (~\\.grok\\auth.json).",
        ));
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("read auth.json: {e}"))?;
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
            let _ = std::fs::copy(&path, path.with_extension("json.pane-bak"));
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_string_pretty(&doc).unwrap_or(raw))
                .and_then(|_| std::fs::rename(&tmp, &path))
                .map_err(|e| format!("write refreshed auth.json: {e}"))?;
        }
    }

    let billing_req = http()
        .get("https://cli-chat-proxy.grok.com/v1/billing?format=credits")
        .bearer_auth(&token)
        .send();
    let settings_req = http()
        .get("https://cli-chat-proxy.grok.com/v1/settings")
        .bearer_auth(&token)
        .send();
    let (billing_resp, settings_resp) = tokio::join!(billing_req, settings_req);

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

    let mut plan = None;
    if let Ok(resp) = settings_resp {
        if resp.status().is_success() {
            if let Ok(doc) = resp.json::<Value>().await {
                plan = ["plan", "tier", "subscription", "plan_name"]
                    .iter()
                    .find_map(|k| doc.get(*k).and_then(Value::as_str))
                    .map(str::to_string);
            }
        }
    }

    Ok(Snapshot::ok(ID, NAME, plan, metrics))
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
}