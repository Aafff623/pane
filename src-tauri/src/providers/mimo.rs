//! MiMo — Xiaomi's MiMo coding platform (platform.xiaomimimo.com; port of
//! the quotas project's provider, endpoints and parsing unchanged).
//!
//! One stored slot accepts either credential shape:
//!   - the platform's browser serviceToken → monthly Token Plan usage via
//!     cookie auth (`api-platform_serviceToken` on platform.xiaomimimo.com)
//!   - an API key → token-plan SGP / PAYG bearer endpoints
//! The pasted value is tried as a cookie first; if the platform rejects it
//! the bearer chain runs. A key valid on /models but with no quota surface
//! reports "no quota API" instead of "invalid key".

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;
use std::time::Duration;

const ID: &str = "mimo";
const NAME: &str = "MiMo";
const PAYG_BASE: &str = "https://api.xiaomimimo.com/v1";
const TOKEN_PLAN_BASE: &str = "https://token-plan-sgp.xiaomimimo.com/v1";
const PLATFORM_USAGE: &str = "https://platform.xiaomimimo.com/api/v1/tokenPlan/usage";
const TOKEN_PLAN_SGP_USAGE: &str = "https://token-plan-sgp.xiaomimimo.com/v1/tokenPlan/usage";
const MONTH_MS: i64 = 30 * 24 * 3600 * 1000;

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["MIMO_API_KEY"]).map(|_| "Pane MiMo credential".into())
}

/// Live test of a pasted credential (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_credential(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(cred) = stored_api_key(ID, &["MIMO_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a MiMo serviceToken (platform.xiaomimimo.com) or API key in Settings (gear icon).",
        ));
    };
    fetch_with_credential(&cred).await
}

/// Accepts a raw serviceToken value or a full `Cookie:`-style paste that
/// carries `api-platform_serviceToken="…"`.
fn service_token_value(cred: &str) -> String {
    let cred = cred.trim().trim_start_matches("Cookie:").trim();
    if let Some((_, rest)) = cred.split_once("api-platform_serviceToken=") {
        let v = rest.trim().trim_start_matches('"');
        let v = v.split_once('"').map_or(v, |(v, _)| v);
        return v.trim().to_string();
    }
    cred.to_string()
}

