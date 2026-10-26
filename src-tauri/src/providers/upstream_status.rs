//! Vendor incident attribution (#70). Only a small, explicitly proven
//! endpoint table is probed; an anonymous Statuspage GET (no token, no user
//! data) explains a stale/error card instead of leaving the user to blame
//! Pane. Fail-soft: this module never changes the refresh outcome.

use super::Snapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

const TTL_MS: i64 = 5 * 60_000;
const LAST_GOOD_MS: i64 = 30 * 60_000;
const TIMEOUT_SECS: u64 = 6;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamStatus {
    pub indicator: String,
    pub description: String,
    pub updated_at: Option<i64>,
}

#[derive(Clone)]
struct Entry {
    probed_at: i64,
    result: Option<UpstreamStatus>,
    last_good: Option<(UpstreamStatus, i64)>,
}

fn cache() -> &'static Mutex<HashMap<String, Entry>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, Entry>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn endpoint(family: &str) -> Option<&'static str> {
    match family {
        "claude" => Some("https://status.claude.com/api/v2/status.json"),
        _ => None,
    }
}

fn timestamp_ms(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::String(text) => chrono::DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|dt| dt.timestamp_millis()),
        Value::Number(number) => number.as_i64(),
        _ => None,
    }
}

/// Statuspage contract: `status.indicator` is one of none/minor/major/
/// critical; an unknown value is not rendered by the UI.
pub fn parse_status(doc: &Value) -> Option<UpstreamStatus> {
    let status = doc.get("status")?;
    let indicator = status.get("indicator")?.as_str()?.trim().to_ascii_lowercase();
    if !matches!(indicator.as_str(), "none" | "minor" | "major" | "critical") {
        return None;
    }
    let description = status
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Some(UpstreamStatus {
        indicator,
        description,
        updated_at: timestamp_ms(doc.get("page").and_then(|page| page.get("updated_at"))),
    })
}

async fn fetch(family: &str) -> Option<UpstreamStatus> {
    let url = endpoint(family)?;
    let response = super::http()
        .get(url)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() || response.content_length().is_some_and(|len| len > 64 * 1024) {
        return None;
    }
    let body = response.text().await.ok()?;
    if body.len() > 64 * 1024 {
        return None;
    }
    parse_status(&serde_json::from_str(&body).ok()?)
}

async fn probe(family: &str, now_ms: i64) -> Option<UpstreamStatus> {
    if endpoint(family).is_none() {
        return None;
    }
    let cached = cache()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(family)
        .cloned();
    if let Some(entry) = cached.as_ref() {
        if now_ms - entry.probed_at < TTL_MS {
            return entry.result.clone();
        }
    }
    let result = fetch(family).await;
    let mut map = cache().lock().unwrap_or_else(|error| error.into_inner());
    let entry = map.entry(family.to_string()).or_insert(Entry {
        probed_at: now_ms,
        result: None,
        last_good: None,
    });
    entry.probed_at = now_ms;
    match result {
        Some(probe) => {
            entry.result = Some(probe.clone());
            entry.last_good = Some((probe.clone(), now_ms));
            Some(probe)
        }
        None => {
            // Keep the last good answer for half an hour, then admit we
            // don't know instead of showing a stale incident forever.
            entry.result = entry
                .last_good
                .as_ref()
                .filter(|(_, at)| now_ms - at <= LAST_GOOD_MS)
                .map(|(probe, _)| probe.clone());
            entry.result.clone()
        }
    }
}

/// Attaches attribution only to failed/stale cards; a healthy snapshot is
/// never annotated (an upstream incident is irrelevant when the read works).
pub async fn attach(snapshots: &mut [Snapshot], enabled: bool) {
    if !enabled {
        return;
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    for snapshot in snapshots.iter_mut() {
        if snapshot.status == "ok" && !snapshot.stale {
            continue;
        }
        let family = snapshot.id.split('@').next().unwrap_or(&snapshot.id).to_string();
        if endpoint(&family).is_none() {
            continue;
        }
        snapshot.upstream_status = probe(&family, now_ms).await;
    }
}

#[cfg(test)]
mod tests {
    use super::parse_status;
    use serde_json::json;

    #[test]
    fn statuspage_indicators_parse_and_unknown_values_are_ignored() {
        let doc = json!({
            "page": {"updated_at": "2026-10-26T12:00:00Z"},
            "status": {"indicator": "major", "description": "Partial outage"}
        });
        let parsed = parse_status(&doc).expect("parsed");
        assert_eq!(parsed.indicator, "major");
        assert_eq!(parsed.description, "Partial outage");
        assert!(parsed.updated_at.is_some());
        assert!(parse_status(&json!({"status": {"indicator": "weird"}})).is_none());
        assert!(parse_status(&json!({})).is_none());
        let none = parse_status(&json!({"status": {"indicator": "none"}})).unwrap();
        assert_eq!(none.indicator, "none");
    }
}
