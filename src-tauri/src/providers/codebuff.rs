use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "codebuff";
const NAME: &str = "Codebuff";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// `codebuff login` writes ~/.config/manicode/credentials.json (the CLI's
/// former name): { "default": { "authToken": … } } or a top-level authToken.
fn cli_token() -> Option<String> {
    let path = dirs::home_dir()?.join(".config").join("manicode").join("credentials.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    doc.pointer("/default/authToken")
        .or_else(|| doc.get("authToken"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Pure local probe for the Customize gear panel (no network): the
/// codebuff CLI's credentials file carries a token.
pub fn local_credential_hint() -> Option<String> {
    cli_token().map(|_| "codebuff CLI sign-in (~/.config/manicode/credentials.json)".to_string())
}

fn parse_reset_ms(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.timestamp_millis())
            .or_else(|| s.parse::<i64>().ok().map(|n| if n > 10_000_000_000 { n } else { n * 1000 })),
        Value::Number(n) => {
            let n = n.as_i64()?;
            Some(if n > 10_000_000_000 { n } else { n * 1000 })
        }
        _ => None,
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
    let key = stored_api_key("codebuff", &["CODEBUFF_API_KEY"]).or_else(cli_token);
    let Some(key) = key else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Sign in with `codebuff login` or paste an API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let usage_req = http()
        .post("https://www.codebuff.com/api/v1/usage")
        .bearer_auth(&key)
        .json(&serde_json::json!({ "fingerprintId": "pane-usage" }))
        .send();
    let sub_req = http()
        .get("https://www.codebuff.com/api/user/subscription")
        .bearer_auth(&key)
        .send();
    let (usage_resp, sub_resp) = tokio::join!(usage_req, sub_req);

    let usage_resp = usage_resp.map_err(|e| format!("usage request: {e}"))?;
    if matches!(usage_resp.status().as_u16(), 401 | 403) {
        return Err("token was rejected — run `codebuff login` again".into());
    }
    if !usage_resp.status().is_success() {
        return Err(format!("usage endpoint: HTTP {}", usage_resp.status()));
    }
    let doc: Value = usage_resp.json().await.map_err(|e| format!("usage parse: {e}"))?;

    let used = doc.get("usage").or_else(|| doc.get("used")).and_then(Value::as_f64);
    let total = doc.get("quota").or_else(|| doc.get("limit")).and_then(Value::as_f64);
    let remaining =
        doc.get("remainingBalance").or_else(|| doc.get("remaining")).and_then(Value::as_f64);
    let resets_at = parse_reset_ms(doc.get("next_quota_reset"));

    let mut metrics = Vec::new();
    let effective_total = total.or(used.zip(remaining).map(|(u, r)| u + r));
    match (used, effective_total) {
        (Some(u), Some(t)) if t > 0.0 => metrics.push(
            Metric::progress(
                "Credits",
                (u / t * 100.0).clamp(0.0, 100.0),
                Some(format!("{u:.0} of {t:.0} credits used")),
            )
            .with_reset(resets_at, None),
        ),
        (Some(_), _) => metrics.push(Metric::progress("Credits", 100.0, Some("Exhausted".into()))),
        _ => {}
    }

    // Subscription is best-effort: plan name + weekly rate-limit window.
    let mut plan = None;
    if let Ok(resp) = sub_resp {
        if resp.status().is_success() {
            if let Ok(sub) = resp.json::<Value>().await {
                plan = sub
                    .pointer("/subscription/displayName")
                    .or_else(|| sub.get("displayName"))
                    .or_else(|| sub.pointer("/subscription/tier"))
                    .or_else(|| sub.get("tier"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let w_used = sub
                    .pointer("/rateLimit/weeklyUsed")
                    .or_else(|| sub.pointer("/rateLimit/used"))
                    .and_then(Value::as_f64);
                let w_limit = sub
                    .pointer("/rateLimit/weeklyLimit")
                    .or_else(|| sub.pointer("/rateLimit/limit"))
                    .and_then(Value::as_f64);
                if let (Some(u), Some(l)) = (w_used, w_limit) {
                    if l > 0.0 {
                        metrics.push(
                            Metric::progress(
                                "Weekly",
                                (u / l * 100.0).clamp(0.0, 100.0),
                                Some(format!("{u:.0} of {l:.0} used")),
                            )
                            .with_reset(
                                parse_reset_ms(sub.pointer("/rateLimit/weeklyResetsAt")),
                                Some(7 * 24 * 3600 * 1000),
                            ),
                        );
                    }
                }
            }
        }
    }

    if metrics.is_empty() {
        return Err("no usage data in response".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}