async fn try_json(headers: Vec<(&str, String)>, url: &str) -> Result<Option<(u16, Value)>, String> {
    let mut req = http().get(url).timeout(Duration::from_secs(10));
    for (k, v) in headers {
        req = req.header(k, v);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("request {url}: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| format!("read {url}: {e}"))?;
    match serde_json::from_str::<Value>(&text) {
        Ok(body) => Ok(Some((status, body))),
        Err(_) => Ok(None),
    }
}

async fn fetch_with_credential(cred: &str) -> Result<Snapshot, String> {
    // 1) Cookie channel: monthly Token Plan usage, the freshest surface.
    let cookie = service_token_value(cred);
    if let Some(snap) = fetch_platform(&cookie).await? {
        return Ok(snap);
    }
    // 2) Bearer chain: tokenPlan/usage (sgp → platform), then balances
    //    (sgp → PAYG), then a /models probe to tell "no quota API" from an
    //    outright rejected credential.
    fetch_bearer(cred).await
}

/// Cookie channel → Some(snapshot) on a usable payload. Auth failures and
/// business errors return None: the pasted value may be an API key, so the
/// bearer chain gets its chance.
async fn fetch_platform(cookie: &str) -> Result<Option<Snapshot>, String> {
    let Some((status, body)) = try_json(
        vec![
            ("Cookie", format!("api-platform_serviceToken=\"{cookie}\"")),
            ("accept", "application/json".into()),
            ("accept-language", "en".into()),
        ],
        PLATFORM_USAGE,
    )
    .await?
    else {
        return Ok(None);
    };
    if matches!(status, 401 | 403) || status >= 400 {
        return Ok(None);
    }
    if body
        .get("code")
        .and_then(Value::as_i64)
        .is_some_and(|c| c != 0)
    {
        return Ok(None);
    }
    match plan_metrics(&body) {
        Ok(metrics) => Ok(Some(Snapshot::ok(
            ID,
            NAME,
            Some("MiMo · Token Plan".into()),
            metrics,
        ))),
        Err(_) => Ok(None),
    }
}

async fn fetch_bearer(key: &str) -> Result<Snapshot, String> {
    for url in [TOKEN_PLAN_SGP_USAGE, PLATFORM_USAGE] {
        let Some((status, body)) =
            try_json(vec![("Authorization", format!("Bearer {key}"))], url).await?
        else {
            continue;
        };
        if matches!(status, 401 | 403) || status >= 400 {
            continue;
        }
        if body
            .get("code")
            .and_then(Value::as_i64)
            .is_some_and(|c| c != 0)
        {
            continue;
        }
        if let Ok(metrics) = plan_metrics(&body) {
            return Ok(Snapshot::ok(
                ID,
                NAME,
                Some("MiMo · Token Plan".into()),
                metrics,
            ));
        }
    }
    for base in [TOKEN_PLAN_BASE, PAYG_BASE] {
        let Some((status, body)) = try_json(
            vec![("Authorization", format!("Bearer {key}"))],
            &format!("{base}/user/balance"),
        )
        .await?
        else {
            continue;
        };
        if matches!(status, 401 | 403) || status >= 400 {
            continue;
        }
        let (plan, metrics) = balance_metrics(&body, base)?;
        return Ok(Snapshot::ok(ID, NAME, plan, metrics));
    }
    // Nothing answered. A key that /models accepts is valid but has no
    // quota surface; anything else is a rejected credential.
    for base in [TOKEN_PLAN_BASE, PAYG_BASE] {
        if let Some((status, _)) = try_json(
            vec![("Authorization", format!("Bearer {key}"))],
            &format!("{base}/models"),
        )
        .await?
        {
            if status == 200 {
                return Err(
                    "no quota API for this key — check usage at platform.xiaomimimo.com".into(),
                );
            }
        }
    }
    Err(
        "credential was rejected — paste a fresh serviceToken or API key in Settings (gear icon)"
            .into(),
    )
}

/// Platform monthUsage → token windows. `{ "data": { "monthUsage": {
/// "items": [{ "name": "month_total_token", "used": N, "limit": N }] } } }`
fn plan_metrics(body: &Value) -> Result<Vec<Metric>, String> {
    let items = body
        .pointer("/data/monthUsage/items")
        .and_then(Value::as_array)
        .ok_or("missing data.monthUsage.items")?;
    let mut metrics = Vec::new();
    for item in items {
        let name = item.get("name").and_then(Value::as_str).unwrap_or("token");
        let (Some(used), Some(limit)) = (
            item.get("used").and_then(json_f64),
            item.get("limit").and_then(json_f64),
        ) else {
            continue;
        };
        if limit <= 0.0 {
            continue;
        }
        let label = match name {
            "month_total_token" => "Monthly tokens".to_string(),
            other => other.replace('_', " "),
        };
        let pct = (used / limit * 100.0).clamp(0.0, 100.0);
        metrics.push(
            Metric::progress(
                &label,
                pct,
                Some(format!(
                    "{} of {} tokens used",
                    fmt_tokens(used),
                    fmt_tokens(limit)
                )),
            )
            .with_reset(None, Some(MONTH_MS)),
        );
    }
    if metrics.is_empty() {
        return Err("no usable monthUsage items".into());
    }
    Ok(metrics)
}

/// /user/balance → (plan label, rows). Balance amounts arrive as strings or
/// numbers depending on the surface; CNY rows only render when > 0.
fn balance_metrics(body: &Value, base: &str) -> Result<(Option<String>, Vec<Metric>), String> {
    let data = body.get("data").unwrap_or(body);

    let mut metrics = Vec::new();
    if let (Some(remaining), Some(limit)) = (
        data.get("token_balance").and_then(json_f64),
        data.get("token_limit").and_then(json_f64),
    ) {
        if remaining > limit {
            metrics.push(Metric::text(
                "Monthly tokens",
                format!(
                    "{} tokens left · includes extra allowance",
                    fmt_tokens(remaining)
                ),
            ));
        } else if limit > 0.0 {
            let used = limit - remaining;
            let pct = (used / limit * 100.0).clamp(0.0, 100.0);
            metrics.push(
                Metric::progress(
                    "Monthly tokens",
                    pct,
                    Some(format!(
                        "{} of {} tokens used",
                        fmt_tokens(used),
                        fmt_tokens(limit)
                    )),
                )
                .with_reset(None, Some(MONTH_MS)),
            );
        }
    }
    for (label, key) in [
        ("Balance", "balance"),
        ("Paid", "charge_balance"),
        ("Granted", "granted_balance"),
    ] {
        if let Some(v) = data.get(key).and_then(json_f64).filter(|v| *v >= 0.0) {
            metrics.push(Metric::text(label, format!("¥{v:.2}")));
        }
    }
    if metrics.is_empty() {
        return Err("no balances in response".into());
    }

    let plan_label = ["plan_name", "plan"]
        .iter()
        .find_map(|k| data.get(*k).and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if base == TOKEN_PLAN_BASE {
                "Token Plan".into()
            } else {
                "PAYG".into()
            }
        });
    Ok((Some(format!("MiMo · {plan_label}")), metrics))
}

/// Vendor surfaces send numbers as numbers or strings; accept both.
fn json_f64(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str()?.trim().parse().ok())
        .filter(|n| n.is_finite() && *n >= 0.0)
}

