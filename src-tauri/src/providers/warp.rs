use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::{json, Value};

const ID: &str = "warp";
const NAME: &str = "Warp";

// The GraphQL query CodexBar ships; the edge 429s without `User-Agent: Warp/1.0`.
const QUERY: &str = "query GetRequestLimitInfo($requestContext: RequestContext!){ user(requestContext:$requestContext){ __typename ... on UserOutput { user { requestLimitInfo { isUnlimited nextRefreshTime requestLimit requestsUsedSinceLastRefresh } bonusGrants { requestCreditsGranted requestCreditsRemaining expiration } workspaces { bonusGrantsInfo { grants { requestCreditsGranted requestCreditsRemaining expiration } } } } } } }";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

fn parse_iso_ms(v: Option<&Value>) -> Option<i64> {
    let s = v?.as_str()?;
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis())
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["WARP_API_KEY", "WARP_TOKEN"]).map(|_| "Pane Warp API key".into())
}

/// Live test of a pasted key (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = stored_api_key("warp", &["WARP_API_KEY", "WARP_TOKEN"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Warp API key (docs.warp.dev → API keys) in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key).await
}

async fn fetch_with_key(key: &str) -> Result<Snapshot, String> {
    let body = json!({
        "query": QUERY,
        "variables": { "requestContext": { "clientContext": {}, "osContext": {
            "category": "Windows", "name": "Windows", "version": "11" } } },
        "operationName": "GetRequestLimitInfo"
    });
    let resp = http()
        .post("https://app.warp.dev/graphql/v2?op=GetRequestLimitInfo")
        .header("User-Agent", "Warp/1.0")
        .header("x-warp-client-id", "warp-app")
        .header("x-warp-os-category", "Windows")
        .header("x-warp-os-name", "Windows")
        .header("x-warp-os-version", "11")
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("graphql request: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("key was rejected — paste a fresh key in Settings (gear icon)".into());
    }
    if !resp.status().is_success() {
        return Err(format!("graphql endpoint: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("graphql parse: {e}"))?;
    if let Some(errors) = doc.get("errors").and_then(Value::as_array) {
        let msgs: Vec<&str> =
            errors.iter().filter_map(|e| e.get("message").and_then(Value::as_str)).take(3).collect();
        if !msgs.is_empty() {
            return Err(format!("GraphQL: {}", msgs.join("; ")));
        }
    }
    parse_usage(&doc)
}

/// The GraphQL payload → card rows: requestLimitInfo drives the Requests
/// bar, personal and workspace bonus grants are summed into one Bonus bar.
fn parse_usage(doc: &Value) -> Result<Snapshot, String> {
    let user = doc
        .pointer("/data/user/user")
        .ok_or("no user payload in response (is the key valid?)")?;
    let info = user.get("requestLimitInfo").ok_or("no requestLimitInfo in response")?;
    let unlimited = match info.get("isUnlimited") {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        _ => false,
    };
    let limit = info.get("requestLimit").and_then(Value::as_f64).unwrap_or(0.0);
    let used = info.get("requestsUsedSinceLastRefresh").and_then(Value::as_f64).unwrap_or(0.0);
    let resets_at = parse_iso_ms(info.get("nextRefreshTime"));

    let mut metrics = Vec::new();
    if unlimited {
        metrics.push(Metric::text("Requests", "Unlimited".into()));
    } else if limit > 0.0 {
        metrics.push(
            Metric::progress(
                "Requests",
                (used / limit * 100.0).clamp(0.0, 100.0),
                Some(format!("{used:.0} of {limit:.0} credits used")),
            )
            .with_reset(resets_at, None),
        );
    }

    // Personal bonus grants + every workspace's grants, summed.
    let mut granted = 0.0f64;
    let mut remaining = 0.0f64;
    let mut next_expiry: Option<i64> = None;
    let mut eat = |g: &Value| {
        let total = g.get("requestCreditsGranted").and_then(Value::as_f64).unwrap_or(0.0);
        let left = g.get("requestCreditsRemaining").and_then(Value::as_f64).unwrap_or(0.0);
        granted += total;
        remaining += left;
        if left > 0.0 {
            if let Some(exp) = parse_iso_ms(g.get("expiration")) {
                next_expiry = Some(next_expiry.map_or(exp, |e: i64| e.min(exp)));
            }
        }
    };
    for g in user.get("bonusGrants").and_then(Value::as_array).unwrap_or(&vec![]) {
        eat(g);
    }
    for ws in user.get("workspaces").and_then(Value::as_array).unwrap_or(&vec![]) {
        for g in ws
            .pointer("/bonusGrantsInfo/grants")
            .and_then(Value::as_array)
            .unwrap_or(&vec![])
        {
            eat(g);
        }
    }
    if granted > 0.0 {
        metrics.push(
            Metric::progress(
                "Bonus credits",
                ((granted - remaining) / granted * 100.0).clamp(0.0, 100.0),
                Some(format!("{remaining:.0} bonus credits left")),
            )
            .with_reset(next_expiry, None),
        );
    }

    if metrics.is_empty() {
        return Err("no request-limit data in response".into());
    }
    let plan = unlimited.then(|| "Unlimited".to_string());
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit_doc(unlimited: Value) -> Value {
        json!({
            "data": { "user": { "__typename": "UserOutput", "user": {
                "requestLimitInfo": {
                    "isUnlimited": unlimited,
                    "requestLimit": 1000,
                    "requestsUsedSinceLastRefresh": 250,
                    "nextRefreshTime": "2026-01-15T00:00:00Z"
                },
                "bonusGrants": [
                    { "requestCreditsGranted": 100, "requestCreditsRemaining": 60, "expiration": "2026-02-01T00:00:00Z" },
                    { "requestCreditsGranted": 50, "requestCreditsRemaining": 50, "expiration": "2026-02-10T00:00:00Z" }
                ],
                "workspaces": [ { "bonusGrantsInfo": { "grants": [
                    { "requestCreditsGranted": 40, "requestCreditsRemaining": 10, "expiration": "2026-01-20T00:00:00Z" }
                ] } } ]
            } } }
        })
    }

    #[test]
    fn requests_and_bonus_grants_read_from_graphql() {
        let snap = parse_usage(&limit_doc(json!(false))).unwrap();
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan, None);
        let requests = &snap.metrics[0];
        assert_eq!(requests.label, "Requests");
        assert!((requests.used_percent.unwrap() - 25.0).abs() < 1e-9);
        assert_eq!(requests.detail.as_deref(), Some("250 of 1000 credits used"));
        assert_eq!(requests.resets_at, Some(1_768_435_200_000));
        // granted 190 (100+50 personal, 40 workspace), 120 left → 70 spent.
        let bonus = &snap.metrics[1];
        assert_eq!(bonus.label, "Bonus credits");
        assert!((bonus.used_percent.unwrap() - (70.0 / 190.0 * 100.0)).abs() < 1e-9);
        assert_eq!(bonus.detail.as_deref(), Some("120 bonus credits left"));
        // Earliest expiry among grants with credits left wins (2026-01-20).
        assert_eq!(bonus.resets_at, Some(1_768_867_200_000));
    }

    #[test]
    fn unlimited_accounts_show_text_instead_of_a_bar() {
        let snap = parse_usage(&limit_doc(json!(true))).unwrap();
        assert_eq!(snap.plan.as_deref(), Some("Unlimited"));
        assert_eq!(snap.metrics[0].label, "Requests");
        assert_eq!(snap.metrics[0].value.as_deref(), Some("Unlimited"));
        // isUnlimited has also arrived as the string "true".
        let stringy = limit_doc(json!("true"));
        assert_eq!(parse_usage(&stringy).unwrap().plan.as_deref(), Some("Unlimited"));
    }

    #[test]
    fn a_payload_without_the_user_path_is_an_error() {
        assert!(parse_usage(&json!({ "data": { "user": {} } })).is_err());
    }
}
