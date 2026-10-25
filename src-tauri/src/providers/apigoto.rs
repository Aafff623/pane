//! APIGOTO (www.apigoto.com) subscription quota — the official read-only
//! `GET /v1/usage` endpoint (built exactly for client balance displays
//! like CC Switch). Bearer sk- key, no session, no rotation. Facts and
//! schema verified live 2026-10-03; rechecked 2026-10-23 through Pane's
//! stored account key (RouterCode Free, 7d window, expiry 2026-11-03).
//! See temp/handoff/20261003-153737-apigoto-provider-impl.md.
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

use super::{json_body, send_with_direct_fallback, stored_api_key, Metric, Snapshot};
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

/// The label of the fold-count row `fold_same_account_keys` appends.
const KEYS_ROW: &str = "API keys";

/// Content signature identifying the ACCOUNT a snapshot describes. The
/// vendor reports the account's subscription, not the key's, so every key
/// of one account returns the same `/v1/usage` payload and the rendered
/// rows match exactly (plan + expiry to the second + window + amounts).
/// `pace` is excluded — it projects from fetch time, not account state.
/// Cards that are not fresh `ok` readings return None: there is no account
/// fingerprint in an error, and a rejected key must keep its own card.
fn account_signature(snap: &Snapshot) -> Option<String> {
    if snap.status != "ok" {
        return None;
    }
    let mut metrics = snap.metrics.clone();
    for metric in &mut metrics {
        metric.pace = None;
        metric.available = None;
    }
    // The fold-count row itself is not account state — a restored
    // last-good card may carry it while its fresh twin does not.
    metrics.retain(|metric| metric.label != KEYS_ROW);
    let metrics = serde_json::to_string(&metrics).ok()?;
    Some(format!("{}|{metrics}", snap.plan.as_deref().unwrap_or("")))
}

/// One APIGOTO account may hold many API keys, and each key used to paint
/// its own card showing the same account numbers — a key is a credential,
/// not an identity. Snapshots are collected in account order (the default
/// account publishes under the bare family id), so the FIRST card per
/// signature wins and later same-account snapshots are dropped; the kept
/// card gains an "API keys: N · one account" row so the fold is visible.
/// Two genuinely different accounts collide only when their whole payloads
/// match — same plan bought in the same second with the same balance.
pub fn fold_same_account_keys(all: &mut Vec<Snapshot>) {
    let mut kept: Vec<(String, usize)> = Vec::new();
    let mut folded: Vec<usize> = vec![0; all.len()];
    let mut drop_at: Vec<bool> = vec![false; all.len()];
    for (index, snap) in all.iter().enumerate() {
        let is_family_card = snap.id == ID || snap.id.starts_with(&format!("{ID}@"));
        if !is_family_card {
            continue;
        }
        // A restored last-good snapshot may still carry the count row from
        // an earlier cycle; strip it so re-folding cannot stack rows.
        let Some(signature) = account_signature(snap) else {
            continue;
        };
        if let Some(&(_, kept_index)) = kept.iter().find(|(seen, _)| *seen == signature) {
            drop_at[index] = true;
            folded[kept_index] += 1;
        } else {
            kept.push((signature, index));
        }
    }
    for (index, snap) in all.iter_mut().enumerate() {
        let keys = folded[index];
        if keys == 0 {
            // Not a fold target in THIS call. The refresh pipeline invokes
            // fold twice; a card whose row an earlier pass already wrote
            // must keep it here, or the second pass would strip it.
            continue;
        }
        // Rewrite (never stack) the count row.
        snap.metrics.retain(|metric| metric.label != KEYS_ROW);
        snap.metrics
            .push(Metric::text(KEYS_ROW, format!("{} · one account", keys + 1)));
    }
    let mut index = 0;
    *all = all
        .drain(..)
        .filter(|_| {
            let drop = drop_at[index];
            index += 1;
            !drop
        })
        .collect();
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
    let resp = send_with_direct_fallback(|client| client.get(USAGE_URL).bearer_auth(key.trim()))
        .await
        .map_err(|e| format!("APIGOTO request: {e}"))?;
    if let Some(error) = status_error(resp.status().as_u16()) {
        return Err(error);
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "APIGOTO").await?;
    parse_snapshot(&doc)
}

/// 40101 = key not found/deleted; 40001 = malformed auth header. Either
/// way it is a credential problem, decided by status, never by body text.
/// 503 = config store briefly unavailable: transient, the next refresh
/// cycle retries and the stale-snapshot layer covers it.
fn status_error(status: u16) -> Option<String> {
    match status {
        401 => Some("APIGOTO API key was rejected — check it in Settings".into()),
        s if !(200..300).contains(&s) => Some(format!("APIGOTO endpoint: HTTP {s}")),
        _ => None,
    }
}

