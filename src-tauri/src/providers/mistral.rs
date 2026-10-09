//! Mistral — cookie card over the admin.mistral.ai web session (mechanism
//! per CodexBar's Mistral provider). The stored slot holds the pasted
//! `Cookie:` header (an `ory_session_*` cookie must be in it).
//!
//! Three reads: current-month usage (token totals + vibe usage), and the
//! credit wallet (`wallet + credit notes − ongoing usage`).

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "mistral";
const NAME: &str = "Mistral";
const BASE: &str = "https://admin.mistral.ai";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["MISTRAL_COOKIE"]).map(|_| "Pane Mistral session".into())
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(cookie) = stored_api_key(ID, &["MISTRAL_COOKIE"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste an admin.mistral.ai Cookie header (with ory_session_…) in Settings (gear icon).",
        ));
    };
    fetch_with_credential(&cookie).await
}

/// Live test of a pasted credential (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_credential(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn get(cookie: &str, path: &str) -> Result<Value, String> {
    let resp = http()
        .get(format!("{BASE}{path}"))
        .header("Cookie", cookie.trim().trim_start_matches("Cookie:").trim())
        .header("Accept", "*/*")
        .header("Referer", "https://admin.mistral.ai/organization/billing")
        .send()
        .await
        .map_err(|e| format!("{path}: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("Mistral session expired — paste a fresh Cookie header (ory_session_…) in Settings".into());
    }
    if !status.is_success() {
        return Err(format!("{path}: HTTP {status}"));
    }
    resp.json().await.map_err(|e| format!("{path} parse: {e}"))
}

async fn fetch_with_credential(cred: &str) -> Result<Snapshot, String> {
    use chrono::{Datelike, Utc};
    let now = Utc::now();
    let usage = get(
        cred,
        &format!("/api/billing/v2/usage?month={}&year={}", now.month(), now.year()),
    )
    .await?;
    let mut metrics = usage_metrics(&usage)?;

    // Best-effort wallet; missing/unreadable credits never sink the card.
    if let Ok(credits) = get(cred, "/api/billing/credits").await {
        if let Some(row) = credits_row(&credits) {
            metrics.push(row);
        }
    }
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

/// Sum every usage entry's value across the token categories (chat,
/// completion, ocr, connectors, audio, vibe-code) + the vibe_usage scalar.
fn usage_metrics(doc: &Value) -> Result<Vec<Metric>, String> {
    let mut tokens: i64 = 0;
    let mut walk = |model_map: &Value| {
        for entries in model_map.get("models").and_then(Value::as_object).into_iter().flat_map(|m| m.values()) {
            for side in ["input", "output", "cached"] {
                for entry in entries.get(side).and_then(Value::as_array).into_iter().flatten() {
                    tokens += entry.get("value").and_then(Value::as_i64).unwrap_or(0);
                }
            }
        }
    };
    for key in ["chat", "completion", "ocr", "connectors", "audio"] {
        if let Some(cat) = doc.get(key) {
            walk(cat);
        }
    }
    if let Some(vibe) = doc.get("vibe_code") {
        walk(vibe);
        if let Some(inner) = vibe.get("completion") {
            walk(inner);
        }
    }
    let mut metrics = Vec::new();
    if tokens > 0 {
        metrics.push(Metric::text("Monthly tokens", fmt_tokens(tokens as f64)));
    }
    if let Some(vibe) = doc.get("vibe_usage").and_then(Value::as_f64).filter(|v| *v != 0.0) {
        metrics.push(Metric::text("Vibe usage", format!("{vibe:.1}")));
    }
    if metrics.is_empty() {
        return Err("no usage numbers in response".into());
    }
    Ok(metrics)
}

fn credits_row(doc: &Value) -> Option<Metric> {
    let wallet = doc.get("wallet_amount").and_then(Value::as_f64)?;
    let notes = doc.get("credit_notes_amount").and_then(Value::as_f64).unwrap_or(0.0);
    let ongoing = doc.get("ongoing_usage_balance").and_then(Value::as_f64).unwrap_or(0.0);
    let currency = doc.get("currency").and_then(Value::as_str).unwrap_or("USD");
    let available = wallet + notes - ongoing;
    let value = if currency.eq_ignore_ascii_case("usd") {
        format!("${available:.2}")
    } else {
        format!("{available:.2} {currency}")
    };
    Some(Metric::text("Credits", value))
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
    fn usage_sums_every_category_and_side() {
        let doc = json!({
            "chat": { "models": { "mistral-large": {
                "input": [ { "value": 1000 }, { "value": 250 } ],
                "output": [ { "value": 500 } ]
            } } },
            "completion": { "models": { "codestral": {
                "input": [ { "value": 2000 } ], "output": [ { "value": 750 } ]
            } } },
            "vibe_code": { "completion": { "models": { "vibe": {
                "input": [ { "value": 500 } ]
            } } } },
            "vibe_usage": 42.7
        });
        let metrics = usage_metrics(&doc).unwrap();
        assert_eq!(metrics[0].value.as_deref(), Some("5.0K")); // 1000+250+500+2000+750+500
        assert_eq!(metrics[1].value.as_deref(), Some("42.7"));
        assert!(usage_metrics(&json!({})).is_err());
    }

    #[test]
    fn credits_combine_wallet_notes_and_ongoing() {
        let doc = json!({ "wallet_amount": 10.0, "credit_notes_amount": 5.0, "ongoing_usage_balance": 2.5, "currency": "USD" });
        assert_eq!(credits_row(&doc).unwrap().value.as_deref(), Some("$12.50"));
        assert!(credits_row(&json!({})).is_none());
    }
}