fn fmt_tokens(n: f64) -> String {
    if n >= 1e9 {
        format!("{:.1}B", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1}K", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_or_missing_token_usage_does_not_become_zero() {
        assert!(plan_metrics(
            &serde_json::json!({"data":{"monthUsage":{"items":[{"limit":1000000}]}}})
        )
        .is_err());
        assert_eq!(json_f64(&serde_json::json!("NaN")), None);
        assert_eq!(json_f64(&serde_json::json!("bad")), None);
        assert!(balance_metrics(
            &serde_json::json!({"data":{"token_balance":"bad","token_limit":1000}}),
            TOKEN_PLAN_BASE
        )
        .is_err());
        let (_, rows) = balance_metrics(
            &serde_json::json!({"data":{"token_balance":2000,"token_limit":1000}}),
            TOKEN_PLAN_BASE,
        )
        .unwrap();
        assert!(rows.iter().all(|m| m.used_percent.is_none()));
    }

    use serde_json::json;

    #[test]
    fn payg_balance_rows_in_cny() {
        let body = json!({ "data": {
            "balance": "12.5000",
            "charge_balance": "10.0000",
            "granted_balance": "2.5000",
            "plan": "PAYG"
        }});
        let (plan, metrics) = balance_metrics(&body, PAYG_BASE).unwrap();
        assert_eq!(plan.as_deref(), Some("MiMo · PAYG"));
        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[0].label, "Balance");
        assert_eq!(metrics[0].value.as_deref(), Some("¥12.50"));
        assert_eq!(metrics[1].value.as_deref(), Some("¥10.00"));
        assert_eq!(metrics[2].value.as_deref(), Some("¥2.50"));
    }

    #[test]
    fn token_plan_balance_reads_quota_and_tier_name() {
        let body = json!({ "data": {
            "token_balance": 800_000,
            "token_limit": 1_000_000,
            "plan_name": "Pro"
        }});
        let (plan, metrics) = balance_metrics(&body, TOKEN_PLAN_BASE).unwrap();
        assert_eq!(plan.as_deref(), Some("MiMo · Pro"));
        let tokens = &metrics[0];
        assert_eq!(tokens.label, "Monthly tokens");
        assert!((tokens.used_percent.unwrap() - 20.0).abs() < 0.001); // 200K of 1M spent
        assert_eq!(tokens.detail.as_deref(), Some("200.0K of 1.0M tokens used"));
    }

    #[test]
    fn platform_month_usage_becomes_a_window_row() {
        let body = json!({ "code": 0, "data": { "monthUsage": { "items": [
            { "name": "month_total_token", "used": 265_741_632, "limit": 1_600_000_000 }
        ]}}});
        let metrics = plan_metrics(&body).unwrap();
        assert_eq!(metrics.len(), 1);
        let row = &metrics[0];
        assert_eq!(row.label, "Monthly tokens");
        assert!((row.used_percent.unwrap() - (265_741_632.0 / 1.6e9 * 100.0)).abs() < 0.001);
        assert_eq!(row.detail.as_deref(), Some("265.7M of 1.6B tokens used"));
        assert_eq!(row.period_ms, Some(MONTH_MS));
        assert!(plan_metrics(&json!({ "data": { "monthUsage": { "items": [] } } })).is_err());
    }

    #[test]
    fn service_token_extraction_takes_the_cookie_form_or_the_raw_value() {
        assert_eq!(service_token_value("raw-token"), "raw-token");
        assert_eq!(
            service_token_value("Cookie: a=1; api-platform_serviceToken=\"abc==\"; b=2"),
            "abc=="
        );
        assert_eq!(
            service_token_value("api-platform_serviceToken=plain"),
            "plain"
        );
    }
}
