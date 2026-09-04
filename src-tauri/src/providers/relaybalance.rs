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
use reqwest::Url;
use serde_json::Value;
use std::net::{Ipv4Addr, Ipv6Addr};

const ID: &str = "relaybalance";
const NAME: &str = "Custom Balance";
const MAX_BODY_BYTES: usize = 64 * 1024;

/// The `total_usage` field's unit. Relays follow the classic OpenAI
/// dashboard-billing convention: usage is reported in **cents**.
const USAGE_TO_USD: f64 = 0.01;

fn is_local_http_ip(hostname: &str) -> bool {
    let host = hostname
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(hostname);

    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return ip.is_private() || ip.is_loopback() || ip.is_link_local();
    }
    if let Ok(ip) = host.parse::<Ipv6Addr>() {
        return ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local();
    }
    false
}

/// Validate a user-supplied relay URL before any API key can be attached to
/// a request. HTTPS is allowed for public hosts; plaintext HTTP is limited to
/// private, loopback, and link-local IP literals so a key cannot be sent over
/// the network in clear text by accident.
pub fn validate_base_url(raw: &str) -> Result<(), String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("a base URL is required for Custom Balance".into());
    }
    let url = Url::parse(raw).map_err(|_| "invalid Custom Balance base URL".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("Custom Balance URL must use http:// or https://".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Custom Balance URL must not include a username or password".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("Custom Balance URL must not include a query or fragment".into());
    }
    let hostname = url
        .host_str()
        .ok_or_else(|| "Custom Balance URL is missing a host".to_string())?;
    if hostname.is_empty() {
        return Err("Custom Balance URL is missing a host".into());
    }
    if url.scheme() == "http" && !is_local_http_ip(hostname) {
        return Err(
            "plain HTTP is only allowed for private, loopback, or link-local IP addresses"
                .into(),
        );
    }
    Ok(())
}

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
    snapshot_with_key_at(key, base_url, ID, NAME).await
}

/// Fetches a key while preserving the identity of the account card that owns
/// both the key and its relay base URL. The family wrapper above keeps the
/// original public behavior.
pub async fn snapshot_with_key_at(
    key: &str,
    base_url: &str,
    card_id: &str,
    card_name: &str,
) -> Snapshot {
    match fetch_with_key(key, base_url, card_id, card_name).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(card_id, card_name, e),
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
    fetch_with_key(&key, &base, ID, NAME).await
}

async fn fetch_with_key(
    key: &str,
    base: &str,
    card_id: &str,
    card_name: &str,
) -> Result<Snapshot, String> {
    validate_base_url(base)?;
    let mut last_error = String::from("billing endpoint unreachable");
    for (sub_url, usage_url) in billing_endpoints(base) {
        let sub_req = http().get(&sub_url).bearer_auth(key).send();
        let usage_req = http().get(&usage_url).bearer_auth(key).send();
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
                card_id,
                card_name,
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
                card_id,
                card_name,
                "no hard_limit_usd in the subscription response".into(),
            ));
        };
        return Ok(Snapshot::ok(
            card_id,
            card_name,
            Some("Pay as you go".into()),
            metrics,
        ));
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
fn snapshot_error_for_card(message: &str, card_id: &str, card_name: &str) -> Snapshot {
    Snapshot::error(card_id, card_name, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;

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

    #[test]
    fn base_url_requires_https_or_local_http() {
        assert!(validate_base_url("https://relay.example.com").is_ok());
        assert!(validate_base_url("http://127.0.0.1:8080").is_ok());
        assert!(validate_base_url("http://10.0.0.4:8080").is_ok());
        assert!(validate_base_url("http://example.com").is_err());
        assert!(validate_base_url("http://localhost:8080").is_err());
        assert!(validate_base_url("ftp://relay.example.com").is_err());
    }

    #[test]
    fn base_url_rejects_credentials_query_and_fragment() {
        assert!(validate_base_url("https://user:pass@relay.example.com").is_err());
        assert!(validate_base_url("https://relay.example.com/?token=1").is_err());
        assert!(validate_base_url("https://relay.example.com/#secret").is_err());
    }

    #[test]
    fn named_error_keeps_account_identity() {
        let snap = snapshot_error_for_card("network", "relaybalance@2", "Custom Balance — Work");
        assert_eq!(snap.id, "relaybalance@2");
        assert_eq!(snap.name, "Custom Balance — Work");
    }

    #[test]
    fn named_cards_fetch_independently_from_one_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock relay");
        let addr = listener.local_addr().expect("mock relay address");
        let server = std::thread::spawn(move || {
            for _ in 0..4 {
                let (mut stream, _) = listener.accept().expect("accept mock request");
                let mut request = Vec::new();
                loop {
                    let mut chunk = [0_u8; 1024];
                    let read = stream.read(&mut chunk).expect("read mock request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..read]);
                    if request.windows(4).any(|w| w == b"\r\n\r\n") || request.len() > 8192 {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&request);
                let is_first_key = request.contains("key-one");
                let body = if request.contains("/usage") {
                    if is_first_key {
                        r#"{"total_usage":100}"#
                    } else {
                        r#"{"total_usage":1000}"#
                    }
                } else if is_first_key {
                    r#"{"hard_limit_usd":10.0}"#
                } else {
                    r#"{"hard_limit_usd":20.0}"#
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write mock response");
            }
        });

        let base = format!("http://{}", addr);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build test runtime");
        let (work, personal) = runtime.block_on(async {
            tokio::join!(
                snapshot_with_key_at(
                    "key-one",
                    &base,
                    "relaybalance@1",
                    "Custom Balance — Work"
                ),
                snapshot_with_key_at(
                    "key-two",
                    &base,
                    "relaybalance@2",
                    "Custom Balance — Personal"
                )
            )
        });
        server.join().expect("mock relay server");

        assert_eq!(work.id, "relaybalance@1");
        assert_eq!(work.name, "Custom Balance — Work");
        assert_eq!(
            work.metrics
                .iter()
                .find(|metric| metric.label == "Balance")
                .and_then(|metric| metric.value.as_deref()),
            Some("$9.00")
        );
        assert_eq!(personal.id, "relaybalance@2");
        assert_eq!(personal.name, "Custom Balance — Personal");
        assert_eq!(
            personal
                .metrics
                .iter()
                .find(|metric| metric.label == "Balance")
                .and_then(|metric| metric.value.as_deref()),
            Some("$10.00")
        );
    }
}
