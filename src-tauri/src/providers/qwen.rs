//! Qwen Code — Alibaba Model Studio's Coding Plan used through the Qwen
//! Code CLI. The plan meters *requests* (not tokens) across three windows:
//! a rolling 5-hour session, a week (Monday 00:00 UTC+8), and a monthly
//! cycle on the subscription renewal date.
//!
//! Quota comes from the Model Studio console's own RPC — the exact call
//! the Coding Plan page makes (no public quota API exists; approach
//! borrowed from CodexBar's alibaba-coding-plan notes). The endpoint's
//! accepted auth shapes vary, so three header spellings are tried against
//! both regional consoles. When none work, the card falls back to the
//! CLI's local usage ledger (`~/.qwen/usage/token-usage-YYYY-MM.jsonl`) so
//! there is always something truthful to show.
//!
//! Key source: pasted in Settings, `BAILIAN_TOKEN_PLAN_API_KEY` (the env
//! var Qwen Code itself reads), or `DASHSCOPE_API_KEY`.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;
use std::sync::atomic::{AtomicI64, Ordering};

const ID: &str = "qwen";
const NAME: &str = "Qwen Code";

const RPC_QUERY: &str = "data/api.json?action=zeldaEasy.broadscope-bailian.codingPlan.queryCodingPlanInstanceInfoV2&product=broadscope-bailian&api=queryCodingPlanInstanceInfoV2";
const CONSOLES: [&str; 2] = [
    "https://modelstudio.console.alibabacloud.com/", // international
    "https://bailian.console.aliyun.com/",           // China mainland
];

const HOUR_MS: i64 = 3_600_000;

fn find_api_key() -> Option<String> {
    stored_api_key(ID, &["BAILIAN_TOKEN_PLAN_API_KEY", "DASHSCOPE_API_KEY"])
}

/// No local credential source — the CLI's ledger carries usage, not a key;
/// Settings key and env vars are reported separately by
/// get_credential_status.
pub fn local_credential_hint() -> Option<String> {
    None
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a user-pasted key, without saving it (Customize "Test").
/// Tests the key against the console RPC only — the local-ledger fallback
/// says nothing about the key, so it never runs here.
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_quota(key).await {
        Some(snap) => snap,
        None => Snapshot::error(ID, NAME, "quota endpoint unreachable".into()),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let key = find_api_key();
    if let Some(key) = &key {
        if let Some(snap) = fetch_quota(key).await {
            return Ok(snap);
        }
    }
    if let Some(snap) = local_ledger() {
        return Ok(snap);
    }
    match key {
        Some(_) => Err("quota endpoint unreachable and no local Qwen Code usage found".into()),
        None => Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Set BAILIAN_TOKEN_PLAN_API_KEY (Qwen Code's own variable) or paste your sk-sp-… key in Settings.",
        )),
    }
}

/// The console currently answers ConsoleNeedLogin to every API-key auth
/// shape (it wants a browser session, which Pane will never touch). Keep
/// trying — Alibaba may open key auth someday — but after a failure stand
/// down for 6 hours so refreshes don't hammer their console.
static QUOTA_BLOCKED_UNTIL: AtomicI64 = AtomicI64::new(0);

/// The console RPC speaks whichever auth header it happens to accept;
/// try the common spellings against both regions, first hit wins.
async fn fetch_quota(key: &str) -> Option<Snapshot> {
    let now = chrono::Utc::now().timestamp();
    if now < QUOTA_BLOCKED_UNTIL.load(Ordering::Relaxed) {
        return None;
    }
    // Armed only when the console actually ANSWERED and refused (the
    // ConsoleNeedLogin case) — a DNS failure, timeout, or 5xx must not
    // silence quota checks for six hours after connectivity returns.
    let mut console_refused = false;
    for console in CONSOLES {
        for header in ["authorization", "x-api-key", "x-dashscope-api-key"] {
            let value = if header == "authorization" { format!("Bearer {key}") } else { key.to_string() };
            let resp = http()
                .post(format!("{console}{RPC_QUERY}"))
                .header(header, value)
                .header("accept", "application/json")
                .json(&serde_json::json!({}))
                .send()
                .await;
            let Ok(resp) = resp else { continue };
            if !resp.status().is_success() {
                // 401/403 is the console answering and refusing the key —
                // as final as ConsoleNeedLogin, so it arms the stand-down.
                // 5xx and everything else stays transient.
                if matches!(resp.status().as_u16(), 401 | 403) {
                    console_refused = true;
                }
                continue;
            }
            let Ok(doc) = resp.json::<Value>().await else { continue };
            if let Some(snap) = parse_quota(&doc) {
                // Working auth: lift the stand-down so quota stays live.
                QUOTA_BLOCKED_UNTIL.store(0, Ordering::Relaxed);
                return Some(snap);
            }
            // A parsed answer with no quota in it = the console received
            // the key and declined it.
            console_refused = true;
        }
    }
    if console_refused {
        QUOTA_BLOCKED_UNTIL.store(now + 6 * 3600, Ordering::Relaxed);
    }
    None
}

/// Depth-first search for the object carrying the quota fields — the RPC
/// wraps its payload in envelope layers we'd rather not hardcode.
fn find_object_with<'a>(v: &'a Value, marker: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) => {
            if m.contains_key(marker) {
                return Some(v);
            }
            m.values().find_map(|v| find_object_with(v, marker))
        }
        Value::Array(a) => a.iter().find_map(|v| find_object_with(v, marker)),
        _ => None,
    }
}

