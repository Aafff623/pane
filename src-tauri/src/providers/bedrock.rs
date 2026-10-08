use super::{http, Metric, Snapshot};
use serde_json::{json, Value};

const ID: &str = "bedrock";
const NAME: &str = "AWS Bedrock";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

struct AwsCreds {
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
}

/// Static env keys first; else, if a profile is configured, let the AWS CLI
/// resolve it (SSO, assume-role — we never parse ~/.aws ourselves).
async fn resolve_creds() -> Result<Option<AwsCreds>, String> {
    let akid = std::env::var("AWS_ACCESS_KEY_ID").ok().filter(|s| !s.is_empty());
    let secret = std::env::var("AWS_SECRET_ACCESS_KEY").ok().filter(|s| !s.is_empty());
    if let (Some(access_key), Some(secret_key)) = (akid, secret) {
        return Ok(Some(AwsCreds {
            access_key,
            secret_key,
            session_token: std::env::var("AWS_SESSION_TOKEN").ok().filter(|s| !s.is_empty()),
        }));
    }
    let Ok(profile) = std::env::var("AWS_PROFILE") else {
        return Ok(None);
    };
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        tokio::process::Command::new("aws")
            .args(["configure", "export-credentials", "--profile", &profile, "--format", "process"])
            .env_remove("AWS_PROFILE")
            .output(),
    )
    .await
    .map_err(|_| "aws CLI timed out".to_string())?
    .map_err(|e| format!("aws CLI not runnable: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.to_lowercase().contains("sso") || err.to_lowercase().contains("expired") {
            return Err(format!("AWS profile session expired — run `aws sso login --profile {profile}`"));
        }
        return Err(format!("aws export-credentials failed: {}", err.trim()));
    }
    let doc: Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("aws CLI output: {e}"))?;
    Ok(Some(AwsCreds {
        access_key: doc.get("AccessKeyId").and_then(Value::as_str).unwrap_or_default().into(),
        secret_key: doc.get("SecretAccessKey").and_then(Value::as_str).unwrap_or_default().into(),
        session_token: doc.get("SessionToken").and_then(Value::as_str).map(str::to_string),
    }))
}

/// Minimal SigV4 for a POST with a fixed path ("/") and no query string —
/// all Cost Explorer needs.
fn sigv4_headers(
    creds: &AwsCreds,
    service: &str,
    region: &str,
    host: &str,
    target: &str,
    content_type: &str,
    body: &str,
) -> Vec<(String, String)> {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    type HmacSha256 = Hmac<Sha256>;

    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let hmac = |key: &[u8], data: &str| {
        let mut m = HmacSha256::new_from_slice(key).unwrap();
        m.update(data.as_bytes());
        m.finalize().into_bytes().to_vec()
    };

    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date = now.format("%Y%m%d").to_string();
    let payload_hash = hex(&Sha256::digest(body.as_bytes()));

    // Canonical headers, sorted, lowercased.
    let mut headers: Vec<(String, String)> = vec![
        ("content-type".into(), content_type.to_string()),
        ("host".into(), host.to_string()),
        ("x-amz-content-sha256".into(), payload_hash.clone()),
        ("x-amz-date".into(), amz_date.clone()),
        ("x-amz-target".into(), target.to_string()),
    ];
    if let Some(tok) = &creds.session_token {
        headers.push(("x-amz-security-token".into(), tok.clone()));
    }
    headers.sort();

    let canonical_headers: String =
        headers.iter().map(|(k, v)| format!("{k}:{}\n", v.trim())).collect();
    let signed_headers: String =
        headers.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>().join(";");
    let canonical_request =
        format!("POST\n/\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let scope = format!("{date}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical_request.as_bytes()))
    );
    let k_date = hmac(format!("AWS4{}", creds.secret_key).as_bytes(), &date);
    let k_region = hmac(&k_date, region);
    let k_service = hmac(&k_region, service);
    let k_signing = hmac(&k_service, "aws4_request");
    let signature = hex(&hmac(&k_signing, &string_to_sign));

    let auth = format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        creds.access_key
    );
    let mut out: Vec<(String, String)> =
        headers.into_iter().filter(|(k, _)| k != "host").collect();
    out.push(("authorization".into(), auth));
    out
}

