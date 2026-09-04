//! SiliconFlow pay-as-you-go balance. Bearer API key against the user-info
//! endpoint; api.siliconflow.cn is the primary host, api.siliconflow.com
//! the international mirror — both serve the same response, so we try them
//! in turn. Key sources: our Settings pane or SILICONFLOW_API_KEY.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "siliconflow";
const NAME: &str = "SiliconFlow";
const MAX_BODY_BYTES: usize = 64 * 1024;

const ENDPOINTS: [&str; 2] = [
    "https://api.siliconflow.cn/v1/user/info",
    "https://api.siliconflow.com/v1/user/info",
];

/// No local credential source — a Settings key or SILICONFLOW_API_KEY only
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
    let Some(key) = stored_api_key(ID, &["SILICONFLOW_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a SiliconFlow API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key, ID, NAME).await
}

async fn fetch_with_key(key: &str, card_id: &str, card_name: &str) -> Result<Snapshot, String> {
    let mut last_error = String::from("balance endpoint unreachable");
    for endpoint in ENDPOINTS {
        let resp = match http().get(endpoint).bearer_auth(key).send().await {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("balance request: {e}");
                continue;
            }
        };
        // A rejected key is final — the other host won't accept it either.
        if matches!(resp.status().as_u16(), 401 | 403) {
            return Err("API key was rejected — check it in Settings".into());
        }
        if !resp.status().is_success() {
            last_error = format!("balance endpoint: HTTP {}", resp.status());
            continue;
        }
        let doc = match super::json_body(resp, MAX_BODY_BYTES, "balance").await {
            Ok(d) => d,
            Err(e) => {
                last_error = e;
                continue;
            }
        };
        let Some((balance, mut extra)) = parse_balance(&doc) else {
            return Err("no balance in response".into());
        };
        let mut metrics = Vec::new();
        // Usage line + low-credit notifications, metered against the
        // highest balance seen (top-ups raise it).
        if let Some(meter) = super::credit_meter(card_id, "¥", balance) {
            metrics.push(meter);
        }
        metrics.push(Metric::text("Balance", format!("¥{balance:.2}")));
        metrics.append(&mut extra);
        return Ok(Snapshot::ok(
            card_id,
            card_name,
            Some("Pay as you go".into()),
            metrics,
        ));
    }
    Err(last_error)
}

/// `data.totalBalance` is CNY and arrives as a stringified decimal;
/// `chargeBalance` (the top-up-only portion) rides along as a text row.
fn parse_balance(doc: &Value) -> Option<(f64, Vec<Metric>)> {
    let data = doc.get("data").unwrap_or(doc);
    let balance = json_f64(data.get("totalBalance"))?;
    let mut extra = Vec::new();
    if let Some(charge) = json_f64(data.get("chargeBalance")) {
        extra.push(Metric::text("Charged", format!("¥{charge:.2}")));
    }
    Some((balance, extra))
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
    fn parses_total_balance_and_charge_row() {
        let doc = json!({
            "code": 20000,
            "status": true,
            "data": {
                "id": "user-1",
                "totalBalance": "88.50",
                "chargeBalance": "80.00"
            }
        });
        let (balance, extra) = parse_balance(&doc).expect("parses");
        assert!((balance - 88.50).abs() < 1e-9);
        assert_eq!(extra.len(), 1);
        assert_eq!(
            (extra[0].label.as_str(), extra[0].value.as_deref()),
            ("Charged", Some("¥80.00"))
        );
    }

    #[test]
    fn bare_data_object_also_parses() {
        let doc = json!({ "totalBalance": 0.5 });
        let (balance, extra) = parse_balance(&doc).expect("parses");
        assert!((balance - 0.5).abs() < 1e-9);
        assert!(extra.is_empty());
    }

    #[test]
    fn missing_total_balance_is_rejected() {
        assert!(parse_balance(&json!({ "data": {} })).is_none());
    }

    #[test]
    fn named_error_keeps_account_identity() {
        let snap = snapshot_error_for_card("network", "siliconflow@2", "SiliconFlow — Work");
        assert_eq!(snap.id, "siliconflow@2");
        assert_eq!(snap.name, "SiliconFlow — Work");
    }
}
