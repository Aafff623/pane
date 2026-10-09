//! 火山方舟 (Volcengine Ark) — Coding Plan / Agent Plan quota via Volcengine's
//! V4-signed OpenAPI (signature chain ported from coding-plan-dashboard's
//! `volcengine_sign`; same HMAC/sha2 crates Bedrock uses).
//!
//! Credentials: AccessKeyId + SecretAccessKey from ark.volcengine.com,
//! pasted as `AK:SK` in one slot (or VOLC_ACCESS_KEY / VOLC_SECRET_KEY).
//! Both plan surfaces are probed: GetAgentPlanAFPUsage (AFP 5h/weekly/
//! monthly windows) and GetCodingPlanUsage (per-level Percent windows).

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::{json, Value};

const ID: &str = "volcengine";
const NAME: &str = "Volcengine Ark";
const HOST: &str = "open.volcengineapi.com";
const REGION: &str = "cn-beijing";
const SERVICE: &str = "ark";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, &["VOLC_ACCESS_KEY"]).map(|_| "Pane Volcengine AK/SK".into())
}

/// Live test of a pasted credential (Customize "Test"); never saved here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_credential(key).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let (env_ak, env_sk) = (
        std::env::var("VOLC_ACCESS_KEY").ok().filter(|s| !s.trim().is_empty()),
        std::env::var("VOLC_SECRET_KEY").ok().filter(|s| !s.trim().is_empty()),
    );
    if let (Some(ak), Some(sk)) = (env_ak, env_sk) {
        return fetch_with(&ak, &sk).await;
    }
    let Some(cred) = stored_api_key(ID, &["VOLC_ACCESS_KEY"]) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste your Volcengine AK:SK (ark.volcengine.com → API key account) in Settings (gear icon).",
        ));
    };
    fetch_with_credential(&cred).await
}

/// The stored slot carries `AK:SK` together (one paste, one secret store).
fn split_credential(cred: &str) -> Result<(String, String), String> {
    let (ak, sk) = cred.trim().split_once(':').ok_or(
        "paste the credential as AK:SK — AccessKeyId:SecretAccessKey from ark.volcengine.com",
    )?;
    let (ak, sk) = (ak.trim(), sk.trim());
    if ak.is_empty() || sk.is_empty() {
        return Err("AK:SK paste is missing one half".into());
    }
    Ok((ak.into(), sk.into()))
}

async fn fetch_with_credential(cred: &str) -> Result<Snapshot, String> {
    let (ak, sk) = split_credential(cred)?;
    fetch_with(&ak, &sk).await
}

async fn call(ak: &str, sk: &str, action: &str) -> Result<Value, String> {
    let query = norm_query(&[("Action", action), ("Version", "2024-01-01")]);
    let body = "{}";
    let headers = sign(ak, sk, "POST", "/", &query, body);
    let mut req = http().post(format!("https://{HOST}/?{query}")).body(body.to_string());
    for (k, v) in headers {
        req = req.header(k, v);
    }
    let resp = req.send().await.map_err(|e| format!("{action} request: {e}"))?;
    let status = resp.status();
    let doc: Value = resp.json().await.map_err(|e| format!("{action} parse: {e}"))?;
    if !status.is_success() {
        // Volcengine errors carry a message under ResponseMetadata.
        let msg = doc
            .pointer("/ResponseMetadata/Error/Message")
            .or_else(|| doc.pointer("/ResponseMetadata/Error/Code"))
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(format!("{action}: HTTP {status} {msg}"));
    }
    Ok(doc)
}

