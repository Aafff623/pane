//! Novita AI pay-as-you-go balance. Bearer API key against the account
//! balance endpoint. Key sources: our Settings pane or NOVITA_API_KEY.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "novita";
const NAME: &str = "Novita AI";
const MAX_BODY_BYTES: usize = 64 * 1024;

const BALANCE_URL: &str = "https://api.novita.ai/v3/user/balance";

/// No local credential source — a Settings key or NOVITA_API_KEY only
/// (both reported separately by get_credential_status).
pub fn local_credential_hint() -> Option<String> {
    None
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a user-pasted key, without saving it (Customize "Test").
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    snapshot_with_key_as(key, ID, NAME).await
}

/// Fetches a key while preserving the identity of the account card that owns
/// it. The family wrapper above keeps the original public behavior.
pub async fn snapshot_with_key_as(key: &str, card_id: &str, card_name: &str) -> Snapshot {
    match fetch_with_key(key, card_id, card_name).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(card_id, card_name, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(ID, &["NOVITA_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Novita AI API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key, ID, NAME).await
}

async fn fetch_with_key(key: &str, card_id: &str, card_name: &str) -> Result<Snapshot, String> {
    let resp = http()
        .get(BALANCE_URL)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| format!("balance request: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("API key was rejected — check it in Settings".into());
    }
    if !resp.status().is_success() {
        return Err(format!("balance endpoint: HTTP {}", resp.status()));
    }
    let doc = super::json_body(resp, MAX_BODY_BYTES, "balance").await?;

    let Some(balance) = parse_balance(&doc) else {
        return Err("no balance in response".into());
    };
    let mut metrics = Vec::new();
    // Usage line + low-credit notifications, metered against the highest
    // balance seen (top-ups raise it).
    if let Some(meter) = super::credit_meter(card_id, "$", balance) {
        metrics.push(meter);
    }
    metrics.push(Metric::text("Balance", format!("${balance:.2}")));
    Ok(Snapshot::ok(
        card_id,
        card_name,
        Some("Pay as you go".into()),
        metrics,
    ))
}

/// `availableBalance` is reported in units of 0.0001 USD — divide by
/// 10000 to get dollars.
fn parse_balance(doc: &Value) -> Option<f64> {
    let raw = json_f64(doc.get("availableBalance"))?;
    Some(raw / 10_000.0)
}

fn json_f64(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
fn snapshot_error_for_card(message: &str, card_id: &str, card_name: &str) -> Snapshot {
    Snapshot::error(card_id, card_name, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn available_balance_is_scaled_to_dollars() {
        let doc = json!({ "availableBalance": 123456 });
        let balance = parse_balance(&doc).expect("parses");
        assert!((balance - 12.3456).abs() < 1e-9);
    }

    #[test]
    fn stringified_balance_parses() {
        let doc = json!({ "availableBalance": "50000" });
        assert!((parse_balance(&doc).unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn missing_balance_is_rejected() {
        assert!(parse_balance(&json!({})).is_none());
    }

    #[test]
    fn named_error_keeps_account_identity() {
        let snap = snapshot_error_for_card("network", "novita@2", "Novita AI — Work");
        assert_eq!(snap.id, "novita@2");
        assert_eq!(snap.name, "Novita AI — Work");
    }
}
