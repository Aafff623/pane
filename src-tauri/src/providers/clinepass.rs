//! ClinePass (Cline subscription) quota — three rolling windows off the
//! platform's console endpoint. Credentials are a plain sk_ API key
//! (created at app.cline.bot → Account → API Keys); keys carry no expiry,
//! so this is a paste-key provider with no session machinery at all.
//!
//! Endpoint facts verified 2026-10-03 (see
//! temp/handoff/20261003-145500-clinepass-provider-impl.md): rolling
//! windows anchored at first use (weekly = anchor+7d, monthly = +30d —
//! the docs' "calendar" wording is wrong, trust the API), `resetsAt` is
//! RFC3339 with nanoseconds and is ABSENT while a window has zero usage,
//! and `percentUsed` is the big-pool share (light use reads 0).

use super::{http, json_body, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "clinepass";
const NAME: &str = "ClinePass";
const MAX_BODY_BYTES: usize = 64 * 1024;
const LIMITS_URL: &str = "https://api.cline.bot/api/v1/users/me/plan/usage-limits";

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
    let resp = http()
        .get(LIMITS_URL)
        .bearer_auth(key.trim())
        .send()
        .await
        .map_err(|e| format!("ClinePass request: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("ClinePass API key was rejected — check it in Settings".into());
    }
    if !resp.status().is_success() {
        return Err(format!("ClinePass endpoint: HTTP {}", resp.status()));
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "ClinePass").await?;
    parse_snapshot(&doc)
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
        let Some(kind) = w.get("type").and_then(Value::as_str) else { continue };
        let (label, period_ms) = match kind {
            "five_hour" => ("Session", 5 * 3_600_000i64),
            "weekly" => ("Weekly", 7 * 86_400_000i64),
            "monthly" => ("Monthly", 30 * 86_400_000i64),
            // unknown window kind: ignore, don't guess
            _ => continue,
        };
        let used = number(w.get("percentUsed")).unwrap_or(0.0);
        let reset = w
            .get("resetsAt")
            .and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp_millis())
            // A reset already in the past means the window rolled over
            // between server write and our read — no stale countdown.
            .filter(|ms| *ms > chrono::Utc::now().timestamp_millis());
        metrics.push(
            Metric::progress(label, used.clamp(0.0, 100.0), Some(format!("{used:.0}% used")))
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
        assert!(snap.metrics[0].resets_at.is_none(), "rolled-over window must not fake a countdown");
    }

    #[test]
    fn empty_or_malformed_limits_is_an_error() {
        assert!(parse_snapshot(&serde_json::json!({"data": {"limits": []}})).is_err());
        assert!(parse_snapshot(&serde_json::json!({})).is_err());
    }
}