async fn fetch_with(ak: &str, sk: &str) -> Result<Snapshot, String> {
    let afp = call(ak, sk, "GetAgentPlanAFPUsage").await;
    let coding = call(ak, sk, "GetCodingPlanUsage").await;

    let mut metrics = Vec::new();
    if let Ok(doc) = &afp {
        metrics = afp_metrics(doc);
    }
    if metrics.is_empty() {
        if let Ok(doc) = &coding {
            metrics = coding_metrics(doc);
        }
    }
    if metrics.is_empty() {
        // Surface whichever call actually failed — both empty means the key
        // likely has no plan at all.
        let err = |r: &Result<Value, String>| r.as_ref().err().cloned().unwrap_or_default();
        let afp_err = err(&afp);
        let detail = if afp_err.is_empty() { err(&coding) } else { afp_err };
        if detail.is_empty() {
            return Err("no plan quota found for this AK/SK".into());
        }
        return Err(detail);
    }
    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

/// Agent Plan: Result.{AFPFiveHour,AFPWeekly,AFPMonthly}{Quota,Used,ResetTime}.
fn afp_metrics(doc: &Value) -> Vec<Metric> {
    let Some(result) = doc.get("Result") else { return vec![] };
    let mut out = Vec::new();
    for (key, label) in [("AFPFiveHour", "5-hour"), ("AFPWeekly", "Weekly"), ("AFPMonthly", "Monthly")] {
        let Some(item) = result.get(key) else { continue };
        let quota = item.get("Quota").and_then(Value::as_f64).unwrap_or(0.0);
        let used = item.get("Used").and_then(Value::as_f64).unwrap_or(0.0);
        if quota <= 0.0 && used <= 0.0 {
            continue;
        }
        let pct = if quota > 0.0 { (used / quota * 100.0).clamp(0.0, 100.0) } else { 100.0 };
        let reset = item
            .get("ResetTime")
            .and_then(Value::as_f64)
            .filter(|t| *t > 0.0)
            .map(|t| if t > 1e12 { t as i64 } else { (t * 1000.0) as i64 });
        out.push(
            Metric::progress(label, pct, Some(format!("{used:.0} of {quota:.0} credits")))
                .with_reset(reset, None),
        );
    }
    out
}

/// Coding Plan: Result.QuotaUsage[]{Level, Percent, ResetTimestamp(秒)}.
fn coding_metrics(doc: &Value) -> Vec<Metric> {
    let Some(list) = doc.pointer("/Result/QuotaUsage").and_then(Value::as_array) else {
        return vec![];
    };
    let mut out = Vec::new();
    for item in list {
        let level = item.get("Level").and_then(Value::as_str).unwrap_or("").to_lowercase();
        let Some(mut pct) = item.get("Percent").and_then(Value::as_f64) else { continue };
        if pct <= 1.0 {
            pct *= 100.0; // ratios arrive as 0…1
        }
        let reset = item
            .get("ResetTimestamp")
            .and_then(Value::as_f64)
            .filter(|t| *t > 0.0)
            .map(|t| if t > 1e12 { t as i64 } else { (t * 1000.0) as i64 });
        out.push(
            Metric::progress(&level, pct.clamp(0.0, 100.0), Some(format!("{pct:.0}% used")))
                .with_reset(reset, None),
        );
    }
    out
}

// ── V4 signing (volcengine flavour: HMAC-SHA256 chain, scope suffix
//    "request"; canonical query percent-encoded except -_.~) ────────────────

fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn norm_query(params: &[(&str, &str)]) -> String {
    let mut sorted: Vec<(&str, &str)> = params.to_vec();
    sorted.sort();
    sorted
        .iter()
        .map(|(k, v)| format!("{}={}", pct_encode(k), pct_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn hmac_sha256(key: &[u8], data: &str) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn sign(ak: &str, sk: &str, method: &str, path: &str, query: &str, body: &str) -> Vec<(String, String)> {
    use sha2::{Digest, Sha256};
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let now = chrono::Utc::now();
    let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date = &x_date[..8];
    let body_hash = hex(&Sha256::digest(body.as_bytes()));

    // Signed headers, sorted, lowercased (host included, like Bedrock).
    let signed: [(&str, String); 4] = [
        ("content-type", "application/json".into()),
        ("host", HOST.into()),
        ("x-content-sha256", body_hash.clone()),
        ("x-date", x_date.clone()),
    ];
    let canonical_headers: String = signed.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    let signed_headers = signed.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(";");

    let canonical_request = format!(
        "{method}\n{path}\n{query}\n{canonical_headers}\n{signed_headers}\n{body_hash}"
    );
    let scope = format!("{date}/{REGION}/{SERVICE}/request");
    let string_to_sign = format!(
        "HMAC-SHA256\n{x_date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical_request.as_bytes()))
    );
    let k_date = hmac_sha256(sk.as_bytes(), date);
    let k_region = hmac_sha256(&k_date, REGION);
    let k_service = hmac_sha256(&k_region, SERVICE);
    let k_signing = hmac_sha256(&k_service, "request");
    let signature = hex(&hmac_sha256(&k_signing, &string_to_sign));

    vec![
        ("Content-Type".into(), "application/json".into()),
        ("X-Date".into(), x_date),
        ("X-Content-Sha256".into(), body_hash),
        (
            "Authorization".into(),
            format!("HMAC-SHA256 Credential={ak}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ak_sk_paste_splits_and_validates() {
        let (ak, sk) = split_credential("AKTP-abc : c2VjcmV0").unwrap();
        assert_eq!(ak, "AKTP-abc");
        assert_eq!(sk, "c2VjcmV0");
        assert!(split_credential("no-separator").is_err());
        assert!(split_credential(":onlysk").is_err());
    }

    #[test]
    fn query_normalization_sorts_and_percent_encodes() {
        let q = norm_query(&[("Version", "2024-01-01"), ("Action", "Get Coding/Plan")]);
        assert_eq!(q, "Action=Get%20Coding%2FPlan&Version=2024-01-01");
    }

    #[test]
    fn signature_carries_the_volcengine_chain() {
        let headers = sign("AKTEST", "SKTEST", "POST", "/", "Action=X&Version=2024-01-01", "{}");
        let get = |k: &str| {
            headers.iter().find(|(h, _)| h == k).map(|(_, v)| v.clone()).unwrap()
        };
        let auth = get("Authorization");
        assert!(auth.starts_with("HMAC-SHA256 Credential=AKTEST/20"));
        assert!(auth.contains("/cn-beijing/ark/request"));
        assert!(auth.contains("SignedHeaders=content-type;host;x-content-sha256;x-date"));
        assert_eq!(get("X-Date").len(), 16);
        // Signature is a full sha256 hex.
        let sig = auth.rsplit("Signature=").next().unwrap();
        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn afp_periods_parse_with_resets() {
        let doc = json!({ "Result": {
            "AFPFiveHour": { "Quota": 100, "Used": 40, "ResetTime": 1_768_435_200 },
            "AFPWeekly": { "Quota": 600, "Used": 300, "ResetTime": 1_768_435_200 },
            "AFPMonthly": { "Quota": 0, "Used": 0 }
        }});
        let metrics = afp_metrics(&doc);
        assert_eq!(metrics.len(), 2); // empty monthly dropped
        assert_eq!(metrics[0].label, "5-hour");
        assert_eq!(metrics[0].used_percent, Some(40.0));
        assert_eq!(metrics[0].resets_at, Some(1_768_435_200_000));
    }

    #[test]
    fn coding_plan_windows_scale_ratio_percents() {
        let doc = json!({ "Result": { "QuotaUsage": [
            { "Level": "FREE_5H", "Percent": 0.42, "ResetTimestamp": 1_768_435_200 },
            { "Level": "WEEK", "Percent": 61.0 }
        ]}});
        let metrics = coding_metrics(&doc);
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].label, "free_5h");
        assert_eq!(metrics[0].used_percent, Some(42.0)); // ratio scaled
        assert_eq!(metrics[1].used_percent, Some(61.0)); // raw percent kept
    }
}
