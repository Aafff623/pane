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

/// The `total_usage` field's unit. AihubMix (a new-api-family gateway)
/// reports the usage total in whole dollars, unlike the classic OpenAI
/// endpoint which used cents. If a card ever shows spend 100x off, this is
/// the knob.
const USAGE_TO_USD: f64 = 1.0;

fn find_api_key() -> Option<String> {
    stored_api_key(ID, &["AIHUBMIX_API_KEY"])
        .or_else(|| super::opencode::auth_entry_key("aihubmix"))
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
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

    let sub_req = http().get(SUBSCRIPTION).bearer_auth(&key).send();
    let usage_req = http().get(USAGE).bearer_auth(&key).send();
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
    // threshold. Meter against the hard limit — the real ceiling.
    let limit = sub
        .get("hard_limit_usd")
        .or_else(|| sub.get("system_hard_limit_usd"))
        .and_then(Value::as_f64)
        .filter(|v| *v > 0.0);

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
