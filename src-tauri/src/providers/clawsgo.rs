//! ClawsGO Science (app.clawsgo.ai) — cloud AI research agent, team
//! credits. RPC over HTTP: `POST https://api.clawsgo.ai/api/<method>`
//! with body `{"data":{...}}`, `Authorization: Bearer <token>` — the
//! token is the web app's `clawsgo_token` from browser localStorage
//! (no public API-key mechanism exists; pasting it into Settings is the
//! credential path, env `CLAWSGO_TOKEN` as fallback).
//!
//! Semantics (live-verified 2026-09-13): credits are milli-units
//! (3,000 credits = $1 per billing terms). The plan grants cycle credits
//! every 30 days (`monthlyGrantMilli`) that LAPSE at `planEndAt`; what's
//! left of the pool sits in `balanceMilli` (cycle remainder + permanent
//! credits, not split here). `getUsageStats` carries request/token
//! totals plus a per-model breakdown.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;
use std::time::Duration;

const ID: &str = "clawsgo";
const NAME: &str = "ClawsGO";

const API: &str = "https://api.clawsgo.ai/api";
const DAY_MS: i64 = 86_400_000;
const MAX_BODY: usize = 128 * 1024;

fn find_key() -> Option<String> {
    stored_api_key("clawsgo", &["CLAWSGO_TOKEN"])
}

pub fn local_credential_hint() -> Option<String> {
    find_key().map(|_| "ClawsGO token (Settings)".to_string())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a pasted token, without saving it (Customize "Test").
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = find_key() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste your ClawsGO token in Settings (gear icon) — DevTools console on app.clawsgo.ai: localStorage.getItem('clawsgo_token').",
        ));
    };
    fetch_with_key(&key).await
}

