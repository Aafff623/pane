//! Generic API-key balance adapter.
//!
//! The vendor list is a fixed, audited spec table: fixed HTTPS origin, GET
//! path, auth placement and JSON pointers. No user-controlled URL, no redirect
//! following, no script/JSONPath, and no secret in errors. `providers/mod.rs`
//! owns the request/body guards; this module only shapes the result.

use super::{json_body, stored_api_key, Metric, Snapshot};
use crate::usage_units::UsageUnit;
use serde_json::Value;
use std::time::Duration;

const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy)]
enum Auth {
    Bearer,
    Header(&'static str, &'static str),
    Query(&'static str),
}

#[derive(Clone, Copy)]
struct ApiKeySpec {
    family: &'static str,
    display: &'static str,
    host: &'static str,
    path: &'static str,
    auth: Auth,
    unit: UsageUnit,
    used: Option<&'static str>,
    limit: Option<&'static str>,
    remaining: Option<&'static str>,
    label: &'static str,
    unit_text: &'static str,
    auth_kind: &'static str,
    probe_kind: &'static str,
    quota_scope: &'static str,
    reset_semantics: &'static str,
    pace_capability: bool,
}

const SPECS: &[ApiKeySpec] = &[
    ApiKeySpec {
        family: "deepl",
        display: "DeepL",
        host: "https://api-free.deepl.com",
        path: "/v2/usage",
        auth: Auth::Header("Authorization", "DeepL-Auth-Key "),
        unit: UsageUnit::Characters,
        used: Some("/character_count"),
        limit: Some("/character_limit"),
        remaining: None,
        label: "Usage",
        unit_text: "characters",
        auth_kind: "api_key",
        probe_kind: "active_get",
        quota_scope: "account",
        reset_semantics: "monthly_inferred",
        pace_capability: false,
    },
    ApiKeySpec {
        family: "serpapi",
        display: "SerpApi",
        host: "https://serpapi.com",
        path: "/account",
        auth: Auth::Query("api_key"),
        unit: UsageUnit::Requests,
        used: None,
        limit: Some("/searches_per_month"),
        remaining: Some("/plan_searches_left"),
        label: "Requests this month",
        unit_text: "requests",
        auth_kind: "api_key",
        probe_kind: "active_get",
        quota_scope: "account",
        reset_semantics: "monthly_inferred",
        pace_capability: false,
    },
    ApiKeySpec {
        family: "stability",
        display: "Stability AI",
        host: "https://api.stability.ai",
        path: "/v1/user/balance",
        auth: Auth::Bearer,
        unit: UsageUnit::Credits,
        used: None,
        limit: None,
        remaining: Some("/credits"),
        label: "Balance",
        unit_text: "credits",
        auth_kind: "api_key",
        probe_kind: "active_get",
        quota_scope: "account",
        reset_semantics: "none",
        pace_capability: false,
    },
];

pub const FAMILIES: &[&str] = &["deepl", "serpapi", "stability"];

fn spec(family: &str) -> Option<&'static ApiKeySpec> {
    SPECS.iter().find(|spec| spec.family == family)
}

pub fn supports(family: &str) -> bool {
    spec(family).is_some()
}

#[derive(serde::Serialize)]
pub struct Descriptor {
    pub family: &'static str,
    pub auth_kind: &'static str,
    pub probe_kind: &'static str,
    pub quota_scope: &'static str,
    pub reset_semantics: &'static str,
    pub pace_capability: bool,
}

pub fn descriptor(family: &str) -> Option<Descriptor> {
    spec(family).map(|spec| Descriptor {
        family: spec.family,
        auth_kind: spec.auth_kind,
        probe_kind: spec.probe_kind,
        quota_scope: spec.quota_scope,
        reset_semantics: spec.reset_semantics,
        pace_capability: spec.pace_capability,
    })
}

pub fn snapshot_owned(family: &str) -> impl std::future::Future<Output = Snapshot> + Send + 'static {
    let family = family.to_string();
    async move { snapshot(&family).await }
}

fn number_at(doc: &Value, pointer: Option<&str>) -> Option<f64> {
    let value = doc.pointer(pointer?)?;
    let number = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(text) => text.trim().parse::<f64>().ok()?,
        _ => return None,
    };
    (number.is_finite() && number >= 0.0).then_some(number)
}

fn metrics_from(spec: &ApiKeySpec, doc: &Value) -> Result<Vec<Metric>, String> {
    let used = number_at(doc, spec.used);
    let limit = number_at(doc, spec.limit);
    let remaining = number_at(doc, spec.remaining);
    if let (Some(limit), Some(remaining)) = (limit, remaining) {
        let used = used.unwrap_or((limit - remaining).max(0.0));
        let percent = if limit > 0.0 { (used / limit) * 100.0 } else { 100.0 };
        return Ok(vec![Metric::progress(
            spec.label,
            percent,
            Some(format!("{} of {} {} left", remaining, limit, spec.unit_text)),
        )
        .with_usage(spec.unit, used, Some(limit))]);
    }
    if let (Some(used), Some(limit)) = (used, limit) {
        let percent = if limit > 0.0 { (used / limit) * 100.0 } else { 100.0 };
        return Ok(vec![Metric::progress(
            spec.label,
            percent,
            Some(format!("{} of {} {} used", used, limit, spec.unit_text)),
        )
        .with_usage(spec.unit, used, Some(limit))]);
    }
    if let Some(remaining) = remaining {
        return Ok(vec![Metric::text(
            spec.label,
            format!("{} {}", remaining, spec.unit_text),
        )]);
    }
    Err(format!("{} response has no readable quota fields", spec.display))
}

