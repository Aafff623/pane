//! Search / MCP service quota providers (Bocha, Tavily, Firecrawl).
//!
//! All three expose read-only GET quota endpoints that cost nothing to
//! call, but the refresh loop ticks every minute — a 45-minute TTL cache
//! keeps real traffic at ~32 calls/day per provider. Keys come from the
//! environment; Firecrawl additionally reads the literal key from the
//! ZCode CLI MCP config because the env var still holds a legacy key
//! that expires 2026-10-16 (the config key is the active one).
//!
//! Deliberately NOT here: Brave (quota only visible in search response
//! headers of OTHER processes — Pane cannot observe them, and probing
//! would burn search quota), Exa (no key on this machine), IQS (Aliyun
//! AK signing + T+1 usage semantics), Keenable/MinerU/Liner (console-only,
//! no endpoint). See temp/handoff/20261002-mcp-quota-panel-handoff.md.

use super::{http, json_body, Metric, Snapshot};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(45 * 60);

fn cache() -> &'static Mutex<HashMap<String, (Instant, Snapshot)>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, (Instant, Snapshot)>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Fresh-snapshot gate shared by the three fetchers: serve the cached copy
/// while younger than the TTL, cache only ok results (failures retry next
/// cycle, same as every other provider).
fn fresh_or(id: &str) -> Option<Snapshot> {
    if let Ok(map) = cache().lock() {
        if let Some((at, snap)) = map.get(id) {
            if at.elapsed() < TTL {
                return Some(snap.clone());
            }
        }
    }
    None
}

fn remember_ok(id: &str, snap: &Snapshot) {
    if snap.status == "ok" {
        if let Ok(mut map) = cache().lock() {
            map.insert(id.into(), (Instant::now(), snap.clone()));
        }
    }
}

fn env_key(vars: &[&str]) -> Option<String> {
    vars.iter()
        .find_map(|v| std::env::var(v).ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()))
}

/// The active Firecrawl key: the ZCode CLI's MCP server config carries the
/// literal current key; env vars may still hold the legacy one.
fn firecrawl_key() -> Option<String> {
    let cfg = dirs::home_dir()?.join(".zcode").join("cli").join("config.json");
    let text = std::fs::read_to_string(cfg).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let key = v
        .pointer("/mcp/servers/firecrawl/env/FIRECRAWL_API_KEY")
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    (!key.is_empty()).then_some(key).or_else(|| {
        env_key(&["FIRECRAWL_FIRECRAWL_API_KEY", "FIRECRAWL_API_KEY"])
    })
}

fn bearer(url: &str, key: &str, what: &str) -> reqwest::RequestBuilder {
    http().get(url).bearer_auth(key.trim()).header("Accept", "application/json").header("User-Agent", format!("pane/{what}"))
}

// ── Bocha ───────────────────────────────────────────────────────────────────

pub async fn bocha_snapshot() -> Snapshot {
    if let Some(snap) = fresh_or("bocha") {
        return snap;
    }
    let Some(key) = env_key(&["BOCHA_API_KEY"]) else {
        return Snapshot::no_credentials("bocha", "BochaAI", "设置环境变量 BOCHA_API_KEY 后可查询余额。");
    };
    match bocha_fetch(&key).await {
        Ok(snap) => {
            remember_ok("bocha", &snap);
            snap
        }
        Err(e) => Snapshot::error("bocha", "BochaAI", e),
    }
}

