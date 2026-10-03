//! APIGOTO (www.apigoto.com) subscription quota — the official read-only
//! `GET /v1/usage` endpoint (built exactly for client balance displays
//! like CC Switch). Bearer sk- key, no session, no rotation. Facts and
//! schema verified live 2026-10-03; see
//! temp/handoff/20261003-153737-apigoto-provider-impl.md.
//!
//! Semantics that shape this parser:
//! - `is_active` reflects the KEY+user status, NOT the subscription. After
//!   the subscription lapses the endpoint still returns 200 with
//!   `subscription` gone and `mode: "payg"` — a naive parser would render
//!   a fake 0% healthy card. We detect that and show an expired state.
//! - The rate window is ANCHOR-rolling; the API deliberately exposes no
//!   reset instant (their own docs say it can't be derived from a clock).
//!   So the progress row carries no window countdown. The reset we DO
//!   surface is `subscription.expires_at` — the moment this metric stops
//!   existing entirely (the free-ride countdown).
//! - Error mapping trusts the numeric `code` field (a documented stability
//!   contract), never the human message.
//! - Nothing is hardcoded: limit/window/plan/expiry all come from the
//!   response (plan upgrades change all three server-side).

use super::{http, json_body, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "apigoto";
const NAME: &str = "APIGOTO";
const MAX_BODY_BYTES: usize = 64 * 1024;
const USAGE_URL: &str = "https://api.apigoto.com/v1/usage";

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["APIGOTO_API_KEY"]).map(|_| "Pane APIGOTO API key".into())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(ID, &["APIGOTO_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste an APIGOTO API key (www.apigoto.com console) in Settings.",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = http()
        .get(USAGE_URL)
        .bearer_auth(key.trim())
        .send()
        .await
        .map_err(|e| format!("APIGOTO request: {e}"))?;
    if resp.status().as_u16() == 401 {
        // 40101 = key not found/deleted; 40001 = malformed auth header.
        // Either way it is a credential problem, decided by status + code,
        // never by the message text.
        return Err("APIGOTO API key was rejected — check it in Settings".into());
    }
    if !resp.status().is_success() {
        // 503 = config store briefly unavailable: transient, the next
        // refresh cycle retries and the stale-snapshot layer covers it.
        return Err(format!("APIGOTO endpoint: HTTP {}", resp.status()));
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "APIGOTO").await?;
    parse_snapshot(&doc)
}

/// Numbers arrive mixed: `used/limit/points` as JSON numbers, balance
/// fields as decimal strings, `remaining` as f64. One tolerant reader.
fn num(v: Option<&Value>) -> Option<f64> {
    v?.as_f64().or_else(|| v?.as_str()?.trim().parse().ok())
}

fn parse_snapshot(doc: &Value) -> Result<Snapshot, String> {
    if doc.get("is_active").and_then(Value::as_bool) == Some(false) {
        return Err("APIGOTO key or account is disabled".into());
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    let sub = doc.get("subscription").filter(|s| !s.is_null());
    let live = sub.is_some_and(|s| {
        s.get("expires_at")
            .and_then(Value::as_str)
            .and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok())
            .is_some_and(|d| d.timestamp_millis() > now_ms)
    });

    if !live {
        // No subscription object, or it already expired: never render a
        // fresh 0% card off the payg fallback numbers. Diagnostics only.
        let mode = doc.get("mode").and_then(Value::as_str).unwrap_or("payg");
        let points = num(doc.get("points")).unwrap_or(0.0);
        let balance = doc
            .get("balance")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty() && num(doc.get("balance")).unwrap_or(0.0) != 0.0);
        let mut rows = vec![Metric::text(
            "Subscription",
            if sub.is_some() { "expired".into() } else { format!("none ({mode})") },
        )];
        if let Some(b) = balance {
            rows.push(Metric::text("Balance", format!("¥{b}")));
        } else if points > 0.0 {
            rows.push(Metric::text("Points", format!("{points:.0}")));
        }
        return Ok(Snapshot::ok(ID, NAME, None, rows));
    }

    let sub = sub.expect("checked live");
    let used = num(sub.get("used")).unwrap_or(0.0);
    let limit = num(sub.get("limit")).unwrap_or(0.0);
    let window_sec = num(sub.get("window_sec")).unwrap_or(0.0);
    let plan = sub
        .get("plan_name")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let expiry_ms = sub
        .get("expires_at")
        .and_then(Value::as_str)
        .and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok())
        .map(|d| d.timestamp_millis());

    let pct = if limit > 0.0 { (used / limit * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    let mut detail = String::new();
    if let Some(rem) = num(doc.get("remaining")).filter(|r| *r > 0.0) {
        let unit = doc.get("unit").and_then(Value::as_str).unwrap_or("USD");
        detail = format!("${rem:.2} {unit} left");
    }
    if window_sec > 0.0 {
        let days = (window_sec / 86_400.0).round() as i64;
        if !detail.is_empty() {
            detail.push_str(" · ");
        }
        detail.push_str(&format!("{}d window", days));
    }
    if detail.is_empty() {
        detail = format!("{used:.0} / {limit:.0} credits");
    }
    let metrics = vec![
        // reset = subscription expiry (the metric's own death), NOT a
        // window reset — the anchor-rolling window intentionally exposes
        // no reset instant (official docs).
        Metric::progress("Credits", pct, Some(detail)).with_reset(expiry_ms, None),
        Metric::text("Plan", plan.clone().unwrap_or_else(|| "subscription".into())),
    ];
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim live response captured 2026-10-03 (numbers + decimal
    /// strings mixed, subscription present).
    const REAL_200: &str = r#"{"is_active":true,"isValid":true,"mode":"subscription","remaining":3.7233503063498166,"unit":"USD","subscription":{"plan_name":"RouterCode Free","policy_name":"APIGOTO FREE","used":0,"limit":25000000,"window_sec":604800,"unit":"rpm_credits","expires_at":"2999-11-03T07:19:25Z"},"points":0,"balance":"0.00000000","balance_usd":"0.00000000"}"#;

    #[test]
    fn parses_the_live_subscription_shape() {
        let doc: Value = serde_json::from_str(REAL_200).unwrap();
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan.as_deref(), Some("RouterCode Free"));
        assert_eq!(snap.metrics.len(), 2);
        let credits = &snap.metrics[0];
        assert_eq!(credits.label, "Credits");
        assert_eq!(credits.used_percent, Some(0.0));
        assert!(credits.detail.as_deref().unwrap().contains("$3.72"));
        assert!(credits.detail.as_deref().unwrap().contains("7d window"));
        // Reset = subscription expiry (far future here), not a window roll.
        assert!(credits.resets_at.is_some());
        assert_eq!(snap.metrics[1].label, "Plan");
        assert_eq!(snap.metrics[1].value.as_deref(), Some("RouterCode Free"));
    }

    #[test]
    fn used_ratio_and_dynamic_window_text() {
        let doc = serde_json::json!({
            "is_active": true, "mode": "subscription",
            "remaining": 1.0, "unit": "USD",
            "subscription": {"plan_name": "Pro", "used": 6000000.0, "limit": "25000000",
                             "window_sec": "2592000", "expires_at": "2999-01-01T00:00:00Z"}
        });
        let snap = parse_snapshot(&doc).expect("parse");
        let credits = &snap.metrics[0];
        assert!((credits.used_percent.unwrap() - 24.0).abs() < 0.001);
        assert!(credits.detail.as_deref().unwrap().contains("30d window"));
    }

    #[test]
    fn missing_subscription_is_not_a_fresh_zero_card() {
        // mode=payg, no subscription object: diagnostics rows only.
        let doc = serde_json::json!({
            "is_active": true, "isValid": true, "mode": "payg",
            "remaining": 0.0, "unit": "USD", "points": 0,
            "balance": "0.00000000", "balance_usd": "0.00000000"
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "Subscription");
        assert!(snap.metrics[0].value.as_deref().unwrap().contains("none (payg)"));
    }

    #[test]
    fn expired_subscription_shows_expired_not_zero_percent() {
        let doc = serde_json::json!({
            "is_active": true, "mode": "payg",
            "subscription": {"plan_name": "RouterCode Free", "used": 0, "limit": 25000000,
                             "window_sec": 604800, "expires_at": "2020-01-01T00:00:00Z"},
            "points": 500000, "balance": "0.50000"
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert!(snap.metrics.iter().all(|m| m.kind != "progress"));
        assert!(snap.metrics[0].value.as_deref().unwrap().contains("expired"));
        // Balance diagnostic rides along when non-zero.
        assert!(snap.metrics.iter().any(|m| m.value.as_deref() == Some("¥0.50000")));
    }

    #[test]
    fn disabled_key_is_an_error() {
        let doc = serde_json::json!({"is_active": false, "isValid": false});
        assert!(parse_snapshot(&doc).is_err());
    }
}
