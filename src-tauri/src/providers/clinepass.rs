//! ClinePass (Cline subscription) quota — three rolling windows off the
//! platform's console endpoint. Credentials are a plain sk_ API key
//! (created at app.cline.bot → Account → API Keys); keys carry no expiry,
//! so this is a paste-key provider with no session machinery at all.
//!
//! The subscription itself DOES have an expiry: `/users/me/plan` answers
//! `currentPeriodEnd` plus `cancelAt`/`canceledAt` (verified 2026-11-03 with
//! a live key). A cancelled plan keeps running until `cancelAt`, so the card
//! labels that date as "ends" instead of "renews".
//!
//! Endpoint facts verified 2026-10-03 (see
//! temp/handoff/20261003-145500-clinepass-provider-impl.md): rolling
//! windows anchored at first use (weekly = anchor+7d, monthly = +30d —
//! the docs' "calendar" wording is wrong, trust the API), `resetsAt` is
//! RFC3339 with nanoseconds and is ABSENT while a window has zero usage,
//! and `percentUsed` is the big-pool share (light use reads 0).

use super::{json_body, send_with_direct_fallback, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "clinepass";
const NAME: &str = "ClinePass";
const MAX_BODY_BYTES: usize = 64 * 1024;
const LIMITS_URL: &str = "https://api.cline.bot/api/v1/users/me/plan/usage-limits";
const PLAN_URL: &str = "https://api.cline.bot/api/v1/users/me/plan";

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["CLINE_API_KEY"]).map(|_| "Pane ClinePass API key".into())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a user-pasted key (Customize "Test"), never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    snapshot_with_key_as(key, ID, NAME).await
}