/// Sum this month's UnblendedCost over every group whose dimension key
/// mentions bedrock ("Amazon Bedrock", "Amazon Bedrock Agent", …).
fn bedrock_spend(doc: &Value) -> f64 {
    let mut spend = 0.0f64;
    for period in doc.get("ResultsByTime").and_then(Value::as_array).unwrap_or(&vec![]) {
        for group in period.get("Groups").and_then(Value::as_array).unwrap_or(&vec![]) {
            let is_bedrock = group
                .get("Keys")
                .and_then(Value::as_array)
                .and_then(|k| k.first())
                .and_then(Value::as_str)
                .is_some_and(|k| k.to_lowercase().contains("bedrock"));
            if is_bedrock {
                spend += group
                    .pointer("/Metrics/UnblendedCost/Amount")
                    .and_then(Value::as_str)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
        }
    }
    spend
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(creds) = resolve_creds().await? else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Set AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY or AWS_PROFILE to track Bedrock spend.",
        ));
    };

    // Cost Explorer lives in us-east-1 regardless of workload region.
    use chrono::Datelike;
    let now = chrono::Utc::now().date_naive();
    let start = now.with_day(1).unwrap_or(now);
    let end = now + chrono::Duration::days(1);
    let body = json!({
        "TimePeriod": { "Start": start.format("%Y-%m-%d").to_string(),
                        "End": end.format("%Y-%m-%d").to_string() },
        "Granularity": "MONTHLY",
        "Metrics": ["UnblendedCost"],
        "GroupBy": [{ "Type": "DIMENSION", "Key": "SERVICE" }]
    })
    .to_string();

    let host = "ce.us-east-1.amazonaws.com";
    let headers = sigv4_headers(
        &creds,
        "ce",
        "us-east-1",
        host,
        "AWSInsightsIndexService.GetCostAndUsage",
        "application/x-amz-json-1.1",
        &body,
    );
    let mut req = http().post(format!("https://{host}/")).body(body);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    let resp = req.send().await.map_err(|e| format!("cost explorer request: {e}"))?;
    let status = resp.status();
    let doc: Value = resp.json().await.map_err(|e| format!("cost explorer parse: {e}"))?;
    if status.as_u16() == 400
        && doc.get("__type").and_then(Value::as_str).is_some_and(|t| t.ends_with("DataUnavailableException"))
    {
        return Ok(Snapshot::ok(
            ID,
            NAME,
            None,
            vec![Metric::text("Bedrock this month", "$0.00 (no data yet)".into())],
        ));
    }
    if !status.is_success() {
        let t = doc.get("__type").and_then(Value::as_str).unwrap_or("");
        return Err(format!("cost explorer: HTTP {status} {t}"));
    }

    let spend = bedrock_spend(&doc);

    let mut metrics = vec![Metric::text("Bedrock this month", format!("${spend:.2}"))];
    // Optional budget → progress bar with a month-end reset.
    let budget = std::env::var("PANE_BEDROCK_BUDGET")
        .or_else(|_| std::env::var("CODEXBAR_BEDROCK_BUDGET"))
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|b| *b > 0.0);
    if let Some(budget) = budget {
        let month_end = {
            let next = if start.month() == 12 {
                chrono::NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)
            } else {
                chrono::NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1)
            };
            next.map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis())
        };
        metrics.insert(
            0,
            Metric::progress(
                "Monthly budget",
                (spend / budget * 100.0).clamp(0.0, 100.0),
                Some(format!("${spend:.2} of ${budget:.2}")),
            )
            .with_reset(month_end, Some(30 * 24 * 3600 * 1000)),
        );
    }
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_bedrock_groups_are_summed() {
        let doc = json!({
            "ResultsByTime": [ { "Groups": [
                { "Keys": ["Amazon Bedrock"], "Metrics": { "UnblendedCost": { "Amount": "12.50" } } },
                { "Keys": ["Amazon EC2"], "Metrics": { "UnblendedCost": { "Amount": "99.00" } } },
                { "Keys": ["Amazon Bedrock Agent"], "Metrics": { "UnblendedCost": { "Amount": "1.25" } } }
            ] } ]
        });
        assert!((bedrock_spend(&doc) - 13.75).abs() < 1e-9);
        // No groups / no ResultsByTime at all → $0, not an error.
        assert_eq!(bedrock_spend(&json!({})), 0.0);
    }

    #[test]
    fn sigv4_headers_carry_the_full_signing_chain() {
        let creds = AwsCreds {
            access_key: "AKIDEXAMPLE".into(),
            secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            session_token: Some("session".into()),
        };
        let headers = sigv4_headers(
            &creds,
            "ce",
            "us-east-1",
            "ce.us-east-1.amazonaws.com",
            "AWSInsightsIndexService.GetCostAndUsage",
            "application/x-amz-json-1.1",
            "{}",
        );
        let get = |k: &str| {
            headers.iter().find(|(h, _)| h == k).map(|(_, v)| v.clone()).unwrap()
        };
        let auth = get("authorization");
        assert!(auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"));
        assert!(auth.contains("/us-east-1/ce/aws4_request"));
        assert!(
            auth.contains("SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date;x-amz-security-token;x-amz-target"),
            "auth = {auth}"
        );
        assert!(auth.ends_with(|c: char| c.is_ascii_hexdigit()));
        assert_eq!(get("x-amz-target"), "AWSInsightsIndexService.GetCostAndUsage");
        assert_eq!(get("x-amz-date").len(), 16); // YYYYMMDDTHHMMSSZ
        // The host header is signed but never sent — reqwest sets its own.
        assert!(headers.iter().all(|(k, _)| k != "host"));
    }
}
