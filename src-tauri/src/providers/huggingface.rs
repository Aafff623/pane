//! Hugging Face — API-key card over the Hub's identity + billing endpoints
//! (mechanism per CodexBar's HuggingFace provider).
//!
//! `whoami-v2` names the account; `settings/billing/usage-v2` (current
//! month) reports inference spend in nano-USD. Billable usage is
//! `max(0, used − included)` — the same arithmetic HF's billing UI uses.
//! Classic `read` tokens work; fine-grained tokens need the Billing read
//! permission or the endpoint answers 403.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "huggingface";
const NAME: &str = "Hugging Face";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(
        ID,
        &["HF_TOKEN", "HUGGINGFACE_API_KEY", "HUGGING_FACE_HUB_TOKEN"],
    )
    .map(|_| "Pane Hugging Face token".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(
        ID,
        &["HF_TOKEN", "HUGGINGFACE_API_KEY", "HUGGING_FACE_HUB_TOKEN"],
    ) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Hugging Face access token (huggingface.co/settings/tokens) in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let whoami = http()
        .get("https://huggingface.co/api/whoami-v2")
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("whoami request: {e}"))?;
    if whoami.status().as_u16() == 401 {
        return Err(
            "token was rejected — paste a fresh access token in Settings (gear icon)".into(),
        );
    }
    let whoami: Value = whoami
        .json()
        .await
        .map_err(|e| format!("whoami parse: {e}"))?;
    let name = whoami
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("signed in")
        .to_string();
    let plan = whoami
        .pointer("/plan/name")
        .or_else(|| whoami.get("plan"))
        .and_then(Value::as_str)
        .map(str::to_string);

    use chrono::{Datelike, Utc};
    let now = Utc::now().date_naive();
    let start = now.with_day(1).unwrap_or(now);
    let end = now + chrono::Duration::days(1);
    let url = format!(
        "https://huggingface.co/api/settings/billing/usage-v2?startDate={}&endDate={}",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d")
    );
    let resp = http()
        .get(&url)
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("billing request: {e}"))?;
    if resp.status().as_u16() == 403 {
        return Err(
            "token lacks the Billing read permission — create one with billing access".into(),
        );
    }
    if resp.status().as_u16() == 401 {
        return Err(
            "token was rejected — paste a fresh access token in Settings (gear icon)".into(),
        );
    }
    if !resp.status().is_success() {
        return Err(format!("billing endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| format!("billing parse: {e}"))?;
    parse_billing(&doc, &name, plan.as_deref())
}

fn parse_billing(doc: &Value, name: &str, plan: Option<&str>) -> Result<Snapshot, String> {
    let inference = doc
        .get("inference")
        .ok_or("billing response has no inference bucket")?;
    let nano = |key: &str| {
        inference
            .get(key)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| v / 1e9)
    };
    let gross = nano("usedNanoUsd").ok_or("missing valid inference.usedNanoUsd")?;
    let included = nano("includedNanoUsd").ok_or("missing valid inference.includedNanoUsd")?;
    let billable = (gross - included).max(0.0);
    let limit = nano("limitNanoUsd");
    let requests = inference
        .get("numRequests")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let mut metrics = Vec::new();
    if let Some(limit) = limit.filter(|v| *v > 0.0) {
        metrics.push(Metric::progress(
            "Billable usage",
            (billable / limit * 100.0).clamp(0.0, 100.0),
            Some(format!("${billable:.2} of ${limit:.2}")),
        ));
    } else {
        metrics.push(Metric::text("Billable usage", format!("${billable:.2}")));
    }
    if included > 0.0 {
        metrics.push(Metric::text("Included usage", format!("${included:.2}")));
    }
    if requests > 0 {
        metrics.push(Metric::text("Requests", format!("{requests}")));
    }
    if metrics.is_empty() {
        return Err("billing response carried no numbers".into());
    }
    let plan_label = plan.map(str::to_string).unwrap_or_else(|| name.to_string());
    Ok(Snapshot::ok(ID, NAME, Some(plan_label), metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_billing_bucket_is_not_unused_free_allowance() {
        assert!(parse_billing(
            &serde_json::json!({"inference":{"limitNanoUsd":1000000000}}),
            "user",
            None
        )
        .is_err());
    }

    use serde_json::json;

    #[test]
    fn billable_is_gross_minus_included_in_usd() {
        let doc = json!({ "inference": {
            "usedNanoUsd": 12_500_000_000.0,
            "includedNanoUsd": 2_500_000_000.0,
            "limitNanoUsd": 100_000_000_000.0,
            "numRequests": 4211
        }});
        let snap = parse_billing(&doc, "alice", Some("PRO")).unwrap();
        assert_eq!(snap.plan.as_deref(), Some("PRO"));
        let billable = &snap.metrics[0];
        assert_eq!(billable.used_percent, Some(10.0)); // $10 of $100
        assert_eq!(billable.detail.as_deref(), Some("$10.00 of $100.00"));
        assert_eq!(snap.metrics[1].value.as_deref(), Some("$2.50"));
        assert_eq!(snap.metrics[2].value.as_deref(), Some("4211"));
    }

    #[test]
    fn included_larger_than_gross_clamps_to_zero() {
        let doc = json!({ "inference": { "usedNanoUsd": 1.0e9, "includedNanoUsd": 5.0e9 } });
        let snap = parse_billing(&doc, "a", None).unwrap();
        assert_eq!(snap.metrics[0].value.as_deref(), Some("$0.00"));
        assert!(parse_billing(&json!({}), "a", None).is_err());
    }
}
