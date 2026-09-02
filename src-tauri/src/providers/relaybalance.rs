//! Custom Balance — a generic balance check for relay/gateway sites that
//! expose the OpenAI dashboard-billing endpoints (DMXAPI, PackyCode, Micu,
//! CrazyRouter, SudoCode.chat, XycAi, E-FlowCode, CherryIN, AICodeWith, …).
//! Same shape as AihubMix: `hard_limit_usd` from the subscription endpoint
//! metered against month-to-date `total_usage` from the usage endpoint.
//!
//! There is no preset host: the user pastes the relay's base URL and API
//! key in Settings, stored together in %APPDATA%\Pane\relaybalance.json
//! as `{"apiKey": "...", "baseUrl": "https://..."}`. Both path forms are
//! tried — some relays mount the endpoints under /v1, some don't.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "relaybalance";
const NAME: &str = "Custom Balance";
const MAX_BODY_BYTES: usize = 64 * 1024;

/// The `total_usage` field's unit. Relays follow the classic OpenAI
/// dashboard-billing convention: usage is reported in **cents**.
const USAGE_TO_USD: f64 = 0.01;

/// No local credential source — the relay's base URL and key live only in
/// Pane's own Settings file (reported separately by get_credential_status).
pub fn local_credential_hint() -> Option<String> {
    None
}

/// The (subscription, usage) URL pairs to try, /v1 form first. Relays
/// exist in both flavors, and a trailing slash on the pasted base would
/// otherwise produce a double slash.
fn billing_endpoints(base: &str) -> [(String, String); 2] {
    let base = base.trim().trim_end_matches('/');
    [
        (
            format!("{base}/v1/dashboard/billing/subscription"),
            format!("{base}/v1/dashboard/billing/usage"),
        ),
        (
            format!("{base}/dashboard/billing/subscription"),
            format!("{base}/dashboard/billing/usage"),
        ),
    ]
}

/// A rejected key is final — the other path form won't accept it either.
/// Any other non-2xx might be a relay that only mounts the other form.
fn is_final_rejection(status: u16) -> bool {
    matches!(status, 401 | 403)
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a pasted key against a given relay base URL, without
/// saving either (Customize "Test"). `base_url` is required — the stored
/// one belongs to the saved key, not to what's being tested.
pub async fn snapshot_with_key(key: &str, base_url: &str) -> Snapshot {
    match fetch_with_key(key, base_url).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let (Some(base), Some(key)) = (super::stored_base_url(ID), stored_api_key(ID, &[])) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste the relay's base URL and API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key, &base).await
}

async fn fetch_with_key(key: &str, base: &str) -> Result<Snapshot, String> {
    let mut last_error = String::from("billing endpoint unreachable");
    for (sub_url, usage_url) in billing_endpoints(&base) {
        let sub_req = http().get(&sub_url).bearer_auth(&key).send();
        let usage_req = http().get(&usage_url).bearer_auth(&key).send();
        let (sub_resp, usage_resp) = tokio::join!(sub_req, usage_req);

        let sub_resp = match sub_resp {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("subscription request: {e}");
                continue;
            }
        };
        if is_final_rejection(sub_resp.status().as_u16()) {
            return Ok(Snapshot::error(
                ID,
                NAME,
                "API key was rejected — check it in Settings".into(),
            ));
        }
        if !sub_resp.status().is_success() {
            last_error = format!("subscription endpoint: HTTP {}", sub_resp.status());
            continue;
        }
        let sub = super::json_body(sub_resp, MAX_BODY_BYTES, "subscription").await?;

        // Usage is best-effort: a missing/failed usage call still leaves a
        // valid card showing the limit, rather than erroring the whole card.
        let used = match usage_resp {
            Ok(r) if r.status().is_success() => {
                super::json_body(r, MAX_BODY_BYTES, "usage").await.ok()
            }
            _ => None,
        };

        let Some(metrics) = billing_metrics(&sub, used.as_ref()) else {
            return Ok(Snapshot::error(
                ID,
                NAME,
                "no hard_limit_usd in the subscription response".into(),
            ));
        };
        return Ok(Snapshot::ok(ID, NAME, Some("Pay as you go".into()), metrics));
    }
    Err(last_error)
}

