use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

const ID: &str = "openai-api";
const NAME: &str = "OpenAI API";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["OPENAI_ADMIN_KEY"]).map(|_| "Pane OpenAI API API key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// The /organization/costs payload → (last-30-days total, running day).
/// Every bucket sums into the total; buckets whose end_time is still in the
/// future are the day in progress.
fn cost_totals(doc: &Value, now: i64) -> (f64, f64) {
    let mut total = 0.0f64;
    let mut today = 0.0f64;
    for bucket in doc.get("data").and_then(Value::as_array).unwrap_or(&vec![]) {
        let amount: f64 = bucket
            .get("results")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|r| r.get("amount")?.get("value")?.as_f64())
                    .sum()
            })
            .unwrap_or(0.0);
        total += amount;
        let end = bucket.get("end_time").and_then(Value::as_i64).unwrap_or(0);
        if end >= now {
            today += amount;
        }
    }
    (total, today)
}

async fn fetch() -> Result<Snapshot, String> {
    // The org costs endpoint needs an *Admin* key (sk-admin-…) — a regular
    // sk-… project key gets a 401, which we translate below.
    let Some(key) = stored_api_key(ID, &["OPENAI_ADMIN_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste an OpenAI Admin API key (platform.openai.com → Admin keys) in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let start = now - 30 * 24 * 3600;
    let url = format!(
        "https://api.openai.com/v1/organization/costs?start_time={start}&bucket_width=1d&limit=31"
    );
    let resp =
        http().get(&url).bearer_auth(&key).send().await.map_err(|e| format!("costs request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err(
            "key was rejected — org costs need an Admin key from platform.openai.com/settings/organization/admin-keys"
                .into(),
        );
    }
    if !resp.status().is_success() {
        return Err(format!("costs endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("costs parse: {e}"))?;

    let (total, today) = cost_totals(&doc, now);

    let metrics = vec![
        Metric::text("Today", format!("${today:.2}")),
        Metric::text("Last 30 days", format!("${total:.2}")),
    ];
    Ok(Snapshot::ok(ID, NAME, Some("API".into()), metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cost_buckets_split_into_total_and_running_day() {
        let doc = json!({ "data": [
            // Finished day with two line items.
            { "end_time": 100, "results": [ { "amount": { "value": 2.25 } }, { "amount": { "value": 0.5 } } ] },
            // The bucket still in progress counts as "today" too.
            { "end_time": 100, "results": [ { "amount": { "value": 1.0 } } ] },
            // Older history.
            { "end_time": 50, "results": [ { "amount": { "value": 10.0 } } ] }
        ]});
        let (total, today) = cost_totals(&doc, 100);
        assert!((total - 13.75).abs() < 1e-9);
        assert!((today - 3.75).abs() < 1e-9);
    }
}
