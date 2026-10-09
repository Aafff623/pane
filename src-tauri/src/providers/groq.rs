//! Groq — API-key card reading the rate-limit response headers of a plain
//! `GET /models` call (mechanism per openusage's Groq provider): Groq's
//! public API answers every request with `x-ratelimit-*-day` headers
//! describing the key's daily request/token budgets.

use super::{http, stored_api_key, Metric, Snapshot};

const ID: &str = "groq";
const NAME: &str = "Groq";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["GROQ_API_KEY"]).map(|_| "Pane Groq API key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key(ID, &["GROQ_API_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Groq API key (console.groq.com/keys) in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let resp = http()
        .get("https://api.groq.com/openai/v1/models")
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("models request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err("key was rejected — paste a fresh Groq API key in Settings (gear icon)".into());
    }
    if !resp.status().is_success() {
        return Err(format!("models endpoint: HTTP {}", resp.status()));
    }
    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .filter_map(|(k, v)| Some((k.to_string(), v.to_str().ok()?.to_string())))
        .collect();
    parse_headers(&headers)
}

/// x-ratelimit headers → daily progress rows. The reset header is a duration
/// or a timestamp depending on Groq's mood; accept both spellings.
fn parse_headers(headers: &[(String, String)]) -> Result<Snapshot, String> {
    let get = |name: &str| -> Option<String> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    let parse_reset = |raw: Option<String>| -> Option<i64> {
        let raw = raw?;
        if let Ok(secs) = raw.parse::<f64>() {
            // Groq sends remaining-seconds or an epoch; seconds dominate.
            if secs < 1e9 {
                return Some(chrono::Utc::now().timestamp_millis() + (secs * 1000.0) as i64);
            }
            return Some(if secs > 1e12 { secs as i64 } else { (secs * 1000.0) as i64 });
        }
        chrono::DateTime::parse_from_rfc3339(&raw).ok().map(|d| d.timestamp_millis())
    };

    let mut metrics = Vec::new();
    let req_limit = get("x-ratelimit-limit-requests-day").and_then(|v| v.parse::<f64>().ok());
    let req_left = get("x-ratelimit-remaining-requests-day").and_then(|v| v.parse::<f64>().ok());
    if let (Some(limit), Some(left)) = (req_limit, req_left) {
        if limit > 0.0 {
            let used = (limit - left).max(0.0);
            metrics.push(
                Metric::progress(
                    "Requests / day",
                    (used / limit * 100.0).clamp(0.0, 100.0),
                    Some(format!("{:.0} of {:.0} requests", used, limit)),
                )
                .with_reset(parse_reset(get("x-ratelimit-reset-requests-day")), None),
            );
        }
    }
    let tok_limit = get("x-ratelimit-limit-tokens-day").and_then(|v| v.parse::<f64>().ok());
    let tok_left = get("x-ratelimit-remaining-tokens-day").and_then(|v| v.parse::<f64>().ok());
    if let (Some(limit), Some(left)) = (tok_limit, tok_left) {
        if limit > 0.0 {
            let used = (limit - left).max(0.0);
            metrics.push(
                Metric::progress(
                    "Tokens / day",
                    (used / limit * 100.0).clamp(0.0, 100.0),
                    Some(format!("{} of {} tokens", fmt(used), fmt(limit))),
                )
                .with_reset(parse_reset(get("x-ratelimit-reset-tokens-day")), None),
            );
        }
    }
    if metrics.is_empty() {
        return Err("response carried no daily rate-limit headers".into());
    }
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

fn fmt(n: f64) -> String {
    if n >= 1e6 {
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
    fn daily_request_and_token_budgets_become_rows() {
        let headers = vec![
            ("x-ratelimit-limit-requests-day".into(), "14400".into()),
            ("x-ratelimit-remaining-requests-day".into(), "10800".into()),
            ("x-ratelimit-reset-requests-day".into(), "3600".into()),
            ("x-ratelimit-limit-tokens-day".into(), "200000".into()),
            ("x-ratelimit-remaining-tokens-day".into(), "50000".into()),
        ];
        let snap = parse_headers(&headers).unwrap();
        assert_eq!(snap.status, "ok");
        let req = &snap.metrics[0];
        assert_eq!(req.label, "Requests / day");
        assert_eq!(req.used_percent, Some(25.0));
        assert!(req.resets_at.is_some());
        let tokens = &snap.metrics[1];
        assert_eq!(tokens.used_percent, Some(75.0));
        assert_eq!(tokens.detail.as_deref(), Some("150.0K of 200.0K tokens"));
    }

    #[test]
    fn no_headers_is_an_error() {
        assert!(parse_headers(&[]).is_err());
        assert!(parse_headers(&[("content-type".into(), "application/json".into())]).is_err());
    }
}