/// Numbers may arrive as JSON numbers or quoted strings; take either.
fn num(v: &Value, key: &str) -> Option<f64> {
    let f = v.get(key)?;
    f.as_f64().or_else(|| f.as_str().and_then(|s| s.trim().parse().ok()))
}

fn parse_quota(doc: &Value) -> Option<Snapshot> {
    let q = find_object_with(doc, "per5HourTotalQuota")?;
    let window = |label: &str, used_key: &str, total_key: &str, reset_key: &str, period: i64| {
        let total = num(q, total_key).filter(|t| *t > 0.0)?;
        let used = num(q, used_key).unwrap_or(0.0);
        let resets = num(q, reset_key).map(|ms| ms as i64).filter(|ms| *ms > 0);
        Some(
            Metric::progress(
                label,
                (used / total * 100.0).clamp(0.0, 100.0),
                Some(format!("{used:.0} of {total:.0} requests")),
            )
            .with_reset(resets, Some(period)),
        )
    };
    let metrics: Vec<Metric> = [
        window("Session", "per5HourUsedQuota", "per5HourTotalQuota", "per5HourQuotaNextRefreshTime", 5 * HOUR_MS),
        window("Weekly", "perWeekUsedQuota", "perWeekTotalQuota", "perWeekQuotaNextRefreshTime", 7 * 24 * HOUR_MS),
        window("Monthly", "perBillMonthUsedQuota", "perBillMonthTotalQuota", "perBillMonthQuotaNextRefreshTime", 30 * 24 * HOUR_MS),
    ]
    .into_iter()
    .flatten()
    .collect();
    if metrics.is_empty() {
        return None;
    }
    let plan = find_object_with(doc, "planName")
        .and_then(|o| o.get("planName").and_then(Value::as_str))
        .map(str::to_string)
        .or(Some("Coding Plan".into()));
    Some(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live diagnostic (ignored): shows what each console/header attempt
    /// returns so quota-auth failures can be debugged without ever
    /// printing the key. Run:
    ///   cargo test qwen_quota_live_probe -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn qwen_quota_live_probe() {
        let Some(key) = find_api_key() else {
            println!("no key found");
            return;
        };
        for console in CONSOLES {
            for header in ["authorization", "x-api-key", "x-dashscope-api-key"] {
                let value =
                    if header == "authorization" { format!("Bearer {key}") } else { key.clone() };
                let resp = http()
                    .post(format!("{console}{RPC_QUERY}"))
                    .header(header, value)
                    .header("accept", "application/json")
                    .json(&serde_json::json!({}))
                    .send()
                    .await;
                match resp {
                    Ok(r) => {
                        let status = r.status();
                        let body = r.text().await.unwrap_or_default();
                        let trimmed: String = body.chars().take(300).collect();
                        println!("{console} [{header}] -> {status}: {trimmed}");
                    }
                    Err(e) => println!("{console} [{header}] -> error: {e}"),
                }
            }
        }
        // Does the coding endpoint itself expose quota via response headers?
        for base in
            ["https://coding-intl.dashscope.aliyuncs.com/v1", "https://coding.dashscope.aliyuncs.com/v1"]
        {
            match http().get(format!("{base}/models")).bearer_auth(&key).send().await {
                Ok(r) => {
                    println!("{base}/models -> {}", r.status());
                    for (name, value) in r.headers() {
                        let n = name.as_str().to_ascii_lowercase();
                        if n.contains("limit") || n.contains("quota") || n.contains("remain") || n.contains("usage") {
                            println!("  header {n}: {:?}", value);
                        }
                    }
                }
                Err(e) => println!("{base}/models -> error: {e}"),
            }
        }
    }
}

/// Fallback card from the CLI's own per-request ledger: request and token
/// counts for today and the current month. No percentages — the plan's
/// limits aren't knowable locally.
fn local_ledger() -> Option<Snapshot> {
    let now = chrono::Local::now();
    let path = dirs::home_dir()?
        .join(".qwen")
        .join("usage")
        .join(format!("token-usage-{}.jsonl", now.format("%Y-%m")));
    let raw = std::fs::read_to_string(path).ok()?;
    let today = now.format("%Y-%m-%d").to_string();
    let (mut day_req, mut mon_req) = (0u64, 0u64);
    for line in raw.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v.get("totalTokens").and_then(Value::as_f64).unwrap_or(0.0) <= 0.0 {
            continue;
        }
        mon_req += 1;
        if v.get("localDate").and_then(Value::as_str) == Some(today.as_str()) {
            day_req += 1;
        }
    }
    if mon_req == 0 {
        return None;
    }
    // The plan bills per REQUEST, so counts are the headline; tokens and
    // dollars already live in the spend rows. Labels must not collide with
    // the spend rows ("Today", "Yesterday", "Last 30 Days") — the card
    // keeps one row per label and the spend row would swallow ours.
    Some(Snapshot::ok(
        ID,
        NAME,
        None,
        vec![
            Metric::text("Requests today", format!("{day_req}")),
            Metric::text("Requests this month", format!("{mon_req}")),
        ],
    ))
}
