//! Perplexity — session-cookie card over the account's credit endpoint
//! (mechanism per CodexBar's Perplexity provider; no API-key surface).
//!
//! The stored slot takes a bare session-token value or a full `Cookie:`
//! header. Spend attribution follows Perplexity's own order: the period's
//! total usage burns recurring credits first, then purchased, then promo.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "perplexity";
const NAME: &str = "Perplexity";
/// Cookie names tried in order for a bare token paste.
const COOKIE_NAMES: [&str; 4] = [
    "__Secure-authjs.session-token",
    "authjs.session-token",
    "__Secure-next-auth.session-token",
    "next-auth.session-token",
];

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["PERPLEXITY_SESSION_TOKEN", "PERPLEXITY_COOKIE"])
        .map(|_| "Pane Perplexity session".into())
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(cred) = stored_api_key(ID, &["PERPLEXITY_SESSION_TOKEN", "PERPLEXITY_COOKIE"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a perplexity.ai session token (or Cookie header) in Settings (gear icon).",
        ));
    };
    fetch_with_credential(&cred).await
}

/// Live test of a pasted credential (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_credential(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

fn cookie_candidates(cred: &str) -> Vec<String> {
    let cred = cred.trim().trim_start_matches("Cookie:").trim();
    if cred.contains("=") {
        return vec![cred.to_string()];
    }
    COOKIE_NAMES.iter().map(|name| format!("{name}={cred}")).collect()
}

async fn fetch_with_credential(cred: &str) -> Result<Snapshot, String> {
    let mut last_err = String::new();
    for cookie in cookie_candidates(cred) {
        let resp = match http()
            .get("https://www.perplexity.ai/rest/billing/credits?version=2.18&source=default")
            .header("Cookie", &cookie)
            .header("Origin", "https://www.perplexity.ai")
            .header("Referer", "https://www.perplexity.ai/account/usage")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/143.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return Err(format!("credits request: {e}")),
        };
        let status = resp.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            last_err = "session rejected — paste a fresh Perplexity session token in Settings".into();
            continue; // next cookie name
        }
        if !status.is_success() {
            return Err(format!("credits endpoint: HTTP {status}"));
        }
        let doc: Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => return Err(format!("credits parse: {e}")),
        };
        return parse_credits(&doc);
    }
    Err(last_err)
}

fn parse_credits(doc: &Value) -> Result<Snapshot, String> {
    let f64_at = |keys: [&str; 2]| keys.iter().find_map(|k| doc.get(*k).and_then(Value::as_f64));
    let spent = f64_at(["total_usage_cents", "totalUsageCents"]).ok_or("no usage total in response")?;
    let renewal = f64_at(["renewal_date_ts", "renewalDateTs"]);
    let purchased_field = f64_at(["current_period_purchased_cents", "currentPeriodPurchasedCents"]).unwrap_or(0.0);

    let grants = doc
        .get("credit_grants")
        .or_else(|| doc.get("creditGrants"))
        .and_then(Value::as_array)
        .ok_or("no credit grants in response")?;
    let amount = |g: &Value| g.get("amount_cents").or_else(|| g.get("amountCents")).and_then(Value::as_f64).unwrap_or(0.0);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let recurring: f64 = grants.iter().filter(|g| g.get("type").and_then(Value::as_str) == Some("recurring")).map(amount).sum();
    let promo: f64 = grants
        .iter()
        .filter(|g| {
            g.get("type").and_then(Value::as_str) == Some("promotional")
                && g.get("expires_at_ts")
                    .or_else(|| g.get("expiresAtTs"))
                    .and_then(Value::as_f64)
                    .map(|e| e > now_ms as f64 / 1000.0)
                    .unwrap_or(true)
        })
        .map(amount)
        .sum();
    let purchased = grants
        .iter()
        .filter(|g| g.get("type").and_then(Value::as_str) == Some("purchased"))
        .map(amount)
        .sum::<f64>()
        .max(purchased_field);

    // Attribution: spent burns recurring first, then purchased, then promo.
    let mut left = spent;
    let mut take = |pool: f64| {
        let used = left.min(pool.max(0.0));
        left -= used;
        used
    };
    let recurring_used = take(recurring);
    let purchased_used = take(purchased);
    let promo_used = take(promo);

    let mut metrics = Vec::new();
    if recurring > 0.0 {
        let pct = (recurring_used / recurring * 100.0).clamp(0.0, 100.0);
        let reset = renewal.map(|ts| if ts > 1e12 { ts as i64 } else { (ts * 1000.0) as i64 });
        metrics.push(
            Metric::progress(
                "Credits",
                pct,
                Some(format!("{} of {} credits", (recurring - recurring_used) as i64, recurring as i64)),
            )
            .with_reset(reset, None),
        );
    }
    if purchased > 0.0 {
        metrics.push(Metric::text("Purchased", format!("{} of {} credits left", (purchased - purchased_used) as i64, purchased as i64)));
    }
    if promo > 0.0 {
        metrics.push(Metric::text("Bonus credits", format!("{} of {} left", (promo - promo_used) as i64, promo as i64)));
    }
    if metrics.is_empty() {
        return Err("no credits reported for this account".into());
    }
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn spend_burns_recurring_then_purchased_then_promo() {
        let doc = json!({
            "total_usage_cents": 1200,
            "renewal_date_ts": 1_768_435_200,
            "current_period_purchased_cents": 0,
            "credit_grants": [
                { "type": "recurring", "amount_cents": 2000 },
                { "type": "promotional", "amount_cents": 500, "expires_at_ts": 4_000_000_000_i64 },
                { "type": "purchased", "amount_cents": 300 }
            ]
        });
        let snap = parse_credits(&doc).unwrap();
        let credits = &snap.metrics[0];
        assert_eq!(credits.used_percent, Some(60.0)); // 1200 of 2000 recurring
        assert_eq!(credits.resets_at, Some(1_768_435_200_000));
        assert_eq!(snap.metrics[1].value.as_deref(), Some("300 of 300 credits left"));
        assert_eq!(snap.metrics[2].value.as_deref(), Some("500 of 500 left"));
    }

    #[test]
    fn overspend_spills_into_purchased_and_promo() {
        let doc = json!({
            "total_usage_cents": 2500,
            "credit_grants": [
                { "type": "recurring", "amount_cents": 2000 },
                { "type": "purchased", "amount_cents": 400 },
                { "type": "promotional", "amount_cents": 600, "expires_at_ts": 4_000_000_000_i64 }
            ]
        });
        let snap = parse_credits(&doc).unwrap();
        assert_eq!(snap.metrics[0].used_percent, Some(100.0));
        assert_eq!(snap.metrics[1].value.as_deref(), Some("0 of 400 credits left")); // fully burned
        assert_eq!(snap.metrics[2].value.as_deref(), Some("500 of 600 left")); // 100 burned
    }

    #[test]
    fn cookie_candidates_expand_a_bare_token() {
        let cands = cookie_candidates("tok-1");
        assert_eq!(cands.len(), COOKIE_NAMES.len());
        assert_eq!(cands[0], "__Secure-authjs.session-token=tok-1");
        assert_eq!(cookie_candidates("Cookie: a=1").len(), 1);
        assert!(parse_credits(&json!({})).is_err());
    }
}