/// Account-pool variant: preserve the account card id and label while using
/// the same ClinePass quota parser as the family card.
pub async fn snapshot_with_key_as(key: &str, card_id: &str, card_name: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(mut s) => {
            s.id = card_id.into();
            s.name = card_name.into();
            s
        }
        Err(e) => Snapshot::error(card_id, card_name, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(ID, &["CLINE_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a ClinePass API key (app.cline.bot → Account → API Keys) in Settings.",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = send_with_direct_fallback(|client| client.get(LIMITS_URL).bearer_auth(key.trim()))
        .await
        .map_err(|e| format!("ClinePass request: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("ClinePass API key was rejected — check it in Settings".into());
    }
    // 404 on the usage endpoint means the account has no ClinePass plan at
    // all (its /api-keys still answers 200), not a broken credential: say so
    // instead of painting a stale error card.
    if resp.status().as_u16() == 404 {
        return Ok(Snapshot::needs_setup(
            ID,
            NAME,
            None,
            Vec::new(),
            "This Cline account has no ClinePass subscription",
        ));
    }
    if !resp.status().is_success() {
        return Err(format!(
            "ClinePass endpoint: HTTP {}{}",
            resp.status(),
            super::retry_hint(&resp)
        ));
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "ClinePass").await?;
    let mut snap = parse_snapshot(&doc)?;
    // The plan lookup is informational: a failure must never hide the quotas
    // that already answered.
    if let Some(plan) = fetch_plan(key).await {
        if let Some(name) = plan.display_name.clone() {
            snap.plan = Some(name);
        }
        if let Some(row) = plan_metric(&plan) {
            snap.metrics.insert(0, row);
        }
    }
    Ok(snap)
}

#[derive(Clone, Debug, PartialEq)]
struct PlanInfo {
    display_name: Option<String>,
    period_end_ms: Option<i64>,
    cancel_at_ms: Option<i64>,
    cancelled: bool,
    unpaid: bool,
}

async fn fetch_plan(key: &str) -> Option<PlanInfo> {
    let resp = send_with_direct_fallback(|client| client.get(PLAN_URL).bearer_auth(key.trim()))
        .await
        .ok()?;
    match resp.status().as_u16() {
        200 => {}
        402 => {
            return Some(PlanInfo {
                display_name: None,
                period_end_ms: None,
                cancel_at_ms: None,
                cancelled: false,
                unpaid: true,
            })
        }
        // 404 = no current plan (Cline's own contract); anything else is a
        // transient read failure and simply omits the row.
        _ => return None,
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "ClinePass plan").await.ok()?;
    parse_plan(&doc)
}

fn parse_plan(doc: &Value) -> Option<PlanInfo> {
    let data = doc.get("data")?;
    let plan = data.get("plan").filter(|value| !value.is_null())?;
    let display_name = plan
        .get("displayName")
        .and_then(Value::as_str)
        .or_else(|| plan.get("name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string);
    let interval = plan
        .get("interval")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let display_name = match (display_name, interval) {
        (Some(name), Some(interval)) if !name.to_ascii_lowercase().contains(&interval.to_ascii_lowercase()) => {
            Some(format!("{name} ({interval})"))
        }
        (name, _) => name,
    };
    Some(PlanInfo {
        display_name,
        period_end_ms: rfc3339_ms(data, "currentPeriodEnd"),
        cancel_at_ms: rfc3339_ms(data, "cancelAt"),
        cancelled: data
            .get("canceledAt")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
        unpaid: false,
    })
}

fn rfc3339_ms(doc: &Value, key: &str) -> Option<i64> {
    doc.get(key)
        .and_then(Value::as_str)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp_millis())
}

fn plan_metric(plan: &PlanInfo) -> Option<Metric> {
    if plan.unpaid {
        return Some(Metric::text(
            "Subscription",
            "ClinePass subscription is unpaid".into(),
        ));
    }
    let name = plan
        .display_name
        .clone()
        .unwrap_or_else(|| "ClinePass".into());
    let now = chrono::Utc::now().timestamp_millis();
    let end = plan.cancel_at_ms.or(plan.period_end_ms);
    let value = match end {
        Some(ms) if plan.cancelled && ms > now => {
            format!("{name} · ends {} (cancelled)", local_datetime(ms)?)
        }
        Some(ms) if plan.cancelled => format!("{name} · expired {}", local_datetime(ms)?),
        Some(ms) if ms > now => format!("{name} · renews {}", local_datetime(ms)?),
        Some(ms) => format!("{name} · expired {}", local_datetime(ms)?),
        None if plan.cancelled => format!("{name} · cancelled"),
        None => name,
    };
    Some(Metric::text("Subscription", value))
}

fn local_datetime(ms: i64) -> Option<String> {
    let utc = chrono::DateTime::from_timestamp_millis(ms)?;
    Some(
        utc.with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
    )
}

/// Window periods: rolling anchors per the API (weekly = +7d, monthly =
/// +30d), NOT calendar windows.
fn parse_snapshot(doc: &Value) -> Result<Snapshot, String> {
    let limits = doc
        .pointer("/data/limits")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or("ClinePass response has no usage limits")?;
    let mut metrics = Vec::new();
    for w in limits {
        let Some(kind) = w.get("type").and_then(Value::as_str) else {
            continue;
        };
        let (label, period_ms) = match kind {
            "five_hour" => ("Session", 5 * 3_600_000i64),
            "weekly" => ("Weekly", 7 * 86_400_000i64),
            "monthly" => ("Monthly", 30 * 86_400_000i64),
            // unknown window kind: ignore, don't guess
            _ => continue,
        };
        let Some(used) = number(w.get("percentUsed")).filter(|v| v.is_finite() && *v >= 0.0) else {
            continue;
        };
        let reset = w
            .get("resetsAt")
            .and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp_millis())
            // A reset already in the past means the window rolled over
            // between server write and our read — no stale countdown.
            .filter(|ms| *ms > chrono::Utc::now().timestamp_millis());
        metrics.push(
            Metric::progress(
                label,
                used.clamp(0.0, 100.0),
                Some(format!("{used:.0}% used")),
            )
            .with_reset(reset, Some(period_ms)),
        );
    }
    if metrics.is_empty() {
        return Err("ClinePass response had no recognized windows".into());
    }
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

fn number(v: Option<&Value>) -> Option<f64> {
    v?.as_f64().or_else(|| v?.as_str()?.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_percent_is_not_zero() {
        assert!(
            parse_snapshot(&serde_json::json!({"data":{"limits":[{"type":"monthly"}]}})).is_err()
        );
    }

    #[test]
    fn parses_three_windows_with_resets() {
        let doc = serde_json::json!({
            "data": {"limits": [
                {"type": "five_hour", "percentUsed": 12.5, "resetsAt": "2262-12-31T23:59:59.097343092Z"},
                {"type": "weekly", "percentUsed": 3.0, "resetsAt": "2262-12-31T23:59:59.099429973Z"},
                {"type": "monthly", "percentUsed": 0, "resetsAt": "2262-12-31T23:59:59.101361102Z"}
            ]},
            "success": true
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.metrics.len(), 3);
        assert_eq!(snap.metrics[0].label, "Session");
        assert!((snap.metrics[0].used_percent.unwrap() - 12.5).abs() < 0.001);
        assert_eq!(snap.metrics[0].period_ms, Some(5 * 3_600_000));
        assert!(snap.metrics[0].resets_at.is_some());
        assert_eq!(snap.metrics[2].label, "Monthly");
        assert_eq!(snap.metrics[2].period_ms, Some(30 * 86_400_000));
    }

    #[test]
    fn zero_usage_windows_have_no_reset_and_survive() {
        // The documented shape while usage is zero: resetsAt absent.
        let doc = serde_json::json!({
            "data": {"limits": [
                {"type": "five_hour", "percentUsed": 0},
                {"type": "weekly", "percentUsed": "0"},
                {"type": "monthly", "percentUsed": 0}
            ]}
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(snap.metrics.len(), 3);
        assert!(snap.metrics.iter().all(|m| m.resets_at.is_none()));
        // String-encoded percents parse too.
        assert_eq!(snap.metrics[1].used_percent, Some(0.0));
    }

    #[test]
    fn past_reset_is_dropped_and_unknown_window_ignored() {
        let doc = serde_json::json!({
            "data": {"limits": [
                {"type": "five_hour", "percentUsed": 9.0, "resetsAt": "2020-01-01T00:00:00Z"},
                {"type": "quarterly", "percentUsed": 42.0, "resetsAt": "2262-12-31T00:00:00Z"}
            ]}
        });
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "Session");
        assert!(
            snap.metrics[0].resets_at.is_none(),
            "rolled-over window must not fake a countdown"
        );
    }

    #[test]
    fn cancelled_plan_reports_its_end_date() {
        let end = (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339();
        let doc = serde_json::json!({"data":{
            "plan":{"displayName":"Cline Pass (Monthly)","interval":"Monthly","isActive":true},
            "currentPeriodEnd":end,
            "cancelAt":end,
            "canceledAt":"2026-10-04T04:48:37Z"
        },"success":true});
        let plan = parse_plan(&doc).expect("plan");
        assert!(plan.cancelled);
        assert_eq!(plan.period_end_ms, plan.cancel_at_ms);
        let row = plan_metric(&plan).expect("row");
        assert_eq!(row.label, "Subscription");
        let value = row.value.unwrap();
        assert!(value.starts_with("Cline Pass (Monthly)"), "{value}");
        assert!(value.contains("ends "), "{value}");
        assert!(!value.contains("renews "), "{value}");
    }

    #[test]
    fn active_plan_reports_the_renewal_date() {
        let renews = (chrono::Utc::now() + chrono::Duration::days(15)).to_rfc3339();
        let doc = serde_json::json!({"data":{
            "plan":{"name":"Cline Pass","interval":"Monthly","isActive":true},
            "currentPeriodEnd":renews,
            "cancelAt":null,
            "canceledAt":null
        },"success":true});
        let plan = parse_plan(&doc).expect("plan");
        assert!(!plan.cancelled);
        let value = plan_metric(&plan).expect("row").value.unwrap();
        assert!(value.contains("renews "), "{value}");
        assert!(value.contains("Cline Pass (Monthly)"), "interval is appended when missing: {value}");
    }

    #[test]
    fn plan_without_a_subscription_is_omitted() {
        let doc = serde_json::json!({"data":{"plan":null},"success":true});
        assert!(parse_plan(&doc).is_none());
        let empty = serde_json::json!({"success":true});
        assert!(parse_plan(&empty).is_none());
    }

    #[test]
    fn unpaid_plan_says_so() {
        let plan = PlanInfo {
            display_name: None,
            period_end_ms: None,
            cancel_at_ms: None,
            cancelled: false,
            unpaid: true,
        };
        let value = plan_metric(&plan).expect("row").value.unwrap();
        assert!(value.contains("unpaid"), "{value}");
    }

    #[test]
    fn empty_or_malformed_limits_is_an_error() {
        assert!(parse_snapshot(&serde_json::json!({"data": {"limits": []}})).is_err());
        assert!(parse_snapshot(&serde_json::json!({})).is_err());
    }
}
