use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "openrouter";
const NAME: &str = "OpenRouter";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a user-pasted key, without saving it (Customize "Test").
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key, "the pasted key").await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Pure local probe for the Customize gear panel (no network): the
/// OpenRouter key stored in OpenCode's auth.json.
pub fn local_credential_hint() -> Option<String> {
    super::opencode::auth_entry_key("openrouter")
        .map(|_| "OpenRouter key in OpenCode's auth.json".to_string())
}

async fn fetch() -> Result<Snapshot, String> {
    // Saved key or env var first, then the key OpenCode stores if the user
    // connected OpenRouter there.
    match stored_api_key("openrouter", &["OPENROUTER_API_KEY"]) {
        Some(key) => fetch_with_key(&key, "your saved key").await,
        None => match super::opencode::auth_entry_key("openrouter") {
            Some(key) => fetch_with_key(&key, "the key found in OpenCode's auth.json").await,
            None => Ok(Snapshot::no_credentials(
                ID,
                NAME,
                "Paste an OpenRouter API key in Settings (gear icon).",
            )),
        },
    }
}

async fn fetch_with_key(key: &str, source: &str) -> Result<Snapshot, String> {
    let credits_req = http()
        .get("https://openrouter.ai/api/v1/credits")
        .bearer_auth(&key)
        .send();
    let key_req = http()
        .get("https://openrouter.ai/api/v1/key")
        .bearer_auth(&key)
        .send();
    let (credits_resp, key_resp) = tokio::join!(credits_req, key_req);

    let credits_resp = credits_resp.map_err(|e| format!("credits request: {e}"))?;
    if credits_resp.status().as_u16() == 401 {
        return Err(format!(
            "{source} was rejected — paste a fresh key in Settings (gear icon)"
        ));
    }
    if !credits_resp.status().is_success() {
        return Err(format!("credits endpoint: HTTP {}", credits_resp.status()));
    }
    let credits: Value = credits_resp.json().await.map_err(|e| format!("credits parse: {e}"))?;
    let data = credits.get("data").unwrap_or(&credits);

    let mut metrics = Vec::new();
    let total = data.get("total_credits").and_then(Value::as_f64);
    let used = data.get("total_usage").and_then(Value::as_f64);
    if let (Some(total), Some(used)) = (total, used) {
        metrics.push(Metric::text("Balance", format!("${:.2}", (total - used).max(0.0))));
        if total > 0.0 {
            metrics.push(Metric::progress(
                "Credits",
                used / total * 100.0,
                Some(format!("${used:.2} of ${total:.2} used")),
            ));
        }
    }

    let mut plan = None;
    if let Ok(resp) = key_resp {
        if resp.status().is_success() {
            if let Ok(info) = resp.json::<Value>().await {
                let data = info.get("data").unwrap_or(&info).clone();
                plan = data.get("is_free_tier").and_then(Value::as_bool).map(|free| {
                    if free { "Free tier".to_string() } else { "Pay as you go".to_string() }
                });
                if let (Some(limit), Some(usage)) = (
                    data.get("limit").and_then(Value::as_f64),
                    data.get("usage").and_then(Value::as_f64),
                ) {
                    if limit > 0.0 {
                        metrics.push(Metric::progress(
                            "Key limit",
                            usage / limit * 100.0,
                            Some(format!("${usage:.2} of ${limit:.2}")),
                        ));
                    }
                }
            }
        }
    }

    if metrics.is_empty() {
        return Err("no credit data in response".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}
