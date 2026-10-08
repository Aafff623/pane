use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "deepgram";
const NAME: &str = "Deepgram";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["DEEPGRAM_API_KEY"]).map(|_| "Pane Deepgram API key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// The /projects payload → (project_id, name) rows; entries without a
/// project_id are skipped, a missing name falls back to "Project".
fn project_rows(doc: &Value) -> Vec<(String, String)> {
    doc.get("projects")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|p| {
                    Some((
                        p.get("project_id")?.as_str()?.to_string(),
                        p.get("name").and_then(Value::as_str).unwrap_or("Project").to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The /balances payload → sum of every balance's amount.
fn balances_total(doc: &Value) -> f64 {
    doc.get("balances")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(|b| b.get("amount").and_then(Value::as_f64)).sum())
        .unwrap_or(0.0)
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key("deepgram", &["DEEPGRAM_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Deepgram API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = http()
        .get("https://api.deepgram.com/v1/projects")
        .header("Authorization", format!("Token {key}"))
        .send()
        .await
        .map_err(|e| format!("projects request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err("key was rejected — paste a fresh key in Settings (gear icon)".into());
    }
    if !resp.status().is_success() {
        return Err(format!("projects endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("projects parse: {e}"))?;
    let projects = project_rows(&doc);
    if projects.is_empty() {
        return Err("no projects visible to this key".into());
    }

    // A key is usually scoped to one project; cap at 3 to stay snappy.
    let mut metrics = Vec::new();
    for (project_id, name) in projects.iter().take(3) {
        let url = format!("https://api.deepgram.com/v1/projects/{project_id}/balances");
        let resp = http()
            .get(&url)
            .header("Authorization", format!("Token {key}"))
            .send()
            .await
            .map_err(|e| format!("balances request: {e}"))?;
        if !resp.status().is_success() {
            continue; // key may lack balances scope on this project
        }
        let doc: Value = resp.json().await.map_err(|e| format!("balances parse: {e}"))?;
        let total = balances_total(&doc);
        let label =
            if projects.len() == 1 { "Balance".to_string() } else { format!("Balance — {name}") };
        metrics.push(Metric::text(&label, format!("${total:.2}")));
    }
    if metrics.is_empty() {
        return Err("key has no access to project balances".into());
    }
    Ok(Snapshot::ok(ID, NAME, Some("Pay as you go".into()), metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_rows_skip_broken_entries_and_default_the_name() {
        let doc = json!({ "projects": [
            { "project_id": "p1", "name": "Prod" },
            { "name": "Broken" },   // no project_id → skipped
            { "project_id": "p2" }  // no name → "Project"
        ]});
        let rows = project_rows(&doc);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("p1".to_string(), "Prod".to_string()));
        assert_eq!(rows[1], ("p2".to_string(), "Project".to_string()));
        // A response with no projects array is an empty list, not an error —
        // the caller turns that into "no projects visible to this key".
        assert!(project_rows(&json!({})).is_empty());
    }

    #[test]
    fn balances_amounts_sum_across_the_array() {
        let doc = json!({ "balances": [ { "amount": 12.5 }, { "amount": 7.5 }, {} ] });
        assert!((balances_total(&doc) - 20.0).abs() < 1e-9);
        assert_eq!(balances_total(&json!({ "balances": [] })), 0.0);
    }
}