pub async fn snapshot(family: &str) -> Snapshot {
    let Some(spec) = spec(family) else {
        return Snapshot::error(family, family, "unknown API-key adapter".into());
    };
    let Some(key) = stored_api_key(family, &[]) else {
        return Snapshot::no_credentials(
            family,
            spec.display,
            "Paste this provider's API key in Settings.",
        );
    };
    snapshot_with_key_as(family, &key, family, spec.display).await
}

pub async fn snapshot_with_key(family: &str, key: &str) -> Snapshot {
    let Some(spec) = spec(family) else {
        return Snapshot::error(family, family, "unknown API-key adapter".into());
    };
    snapshot_with_key_as(family, key, family, spec.display).await
}

pub async fn snapshot_with_key_as(
    family: &str,
    key: &str,
    card_id: &str,
    card_name: &str,
) -> Snapshot {
    let Some(spec) = spec(family) else {
        return Snapshot::error(card_id, card_name, "unknown API-key adapter".into());
    };
    let mut request = super::http_no_redirect()
        .get(format!("{}{}", spec.host, spec.path))
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(10));
    request = match spec.auth {
        Auth::Bearer => request.bearer_auth(key),
        Auth::Header(name, prefix) => request.header(name, format!("{prefix}{key}")),
        Auth::Query(name) => request.query(&[(name, key)]),
    };
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => return Snapshot::error(card_id, card_name, format!("{} request failed: {error}", spec.display)),
    };
    let status = response.status().as_u16();
    if status == 401 {
        return Snapshot::error(card_id, card_name, format!("{} rejected this key", spec.display));
    }
    if status == 402 {
        return Snapshot::error(card_id, card_name, format!("{} says this account needs payment", spec.display));
    }
    if status == 429 {
        return Snapshot::error(card_id, card_name, format!("{} rate-limited the quota check", spec.display));
    }
    if !(200..300).contains(&status) {
        return Snapshot::error(card_id, card_name, format!("{} quota endpoint: HTTP {status}", spec.display));
    }
    let doc = match json_body(response, MAX_BODY_BYTES, "quota").await {
        Ok(doc) => doc,
        Err(error) => return Snapshot::error(card_id, card_name, error),
    };
    match metrics_from(spec, &doc) {
        Ok(metrics) => Snapshot::ok(card_id, card_name, None, metrics),
        Err(error) => Snapshot::error(card_id, card_name, error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deepl_reads_character_count_and_limit() {
        let metrics = metrics_from(spec("deepl").unwrap(), &json!({
            "character_count": "12345",
            "character_limit": 500000
        })).unwrap();
        assert_eq!(metrics[0].label, "Usage");
        assert_eq!(metrics[0].usage.as_ref().unwrap().unit, UsageUnit::Characters);
        assert_eq!(metrics[0].usage.as_ref().unwrap().used, 12345.0);
        assert_eq!(metrics[0].usage.as_ref().unwrap().total, Some(500000.0));
    }

    #[test]
    fn serpapi_derives_used_from_remaining() {
        let metrics = metrics_from(spec("serpapi").unwrap(), &json!({
            "searches_per_month": 100,
            "plan_searches_left": "25"
        })).unwrap();
        assert_eq!(metrics[0].label, "Requests this month");
        assert_eq!(metrics[0].usage.as_ref().unwrap().used, 75.0);
        assert_eq!(metrics[0].usage.as_ref().unwrap().unit, UsageUnit::Requests);
    }

    #[test]
    fn stability_balance_is_a_text_row_not_a_fake_percentage() {
        let metrics = metrics_from(spec("stability").unwrap(), &json!({"credits": 42.5})).unwrap();
        assert_eq!(metrics[0].label, "Balance");
        assert_eq!(metrics[0].value.as_deref(), Some("42.5 credits"));
        assert!(metrics[0].usage.is_none());
    }

    #[test]
    fn missing_or_malformed_fields_fail_honestly() {
        assert!(metrics_from(spec("deepl").unwrap(), &json!({})).is_err());
        assert!(metrics_from(spec("stability").unwrap(), &json!({"credits": "not-a-number"})).is_err());
        assert!(metrics_from(spec("serpapi").unwrap(), &json!({"plan_searches_left": -1})).is_err());
    }

    #[test]
    fn unknown_families_have_no_spec() {
        assert!(spec("unknown").is_none());
    }

    #[test]
    fn descriptors_name_the_fetch_contract_without_a_user_url() {
        let deepl = descriptor("deepl").unwrap();
        assert_eq!(deepl.auth_kind, "api_key");
        assert_eq!(deepl.probe_kind, "active_get");
        assert_eq!(deepl.reset_semantics, "monthly_inferred");
        assert!(!deepl.pace_capability);
        assert!(descriptor("stability").is_some());
    }
}