/// Numbers arrive mixed: `used/limit/points` as JSON numbers, balance
/// fields as decimal strings, `remaining` as f64. One tolerant reader.
fn num(v: Option<&Value>) -> Option<f64> {
    v?.as_f64().or_else(|| v?.as_str()?.trim().parse().ok())
}

/// Non-zero decimal string for display (the ¥ bucket keeps its original
/// precision; only presence and zero-ness are judged here).
fn decimal_string(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?.trim();
    if s.is_empty() || s.parse::<f64>().ok()? == 0.0 {
        return None;
    }
    Some(s.to_string())
}

fn parse_snapshot(doc: &Value) -> Result<Snapshot, String> {
    let active = doc.get("is_active").and_then(Value::as_bool);
    let valid = doc.get("isValid").and_then(Value::as_bool);
    if active == Some(false) || valid == Some(false) {
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
        let remaining = num(doc.get("remaining")).filter(|v| *v > 0.0);
        let balance = decimal_string(doc.get("balance"));
        let balance_usd = num(doc.get("balance_usd")).filter(|v| *v > 0.0);
        let mut rows = vec![Metric::text(
            "Subscription",
            if sub.is_some() {
                "expired".into()
            } else {
                format!("none ({mode})")
            },
        )];
        if let Some(r) = remaining {
            rows.push(Metric::text("Remaining", format!("${r:.2}")));
        } else if let Some(b) = balance {
            rows.push(Metric::text("Balance", format!("¥{b}")));
        } else if let Some(b) = balance_usd {
            rows.push(Metric::text("Balance", format!("${b:.2}")));
        } else if points > 0.0 {
            rows.push(Metric::text("Points", format!("{points:.0}")));
        }
        return Ok(Snapshot::ok(ID, NAME, None, rows));
    }

    let sub = sub.expect("checked live");
    let used = num(sub.get("used"))
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("missing valid subscription.used")?;
    let limit = num(sub.get("limit"))
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or("missing positive subscription.limit")?;
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

    let pct = if limit > 0.0 {
        (used / limit * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let mut detail = String::new();
    if let Some(rem) = num(doc.get("remaining")).filter(|r| *r > 0.0) {
        let unit = doc.get("unit").and_then(Value::as_str).unwrap_or("USD");
        detail = if unit.eq_ignore_ascii_case("USD") {
            format!("${rem:.2} left")
        } else {
            format!("{rem:.2} {unit} left")
        };
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
        Metric::text(
            "Plan",
            plan.clone().unwrap_or_else(|| "subscription".into()),
        ),
    ];
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_subscription_missing_usage_is_not_free_empty_allowance() {
        let mut doc: Value = serde_json::from_str(REAL_200).unwrap();
        doc["subscription"].as_object_mut().unwrap().remove("used");
        assert!(parse_snapshot(&doc).is_err());
    }

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
        assert!(snap.metrics[0]
            .value
            .as_deref()
            .unwrap()
            .contains("none (payg)"));
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
        assert!(snap.metrics[0]
            .value
            .as_deref()
            .unwrap()
            .contains("expired"));
        // Balance diagnostic rides along when non-zero.
        assert!(snap
            .metrics
            .iter()
            .any(|m| m.value.as_deref() == Some("¥0.50000")));
    }

    #[test]
    fn disabled_key_is_an_error() {
        let doc = serde_json::json!({"is_active": false, "isValid": false});
        assert!(parse_snapshot(&doc).is_err());
    }

    #[test]
    fn is_valid_false_is_an_error_even_when_is_active_is_true() {
        let doc = serde_json::json!({"is_active": true, "isValid": false, "mode": "payg"});
        assert!(parse_snapshot(&doc).is_err());
    }

    #[test]
    fn http_statuses_map_to_distinct_errors() {
        assert!(status_error(401).unwrap().contains("rejected"));
        assert_eq!(status_error(503).as_deref(), Some("APIGOTO endpoint: HTTP 503"));
        assert!(status_error(429).unwrap().contains("HTTP 429"));
        assert!(status_error(200).is_none());
        assert!(status_error(204).is_none());
    }

    #[test]
    fn payg_prefers_remaining_over_balance_buckets() {
        let doc = serde_json::json!({
            "is_active": true, "mode": "payg",
            "remaining": 5.8, "unit": "USD",
            "points": 1250000, "balance": "3.20000000", "balance_usd": "0.45000000"
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert!(snap.metrics.iter().any(|m| m.label == "Remaining"
            && m.value.as_deref() == Some("$5.80")));
        assert!(!snap.metrics.iter().any(|m| m.label == "Balance"));
    }

    #[test]
    fn payg_falls_back_to_usd_balance_when_yuan_bucket_is_empty() {
        // The official example ships balance=0 with a non-zero balance_usd.
        let doc: Value = serde_json::json!({
            "is_active": true, "mode": "payg", "remaining": 0,
            "points": 0, "balance": "0.00000000", "balance_usd": "3.20000000"
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert!(snap.metrics.iter().any(|m| m.label == "Balance"
            && m.value.as_deref() == Some("$3.20")));
    }

    /// One account's /v1/usage payload, re-branded as the Nth stored key
    /// would render it (only id and name differ).
    fn account_card(id: &str, name: &str, doc: &Value) -> Snapshot {
        let mut snap = parse_snapshot(doc).expect("parse");
        snap.id = id.into();
        snap.name = name.into();
        snap
    }

    #[test]
    fn keys_of_one_account_fold_into_a_single_card() {
        let doc: Value = serde_json::from_str(REAL_200).unwrap();
        let mut all = vec![
            account_card("apigoto", "APIGOTO", &doc),
            account_card("apigoto@24758b90", "APIGOTO — 2", &doc),
            account_card("apigoto@d4caeb8d", "APIGOTO — 3", &doc),
            Snapshot::ok("deepseek", "DeepSeek", None, vec![]),
        ];
        fold_same_account_keys(&mut all);
        let ids: Vec<&str> = all.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["apigoto", "deepseek"]);
        let card = &all[0];
        let keys_row = card.metrics.iter().find(|m| m.label == "API keys");
        assert_eq!(
            keys_row.and_then(|m| m.value.as_deref()),
            Some("3 · one account"),
        );
    }

    #[test]
    fn distinct_accounts_keep_their_own_cards() {
        let base: Value = serde_json::from_str(REAL_200).unwrap();
        let mut other = base.clone();
        // A different account: its subscription ends at another instant.
        other["subscription"]["expires_at"] =
            Value::String("2999-12-25T00:00:00Z".into());
        let mut all = vec![
            account_card("apigoto", "APIGOTO", &base),
            account_card("apigoto@24758b90", "APIGOTO — 2", &other),
        ];
        fold_same_account_keys(&mut all);
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|s| !s.metrics.iter().any(|m| m.label == "API keys")));
    }

    #[test]
    fn rejected_keys_are_never_folded_away() {
        let doc: Value = serde_json::from_str(REAL_200).unwrap();
        let mut all = vec![
            account_card("apigoto", "APIGOTO", &doc),
            Snapshot::error("apigoto@0f1d072d", "APIGOTO — 2", "key rejected".into()),
        ];
        fold_same_account_keys(&mut all);
        assert_eq!(all.len(), 2, "a failing key keeps its own visible card");
        assert!(!all[0].metrics.iter().any(|m| m.label == "API keys"));
    }

    #[test]
    fn folding_twice_keeps_the_count_row() {
        // The refresh pipeline calls fold twice (before the cache section
        // and after the stale-restore pass); the second pass must not
        // strip the row the first pass just wrote.
        let doc: Value = serde_json::from_str(REAL_200).unwrap();
        let mut all = vec![
            account_card("apigoto", "APIGOTO", &doc),
            account_card("apigoto@24758b90", "APIGOTO — 2", &doc),
        ];
        fold_same_account_keys(&mut all);
        fold_same_account_keys(&mut all);
        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].metrics.iter().find(|m| m.label == "API keys").unwrap().value.as_deref(),
            Some("2 · one account"),
        );
    }

    #[test]
    fn a_restored_card_with_a_stale_count_row_still_folds() {
        let doc: Value = serde_json::from_str(REAL_200).unwrap();
        let mut restored = account_card("apigoto", "APIGOTO", &doc);
        // Last cycle folded 4 keys onto this card; the cache restored it.
        restored
            .metrics
            .push(Metric::text("API keys", "4 · one account".into()));
        restored.stale = true;
        let mut all = vec![
            restored,
            account_card("apigoto@24758b90", "APIGOTO — 2", &doc),
        ];
        fold_same_account_keys(&mut all);
        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].metrics.iter().find(|m| m.label == "API keys").unwrap().value.as_deref(),
            Some("2 · one account"),
            "the stale count is recomputed, never stacked",
        );
    }
}
