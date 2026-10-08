use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "amp";
const NAME: &str = "Amp";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// `amp usage` output (same text the API's displayText carries).
fn parse_display_text(text: &str) -> (Vec<Metric>, Option<String>) {
    let mut metrics = Vec::new();
    let mut plan = None;

    // "Amp Free: $4.20 / $10.00 remaining (replenishes +$0.40/hour)"
    let free = regex::Regex::new(
        r"Amp Free:\s*\$([\d.]+)\s*/\s*\$([\d.]+)\s*remaining(?:\s*\(replenishes \+\$([\d.]+)/hour\))?",
    )
    .unwrap();
    if let Some(c) = free.captures(text) {
        let remaining: f64 = c[1].parse().unwrap_or(0.0);
        let quota: f64 = c[2].parse().unwrap_or(0.0);
        let hourly: f64 = c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        if quota > 0.0 {
            let used = (quota - remaining).max(0.0);
            // Free tier replenishes continuously; "reset" = when it's full again.
            let resets_at = (hourly > 0.0 && used > 0.0).then(|| {
                chrono::Utc::now().timestamp_millis() + ((used / hourly) * 3600.0 * 1000.0) as i64
            });
            metrics.push(
                Metric::progress(
                    "Amp Free",
                    (used / quota * 100.0).clamp(0.0, 100.0),
                    Some(format!("${remaining:.2} of ${quota:.2} left (+${hourly:.2}/h)")),
                )
                .with_reset(resets_at, None),
            );
            plan = Some("Amp Free".into());
        }
    }

    // "Individual credits: $12.34 remaining"
    if let Some(c) = regex::Regex::new(r"Individual credits:\s*\$([\d.]+)\s*remaining")
        .unwrap()
        .captures(text)
    {
        metrics.push(Metric::text("Individual credits", format!("${}", &c[1])));
    }
    // "Workspace acme: $5.00 remaining" (all matches)
    for c in regex::Regex::new(r"Workspace ([^:]+):\s*\$([\d.]+)\s*remaining")
        .unwrap()
        .captures_iter(text)
    {
        metrics.push(Metric::text(&format!("Workspace {}", &c[1]), format!("${}", &c[2])));
    }

    (metrics, plan)
}

async fn cli_usage() -> Option<String> {
    // The amp CLI on PATH (npm installs amp.cmd on Windows).
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        tokio::process::Command::new("cmd")
            .args(["/C", "amp", "usage"])
            .env("NO_COLOR", "1")
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let text = if text.trim().is_empty() {
        String::from_utf8_lossy(&out.stderr).to_string()
    } else {
        text
    };
    (!text.trim().is_empty()).then_some(text)
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["AMP_API_KEY"]).map(|_| "Pane Amp API key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    // 1) The CLI already holds a session — cheapest and freshest.
    if let Some(text) = cli_usage().await {
        let (metrics, plan) = parse_display_text(&text);
        if !metrics.is_empty() {
            return Ok(Snapshot::ok(ID, NAME, plan, metrics));
        }
    }

    // 2) API token (sgamp_…) from Settings or env.
    let Some(key) = stored_api_key("amp", &["AMP_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Install the amp CLI and sign in, or paste an Amp API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = http()
        .post("https://ampcode.com/api/internal?userDisplayBalanceInfo")
        .bearer_auth(&key)
        .json(&serde_json::json!({ "method": "userDisplayBalanceInfo", "params": {} }))
        .send()
        .await
        .map_err(|e| format!("balance request: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("token was rejected — paste a fresh key in Settings (gear icon)".into());
    }
    if !resp.status().is_success() {
        return Err(format!("balance endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("balance parse: {e}"))?;
    if doc.pointer("/error/code").and_then(Value::as_str) == Some("auth-required") {
        return Err("token was rejected — paste a fresh key in Settings (gear icon)".into());
    }
    let text = doc
        .pointer("/result/displayText")
        .and_then(Value::as_str)
        .ok_or("no displayText in response")?;
    let (metrics, plan) = parse_display_text(text);
    if metrics.is_empty() {
        return Err("could not parse balance text".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_usage_text_parses_into_metrics() {
        let text = "Amp Free: $4.20 / $10.00 remaining (replenishes +$0.40/hour)\nIndividual credits: $12.34 remaining\nWorkspace acme: $5.00 remaining";
        let (metrics, plan) = parse_display_text(text);
        assert_eq!(plan.as_deref(), Some("Amp Free"));
        assert_eq!(metrics.len(), 3);
        let free = &metrics[0];
        assert_eq!(free.label, "Amp Free");
        assert!((free.used_percent.unwrap() - 58.0).abs() < 0.01); // $5.80 of $10.00 spent
        assert_eq!(free.detail.as_deref(), Some("$4.20 of $10.00 left (+$0.40/h)"));
        assert!(free.resets_at.is_some()); // full-again projection from the hourly rate
        assert_eq!(metrics[1].label, "Individual credits");
        assert_eq!(metrics[1].value.as_deref(), Some("$12.34"));
        assert_eq!(metrics[2].label, "Workspace acme");
        assert_eq!(metrics[2].value.as_deref(), Some("$5.00"));
    }

    #[test]
    fn balance_only_text_still_reports_the_wallets() {
        let (metrics, plan) = parse_display_text("Individual credits: $1.50 remaining");
        assert_eq!(plan, None);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].value.as_deref(), Some("$1.50"));
    }
}
