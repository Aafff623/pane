//! StepFun Step Plan subscription quota.
use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;
const ID: &str = "stepfun-plan";
const NAME: &str = "StepFun Step Plan";
const RATE_URL: &str =
    "https://platform.stepfun.com/api/step.openapi.devcenter.Dashboard/QueryStepPlanRateLimit";
const STATUS_URL: &str =
    "https://platform.stepfun.com/api/step.openapi.devcenter.Dashboard/GetStepPlanStatus";
pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &[]).map(|_| "Pane StepFun Step Plan session".into())
}
pub async fn snapshot() -> Snapshot {
    let Some(token) = stored_api_key(ID, &[]) else {
        return Snapshot::no_credentials(ID, NAME, "Sign in to StepFun Step Plan in Settings.");
    };
    match fetch(&token).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub async fn snapshot_with_key(token: &str) -> Snapshot {
    match fetch(token.trim()).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}
/// The stored token is "<access JWT>...<device JWT>". The platform rejects
/// a bare token as "embezzled" — requests must also carry Oasis-Webid with
/// the device_id from the trailing (device) JWT's payload.
fn webid_from_token(token: &str) -> Option<String> {
    use base64::Engine;
    let jwt = token.split("...").last()?;
    let payload_b64 = jwt.split('.').nth(1)?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64))
        .ok()?;
    let v: Value = serde_json::from_slice(&payload).ok()?;
    v.get("device_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

async fn post(token: &str, url: &str) -> Result<Value, String> {
    let webid = webid_from_token(token);
    let cookie = match &webid {
        Some(w) => format!("Oasis-Token={token}; Oasis-Webid={w}"),
        None => format!("Oasis-Token={token}"),
    };
    let mut req = http()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Oasis-Token", token)
        .header("Cookie", cookie)
        .header("Oasis-Appid", "10300")
        .header("Oasis-Platform", "web");
    if let Some(w) = &webid {
        req = req.header("Oasis-Webid", w);
    }
    let r = req
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("Step Plan request: {e}"))?;
    if matches!(r.status().as_u16(), 401 | 403) {
        return Err("Step Plan session expired — sign in again".into());
    }
    if !r.status().is_success() {
        return Err(format!("Step Plan endpoint: HTTP {}", r.status()));
    }
    super::json_body(r, 64 * 1024, "Step Plan").await
}
fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(|x| x.as_f64().or_else(|| x.as_str()?.parse().ok()))
}
fn ts(v: Option<&Value>) -> Option<i64> {
    v.and_then(|x| x.as_i64().or_else(|| x.as_str()?.parse().ok()))
        .map(|s| if s < 2_000_000_000_000 { s * 1000 } else { s })
}
async fn fetch(token: &str) -> Result<Snapshot, String> {
    let rate = post(token, RATE_URL).await?;
    let status = post(token, STATUS_URL).await.ok();
    let mut m = Vec::new();
    let c = rate.get("plan_credit_rate_limit");
    if let Some(left) = num(c.and_then(|x| x.get("subscription_credit_left_rate"))) {
        m.push(
            Metric::progress(
                "Credit",
                (1.0 - left) * 100.0,
                Some(format!("{:.1}% remaining", left * 100.0)),
            )
            .with_reset(
                ts(c.and_then(|x| x.get("subscription_credit_reset_time"))),
                Some(30 * 86_400_000),
            ),
        );
    } else if let Some(left) = num(rate.get("five_hour_usage_left_rate")) {
        m.push(
            Metric::progress("Session", (1.0 - left) * 100.0, None).with_reset(
                ts(rate.get("five_hour_usage_reset_time")),
                Some(5 * 60 * 60 * 1000),
            ),
        );
        if let Some(w) = num(rate.get("weekly_usage_left_rate")) {
            m.push(
                Metric::progress("Weekly", (1.0 - w) * 100.0, None).with_reset(
                    ts(rate.get("weekly_usage_reset_time")),
                    Some(7 * 86_400_000),
                ),
            );
        }
    }
    if m.is_empty() {
        return Err("Step Plan returned no quota data".into());
    }
    let plan = status
        .and_then(|v| {
            v.pointer("/subscription/name")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| Some("Step Plan".into()));
    Ok(Snapshot::ok(ID, NAME, plan, m))
}
#[cfg(test)]
mod tests {
    use super::webid_from_token;

    #[test]
    fn separate_id() {
        assert_eq!(super::ID, "stepfun-plan");
    }

    #[test]
    fn webid_extracts_from_device_jwt() {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"app_id":10300,"device_id":"fbf12e2b1b5d3a3a57de1cfa87c069089a365ab0","platform":"web"}"#);
        let token = format!("aaa.bbb.ccc...header.{}.sig", payload);
        assert_eq!(
            webid_from_token(&token).as_deref(),
            Some("fbf12e2b1b5d3a3a57de1cfa87c069089a365ab0"),
        );
    }

    #[test]
    fn webid_absent_for_bare_token() {
        assert_eq!(webid_from_token("header.payload.sig"), None);
        assert_eq!(webid_from_token(""), None);
    }
}
