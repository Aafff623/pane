//! StepFun pay-as-you-go balance. Bearer API key against the accounts
//! endpoint; api.stepfun.com is the long-standing host, api.stepfun.ai the
//! newer one — both serve the same response, so we try them in turn.
//! Key sources: our Settings pane or STEPFUN_API_KEY.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "stepfun";
const NAME: &str = "StepFun";
const MAX_BODY_BYTES: usize = 64 * 1024;

const ENDPOINTS: [&str; 2] = [
    "https://api.stepfun.com/v1/accounts",
    "https://api.stepfun.ai/v1/accounts",
];

/// No local credential source — a Settings key or STEPFUN_API_KEY only
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
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(ID, &["STEPFUN_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a StepFun API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let mut last_error = String::from("balance endpoint unreachable");
    for endpoint in ENDPOINTS {
        let resp = match http().get(endpoint).bearer_auth(&key).send().await {
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
        if let Some(meter) = super::credit_meter(ID, "¥", balance) {
            metrics.push(meter);
        }
        metrics.push(Metric::text("Balance", format!("¥{balance:.2}")));
        metrics.append(&mut extra);
        return Ok(Snapshot::ok(
            ID,
            NAME,
            Some("Pay as you go".into()),
            metrics,
        ));
    }
    Err(last_error)
}

/// `balance` is CNY; the cash/voucher split rides along as text rows when
/// the account reports it.
fn parse_balance(doc: &Value) -> Option<(f64, Vec<Metric>)> {
    let balance = json_f64(doc.get("balance"))?;
    let mut extra = Vec::new();
    if let Some(cash) = json_f64(doc.get("total_cash_balance")) {
        extra.push(Metric::text("Cash", format!("¥{cash:.2}")));
    }
    if let Some(voucher) = json_f64(doc.get("total_voucher_balance")) {
        extra.push(Metric::text("Vouchers", format!("¥{voucher:.2}")));
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_balance_and_split_rows() {
        let doc = json!({
            "balance": 12.34,
            "total_cash_balance": "10.00",
            "total_voucher_balance": 2.34
        });
        let (balance, extra) = parse_balance(&doc).expect("parses");
        assert!((balance - 12.34).abs() < 1e-9);
        assert_eq!(extra.len(), 2);
        assert_eq!(
            (extra[0].label.as_str(), extra[0].value.as_deref()),
            ("Cash", Some("¥10.00"))
        );
        assert_eq!(
            (extra[1].label.as_str(), extra[1].value.as_deref()),
            ("Vouchers", Some("¥2.34"))
        );
    }

    #[test]
    fn stringified_balance_parses_and_extras_are_optional() {
        let doc = json!({ "balance": "56.78" });
        let (balance, extra) = parse_balance(&doc).expect("parses");
        assert!((balance - 56.78).abs() < 1e-9);
        assert!(extra.is_empty());
    }

    #[test]
    fn missing_balance_is_rejected() {
        assert!(parse_balance(&json!({})).is_none());
        assert!(parse_balance(&json!({ "total_cash_balance": 1.0 })).is_none());
    }
}