/// Metrics from the two OpenAI-shaped billing responses: usage metered
/// against the account's spending limit, plus the remaining balance.
/// `None` when the subscription carries no usable limit.
fn billing_metrics(sub: &Value, usage: Option<&Value>) -> Option<Vec<Metric>> {
    // hard_limit_usd is the account's spending cap; fall through on
    // missing OR zero (an account with no custom limit reports
    // hard_limit_usd 0 while the system limit still applies).
    let limit = ["hard_limit_usd", "system_hard_limit_usd"]
        .iter()
        .filter_map(|k| sub.get(*k).and_then(Value::as_f64))
        .find(|v| *v > 0.0)?;
    let used = usage
        .and_then(|u| u.get("total_usage"))
        .and_then(Value::as_f64)
        .map(|v| v * USAGE_TO_USD);

    let mut metrics = Vec::new();
    match used {
        Some(used) => {
            let pct = (used / limit * 100.0).clamp(0.0, 100.0);
            metrics.push(Metric::progress(
                "Usage",
                pct,
                Some(format!("${used:.2} of ${limit:.2}")),
            ));
            metrics.push(Metric::text("Balance", format!("${:.2}", limit - used)));
        }
        None => {
            metrics.push(Metric::text("Limit", format!("${limit:.2}")));
        }
    }
    Some(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn subscription_and_usage_make_a_meter_and_balance() {
        let sub = json!({ "hard_limit_usd": 10.0 });
        let usage = json!({ "total_usage": 500.0 }); // cents → $5.00
        let metrics = billing_metrics(&sub, Some(&usage)).expect("parses");
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].label, "Usage");
        assert!((metrics[0].used_percent.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(metrics[0].detail.as_deref(), Some("$5.00 of $10.00"));
        assert_eq!(
            (metrics[1].label.as_str(), metrics[1].value.as_deref()),
            ("Balance", Some("$5.00"))
        );
    }

    #[test]
    fn missing_or_zero_hard_limit_is_rejected() {
        assert!(billing_metrics(&json!({}), None).is_none());
        assert!(billing_metrics(&json!({ "hard_limit_usd": 0 }), None).is_none());
        // A zero custom limit falls through to the system limit.
        let sub = json!({ "hard_limit_usd": 0, "system_hard_limit_usd": 20.0 });
        let metrics = billing_metrics(&sub, None).expect("system limit");
        assert_eq!(
            (metrics[0].label.as_str(), metrics[0].value.as_deref()),
            ("Limit", Some("$20.00"))
        );
    }

    #[test]
    fn missing_usage_still_shows_the_limit() {
        let sub = json!({ "hard_limit_usd": 10.0 });
        let metrics = billing_metrics(&sub, None).expect("limit only");
        assert_eq!(metrics.len(), 1);
        assert_eq!(
            (metrics[0].label.as_str(), metrics[0].value.as_deref()),
            ("Limit", Some("$10.00"))
        );
    }

    #[test]
    fn both_path_forms_are_tried_v1_first() {
        let pairs = billing_endpoints("https://api.example.com/");
        assert_eq!(
            pairs[0].0,
            "https://api.example.com/v1/dashboard/billing/subscription"
        );
        assert_eq!(pairs[0].1, "https://api.example.com/v1/dashboard/billing/usage");
        assert_eq!(
            pairs[1].0,
            "https://api.example.com/dashboard/billing/subscription"
        );
        assert_eq!(pairs[1].1, "https://api.example.com/dashboard/billing/usage");
    }

    #[test]
    fn only_auth_failures_are_final() {
        assert!(is_final_rejection(401));
        assert!(is_final_rejection(403));
        assert!(!is_final_rejection(404));
        assert!(!is_final_rejection(500));
    }
}
