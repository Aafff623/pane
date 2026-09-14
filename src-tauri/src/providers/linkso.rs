//! Linkso GLM relay quota — coding.link-so.cn and any other host running
//! the same self-hosted Go panel. The panel exposes login-free client
//! endpoints under /usage/api/*, authenticated with the buyer's `sk-` key
//! in an `x-api-key` header (Bearer is untested there, unlike the chat
//! routes). Its OpenAI dashboard-billing and Z.ai monitor paths are 404 —
//! this adapter must not be confused with Custom Balance's relays.
//!
//! A carpool key draws from ONE upstream 5-hour GLM window shared by every
//! buyer while carrying its own per-window points soft cap that may be
//! slightly exceeded. The card therefore shows three rows:
//!   Points        — this key's soft cap (pointsUsed / pointsAllocated)
//!   Shared window — the upstream 5h window everyone draws from
//!   Web Searches  — the monthly search-tool quota, same inverted shape as
//!                   Z.ai's TIME_LIMIT (currentValue = used, usage = cap)
//! In the wild `tokensLimit.usage`/`currentValue` are always 0 — its
//! `percentage` is the only real signal. No preset host: base URL + key
//! live together in %APPDATA%\Pane\linkso.json.

use super::{http, json_body, stored_api_key, stored_base_url, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "linkso";
const NAME: &str = "GLM V1 Pro";
const MAX_BODY_BYTES: usize = 64 * 1024;
const SESSION_MS: i64 = 5 * 3_600_000;
/// The search quota resets on a ~30-day rolling anchor, not a calendar
/// month (observed next reset 2026-09-26 10:20, no month boundary nearby).
const SEARCH_PERIOD_MS: i64 = 30 * 86_400_000;

/// User-supplied relay URLs follow Custom Balance's rule (HTTPS for public
/// hosts, plain HTTP only for private/loopback IPs) — one shared check, with
/// Linkso named in any rejection message.
fn validate_base_url(raw: &str) -> Result<(), String> {
    super::relaybalance::validate_base_url_for("Linkso", raw)
}

fn quota_url(base: &str) -> String {
    format!("{}/usage/api/quota", base.trim().trim_end_matches('/'))
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a pasted key + base URL without saving either (Customize
/// "Test").
pub async fn snapshot_with_key(key: &str, base_url: &str) -> Snapshot {
    snapshot_with_key_at(key, base_url, ID, NAME).await
}

/// The fetch behind every named account card, preserving its identity.
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
    let (Some(base), Some(key)) = (stored_base_url(ID), stored_api_key(ID, &[])) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste the relay panel's base URL and API key in Settings (gear icon).",
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
    let resp = http()
        .get(quota_url(base))
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|e| format!("quota request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err("API key was rejected — check it in Settings".into());
    }
    if !resp.status().is_success() {
        return Err(format!("quota endpoint: HTTP {}", resp.status()));
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "quota").await?;
    parse_quota(&doc, card_id, card_name, base)
}

/// The quota response is one documented shape: `{"ok":true,"glm":{...}}`.
/// A second `kimi` lane exists but is null until the seller opens it.
fn parse_quota(
    doc: &Value,
    card_id: &str,
    card_name: &str,
    base: &str,
) -> Result<Snapshot, String> {
    let glm = doc
        .get("glm")
        .filter(|v| v.is_object())
        .ok_or_else(|| "unexpected quota response shape (no glm object)".to_string())?;
    let metrics = quota_metrics(glm);
    if metrics.is_empty() {
        // A glm object with no recognizable rows means the panel renamed its
        // fields — an empty "ok" card would hide that breakage.
        return Err("quota response carried no usable rows".into());
    }
    let mut snap = Snapshot::ok(card_id, card_name, plan_name(glm), metrics);
    snap.dashboard_url = Some(format!(
        "{}/admin/client-login",
        base.trim().trim_end_matches('/')
    ));
    Ok(snap)
}

fn quota_metrics(glm: &Value) -> Vec<Metric> {
    let mut metrics = Vec::new();

    // This key's own soft cap. It may be slightly exceeded (6096.56 of
    // 6000 was observed) — clamp the bar, keep the real numbers in detail.
    if let Some(alloc) = glm.get("allocation").filter(|v| v.is_object()) {
        if let Some(cap) = num(alloc, "pointsAllocated").filter(|c| *c > 0.0) {
            let used = num(alloc, "pointsUsed").unwrap_or(0.0).max(0.0);
            metrics.push(
                Metric::progress(
                    "Points",
                    (used / cap * 100.0).clamp(0.0, 100.0),
                    Some(format!("{used:.0} of {cap:.0} points")),
                )
                // Rolling window: first consumption + 5h, not a fixed clock
                // time — an idle key reports no end at all.
                .with_reset(ms(alloc, "windowEndMs"), Some(SESSION_MS)),
            );
        }
    }

    // The shared upstream 5h window. Full = every model 429s for every
    // buyer, so a maxed row here is the real "wait for reset" signal.
    if let Some(tokens) = glm.get("tokensLimit").filter(|v| v.is_object()) {
        if let Some(pct) = num(tokens, "percentage") {
            metrics.push(
                Metric::progress("Shared window", pct.clamp(0.0, 100.0), None)
                    .with_reset(ms(tokens, "nextResetTime"), Some(SESSION_MS)),
            );
        }
    }

    // Monthly search-tool quota — Z.ai TIME_LIMIT semantics: currentValue
    // is used, usage is the cap.
    if let Some(search) = glm.get("timeLimit").filter(|v| v.is_object()) {
        let used = num(search, "currentValue").unwrap_or(0.0).max(0.0);
        let cap = num(search, "usage").unwrap_or(0.0).max(0.0);
        if cap > 0.0 {
            metrics.push(
                Metric::progress(
                    "Web Searches",
                    (used / cap * 100.0).clamp(0.0, 100.0),
                    Some(format!("{used:.0} of {cap:.0} searches")),
                )
                .with_reset(ms(search, "nextResetTime"), Some(SEARCH_PERIOD_MS)),
            );
        }
    }

    metrics
}

fn plan_name(glm: &Value) -> Option<String> {
    match glm.get("level").and_then(Value::as_str) {
        Some("pro") => Some("GLM Coding Pro".into()),
        Some("max") => Some("GLM Coding Max".into()),
        Some("lite") => Some("GLM Coding Lite".into()),
        Some(other) if !other.trim().is_empty() => Some(format!("GLM Coding {other}")),
        _ => None,
    }
}

fn num(node: &Value, key: &str) -> Option<f64> {
    node.get(key).and_then(Value::as_f64)
}

/// Epoch-ms reset that only counts when positive — 0/absent means "not
/// started" and must render as such, not as a bogus countdown.
fn ms(node: &Value, key: &str) -> Option<i64> {
    node.get(key).and_then(Value::as_i64).filter(|v| *v > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn labels(metrics: &[Metric]) -> Vec<&str> {
        metrics.iter().map(|m| m.label.as_str()).collect()
    }

    #[test]
    fn normal_window_shows_points_shared_and_searches() {
        let doc = json!({"ok": true, "kimi": null, "glm": {
            "level": "pro",
            "timeLimit": {"usage": 1000, "currentValue": 70, "remaining": 930,
                          "percentage": 7, "nextResetTime": 1790389228998i64},
            "tokensLimit": {"number": 5, "unit": 3, "percentage": 40,
                            "nextResetTime": 1789380885780i64},
            "allocation": {"share": 50, "pointsAllocated": 6000, "pointsUsed": 1200,
                           "windowStartMs": 1789362885780i64, "windowEndMs": 1789380885780i64,
                           "usageFill": 0.2, "peak": false}
        }});
        let snap = parse_quota(&doc, "linkso", "Linkso", "https://coding.link-so.cn")
            .expect("parses");
        assert_eq!(snap.plan.as_deref(), Some("GLM Coding Pro"));
        assert_eq!(
            labels(&snap.metrics),
            ["Points", "Shared window", "Web Searches"]
        );
        assert_eq!(snap.metrics[0].used_percent, Some(20.0));
        assert_eq!(snap.metrics[0].detail.as_deref(), Some("1200 of 6000 points"));
        assert_eq!(snap.metrics[0].resets_at, Some(1789380885780));
        assert_eq!(snap.metrics[0].period_ms, Some(SESSION_MS));
        assert_eq!(snap.metrics[1].used_percent, Some(40.0));
        assert!((snap.metrics[2].used_percent.unwrap() - 7.0).abs() < 1e-9);
        assert_eq!(snap.metrics[2].period_ms, Some(SEARCH_PERIOD_MS));
        assert_eq!(
            snap.dashboard_url.as_deref(),
            Some("https://coding.link-so.cn/admin/client-login")
        );
    }

    #[test]
    fn saturated_key_clamps_overdraft_and_ignores_zero_counters() {
        let doc = json!({"ok": true, "glm": {
            "level": "pro",
            "tokensLimit": {"number": 5, "unit": 3, "percentage": 100,
                            "usage": 0, "currentValue": 0,
                            "nextResetTime": 1789380885780i64},
            "allocation": {"share": 50, "pointsAllocated": 6000, "pointsUsed": 6096.56,
                           "windowStartMs": 1789362885780i64, "windowEndMs": 1789380885780i64,
                           "officialPercentage": 100, "usageFill": 1, "peak": true}
        }});
        let snap = parse_quota(&doc, "linkso", "Linkso", "https://coding.link-so.cn")
            .expect("parses");
        assert_eq!(labels(&snap.metrics), ["Points", "Shared window"]);
        assert_eq!(snap.metrics[0].used_percent, Some(100.0));
        assert_eq!(snap.metrics[0].detail.as_deref(), Some("6097 of 6000 points"));
        assert_eq!(snap.metrics[1].used_percent, Some(100.0));
    }

    #[test]
    fn idle_key_renders_unstarted_rows_without_resets() {
        let doc = json!({"ok": true, "glm": {
            "level": "pro",
            "tokensLimit": {"number": 5, "unit": 3, "percentage": 0},
            "allocation": {"pointsAllocated": 6000, "pointsUsed": 0}
        }});
        let snap = parse_quota(&doc, "linkso", "Linkso", "https://x.example").expect("parses");
        assert_eq!(labels(&snap.metrics), ["Points", "Shared window"]);
        assert_eq!(snap.metrics[0].used_percent, Some(0.0));
        assert_eq!(snap.metrics[0].resets_at, None);
        assert_eq!(snap.metrics[0].period_ms, Some(SESSION_MS));
        assert_eq!(snap.metrics[1].resets_at, None);
    }

    #[test]
    fn unknown_level_passes_through_and_missing_glm_is_an_error() {
        let doc = json!({"ok": true, "glm": {"level": "mega", "tokensLimit": {"percentage": 5}}});
        let snap = parse_quota(&doc, "linkso", "Linkso", "https://x.example").expect("parses");
        assert_eq!(snap.plan.as_deref(), Some("GLM Coding mega"));
        assert!(
            parse_quota(&json!({"ok": true, "glm": null}), "linkso", "Linkso", "https://x.example")
                .is_err()
        );
        assert!(parse_quota(&json!({"ok": false}), "linkso", "Linkso", "https://x.example").is_err());
    }

    #[test]
    fn glm_without_any_usable_row_is_an_error_not_an_empty_card() {
        // Shape drift (renamed fields) must surface as an error, not as an
        // "ok" card with zero rows.
        let doc = json!({"ok": true, "glm": {"level": "pro", "somethingNew": {}}});
        assert!(parse_quota(&doc, "linkso", "Linkso", "https://x.example").is_err());
    }
}