/// `GET /v1/fund/remaining` — `data.remaining` is account balance in CNY
/// (yuan, not requests). A zero balance is a normal state, not an error.
async fn bocha_fetch(key: &str) -> Result<Snapshot, String> {
    let resp = bearer("https://api.bochaai.com/v1/fund/remaining", key, "bocha")
        .send()
        .await
        .map_err(|e| format!("博查请求失败: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("博查 API key 无效".into());
    }
    if !resp.status().is_success() {
        return Err(format!("博查接口返回 HTTP {}", resp.status()));
    }
    let body = json_body(resp, 64 * 1024, "BochaAI").await?;
    let remaining = body
        .pointer("/data/remaining")
        .and_then(Value::as_f64)
        .ok_or("博查返回中没有余额字段")?;
    let mut value = format!("¥{remaining:.2}");
    if remaining <= 0.0 {
        value.push_str(" — 余额耗尽 (open.bocha.cn)");
    }
    Ok(Snapshot::ok("bocha", "BochaAI", None, vec![Metric::text("Balance", value)]))
}

// ── Tavily ──────────────────────────────────────────────────────────────────

pub async fn tavily_snapshot() -> Snapshot {
    if let Some(snap) = fresh_or("tavily") {
        return snap;
    }
    let keys = tavily_keys();
    if keys.is_empty() {
        return Snapshot::no_credentials(
            "tavily",
            "Tavily",
            "设置 TAVILY_API_KEY（或把 key 写入 Pane 数据目录 tavily-keys.json）后可查询额度。",
        );
    }
    match tavily_fetch(&keys).await {
        Ok(snap) => {
            remember_ok("tavily", &snap);
            snap
        }
        Err(e) => Snapshot::error("tavily", "Tavily", e),
    }
}

/// Tavily keys: the Pane-managed list (`tavily-keys.json`, one JSON object
/// with a "keys" array) first, then the env var as a fallback. Deduped.
fn tavily_keys() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let path = super::config_dir().join("tavily-keys.json");
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            for k in v.get("keys").and_then(Value::as_array).into_iter().flatten() {
                if let Some(s) = k.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    if !out.iter().any(|x| x == s) {
                        out.push(s.to_string());
                    }
                }
            }
        }
    }
    if let Some(env) = env_key(&["TAVILY_API_KEY"]) {
        if !out.iter().any(|x| x == &env) {
            out.push(env);
        }
    }
    out
}

struct TavilyKeyUsage {
    used: f64,
    limit: Option<f64>,
    plan: Option<String>,
}

