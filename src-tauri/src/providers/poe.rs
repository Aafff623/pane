use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "poe";
const NAME: &str = "Poe";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["POE_API_KEY"]).map(|_| "Pane Poe API key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// The /usage/current_balance payload — Poe has sent the points as both a
/// number and a string, so both forms are accepted.
fn point_balance(doc: &Value) -> Option<f64> {
    doc.get("current_point_balance")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key("poe", &["POE_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Poe API key (poe.com/api/keys) in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = http()
        .get("https://api.poe.com/usage/current_balance")
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("balance request: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("key was rejected — paste a fresh key in Settings (gear icon)".into());
    }
    if !resp.status().is_success() {
        return Err(format!("balance endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("balance parse: {e}"))?;
    let points = point_balance(&doc).ok_or("no point balance in response")?;

    let metrics = vec![Metric::text("Balance", format!("{points:.0} points"))];
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn point_balance_accepts_number_or_string_payloads() {
        assert_eq!(point_balance(&json!({ "current_point_balance": 1250.0 })), Some(1250.0));
        assert_eq!(point_balance(&json!({ "current_point_balance": "1250" })), Some(1250.0));
        assert_eq!(point_balance(&json!({ "other": 1 })), None);
    }
}
