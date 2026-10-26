//! SiliconFlow key connectivity via authenticated models listing. The public
//! balance endpoint was retired; never infer a wallet amount from connectivity.
//! Key sources: Settings or SILICONFLOW_API_KEY.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "siliconflow";
const NAME: &str = "SiliconFlow";
const MAX_BODY_BYTES: usize = 64 * 1024;

const ENDPOINTS: [&str; 2] = [
    "https://api.siliconflow.cn/v1/models",
    "https://api.siliconflow.com/v1/models",
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
    let mut last_error = String::from("models endpoint unreachable");
    for endpoint in ENDPOINTS {
        let resp = match http().get(endpoint).bearer_auth(key).send().await {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("models request: {e}");
                continue;
            }
        };
        // A rejected key is final — the other host won't accept it either.
        if matches!(resp.status().as_u16(), 401 | 403) {
            return Err("API key was rejected — check it in Settings".into());
        }
        if !resp.status().is_success() {
            last_error = format!(
            "models endpoint: HTTP {}{}",
            resp.status(),
            super::retry_hint(&resp)
        );
            continue;
        }
        // Official release notes retired /user/info on 2026-08-14.
        // An authenticated models response proves connectivity, not balance.
        let doc = super::json_body(resp, MAX_BODY_BYTES, "models").await?;
        return connected_snapshot(&doc, card_id, card_name);

    }
    Err(last_error)
}

fn connected_snapshot(doc: &Value, card_id: &str, card_name: &str) -> Result<Snapshot, String> {
    if !doc.get("data").is_some_and(Value::is_array) {
        return Err("models response has no model list".into());
    }
    Ok(Snapshot::needs_setup(card_id, card_name, Some("API key connected".into()),
        vec![Metric::text("Connection", "API key accepted".into())],
        "API key accepted. SiliconFlow discontinued the public balance API; check the account balance in the official console. Replacing the key cannot enable an unavailable balance API."))
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
    fn authenticated_models_show_connectivity_without_inventing_balance() {
        let snap = connected_snapshot(&json!({"data": [{"id": "model"}]}), "siliconflow@2", "Work").unwrap();
        assert_eq!(snap.id, "siliconflow@2");
        assert_eq!(snap.status, "needs_setup");
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "Connection");
        assert!(snap.metrics[0].used_percent.is_none());
    }

    #[test]
    fn a_retired_balance_document_does_not_prove_models_connectivity() {
        assert!(connected_snapshot(&json!({"data": {"totalBalance": "88.5"}}), ID, NAME).is_err());
    }

    #[test]
    fn named_error_keeps_account_identity() {
        let snap = snapshot_error_for_card("network", "siliconflow@2", "SiliconFlow — Work");
        assert_eq!(snap.id, "siliconflow@2");
        assert_eq!(snap.name, "SiliconFlow — Work");
    }
}
