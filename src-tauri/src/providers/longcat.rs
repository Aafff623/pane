//! LongCat — cookie card over the longcat.chat web session (mechanism per
//! CodexBar's LongCat provider; no API-key surface).
//!
//! The stored slot holds the pasted `Cookie:` header (or a bare token —
//! sent as the session cookie). Sequence: user-current validates the
//! session, the token-pack summary is the primary quota, the legacy
//! tokenUsage endpoint only answers when no active lot exists, and pending
//! fuel packages ride along as a secondary row.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "longcat";
const NAME: &str = "LongCat";
const BASE: &str = "https://longcat.chat";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["LONGCAT_COOKIE"]).map(|_| "Pane LongCat session".into())
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(cookie) = stored_api_key(ID, &["LONGCAT_COOKIE"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a longcat.chat Cookie header (or session token) in Settings (gear icon).",
        ));
    };
    fetch_with_credential(&cookie).await
}

/// Live test of a pasted credential (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_credential(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

fn cookie_header(cred: &str) -> String {
    let cred = cred.trim();
    if cred.contains("=") {
        cred.trim_start_matches("Cookie:").trim().to_string()
    } else {
        format!("__Secure-authjs.session-token={cred}")
    }
}

async fn request(cookie: &str, method_get: bool, path: &str) -> Result<Value, String> {
    let mut req = http()
        .request(if method_get { reqwest::Method::GET } else { reqwest::Method::POST }, format!("{BASE}{path}"))
        .header("Cookie", cookie)
        .header("Accept", "application/json")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/143.0.0.0 Safari/537.36");
    if !method_get {
        req = req.header("Content-Type", "application/json").body("{}");
    }
    let resp = req.send().await.map_err(|e| format!("{path}: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(
            "LongCat session expired — paste a fresh Cookie header in Settings (gear icon)".into(),
        );
    }
    if !status.is_success() {
        return Err(format!("{path}: HTTP {status}"));
    }
    resp.json().await.map_err(|e| format!("{path} parse: {e}"))
}

async fn fetch_with_credential(cred: &str) -> Result<Snapshot, String> {
    let cookie = cookie_header(cred);
    // Validates the session; the account name is nice-to-have.
    let account = request(&cookie, true, "/api/v1/user-current").await.ok();
    let name = account
        .as_ref()
        .and_then(|a| a.pointer("/user/name").or_else(|| a.get("name")))
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut primary: Option<(f64, f64)> = None; // (total, used)
    if let Ok(summary) = request(
        &cookie,
        false,
        "/api/pay/quota/metering/token-packs/summary",
    )
    .await
    {
        let lot = summary.get("currentLot");
        let active = lot
            .and_then(|l| l.get("status"))
            .and_then(Value::as_str)
            .map(|s| s.eq_ignore_ascii_case("ACTIVE"))
            .unwrap_or(false);
        if let Some(l) = lot.filter(|_| active) {
            let total = l.get("totalToken").and_then(Value::as_f64).unwrap_or(0.0);
            if total > 0.0 {
                let used = l
                    .get("consumedToken")
                    .and_then(Value::as_f64)
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or("active token pack missing consumedToken")?;
                primary = Some((total, used));
            }
        }
    }
    if primary.is_none() {
        // Legacy surface — can report stale zeros for pack accounts, so it
        // only answers when the summary had no active lot.
        let usage = request(&cookie, true, "/api/lc-platform/v1/tokenUsage").await?;
        let u = usage.get("usage").unwrap_or(&usage);
        let total = u
            .get("totalToken")
            .and_then(Value::as_f64)
            .ok_or("tokenUsage was missing totalToken")?;
        let used = u
            .get("usedToken")
            .and_then(Value::as_f64)
            .or_else(|| {
                u.get("availableToken")
                    .and_then(Value::as_f64)
                    .map(|remaining| total - remaining)
            })
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or("tokenUsage missing usedToken and availableToken")?;
        primary = Some((total, used));
    }

    let (total, used) = primary.ok_or("no usable quota in response")?;
    if !total.is_finite() || total <= 0.0 {
        return Err("token quota has no positive total".into());
    }
    let mut metrics = vec![Metric::progress(
        "Tokens",
        (used / total * 100.0).clamp(0.0, 100.0),
        Some(format!(
            "{} of {} tokens",
            fmt_tokens(used),
            fmt_tokens(total)
        )),
    )];

    // Pending fuel packages: remaining tokens + nearest expiry.
    if let Ok(fuel) = request(&cookie, true, "/api/lc-platform/v1/pending-fuel-packages").await {
        let mut remaining = 0.0;
        let mut count = 0;
        let mut soonest: Option<i64> = None;
        for pack in fuel
            .get("list")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(avail) = pack.get("availableToken").and_then(Value::as_f64) {
                remaining += avail;
                count += 1;
            }
            let expiry = ["expireTime", "expire_time", "expiresAt"]
                .iter()
                .find_map(|k| pack.get(*k))
                .and_then(|v| match v {
                    Value::Number(n) => n.as_f64().map(|n| {
                        if n > 1e12 {
                            n as i64
                        } else {
                            (n * 1000.0) as i64
                        }
                    }),
                    Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|d| d.timestamp_millis()),
                    _ => None,
                });
            if let Some(e) = expiry {
                soonest = Some(soonest.map_or(e, |prev| prev.min(e)));
            }
        }
        if count > 0 {
            metrics.push(
                Metric::text(
                    "Fuel packs",
                    format!("{} tokens left", fmt_tokens(remaining)),
                )
                .with_reset(soonest, None),
            );
        }
    }
    Ok(Snapshot::ok(ID, NAME, name, metrics))
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
    fn cookie_header_takes_full_header_or_bare_token() {
        assert_eq!(cookie_header("Cookie: a=1; b=2"), "a=1; b=2");
        assert_eq!(cookie_header("a=1"), "a=1");
        assert_eq!(
            cookie_header("raw-token-value"),
            "__Secure-authjs.session-token=raw-token-value"
        );
    }
}
