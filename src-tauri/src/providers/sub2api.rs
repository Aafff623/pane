//! sub2api — self-hosted subscription-to-API gateway card (mechanism per
//! CodexBar's sub2api provider: one `GET /v1/usage`, never a model call).
//!
//! The deployment base URL lives in the provider's stored JSON (`baseUrl`,
//! typed in Settings); the key slot holds the group API key. The response
//! mode decides the rows: quota key, subscription group (daily/weekly/
//! monthly spend), or wallet group — all carry key-scoped request/token
//! totals.

use super::{http, stored_api_key, stored_base_url, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "sub2api";
const NAME: &str = "sub2api";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["SUB2API_API_KEY"]).map(|_| "Pane sub2api group key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(ID, &["SUB2API_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Set your sub2api base URL and paste a group API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let Some(mut base) = stored_base_url(ID) else {
        return Err("no base URL configured — type your sub2api deployment URL in Settings (gear icon)".into());
    };
    base = base.trim_end_matches('/').to_string();
    if !base.starts_with("https://") && !base.starts_with("http://127.0.0.1") && !base.starts_with("http://localhost") {
        return Err("base URL must be HTTPS (loopback HTTP allowed)".into());
    }
    let url = format!("{base}/v1/usage?days=30&timezone={}", pct_encode(&local_tz()));
    let resp = http()
        .get(&url)
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("usage request: {e}"))?;
    if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
        return Err("key was rejected — check that it is active and assigned to a group".into());
    }
    if !resp.status().is_success() {
        return Err(format!("usage endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("usage parse: {e}"))?;
    parse_usage(&doc)
}

fn local_tz() -> String {
    //chrono::Local's offset formats as +08:00 — exactly what the API wants.
    chrono::Local::now().format("%:z").to_string()
}

/// Minimal query encoder (the timezone is the only dynamic piece).
fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn parse_date(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_f64().map(|n| if n > 1e12 { n as i64 } else { (n * 1000.0) as i64 }),
        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis()),
        _ => None,
    }
}

fn money(value: f64, unit: &str) -> String {
    if unit.eq_ignore_ascii_case("usd") {
        format!("${value:.2}")
    } else {
        format!("{value:.2} {unit}")
    }
}

fn window(label: &str, used: f64, limit: Option<f64>, unit: &str, reset: Option<i64>) -> Option<Metric> {
    let limit = limit?;
    if limit <= 0.0 {
        return None;
    }
    Some(
        Metric::progress(
            label,
            (used / limit * 100.0).clamp(0.0, 100.0),
            Some(format!("{} / {}", money(used, unit), money(limit, unit))),
        )
        .with_reset(reset, None),
    )
}

