use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "elevenlabs";
const NAME: &str = "ElevenLabs";

/// No local credential source — a Settings key or ELEVENLABS_API_KEY /
/// XI_API_KEY only (both reported separately by get_credential_status).
pub fn local_credential_hint() -> Option<String> {
    None
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
    let Some(key) = stored_api_key("elevenlabs", &["ELEVENLABS_API_KEY", "XI_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste an ElevenLabs API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = http()
        .get("https://api.elevenlabs.io/v1/user/subscription")
        .header("xi-api-key", key)
        .send()
        .await
        .map_err(|e| format!("subscription request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err("key was rejected — paste a fresh key in Settings (gear icon)".into());
    }
    if !resp.status().is_success() {
        return Err(format!("subscription endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("subscription parse: {e}"))?;

    let used = doc.get("character_count").and_then(Value::as_f64).unwrap_or(0.0);
    let limit = doc.get("character_limit").and_then(Value::as_f64).unwrap_or(0.0);
    if limit <= 0.0 {
        return Err("no character quota in response".into());
    }
    let resets_at = doc
        .get("next_character_count_reset_unix")
        .and_then(Value::as_i64)
        .map(|secs| secs * 1000);

    let mut metrics = vec![Metric::progress(
        "Characters",
        used / limit * 100.0,
        Some(format!("{} of {} characters used", group(used as u64), group(limit as u64))),
    )
    // Monthly quota — period is ~30d, close enough for pacing.
    .with_reset(resets_at, Some(30 * 24 * 3600 * 1000))];

    if let Some(slots) = doc.get("voice_slots_used").and_then(Value::as_f64) {
        if let Some(max) = doc.get("voice_limit").and_then(Value::as_f64) {
            if max > 0.0 {
                metrics.push(Metric::progress(
                    "Voice slots",
                    slots / max * 100.0,
                    Some(format!("{slots:.0} of {max:.0} slots used")),
                ));
            }
        }
    }

    let plan = doc.get("tier").and_then(Value::as_str).map(|t| {
        let mut c = t.chars();
        match c.next() {
            Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
            None => t.to_string(),
        }
    });
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

/// 123456 → "123,456"
fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
