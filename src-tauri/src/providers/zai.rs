//! Z.ai (GLM Coding Plan) quota. Bearer API key against the undocumented
//! monitor endpoints — parsed tolerantly. The plan runs a 5-hour rolling
//! session window and a weekly window side by side; both come back as
//! TOKENS_LIMIT rows, told apart by their `unit` field (3 = session,
//! 6 = weekly). An idle session window reports no nextResetTime — it only
//! starts counting on first use.
//!
//! Key sources: our Settings pane, ZAI_API_KEY / GLM_API_KEY, or the Z.ai
//! CLI's own key file.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "zai";
const NAME: &str = "Z.ai";

fn find_key() -> Option<String> {
    if let Some(key) = stored_api_key("zai", &["ZAI_API_KEY", "GLM_API_KEY"]) {
        return Some(key);
    }
    // The Z.ai CLI's own key file.
    let path = dirs::home_dir()?.join(".config").join("zai").join("key.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    doc.get("apiKey")
        .or_else(|| doc.get("api_key"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Pure local probe for the Customize gear panel (no network): the Z.ai
/// CLI's own key file carries a key.
pub fn local_credential_hint() -> Option<String> {
    let path = dirs::home_dir()?.join(".config").join("zai").join("key.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    doc.get("apiKey")
        .or_else(|| doc.get("api_key"))
        .and_then(Value::as_str)
        .map(|_| "Z.ai CLI key file (~/.config/zai/key.json)".to_string())
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
    let Some(key) = find_key() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Z.ai API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let quota_req = http()
        .get("https://api.z.ai/api/monitor/usage/quota/limit")
        .bearer_auth(&key)
        .send();
    let plan_req = http()
        .get("https://api.z.ai/api/biz/subscription/list")
        .bearer_auth(&key)
        .send();
    let (quota_resp, plan_resp) = tokio::join!(quota_req, plan_req);

    let quota_resp = quota_resp.map_err(|e| format!("quota request: {e}"))?;
    if quota_resp.status().as_u16() == 401 {
        return Err("API key was rejected — check it in Settings".into());
    }
    if !quota_resp.status().is_success() {
        return Err(format!("quota endpoint: HTTP {}", quota_resp.status()));
    }
    let quota: Value = quota_resp.json().await.map_err(|e| format!("quota parse: {e}"))?;

    let mut other = Vec::new();
    let mut tokens = Vec::new();
    collect_quota_metrics(quota.get("data").unwrap_or(&quota), &mut other, &mut tokens);
    // Session/Weekly rows first, then whatever else the endpoint carried.
    let mut metrics = Vec::new();
    push_token_metrics(tokens, &mut metrics);
    metrics.append(&mut other);
    if metrics.is_empty() {
        return Err("unexpected quota response shape (endpoint is undocumented)".into());
    }
    metrics.truncate(5);

    let mut plan = None;
    if let Ok(resp) = plan_resp {
        if resp.status().is_success() {
            if let Ok(doc) = resp.json::<Value>().await {
                plan = find_plan_name(doc.get("data").unwrap_or(&doc));
            }
        }
    }

    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

/// One TOKENS_LIMIT entry from the quota response, staged until the walk
/// finishes: window labels and reset periods come from `unit` across all
/// buckets, so a bucket can't be finalized on its own.
struct TokenBucket {
    used_percent: f64,
    detail: Option<String>,
    unit: Option<i64>,
    resets_at: Option<i64>,
}

/// The quota endpoint is undocumented, so we parse tolerantly: any object
/// carrying a usage/limit pair (or a percentage) becomes a meter.
fn collect_quota_metrics(node: &Value, metrics: &mut Vec<Metric>, tokens: &mut Vec<TokenBucket>) {
    match node {
        Value::Array(items) => {
            for item in items {
                collect_quota_metrics(item, metrics, tokens);
            }
        }
        Value::Object(map) => {
            // TIME_LIMIT is the monthly web-search quota, with inverted field
            // roles vs the other entries: `currentValue` = used, `usage` = cap.
            let type_name = ["type", "name"]
                .iter()
                .find_map(|k| map.get(*k).and_then(Value::as_str));
            if type_name == Some("TIME_LIMIT") {
                let used = map.get("currentValue").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
                let cap = map.get("usage").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
                if cap > 0.0 {
                    let resets_at = map
                        .get("nextResetTime")
                        .and_then(Value::as_i64)
                        .filter(|ms| *ms > 0);
                    metrics.push(
                        Metric::progress(
                            "Web Searches",
                            (used / cap * 100.0).clamp(0.0, 100.0),
                            Some(format!("{used:.0} of {cap:.0} searches")),
                        )
                        .with_reset(resets_at, Some(30 * 86_400_000)),
                    );
                }
                return;
            }

            // TOKENS_LIMIT rows are the plan's 5-hour session and weekly
            // windows. They are staged rather than emitted here — the
            // Session/Weekly split is decided by `unit` once every bucket
            // is known (see push_token_metrics).
            if type_name == Some("TOKENS_LIMIT") {
                let percent = ["percentage", "percent", "usagePercent"]
                    .iter()
                    .find_map(|k| map.get(*k).and_then(Value::as_f64));
                let used = ["usage", "used", "currentValue", "current"]
                    .iter()
                    .find_map(|k| map.get(*k).and_then(Value::as_f64));
                let limit = ["limit", "total", "maxValue", "max"]
                    .iter()
                    .find_map(|k| map.get(*k).and_then(Value::as_f64));
                let parsed = if let Some(p) = percent {
                    Some((p, None))
                } else if let (Some(u), Some(l)) = (used, limit) {
                    (l > 0.0).then(|| (u / l * 100.0, Some(format!("{u:.0} of {l:.0}"))))
                } else {
                    None
                };
                if let Some((used_percent, detail)) = parsed {
                    tokens.push(TokenBucket {
                        used_percent,
                        detail,
                        unit: map.get("unit").and_then(Value::as_i64),
                        resets_at: map
                            .get("nextResetTime")
                            .and_then(Value::as_i64)
                            .filter(|ms| *ms > 0),
                    });
                }
                return;
            }

            let label = ["type", "name", "unit", "quotaType"]
                .iter()
                .find_map(|k| map.get(*k).and_then(Value::as_str))
                .map(nice_label)
                .unwrap_or_else(|| "Quota".to_string());

            let used = ["usage", "used", "currentValue", "current"]
                .iter()
                .find_map(|k| map.get(*k).and_then(Value::as_f64));
            let limit = ["limit", "total", "maxValue", "max"]
                .iter()
                .find_map(|k| map.get(*k).and_then(Value::as_f64));
            let percent = ["percentage", "percent", "usagePercent"]
                .iter()
                .find_map(|k| map.get(*k).and_then(Value::as_f64));

            if let Some(p) = percent {
                metrics.push(Metric::progress(&label, p, None));
            } else if let (Some(u), Some(l)) = (used, limit) {
                if l > 0.0 {
                    metrics.push(Metric::progress(
                        &label,
                        u / l * 100.0,
                        Some(format!("{u:.0} of {l:.0}")),
                    ));
                }
            } else {
                for value in map.values() {
                    collect_quota_metrics(value, metrics, tokens);
                }
            }
        }
        _ => {}
    }
}

/// Emits the staged TOKENS_LIMIT buckets as Session (5h) and Weekly rows
/// with their reset times. `unit` is the only trustworthy window marker —
/// 3 = session, 6 = weekly; sorting by nextResetTime to guess the window
/// mislabels the weekly bucket near period end (cc-switch issue #3036).
/// When `unit` is missing, an idle bucket (no nextResetTime — the 5-hour
/// window only starts on first use) becomes the Session row; the rest
/// fill empty slots by ascending reset. Older plans report a single
/// bucket and naturally degrade to just the Session row.
fn push_token_metrics(buckets: Vec<TokenBucket>, metrics: &mut Vec<Metric>) {
    const SESSION_MS: i64 = 5 * 3_600_000;
    const WEEK_MS: i64 = 7 * 86_400_000;
    let mut session: Option<TokenBucket> = None;
    let mut weekly: Option<TokenBucket> = None;
    let mut unassigned: Vec<TokenBucket> = Vec::new();
    for bucket in buckets {
        match bucket.unit {
            Some(3) if session.is_none() => session = Some(bucket),
            Some(6) if weekly.is_none() => weekly = Some(bucket),
            _ => unassigned.push(bucket),
        }
    }
    if session.is_none() {
        if let Some(pos) = unassigned.iter().position(|b| b.resets_at.is_none()) {
            session = Some(unassigned.remove(pos));
        }
    }
    unassigned.sort_by_key(|b| b.resets_at.unwrap_or(i64::MAX));
    for bucket in unassigned {
        if session.is_none() {
            session = Some(bucket);
        } else if weekly.is_none() {
            weekly = Some(bucket);
        }
    }
    for (bucket, label, period_ms) in [
        (session, "Session", SESSION_MS),
        (weekly, "Weekly", WEEK_MS),
    ] {
        if let Some(b) = bucket {
            // An idle session window carries no reset — with_reset(None, …)
            // renders as "not started" instead of a fake countdown.
            metrics.push(
                Metric::progress(label, b.used_percent, b.detail)
                    .with_reset(b.resets_at, Some(period_ms)),
            );
        }
    }
}

fn nice_label(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("5h") || lower.contains("five") || lower.contains("session") {
        "Session".to_string()
    } else if lower.contains("7d") || lower.contains("week") {
        "Weekly".to_string()
    } else if lower.contains("search") {
        "Web searches".to_string()
    } else {
        raw.to_string()
    }
}

fn find_plan_name(node: &Value) -> Option<String> {
    match node {
        Value::Array(items) => items.iter().find_map(find_plan_name),
        Value::Object(map) => ["productName", "planName", "plan", "name"]
            .iter()
            .find_map(|k| map.get(*k).and_then(Value::as_str))
            .map(str::to_string)
            .or_else(|| map.values().find_map(find_plan_name)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SESSION_MS: i64 = 5 * 3_600_000;
    const WEEK_MS: i64 = 7 * 86_400_000;

    /// The same pipeline fetch() runs: walk, stage, finalize.
    fn metrics_from_quota(data: &Value) -> Vec<Metric> {
        let mut other = Vec::new();
        let mut tokens = Vec::new();
        collect_quota_metrics(data, &mut other, &mut tokens);
        let mut metrics = Vec::new();
        push_token_metrics(tokens, &mut metrics);
        metrics.append(&mut other);
        metrics
    }

    fn labels(metrics: &[Metric]) -> Vec<&str> {
        metrics.iter().map(|m| m.label.as_str()).collect()
    }

    #[test]
    fn dual_token_limits_split_by_unit_not_reset_order() {
        // unit 3 = 5-hour session, unit 6 = weekly. Note the weekly reset
        // is EARLIER than the session reset — sorting by nextResetTime
        // would swap the labels (cc-switch issue #3036).
        let data = json!({ "limits": [
            { "type": "TOKENS_LIMIT", "unit": 6, "number": 7, "percentage": 45,
              "nextResetTime": 1767830400000i64 },
            { "type": "TOKENS_LIMIT", "unit": 3, "number": 5, "percentage": 12,
              "nextResetTime": 1767916800000i64 },
            { "type": "TIME_LIMIT", "currentValue": 10, "usage": 100,
              "nextResetTime": 1767830400000i64 }
        ]});
        let metrics = metrics_from_quota(&data);
        assert_eq!(labels(&metrics), ["Session", "Weekly", "Web Searches"]);
        assert_eq!(metrics[0].used_percent, Some(12.0));
        assert_eq!(metrics[0].resets_at, Some(1767916800000));
        assert_eq!(metrics[0].period_ms, Some(SESSION_MS));
        assert_eq!(metrics[1].used_percent, Some(45.0));
        assert_eq!(metrics[1].resets_at, Some(1767830400000));
        assert_eq!(metrics[1].period_ms, Some(WEEK_MS));
        assert_eq!(metrics[2].period_ms, Some(30 * 86_400_000));
    }

    #[test]
    fn legacy_plan_single_token_limit_degrades_to_session() {
        // Pre-2026-02-12 subscriptions return one TOKENS_LIMIT, no unit.
        let data = json!({ "limits": [
            { "type": "TOKENS_LIMIT", "percentage": 30, "nextResetTime": 1767830400000i64 }
        ]});
        let metrics = metrics_from_quota(&data);
        assert_eq!(labels(&metrics), ["Session"]);
        assert_eq!(metrics[0].resets_at, Some(1767830400000));
        assert_eq!(metrics[0].period_ms, Some(SESSION_MS));
    }

    #[test]
    fn idle_session_window_has_no_reset_but_keeps_its_period() {
        // The 5h window starts on first use — an idle account reports no
        // nextResetTime, which the frontend renders as "not started".
        let data = json!({ "limits": [
            { "type": "TOKENS_LIMIT", "unit": 3, "number": 5, "percentage": 0 },
            { "type": "TOKENS_LIMIT", "unit": 6, "number": 1, "percentage": 8,
              "nextResetTime": 1767830400000i64 }
        ]});
        let metrics = metrics_from_quota(&data);
        assert_eq!(labels(&metrics), ["Session", "Weekly"]);
        assert_eq!(metrics[0].resets_at, None);
        assert_eq!(metrics[0].period_ms, Some(SESSION_MS));
        assert_eq!(metrics[1].resets_at, Some(1767830400000));
    }

    #[test]
    fn missing_unit_falls_back_to_idle_first_then_ascending_reset() {
        // No unit on either row: the one without a reset is the un-started
        // session window; the other fills the weekly slot.
        let data = json!({ "limits": [
            { "type": "TOKENS_LIMIT", "percentage": 60, "nextResetTime": 1767830400000i64 },
            { "type": "TOKENS_LIMIT", "percentage": 0 }
        ]});
        let metrics = metrics_from_quota(&data);
        assert_eq!(labels(&metrics), ["Session", "Weekly"]);
        assert_eq!(metrics[0].used_percent, Some(0.0));
        assert_eq!(metrics[0].resets_at, None);
        assert_eq!(metrics[0].period_ms, Some(SESSION_MS));
        assert_eq!(metrics[1].used_percent, Some(60.0));
        assert_eq!(metrics[1].period_ms, Some(WEEK_MS));
    }

    #[test]
    fn token_used_limit_pair_without_percentage_still_meters() {
        let data = json!({ "limits": [
            { "type": "TOKENS_LIMIT", "unit": 3, "number": 5,
              "currentValue": 400, "limit": 2000, "nextResetTime": 1767830400000i64 }
        ]});
        let metrics = metrics_from_quota(&data);
        assert_eq!(labels(&metrics), ["Session"]);
        assert_eq!(metrics[0].used_percent, Some(20.0));
        assert_eq!(metrics[0].detail.as_deref(), Some("400 of 2000"));
    }
}