fn parse_usage(doc: &Value) -> Result<Snapshot, String> {
    if doc.get("isValid").and_then(Value::as_bool) == Some(false) {
        return Err("key was rejected — check that it is active and assigned to a group".into());
    }
    let quota_unit = doc.pointer("/quota/unit").and_then(Value::as_str).unwrap_or("USD").to_string();
    let unit = doc.get("unit").and_then(Value::as_str).unwrap_or(&quota_unit).to_string();

    let mut metrics = Vec::new();
    let sub = doc.get("subscription").filter(|s| s.is_object());
    if let Some(sub) = sub {
        let get = |k: &str| sub.get(k).and_then(Value::as_f64);
        let reset = sub.get("expires_at").and_then(parse_date);
        for (key, label) in [
            ("daily_usage_usd", "Daily"),
            ("weekly_usage_usd", "Weekly"),
            ("monthly_usage_usd", "Monthly"),
        ] {
            let used = get(key).unwrap_or(0.0);
            let limit = get(&key.replace("usage", "limit"));
            if let Some(m) = window(label, used, limit, "USD", reset) {
                metrics.push(m);
            }
        }
    } else if let Some(quota) = doc.get("quota").filter(|q| q.is_object()) {
        let used = quota.get("used").and_then(Value::as_f64).unwrap_or(0.0);
        let limit = quota.get("limit").and_then(Value::as_f64);
        if let Some(m) = window("Quota", used, limit, &quota_unit, doc.get("expires_at").and_then(parse_date)) {
            metrics.push(m);
        }
    }

    // Optional 5h / 1d / 7d rate-limit windows.
    for rate in doc.get("rate_limits").and_then(Value::as_array).into_iter().flatten() {
        let Some(w) = rate.get("window").and_then(Value::as_str) else { continue };
        let label = match w.to_ascii_lowercase().as_str() {
            "5h" => "5-hour limit",
            "1d" => "Daily limit",
            "7d" => "7-day limit",
            other => return Err(format!("unknown rate-limit window `{other}`")),
        };
        let used = rate.get("used").and_then(Value::as_f64).unwrap_or(0.0);
        let limit = rate.get("limit").and_then(Value::as_f64);
        if let Some(m) = window(label, used, limit, "USD", rate.get("reset_at").and_then(parse_date)) {
            metrics.push(m);
        }
    }

    if let Some(balance) = doc.get("balance").and_then(Value::as_f64) {
        metrics.push(Metric::text("Balance", money(balance, &unit)));
    }
    for (bucket, label) in [("today", "Today"), ("total", "All time")] {
        let Some(t) = doc.pointer(&format!("/usage/{bucket}")).filter(|v| v.is_object()) else { continue };
        let requests = t.get("requests").and_then(Value::as_i64);
        let tokens = t.get("total_tokens").and_then(Value::as_i64);
        if let Some(r) = requests {
            metrics.push(Metric::text(&format!("{label} requests"), format!("{r}")));
        }
        if let Some(n) = tokens {
            let cost = t.get("actual_cost").and_then(Value::as_f64);
            metrics.push(Metric::text(
                &format!("{label} tokens"),
                match cost {
                    Some(c) => format!("{} (${c:.2})", fmt_tokens(n as f64)),
                    None => fmt_tokens(n as f64),
                },
            ));
        }
    }
    if metrics.is_empty() {
        return Err("no quota, subscription or usage data in response".into());
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
    fn subscription_groups_show_three_spend_windows_and_totals() {
        let doc = json!({
            "isValid": true,
            "unit": "USD",
            "subscription": {
                "daily_usage_usd": 3.2, "daily_limit_usd": 10.0,
                "weekly_usage_usd": 21.5, "weekly_limit_usd": 50.0,
                "monthly_usage_usd": 68.0, "monthly_limit_usd": 200.0,
                "expires_at": "2026-11-01T00:00:00Z"
            },
            "rate_limits": [
                { "window": "5h", "limit": 5.0, "used": 2.5, "remaining": 2.5, "reset_at": 1_768_435_200 }
            ],
            "usage": {
                "today": { "requests": 41, "total_tokens": 812_345, "actual_cost": 3.2 },
                "total": { "requests": 900, "total_tokens": 12_000_000 }
            }
        });
        let snap = parse_usage(&doc).unwrap();
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, ["Daily", "Weekly", "Monthly", "5-hour limit", "Today requests", "Today tokens", "All time requests", "All time tokens"]);
        assert_eq!(snap.metrics[0].used_percent, Some(32.0));
        assert_eq!(snap.metrics[0].resets_at, Some(1_793_491_200_000));
        assert_eq!(snap.metrics[3].used_percent, Some(50.0));
        assert_eq!(snap.metrics[5].value.as_deref(), Some("812.3K ($3.20)"));
    }

    #[test]
    fn quota_keys_and_wallet_groups_both_render() {
        let quota = json!({
            "isValid": true,
            "unit": "USD",
            "quota": { "limit": 100.0, "used": 25.0, "remaining": 75.0, "unit": "USD" },
            "balance": 12.5
        });
        let snap = parse_usage(&quota).unwrap();
        assert_eq!(snap.metrics[0].label, "Quota");
        assert_eq!(snap.metrics[0].used_percent, Some(25.0));
        assert_eq!(snap.metrics[1].value.as_deref(), Some("$12.50"));

        let wallet = json!({ "isValid": true, "unit": "CNY", "balance": 88.4 });
        let snap = parse_usage(&wallet).unwrap();
        assert_eq!(snap.metrics[0].value.as_deref(), Some("88.40 CNY"));
        assert!(parse_usage(&json!({ "isValid": false })).is_err());
    }
}
