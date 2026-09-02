//! AihubMix — an OpenAI-compatible multi-model gateway (aihubmix.com),
//! pay-as-you-go against a top-up balance. It exposes the legacy OpenAI
//! dashboard-billing endpoints, so Pane reads the account's spending limit
//! and month-to-date usage and meters one against the other.
//!
//! Key source: pasted in Settings, `AIHUBMIX_API_KEY`, or — since AihubMix
//! is typically used *through* OpenCode — the `aihubmix` entry OpenCode
//! stores in its own auth.json (same reuse the OpenRouter provider does).

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "aihubmix";
const NAME: &str = "AihubMix";
const SUBSCRIPTION: &str = "https://aihubmix.com/v1/dashboard/billing/subscription";
const USAGE: &str = "https://aihubmix.com/v1/dashboard/billing/usage";

/// The `total_usage` field's unit. AihubMix follows the classic OpenAI
/// dashboard-billing convention: usage is reported in **cents**. Verified
/// against a live console (total_usage 7.862 → $0.08 shown). If a card ever
/// shows spend 100x off, this is the knob.
const USAGE_TO_USD: f64 = 0.01;

fn find_api_key() -> Option<String> {
    stored_api_key(ID, &["AIHUBMIX_API_KEY"])
        .or_else(|| super::opencode::auth_entry_key("aihubmix"))
}

/// Pure local probe for the Customize gear panel (no network): the
/// AihubMix key stored in OpenCode's auth.json.
pub fn local_credential_hint() -> Option<String> {
    super::opencode::auth_entry_key("aihubmix")
        .map(|_| "AihubMix key in OpenCode's auth.json".to_string())
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
    let Some(key) = find_api_key() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste an AihubMix API key in Settings (or sign in to AihubMix via OpenCode).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let sub_req = http().get(SUBSCRIPTION).bearer_auth(key).send();
    let usage_req = http().get(USAGE).bearer_auth(key).send();
    let (sub_resp, usage_resp) = tokio::join!(sub_req, usage_req);

    let sub_resp = sub_resp.map_err(|e| format!("subscription request: {e}"))?;
    if sub_resp.status().as_u16() == 401 {
        return Err("key was rejected — paste a fresh AihubMix key in Settings".into());
    }
    if !sub_resp.status().is_success() {
        return Err(format!("subscription endpoint: HTTP {}", sub_resp.status()));
    }
    let sub: Value = sub_resp.json().await.map_err(|e| format!("subscription parse: {e}"))?;

    // hard_limit_usd is the account's spending cap; soft_limit is the alert
    // threshold. Meter against the hard limit — the real ceiling. Each
    // field falls through on missing OR zero (an account with no custom
    // limit reports hard_limit_usd 0 while the system limit still applies).
    let limit = ["hard_limit_usd", "system_hard_limit_usd"]
        .iter()
        .filter_map(|k| sub.get(*k).and_then(Value::as_f64))
        .find(|v| *v > 0.0);

    // Usage is best-effort: a missing/failed usage call still leaves a
    // valid card showing the limit, rather than erroring the whole card.
    let used = match usage_resp {
        Ok(resp) if resp.status().is_success() => resp
            .json::<Value>()
            .await
            .ok()
            .and_then(|d| d.get("total_usage").and_then(Value::as_f64))
            .map(|v| v * USAGE_TO_USD),
        _ => None,
    };

    let mut metrics = Vec::new();
    match (limit, used) {
        (Some(limit), Some(used)) => {
            let pct = (used / limit * 100.0).clamp(0.0, 100.0);
            metrics.push(Metric::progress(
                "Usage",
                pct,
                Some(format!("${used:.2} of ${limit:.2}")),
            ));
        }
        (Some(limit), None) => {
            metrics.push(Metric::text("Limit", format!("${limit:.2}")));
        }
        (None, Some(used)) => {
            metrics.push(Metric::text("Used", format!("${used:.2}")));
        }
        (None, None) => return Err("no billing data in response".into()),
    }

    Ok(Snapshot::ok(ID, NAME, Some("Pay as you go".into()), metrics))
}
