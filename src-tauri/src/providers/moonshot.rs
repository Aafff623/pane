use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;
use std::time::Duration;

const ID: &str = "moonshot";
const NAME: &str = "Kimi API";
const MAX_BALANCE_BYTES: usize = 64 * 1024;

// Global platform first, mainland China second — same key shape either way.
const ENDPOINTS: [&str; 2] = [
    "https://api.moonshot.ai/v1/users/me/balance",
    "https://api.moonshot.cn/v1/users/me/balance",
];

/// No local credential source — a Settings key or MOONSHOT_API_KEY /
/// KIMI_API_KEY only (both reported separately by get_credential_status).
pub fn local_credential_hint() -> Option<String> {
    None
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// True when a Moonshot/Kimi API key is saved or in the environment.
/// Plan-only installs have none — the Kimi card then skips the API row.
pub fn has_api_key() -> bool {
    stored_api_key("moonshot", &["MOONSHOT_API_KEY", "KIMI_API_KEY"]).is_some()
}

/// Wallet fetch is allowed only with a saved key *and* Moonshot switched
/// on. A disabled Moonshot must not be contacted via the folded Kimi card.
pub fn wallet_wanted() -> bool {
    wallet_wanted_from(has_api_key(), super::provider_disabled("moonshot"))
}

fn wallet_wanted_from(has_key: bool, moonshot_disabled: bool) -> bool {
    has_key && !moonshot_disabled
}

/// Wallet rows for the Kimi Code card (Session / Weekly / API). Empty `Ok`
/// when no key is saved or Moonshot is off — the plan bars still stand on
/// their own. `Err` is a failed balance call the caller can warn on
/// without failing the plan.
pub async fn api_rows() -> Result<Vec<Metric>, String> {
    if !wallet_wanted() {
        return Ok(Vec::new());
    }
    fetch_balance(true).await
}

/// Live test of a user-pasted key, without saving it (Customize "Test").
/// Always renders as the standalone wallet card — the test panel only
/// counts metrics.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_balance_with_key(key, false).await {
        Ok(rows) => Snapshot::ok(ID, NAME, Some("Pay as you go".into()), rows),
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    if stored_api_key("moonshot", &["MOONSHOT_API_KEY", "KIMI_API_KEY"]).is_none() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Kimi API key in Settings (gear icon).",
        ));
    }
    let rows = fetch_balance(false).await?;
    Ok(Snapshot::ok(ID, NAME, Some("Pay as you go".into()), rows))
}

async fn fetch_balance(api_label: bool) -> Result<Vec<Metric>, String> {
    let Some(key) = stored_api_key("moonshot", &["MOONSHOT_API_KEY", "KIMI_API_KEY"]) else {
        return Err("no key".into());
    };
    fetch_balance_with_key(&key, api_label).await
}

async fn fetch_balance_with_key(key: &str, api_label: bool) -> Result<Vec<Metric>, String> {
    let mut last_err = String::from("no endpoint reachable");
    for url in ENDPOINTS {
        let resp = match http()
            .get(url)
            .bearer_auth(&key)
            .timeout(Duration::from_secs(8))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("balance request: {e}");
                continue;
            }
        };
        if resp.status().as_u16() == 401 {
            last_err = "key was rejected — paste a fresh key in Settings (gear icon)".into();
            continue; // a .ai key 401s on .cn and vice versa — try the other
        }
        if !resp.status().is_success() {
            last_err = format!("balance endpoint: HTTP {}", resp.status());
            continue;
        }
        let doc: Value = match super::json_body(resp, MAX_BALANCE_BYTES, "balance").await {
            Ok(d) => d,
            Err(e) => {
                last_err = e;
                continue;
            }
        };
        let data = doc.get("data").unwrap_or(&doc);
        let available = data.get("available_balance").and_then(Value::as_f64);
        let voucher = data.get("voucher_balance").and_then(Value::as_f64);
        let cash = data.get("cash_balance").and_then(Value::as_f64);

        let Some(available) = available else {
            last_err = "no balance in response".into();
            continue;
        };
        let sign = if url.contains(".cn") { "¥" } else { "$" };
        return Ok(rows_from_balance(available, voucher, cash, sign, api_label));
    }
    Err(last_err)
}

fn rows_from_balance(
    available: f64,
    voucher: Option<f64>,
    cash: Option<f64>,
    sign: &str,
    api_label: bool,
) -> Vec<Metric> {
    let mut metrics = Vec::new();
    // High-water key stays "moonshot" so a fold onto the Kimi card keeps
    // the same Credits-used baseline the Moonshot card already learned.
    let label = wallet_label(api_label);
    if let Some(meter) = super::credit_meter_labeled(ID, sign, available, label, "") {
        metrics.push(meter);
    }
    metrics.extend(text_rows(available, voucher, cash, sign, api_label));
    metrics
}

fn wallet_label(api_label: bool) -> &'static str {
    if api_label {
        "API"
    } else {
        "Credits used"
    }
}

fn text_rows(
    available: f64,
    voucher: Option<f64>,
    cash: Option<f64>,
    sign: &str,
    api_label: bool,
) -> Vec<Metric> {
    let mut metrics = Vec::new();
    metrics.push(Metric::text("Balance", format!("{sign}{available:.2}")));
    if let Some(v) = voucher {
        if v > 0.0 {
            metrics.push(Metric::text("Vouchers", format!("{sign}{v:.2}")));
        }
    }
    if let Some(c) = cash {
        if !api_label || c > 0.0 {
            metrics.push(Metric::text("Cash", format!("{sign}{c:.2}")));
        }
    }
    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kimi_fold_labels_the_wallet_api() {
        assert_eq!(wallet_label(true), "API");
        let rows = text_rows(80.0, Some(80.0), Some(0.0), "$", true);
        assert_eq!(
            rows.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(),
            ["Balance", "Vouchers"]
        );
    }

    #[test]
    fn standalone_moonshot_keeps_credits_used() {
        assert_eq!(wallet_label(false), "Credits used");
        let rows = text_rows(50.0, None, Some(0.0), "$", false);
        assert_eq!(
            rows.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(),
            ["Balance", "Cash"]
        );
    }

    #[test]
    fn wallet_stays_off_when_moonshot_is_disabled() {
        assert!(wallet_wanted_from(true, false));
        assert!(!wallet_wanted_from(true, true));
        assert!(!wallet_wanted_from(false, false));
        assert!(!wallet_wanted_from(false, true));
    }
}