/// One RPC call: POST /api/<method> with {"data": params}.
async fn rpc(key: &str, method: &str, params: Value) -> Result<Value, String> {
    let resp = http()
        .post(format!("{API}/{method}"))
        .bearer_auth(key)
        .json(&serde_json::json!({ "data": params }))
        .timeout(Duration::from_secs(12))
        .send()
        .await
        .map_err(|e| format!("{method} request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err(
            "ClawsGO token was rejected — grab a fresh clawsgo_token from the browser (localStorage) and paste it in Settings"
                .into(),
        );
    }
    if !resp.status().is_success() {
        return Err(format!("{method} endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = super::json_body(resp, MAX_BODY, method).await?;
    if let Some(err) = doc.get("error").filter(|e| !e.is_null()) {
        let code = err.get("code").and_then(Value::as_str).unwrap_or("ERROR");
        if code == "UNAUTHORIZED" {
            return Err("ClawsGO token was rejected — paste a fresh clawsgo_token in Settings".into());
        }
        return Err(format!("{method}: {code}"));
    }
    Ok(doc.get("data").cloned().unwrap_or(Value::Null))
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let teams = rpc(key, "getTeams", serde_json::json!({})).await?;
    let team_id = teams
        .pointer("/teams/0/id")
        .and_then(Value::as_str)
        .ok_or("no ClawsGO team on this account")?
        .to_string();
    let plan_id = teams
        .pointer("/teams/0/planId")
        .and_then(Value::as_str)
        .unwrap_or("plan")
        .to_string();

    let params = serde_json::json!({ "teamId": team_id });
    let sub_req = rpc(key, "getSubscription", params.clone());
    let usage_params = serde_json::json!({
        "teamId": team_id,
        "rangeDays": 30,
        "timeZone": chrono::Local::now().format("%Z").to_string(),
    });
    let usage_req = rpc(key, "getUsageStats", usage_params);
    let (sub, usage) = tokio::join!(sub_req, usage_req);

    let sub = sub?;
    // Usage is optional — a fresh account may have none; the plan rows
    // still render.
    let usage = usage.ok();

    let (plan, metrics) = metrics_from_docs(&sub, usage.as_ref(), &plan_id)?;
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

/// The parse pipeline fetch() runs, kept pure for the unit tests below.
/// Milli-credits render as whole credits (the UI unit everywhere).
fn metrics_from_docs(
    sub: &Value,
    usage: Option<&Value>,
    plan_id: &str,
) -> Result<(Option<String>, Vec<Metric>), String> {
    let s = sub
        .get("subscription")
        .ok_or("unexpected subscription response shape")?;
    let grant = s.get("monthlyGrantMilli").and_then(Value::as_f64).unwrap_or(0.0);
    let spent = s.get("cycleSpentMilli").and_then(Value::as_f64).unwrap_or(0.0);
    let balance = s.get("balanceMilli").and_then(Value::as_f64).unwrap_or(0.0);

    let mut metrics = Vec::new();

    // Cycle credits: granted monthly, lapse at planEndAt — a real meter.
    if grant > 0.0 {
        let reset = s
            .get("planEndAt")
            .and_then(Value::as_str)
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.timestamp_millis());
        metrics.push(
            Metric::progress(
                "Monthly",
                (spent / grant * 100.0).clamp(0.0, 100.0),
                Some(format!(
                    "{:.0} of {:.0} credits used",
                    spent / 1000.0,
                    grant / 1000.0
                )),
            )
            .with_reset(reset, Some(30 * DAY_MS)),
        );
    }

    // The whole remaining pool (cycle remainder + permanent credits).
    metrics.push(Metric::text(
        "Balance",
        format!("{:.0} credits left", balance / 1000.0),
    ));

    // 30-day usage from getUsageStats: requests + raw tokens (all cache
    // traffic included, same convention as the spend panel).
    if let Some(u) = usage.and_then(|d| d.get("usageStats")) {
        let requests = u.get("requestCount").and_then(Value::as_f64).unwrap_or(0.0);
        let tokens = ["inputTokens", "cacheReadTokens", "cacheCreationTokens", "outputTokens"]
            .iter()
            .filter_map(|k| u.get(*k).and_then(Value::as_f64))
            .sum::<f64>();
        metrics.push(Metric::text(
            "30-Day Usage",
            format!("{:.0} requests · {:.1}M tokens", requests, tokens / 1e6),
        ));
    }

    if metrics.is_empty() {
        return Err("no subscription or usage data returned".into());
    }
    let mut plan = Some(title_case(plan_id));
    if plan.as_deref() == Some("Plan") {
        plan = None;
    }
    Ok((plan, metrics))
}

fn title_case(s: &str) -> String {
    let mut out = String::new();
    for (i, w) in s.split(['-', '_', ' ']).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut cs = w.chars();
        if let Some(c) = cs.next() {
            out.extend(c.to_uppercase());
            out.push_str(cs.as_str());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sub_doc() -> Value {
        json!({"subscription": {
            "teamId": "959e",
            "planId": "plus",
            "planStartAt": "2026-08-31T04:46:35.368Z",
            "planEndAt": "2026-09-30T04:46:35.368Z",
            "cycleStartAt": "2026-08-31T04:46:35.368Z",
            "balanceMilli": 291742.0,
            "cycleSpentMilli": 15073508.0,
            "monthlyGrantMilli": 30000000.0
        }})
    }

    fn usage_doc() -> Value {
        json!({"usageStats": {
            "requestCount": 556.0,
            "inputTokens": 3327658.0,
            "cacheReadTokens": 48513656.0,
            "cacheCreationTokens": 0.0,
            "outputTokens": 470207.0
        }})
    }

    fn labels(metrics: &[Metric]) -> Vec<&str> {
        metrics.iter().map(|m| m.label.as_str()).collect()
    }

    #[test]
    fn live_shapes_render_plan_rows() {
        let (plan, metrics) =
            metrics_from_docs(&sub_doc(), Some(&usage_doc()), "plus").unwrap();
        assert_eq!(plan.as_deref(), Some("Plus"));
        assert_eq!(labels(&metrics), ["Monthly", "Balance", "30-Day Usage"]);

        assert_eq!(metrics[0].used_percent, Some(15073508.0 / 30000000.0 * 100.0));
        assert!(metrics[0].detail.as_deref().unwrap().contains("15074 of 30000"));
        let reset = metrics[0].resets_at.unwrap();
        assert_eq!(reset, chrono::DateTime::parse_from_rfc3339("2026-09-30T04:46:35.368Z").unwrap().timestamp_millis());

        assert_eq!(metrics[1].value.as_deref(), Some("292 credits left"));
        assert_eq!(
            metrics[2].value.as_deref(),
            Some("556 requests · 52.3M tokens")
        );
    }

    #[test]
    fn usage_is_optional() {
        let (_, metrics) = metrics_from_docs(&sub_doc(), None, "plus").unwrap();
        assert_eq!(labels(&metrics), ["Monthly", "Balance"]);
    }

    #[test]
    fn zero_grant_skips_the_meter() {
        let mut sub = sub_doc();
        sub["subscription"]["monthlyGrantMilli"] = json!(0.0);
        let (_, metrics) = metrics_from_docs(&sub, None, "plus").unwrap();
        assert_eq!(labels(&metrics), ["Balance"]);
    }

    #[test]
    fn unexpected_subscription_shape_is_an_error() {
        assert!(metrics_from_docs(&json!({}), None, "plus").is_err());
    }
}
