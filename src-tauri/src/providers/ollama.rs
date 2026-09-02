use super::{http, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "ollama";
const NAME: &str = "Ollama";
const BASE: &str = "http://127.0.0.1:11434";

/// No credential source at all — Ollama is a local service; whether it is
/// running is only knowable by asking it (a network call this probe must
/// not make).
pub fn local_credential_hint() -> Option<String> {
    None
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    // No credentials — either the local server answers or it doesn't.
    let version = match http().get(format!("{BASE}/api/version")).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| v.get("version").and_then(Value::as_str).map(String::from)),
        _ => {
            return Ok(Snapshot::no_credentials(
                ID,
                NAME,
                "Ollama isn't running on this PC (nothing on port 11434).",
            ));
        }
    };

    let mut metrics = Vec::new();

    if let Ok(resp) = http().get(format!("{BASE}/api/tags")).send().await {
        if let Ok(doc) = resp.json::<Value>().await {
            let models = doc.get("models").and_then(Value::as_array).cloned().unwrap_or_default();
            let bytes: u64 =
                models.iter().filter_map(|m| m.get("size").and_then(Value::as_u64)).sum();
            metrics.push(Metric::text(
                "Installed models",
                format!("{} ({:.1} GB on disk)", models.len(), bytes as f64 / 1e9),
            ));
        }
    }

    if let Ok(resp) = http().get(format!("{BASE}/api/ps")).send().await {
        if let Ok(doc) = resp.json::<Value>().await {
            let names: Vec<String> = doc
                .get("models")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(|m| m.get("name").and_then(Value::as_str).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            metrics.push(Metric::text(
                "Loaded now",
                if names.is_empty() { "None".to_string() } else { names.join(", ") },
            ));
        }
    }

    if metrics.is_empty() {
        return Err("server answered but reported no model info".into());
    }
    Ok(Snapshot::ok(ID, NAME, version.map(|v| format!("v{v}")), metrics))
}
