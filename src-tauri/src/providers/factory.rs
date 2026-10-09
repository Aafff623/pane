//! Factory (Droid) — API-key card over Factory's own billing endpoints
//! (mechanism per CodexBar's Factory provider; web/WorkOS sessions are out
//! of scope for the key card).
//!
//! `GET /api/billing/limits` first: token-rate-limits billing answers with
//! 5-hour / weekly / monthly windows; the legacy response instead falls
//! back to `GET /api/organization/subscription/usage` (allowance ratios).

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "factory";
const NAME: &str = "Droid";
const LIMITS_URL: &str = "https://api.factory.ai/api/billing/limits";
const USAGE_URL: &str = "https://api.factory.ai/api/organization/subscription/usage";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["FACTORY_API_KEY"]).map(|_| "Pane Droid API key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(ID, &["FACTORY_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Factory API key (app.factory.ai → Settings → API keys) in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = http()
        .get(LIMITS_URL)
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("billing/limits request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err("key was rejected — paste a fresh Factory API key in Settings (gear icon)".into());
    }
    if !resp.status().is_success() {
        return Err(format!("billing/limits endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("billing/limits parse: {e}"))?;

    if doc.get("usesTokenRateLimitsBilling").and_then(Value::as_bool) == Some(true) {
        let metrics = rate_limit_metrics(&doc)?;
        return Ok(Snapshot::ok(ID, NAME, None, metrics));
    }

    // Legacy allowance billing: subscription usage carries the ratio.
    let resp = http()
        .get(USAGE_URL)
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("subscription/usage request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("subscription/usage endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("subscription/usage parse: {e}"))?;
    parse_usage(&doc)
}

/// One window object → (label, usedPercent, resets_at ms).
fn window(label: &str, w: &Value) -> Option<Metric> {
    let pct = w.get("usedPercent").and_then(Value::as_f64)?;
    let reset = w
        .get("windowEnd")
        .and_then(|v| match v {
            Value::Number(n) => n.as_f64().map(|n| if n > 1e12 { n as i64 } else { (n * 1000.0) as i64 }),
            Value::String(s) => chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis()),
            _ => None,
        })
        .or_else(|| {
            w.get("secondsRemaining")
                .and_then(Value::as_f64)
                .filter(|s| *s > 0.0)
                .map(|s| chrono::Utc::now().timestamp_millis() + (s * 1000.0) as i64)
        });
    let pct = pct.clamp(0.0, 100.0);
    Some(Metric::progress(label, pct, Some(format!("{pct:.0}% used"))).with_reset(reset, None))
}

fn pool_rows(pool: Option<&Value>, prefix: &str) -> Vec<Metric> {
    let Some(pool) = pool else { return vec![] };
    let mut out = vec![];
    for (key, label) in [("fiveHour", "5-hour"), ("weekly", "Weekly"), ("monthly", "Monthly")] {
        if let Some(m) = pool.get(key).and_then(|w| window(&format!("{prefix}{label}"), w)) {
            out.push(m);
        }
    }
    out
}

fn rate_limit_metrics(doc: &Value) -> Result<Vec<Metric>, String> {
    let limits = doc.get("limits").ok_or("billing/limits has no limits object")?;
    let mut metrics = pool_rows(limits.get("standard"), "");
    metrics.extend(pool_rows(limits.get("core"), "Core "));
    if let Some(cents) = doc.get("extraUsageBalanceCents").and_then(Value::as_i64).filter(|c| *c != 0) {
        metrics.push(Metric::text("Extra usage balance", format!("${:.2}", cents as f64 / 100.0)));
    }
    if metrics.is_empty() {
        return Err("no rate-limit windows in response".into());
    }
    Ok(metrics)
}

/// Legacy: `{ "usage": { "standard": { "userTokens", "totalAllowance",
/// "usedRatio", … } } }` — ratio first, token counts as the fallback.
fn parse_usage(doc: &Value) -> Result<Snapshot, String> {
    let std = doc
        .pointer("/usage/standard")
        .ok_or("usage response has no standard bucket")?;
    let mut metrics = Vec::new();
    if let Some(ratio) = std.get("usedRatio").and_then(Value::as_f64) {
        metrics.push(Metric::progress("Usage", (ratio * 100.0).clamp(0.0, 100.0), Some(format!("{:.0}% used", ratio * 100.0))));
    } else if let (Some(used), Some(total)) =
        (std.get("userTokens").and_then(Value::as_i64), std.get("totalAllowance").and_then(Value::as_i64))
    {
        if total > 0 {
            metrics.push(Metric::progress(
                "Usage",
                (used as f64 / total as f64 * 100.0).clamp(0.0, 100.0),
                Some(format!("{} of {} tokens", fmt_tokens(used as f64), fmt_tokens(total as f64))),
            ));
        }
    }
    if metrics.is_empty() {
        return Err("no usage ratio or allowance in response".into());
    }
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

fn fmt_tokens(n: f64) -> String {
    if n >= 1e9 {
        format!("{:.1}B", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1}K", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn token_rate_limits_become_three_windows_plus_balance() {
        let doc = json!({
            "usesTokenRateLimitsBilling": true,
            "extraUsageBalanceCents": 500,
            "limits": { "standard": {
                "fiveHour": { "usedPercent": 40.0, "secondsRemaining": 3600.0 },
                "weekly": { "usedPercent": 61.5, "windowEnd": 1_768_435_200 },
                "monthly": { "usedPercent": 12.0, "windowEnd": "2026-11-01T00:00:00Z" }
            } }
        });
        let metrics = rate_limit_metrics(&doc).unwrap();
        assert_eq!(metrics.len(), 4);
        assert_eq!(metrics[0].label, "5-hour");
        assert_eq!(metrics[0].used_percent, Some(40.0));
        assert!(metrics[0].resets_at.is_some()); // projected from secondsRemaining
        assert_eq!(metrics[1].label, "Weekly");
        assert_eq!(metrics[1].resets_at, Some(1_768_435_200_000)); // seconds → ms
        assert_eq!(metrics[2].resets_at, Some(1_793_491_200_000)); // RFC3339 windowEnd
        assert_eq!(metrics[3].value.as_deref(), Some("$5.00"));
    }

    #[test]
    fn legacy_usage_prefers_ratio_over_tokens() {
        let ratio = json!({ "usage": { "standard": { "usedRatio": 0.42, "userTokens": 5, "totalAllowance": 10 } } });
        let snap = parse_usage(&ratio).unwrap();
        assert_eq!(snap.metrics[0].used_percent, Some(42.0));
        let tokens = json!({ "usage": { "standard": { "userTokens": 250_000, "totalAllowance": 1_000_000 } } });
        let snap = parse_usage(&tokens).unwrap();
        assert_eq!(snap.metrics[0].used_percent, Some(25.0));
        assert!(snap.metrics[0].detail.as_deref().unwrap().contains("250.0K"));
        assert!(parse_usage(&json!({ "usage": {} })).is_err());
    }
}
