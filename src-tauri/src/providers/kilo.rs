use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "kilo";
const NAME: &str = "Kilo";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// The Kilo CLI keeps its session at ~/.local/share/kilo/auth.json → kilo.access
fn cli_token() -> Option<String> {
    let path = dirs::home_dir()?.join(".local").join("share").join("kilo").join("auth.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    doc.pointer("/kilo/access").and_then(Value::as_str).map(str::to_string)
}

/// Pure local probe for the Customize gear panel (no network): the Kilo
/// CLI's auth.json carries a session token.
pub fn local_credential_hint() -> Option<String> {
    cli_token().map(|_| "Kilo CLI sign-in (~/.local/share/kilo/auth.json)".to_string())
}

/// tRPC batch responses wrap each result as result.data.json (with variants).
fn unwrap_trpc(v: &Value) -> Option<&Value> {
    v.pointer("/result/data/json")
        .or_else(|| v.pointer("/result/data"))
        .or_else(|| v.pointer("/result/json"))
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
    let key = stored_api_key("kilo", &["KILO_API_KEY"]).or_else(cli_token);
    let Some(key) = key else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Sign in with the Kilo CLI or paste an API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let input = urlencoding_min("{\"0\":{\"json\":null},\"1\":{\"json\":null}}");
    let url = format!(
        "https://app.kilo.ai/api/trpc/user.getCreditBlocks,kiloPass.getState?batch=1&input={input}"
    );
    let resp = http()
        .get(&url)
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("trpc request: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("token was rejected — sign in with the Kilo CLI again".into());
    }
    if !resp.status().is_success() {
        return Err(format!("trpc endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("trpc parse: {e}"))?;
    let batch = doc.as_array().ok_or("unexpected trpc response shape")?;

    let mut metrics = Vec::new();
    let mut plan = None;

    // [0] user.getCreditBlocks — amounts in micro-USD.
    if let Some(blocks) = batch.first().and_then(unwrap_trpc) {
        let rows = blocks
            .get("creditBlocks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(|| blocks.as_array().cloned().unwrap_or_default());
        let total: f64 = rows
            .iter()
            .filter_map(|b| b.get("amount_mUsd").and_then(Value::as_f64))
            .sum::<f64>()
            / 1_000_000.0;
        let remaining: f64 = rows
            .iter()
            .filter_map(|b| b.get("balance_mUsd").and_then(Value::as_f64))
            .sum::<f64>()
            / 1_000_000.0;
        if total > 0.0 {
            let used = (total - remaining).max(0.0);
            metrics.push(Metric::progress(
                "Credits",
                (used / total * 100.0).clamp(0.0, 100.0),
                Some(format!("${used:.2} of ${total:.2} used")),
            ));
        }
    }

    // [1] kiloPass.getState — subscription window in USD. `subscription`
    // is null for accounts without a Pass; some responses put the fields
    // at the top level instead (only trusted when they look the part).
    let no_pass = Value::Null;
    if let Some(pass) = batch.get(1).and_then(unwrap_trpc) {
        let sub = match pass.get("subscription") {
            Some(s) if s.is_object() => s,
            Some(_) => &no_pass, // explicit null: no Kilo Pass
            None if pass.get("currentPeriodUsageUsd").is_some()
                || pass.get("currentPeriodBaseCreditsUsd").is_some()
                || pass.get("tier").is_some() =>
            {
                pass
            }
            None => &no_pass,
        };
        let used = sub.get("currentPeriodUsageUsd").and_then(Value::as_f64);
        let base = sub.get("currentPeriodBaseCreditsUsd").and_then(Value::as_f64).unwrap_or(0.0);
        let bonus = sub.get("currentPeriodBonusCreditsUsd").and_then(Value::as_f64).unwrap_or(0.0);
        let total = base + bonus;
        let resets = parse_reset_ms(
            sub.get("nextBillingAt")
                .or_else(|| sub.get("nextRenewalAt"))
                .or_else(|| sub.get("renewsAt"))
                .or_else(|| sub.get("renewAt")),
        );
        if let Some(used) = used {
            if total > 0.0 {
                let desc = if bonus > 0.0 {
                    format!("${used:.2} / ${base:.2} (+${bonus:.2} bonus)")
                } else {
                    format!("${used:.2} / ${base:.2}")
                };
                metrics.push(
                    Metric::progress("Kilo Pass", (used / total * 100.0).clamp(0.0, 100.0), Some(desc))
                        .with_reset(resets, Some(30 * 24 * 3600 * 1000)),
                );
            }
        }
        plan = sub.get("tier").and_then(Value::as_str).map(|t| match t {
            "tier_19" => "Starter".to_string(),
            "tier_49" => "Pro".to_string(),
            "tier_199" => "Expert".to_string(),
            other => other.to_string(),
        });
        if plan.is_none() && !sub.is_null() && sub.is_object() {
            plan = Some("Kilo Pass".into());
        }
    }

    // A fresh account answers both procedures with empty data (no credit
    // blocks, subscription null). That's a zero state, not an error.
    if metrics.is_empty() {
        if let Some(blocks) = batch.first().and_then(unwrap_trpc).filter(|b| b.is_object()) {
            let bal =
                blocks.get("totalBalance_mUsd").and_then(Value::as_f64).unwrap_or(0.0) / 1e6;
            let value = if bal > 0.0 {
                format!("${bal:.2} balance")
            } else {
                "None yet — top up on the dashboard".to_string()
            };
            metrics.push(Metric::text("Credits", value));
        }
    }
    if metrics.is_empty() {
        return Err("no credit data in response".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

/// Just enough URL-encoding for the fixed tRPC input literal.
fn urlencoding_min(s: &str) -> String {
    s.replace('{', "%7B").replace('}', "%7D").replace('"', "%22").replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    /// Live probe against this machine's real Kilo session: prints the raw
    /// tRPC response (status + first 2000 chars) so shape changes can be
    /// diagnosed. Run: cargo test --lib kilo -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_probe() {
        let key = super::stored_api_key("kilo", &["KILO_API_KEY"])
            .or_else(super::cli_token)
            .expect("no kilo credentials on this machine");
        let input = super::urlencoding_min("{\"0\":{\"json\":null},\"1\":{\"json\":null}}");
        let url = format!(
            "https://app.kilo.ai/api/trpc/user.getCreditBlocks,kiloPass.getState?batch=1&input={input}"
        );
        let (status, body) = tauri::async_runtime::block_on(async {
            let r = super::http().get(&url).bearer_auth(&key).send().await.expect("request");
            (r.status().as_u16(), r.text().await.unwrap_or_default())
        });
        eprintln!("kilo probe: HTTP {status}");
        eprintln!("{}", body.chars().take(2000).collect::<String>());
    }
}
