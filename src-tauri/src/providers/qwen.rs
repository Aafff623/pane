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

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
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

/// The console RPC speaks whichever auth header it happens to accept;
/// try the common spellings against both regions, first hit wins.
async fn fetch_quota(key: &str) -> Option<Snapshot> {
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
                continue;
            }
            let Ok(doc) = resp.json::<Value>().await else { continue };
            if let Some(snap) = parse_quota(&doc) {
                return Some(snap);
            }
        }
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
    let (mut day_req, mut day_tok, mut mon_req, mut mon_tok) = (0u64, 0f64, 0u64, 0f64);
    for line in raw.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let tokens = v.get("totalTokens").and_then(Value::as_f64).unwrap_or(0.0);
        mon_req += 1;
        mon_tok += tokens;
        if v.get("localDate").and_then(Value::as_str) == Some(today.as_str()) {
            day_req += 1;
            day_tok += tokens;
        }
    }
    if mon_req == 0 {
        return None;
    }
    let fmt = |req: u64, tok: f64| format!("{req} requests · {:.1}M tokens", tok / 1e6);
    Some(Snapshot::ok(
        ID,
        NAME,
        None,
        vec![
            Metric::text("Today", fmt(day_req, day_tok)),
            Metric::text("This month", fmt(mon_req, mon_tok)),
        ],
    ))
}