/// `GET /usage` for ONE key — `account.plan_usage` / `plan_limit`. The
/// response carries no period dates, so no reset time is fabricated.
async fn tavily_fetch_key(key: &str) -> Result<TavilyKeyUsage, String> {
    let resp = bearer("https://api.tavily.com/usage", key, "tavily")
        .send()
        .await
        .map_err(|e| format!("Tavily 请求失败: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("Tavily API key 无效".into());
    }
    if !resp.status().is_success() {
        return Err(format!("Tavily 接口返回 HTTP {}", resp.status()));
    }
    let body = json_body(resp, 64 * 1024, "Tavily").await?;
    let account = body.get("account").ok_or("Tavily 返回中没有 account")?;
    Ok(TavilyKeyUsage {
        used: account.get("plan_usage").and_then(Value::as_f64).unwrap_or(0.0),
        limit: account.get("plan_limit").and_then(Value::as_f64),
        plan: account.get("current_plan").and_then(Value::as_str).map(str::to_string),
    })
}

/// Multi-key Tavily: every key is an independent quota pool — the card
/// shows the summed pool as its "Credits" ring and one text row per key.
async fn tavily_fetch(keys: &[String]) -> Result<Snapshot, String> {
    let mut used = 0.0;
    let mut limit = 0.0;
    let mut has_limit = false;
    let mut plan: Option<String> = None;
    let mut per_key: Vec<Metric> = Vec::new();
    let mut first_error: Option<String> = None;
    let mut ok_count = 0usize;
    for (i, key) in keys.iter().enumerate() {
        match tavily_fetch_key(key).await {
            Ok(u) => {
                ok_count += 1;
                used += u.used;
                match u.limit {
                    Some(l) => {
                        limit += l;
                        has_limit = true;
                    }
                    None => {}
                }
                if plan.is_none() {
                    plan = u.plan;
                }
                per_key.push(Metric::text(
                    &format!("Key {}", i + 1),
                    match u.limit {
                        Some(l) => format!("{:.0} / {:.0} credits", u.used, l),
                        None => format!("{:.0} credits", u.used),
                    },
                ));
            }
            Err(e) => {
                per_key.push(Metric::text(&format!("Key {}", i + 1), "unavailable".into()));
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    if ok_count == 0 {
        return Err(first_error.unwrap_or_else(|| "Tavily 全部 key 查询失败".into()));
    }
    let mut metrics = Vec::new();
    if has_limit && limit > 0.0 {
        metrics.push(Metric::progress(
            "Credits",
            (used / limit * 100.0).clamp(0.0, 100.0),
            Some(format!("{used:.0} / {limit:.0} credits · {} keys", keys.len())),
        ));
    } else {
        metrics.push(Metric::text("Credits", format!("{used:.0} credits")));
    }
    metrics.extend(per_key);
    Ok(Snapshot::ok("tavily", "Tavily", plan, metrics))
}

// ── Firecrawl ───────────────────────────────────────────────────────────────

pub async fn firecrawl_snapshot() -> Snapshot {
    if let Some(snap) = fresh_or("firecrawl") {
        return snap;
    }
    let Some(key) = firecrawl_key() else {
        return Snapshot::no_credentials(
            "firecrawl",
            "Firecrawl",
            "设置 FIRECRAWL_API_KEY（或在 ZCode MCP 配置里配置）后可查询额度。",
        );
    };
    match firecrawl_fetch(&key).await {
        Ok(snap) => {
            remember_ok("firecrawl", &snap);
            snap
        }
        Err(e) => Snapshot::error("firecrawl", "Firecrawl", e),
    }
}

/// `GET /v1/team/credit-usage` — used = plan_credits - remaining_credits;
/// reset = billing_period_end (RFC3339).
async fn firecrawl_fetch(key: &str) -> Result<Snapshot, String> {
    let resp = bearer("https://api.firecrawl.dev/v1/team/credit-usage", key, "firecrawl")
        .send()
        .await
        .map_err(|e| format!("Firecrawl 请求失败: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("Firecrawl API key 无效".into());
    }
    if !resp.status().is_success() {
        return Err(format!("Firecrawl 接口返回 HTTP {}", resp.status()));
    }
    let body = json_body(resp, 64 * 1024, "Firecrawl").await?;
    let data = body.pointer("/data").ok_or("Firecrawl 返回中没有 data")?;
    let remaining = data.get("remaining_credits").and_then(Value::as_f64).unwrap_or(0.0);
    let plan_credits = data.get("plan_credits").and_then(Value::as_f64).unwrap_or(0.0);
    let used = (plan_credits - remaining).max(0.0);
    let pct = if plan_credits > 0.0 { (used / plan_credits * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    let reset = data
        .get("billing_period_end")
        .and_then(Value::as_str)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_millis());
    let metric = Metric::progress("Credits", pct, Some(format!("{used:.0} / {plan_credits:.0} credits")))
        .with_reset(reset, None);
    Ok(Snapshot::ok("firecrawl", "Firecrawl", None, vec![metric]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bocha_balance_parses_and_zero_is_a_normal_state() {
        let body = json!({
            "success": true, "code": "200",
            "data": {"remaining": 0.00},
            "timestamp": 1790935073011i64
        });
        let remaining = body.pointer("/data/remaining").and_then(Value::as_f64).unwrap();
        assert_eq!(remaining, 0.0);
        let mut value = format!("¥{remaining:.2}");
        if remaining <= 0.0 {
            value.push_str(" — 余额耗尽 (open.bocha.cn)");
        }
        assert!(value.contains("¥0.00"));
        assert!(value.contains("余额耗尽"));
    }

    #[test]
    fn tavily_usage_parses_plan_window() {
        let body = json!({
            "key": {"usage": 0, "limit": null},
            "account": {"current_plan": "Researcher", "plan_usage": 0, "plan_limit": 1000}
        });
        let account = body.get("account").unwrap();
        let used = account.get("plan_usage").and_then(Value::as_f64).unwrap_or(0.0);
        let limit = account.get("plan_limit").and_then(Value::as_f64).unwrap();
        assert_eq!((used / limit * 100.0), 0.0);
        // limit=null → text metric, not a divide-by-zero progress.
        let null_limit = json!({"account": {"plan_usage": 3.0, "plan_limit": null}});
        assert!(null_limit.pointer("/account/plan_limit").and_then(Value::as_f64).is_none());
    }

    #[test]
    fn firecrawl_used_and_reset_parse() {
        let body = json!({
            "success": true,
            "data": {
                "remaining_credits": 967.0,
                "plan_credits": 1000.0,
                "billing_period_start": "2026-09-30T01:34:41.381Z",
                "billing_period_end": "2026-10-30T01:34:41.381Z"
            }
        });
        let data = body.pointer("/data").unwrap();
        let used = data.get("remaining_credits").and_then(Value::as_f64).map(|r| {
            data.get("plan_credits").and_then(Value::as_f64).unwrap_or(0.0) - r
        }).unwrap();
        assert_eq!(used, 33.0);
        let reset = data.get("billing_period_end").and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp_millis());
        assert!(reset.is_some());
    }
}
