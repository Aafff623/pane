//! Search / MCP service quota providers (Bocha, Tavily, Firecrawl, Brave).
//!
//! Key resolution: the Pane key vault (`%APPDATA%/Pane/keyvault.json`) is
//! the primary source — entries are keyed by service id and keep a
//! user-facing label that shows up on per-key rows. Legacy sources (env
//! vars, tavily-keys.json, the ZCode CLI MCP config) stay as fallbacks and
//! are deduped by key value. Adding/removing a vault key invalidates this
//! module's cache, so the next refresh tick picks the new key set up
//! automatically.
//!
//! Every provider aggregates multi-key pools the same way: one headline
//! metric (summed pool, the number the overview ring uses) plus one text
//! row per key so the card detail shows each pool's own remainder.
//!
//! Quota endpoints are read-only and free, but the refresh loop ticks
//! every minute — a 45-minute TTL keeps real traffic at ~32 calls/day per
//! provider. Brave is the exception: it has NO quota endpoint, the only
//! signal is rate-limit headers on real search responses, so one real
//! search (1 query off the monthly 2000) is spent per probe and cached
//! for 12 hours (~2 queries/day/key).
//!
//! Deliberately NOT here: Exa (official usage endpoint exists, no key on
//! this machine — ready to wire when a key lands in the vault), Keenable
//! (no quota endpoint anywhere: probed /v1/usage|quota|account|… → 404,
//! real search responses carry no quota headers/fields; only signal is a
//! 429 retry window), MinerU/Liner (console-only). See
//! temp/handoff/20261002-mcp-quota-panel-handoff.md.

use super::{http, json_body, Metric, Snapshot};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(45 * 60);
/// Brave spends a real query per probe — probe at most every 12h per key
/// (2/day ≈ 3% of the free 2000/month budget).
const BRAVE_TTL: Duration = Duration::from_secs(12 * 60 * 60);

fn cache() -> &'static Mutex<HashMap<String, (Instant, Snapshot)>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, (Instant, Snapshot)>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Fresh-snapshot gate shared by the fetchers: serve the cached copy while
/// younger than the TTL. Cache ids are the provider id ("tavily") or
/// per-key slots ("brave:0"); per-key slots honor the per-provider TTL
/// they were written with (Brave writes BRAVE_TTL timestamps).
fn fresh_or(id: &str, ttl: Duration) -> Option<Snapshot> {
    if let Ok(map) = cache().lock() {
        if let Some((at, snap)) = map.get(id) {
            if at.elapsed() < ttl {
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

/// Drop cached entries for one service so the next refresh tick refetches
/// with the new key set (called by the keyvault add/remove commands).
pub fn invalidate_service(service: &str) {
    if let Ok(mut map) = cache().lock() {
        let prefix = format!("{service}:");
        map.retain(|k, _| k != service && !k.starts_with(&prefix));
    }
    // Only Brave keys justify re-probing: a probe costs a real query.
    if service == "brave" {
        if let Ok(mut map) = brave_quota_cache().lock() {
            map.clear();
        }
    }
}

fn env_key(vars: &[&str]) -> Option<String> {
    vars.iter()
        .find_map(|v| std::env::var(v).ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()))
}

// ── unified key resolution ──────────────────────────────────────────────────

/// One key pool with its display label.
struct Keyed {
    label: String,
    key: String,
}

impl Keyed {
    /// `tvly-dev…whtQ` — enough to tell keys apart without exposing them.
    fn masked(key: &str) -> String {
        crate::keyvault::mask_key(key)
    }
}

/// Vault keys (with their labels) first, then legacy keys (masked label),
/// deduped by key value.
fn collect_keys(service: &str, legacy: Vec<String>) -> Vec<Keyed> {
    let mut out: Vec<Keyed> = Vec::new();
    for (label, key) in crate::keyvault::keys_for_service(service) {
        let key = key.trim().to_string();
        if key.is_empty() || out.iter().any(|k| k.key == key) {
            continue;
        }
        let label = if label.trim().is_empty() { Keyed::masked(&key) } else { label };
        out.push(Keyed { label, key });
    }
    for key in legacy {
        let key = key.trim().to_string();
        if key.is_empty() || out.iter().any(|k| k.key == key) {
            continue;
        }
        out.push(Keyed { label: Keyed::masked(&key), key });
    }
    out
}

/// The active Firecrawl legacy key: the ZCode CLI's MCP server config
/// carries the literal current key; env vars may still hold the old one.
fn firecrawl_legacy_key() -> Option<String> {
    let cfg = dirs::home_dir()?.join(".zcode").join("cli").join("config.json");
    let text = std::fs::read_to_string(cfg).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let key = v
        .pointer("/mcp/servers/firecrawl/env/FIRECRAWL_API_KEY")
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    (!key.is_empty()).then_some(key)
}

/// Tavily legacy keys: the Pane-managed list (`tavily-keys.json`) plus env.
fn tavily_legacy_keys() -> Vec<String> {
    let mut out = Vec::new();
    let path = super::config_dir().join("tavily-keys.json");
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            for k in v.get("keys").and_then(Value::as_array).into_iter().flatten() {
                if let Some(s) = k.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    out.push(s.to_string());
                }
            }
        }
    }
    if let Some(env) = env_key(&["TAVILY_API_KEY"]) {
        out.push(env);
    }
    out
}

fn bearer(url: &str, key: &str, what: &str) -> reqwest::RequestBuilder {
    http().get(url).bearer_auth(key.trim()).header("Accept", "application/json").header("User-Agent", format!("pane/{what}"))
}

/// Pure local probe for the Customize gear panel (no network): a key is
/// present in the vault or the environment.
pub fn local_credential_hint(service: &str, env_vars: &[&str]) -> Option<String> {
    if !crate::keyvault::keys_for_service(service).is_empty() {
        return Some("Pane 密钥保险箱".to_string());
    }
    env_key(env_vars).map(|_| "环境变量".to_string())
}

// ── Bocha ───────────────────────────────────────────────────────────────────

pub async fn bocha_snapshot() -> Snapshot {
    if let Some(snap) = fresh_or("bocha", TTL) {
        return snap;
    }
    let keys = collect_keys("bocha", env_key(&["BOCHA_API_KEY"]).into_iter().collect());
    if keys.is_empty() {
        return Snapshot::no_credentials("bocha", "BochaAI", "在密钥保险箱添加 BochaAI key（或设置环境变量 BOCHA_API_KEY）后可查询余额。");
    }
    match bocha_fetch(&keys).await {
        Ok(snap) => {
            remember_ok("bocha", &snap);
            snap
        }
        Err(e) => Snapshot::error("bocha", "BochaAI", e),
    }
}

/// `GET /v1/fund/remaining` per key — `data.remaining` is account balance
/// in CNY (yuan, not requests). A zero balance is a normal state.
async fn bocha_fetch_key(key: &str) -> Result<f64, String> {
    let resp = bearer("https://api.bochaai.com/v1/fund/remaining", key, "bocha")
        .send()
        .await
        .map_err(|e| format!("博查请求失败: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("key 无效".into());
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body = json_body(resp, 64 * 1024, "BochaAI").await?;
    body.pointer("/data/remaining")
        .and_then(Value::as_f64)
        .ok_or_else(|| "返回中没有余额字段".into())
}

async fn bocha_fetch(keys: &[Keyed]) -> Result<Snapshot, String> {
    let mut total = 0.0;
    let mut rows: Vec<Metric> = Vec::new();
    let mut first_error: Option<String> = None;
    let mut ok_count = 0usize;
    for k in keys {
        match bocha_fetch_key(&k.key).await {
            Ok(remaining) => {
                ok_count += 1;
                total += remaining;
                rows.push(Metric::text(&k.label, format!("¥{remaining:.2}")));
            }
            Err(e) => {
                rows.push(Metric::text(&k.label, e.clone()));
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    if ok_count == 0 {
        return Err(format!("博查查询失败: {}", first_error.unwrap_or_default()));
    }
    let mut value = format!("¥{total:.2}");
    if total <= 0.0 {
        value.push_str(" — 余额耗尽 (open.bocha.cn)");
    }
    rows.insert(0, Metric::text("Balance", value));
    Ok(Snapshot::ok("bocha", "BochaAI", None, rows))
}

// ── Tavily ──────────────────────────────────────────────────────────────────

pub async fn tavily_snapshot() -> Snapshot {
    if let Some(snap) = fresh_or("tavily", TTL) {
        return snap;
    }
    let keys = collect_keys("tavily", tavily_legacy_keys());
    if keys.is_empty() {
        return Snapshot::no_credentials(
            "tavily",
            "Tavily",
            "在密钥保险箱添加 Tavily key（或设置环境变量 TAVILY_API_KEY）后可查询额度。",
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
        return Err("key 无效".into());
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body = json_body(resp, 64 * 1024, "Tavily").await?;
    let account = body.get("account").ok_or("返回中没有 account")?;
    Ok(TavilyKeyUsage {
        used: account.get("plan_usage").and_then(Value::as_f64).unwrap_or(0.0),
        limit: account.get("plan_limit").and_then(Value::as_f64),
        plan: account.get("current_plan").and_then(Value::as_str).map(str::to_string),
    })
}

/// Multi-key Tavily: every key is an independent quota pool — the card
/// shows the summed pool as its "Credits" ring and one text row per key.
async fn tavily_fetch(keys: &[Keyed]) -> Result<Snapshot, String> {
    let mut used = 0.0;
    let mut limit = 0.0;
    let mut has_limit = false;
    let mut plan: Option<String> = None;
    let mut per_key: Vec<Metric> = Vec::new();
    let mut first_error: Option<String> = None;
    let mut ok_count = 0usize;
    for k in keys {
        match tavily_fetch_key(&k.key).await {
            Ok(u) => {
                ok_count += 1;
                used += u.used;
                if let Some(l) = u.limit {
                    limit += l;
                    has_limit = true;
                }
                if plan.is_none() {
                    plan = u.plan;
                }
                per_key.push(Metric::text(
                    &k.label,
                    match u.limit {
                        Some(l) => format!("{:.0} / {:.0} credits", u.used, l),
                        None => format!("{:.0} credits", u.used),
                    },
                ));
            }
            Err(e) => {
                per_key.push(Metric::text(&k.label, "查询失败".into()));
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    if ok_count == 0 {
        return Err(format!("Tavily 查询失败: {}", first_error.unwrap_or_default()));
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
    if let Some(snap) = fresh_or("firecrawl", TTL) {
        return snap;
    }
    let keys = collect_keys(
        "firecrawl",
        firecrawl_legacy_key()
            .into_iter()
            .chain(env_key(&["FIRECRAWL_FIRECRAWL_API_KEY", "FIRECRAWL_API_KEY"]))
            .collect(),
    );
    if keys.is_empty() {
        return Snapshot::no_credentials(
            "firecrawl",
            "Firecrawl",
            "在密钥保险箱添加 Firecrawl key（或设置环境变量 FIRECRAWL_API_KEY）后可查询额度。",
        );
    }
    match firecrawl_fetch(&keys).await {
        Ok(snap) => {
            remember_ok("firecrawl", &snap);
            snap
        }
        Err(e) => Snapshot::error("firecrawl", "Firecrawl", e),
    }
}

struct FirecrawlKeyUsage {
    used: f64,
    plan: f64,
    /// billing_period_end in epoch ms.
    reset: Option<i64>,
}

/// `GET /v1/team/credit-usage` — used = plan_credits - remaining_credits;
/// reset = billing_period_end (RFC3339).
async fn firecrawl_fetch_key(key: &str) -> Result<FirecrawlKeyUsage, String> {
    let resp = bearer("https://api.firecrawl.dev/v1/team/credit-usage", key, "firecrawl")
        .send()
        .await
        .map_err(|e| format!("Firecrawl 请求失败: {e}"))?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err("key 无效".into());
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body = json_body(resp, 64 * 1024, "Firecrawl").await?;
    let data = body.pointer("/data").ok_or("返回中没有 data")?;
    let remaining = data.get("remaining_credits").and_then(Value::as_f64).unwrap_or(0.0);
    let plan = data.get("plan_credits").and_then(Value::as_f64).unwrap_or(0.0);
    let reset = data
        .get("billing_period_end")
        .and_then(Value::as_str)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_millis());
    Ok(FirecrawlKeyUsage { used: (plan - remaining).max(0.0), plan, reset })
}

/// Multi-key Firecrawl: independent pools, each with its own billing
/// period. Headline = summed pool with the EARLIEST period end as the
/// reset (the first pool to refill).
async fn firecrawl_fetch(keys: &[Keyed]) -> Result<Snapshot, String> {
    let mut used = 0.0;
    let mut plan = 0.0;
    let mut earliest_reset: Option<i64> = None;
    let mut per_key: Vec<Metric> = Vec::new();
    let mut first_error: Option<String> = None;
    let mut ok_count = 0usize;
    for k in keys {
        match firecrawl_fetch_key(&k.key).await {
            Ok(u) => {
                ok_count += 1;
                used += u.used;
                plan += u.plan;
                if let Some(r) = u.reset {
                    earliest_reset = Some(match earliest_reset {
                        Some(cur) => cur.min(r),
                        None => r,
                    });
                }
                per_key.push(Metric::text(&k.label, format!("{:.0} / {:.0} credits", u.used, u.plan)));
            }
            Err(e) => {
                per_key.push(Metric::text(&k.label, "查询失败".into()));
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    if ok_count == 0 {
        return Err(format!("Firecrawl 查询失败: {}", first_error.unwrap_or_default()));
    }
    let pct = if plan > 0.0 { (used / plan * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    let mut metrics = vec![Metric::progress("Credits", pct, Some(format!("{used:.0} / {plan:.0} credits · {} keys", keys.len())))
        .with_reset(earliest_reset, None)];
    metrics.extend(per_key);
    Ok(Snapshot::ok("firecrawl", "Firecrawl", None, metrics))
}

// ── Brave ───────────────────────────────────────────────────────────────────

pub async fn brave_snapshot() -> Snapshot {
    let keys = collect_keys(
        "brave",
        env_key(&["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"]).into_iter().collect(),
    );
    if keys.is_empty() {
        return Snapshot::no_credentials(
            "brave",
            "Brave Search",
            "在密钥保险箱添加 Brave key（或设置环境变量 BRAVE_API_KEY）后可查询月配额。",
        );
    }
    match brave_fetch(&keys).await {
        Ok(snap) => snap,
        Err(e) => Snapshot::error("brave", "Brave Search", e),
    }
}

/// The monthly figure of a comma-joined header — `X-RateLimit-Remaining:
/// "0, 1994"` is [per-second, per-month], the monthly slot is last.
fn header_monthly(value: Option<&str>) -> Option<f64> {
    let v = value?.trim();
    let last = v.rsplit(',').next()?.trim();
    last.parse::<f64>().ok()
}

#[derive(Clone)]
struct BraveKeyQuota {
    remaining: f64,
    limit: f64,
    /// Seconds until the monthly window resets.
    reset_secs: f64,
}

impl BraveKeyQuota {
    fn used_percent(&self) -> f64 {
        if self.limit <= 0.0 {
            return 0.0;
        }
        ((self.limit - self.remaining) / self.limit * 100.0).clamp(0.0, 100.0)
    }
}

/// One REAL search per key: Brave exposes no quota endpoint, only
/// rate-limit headers on search responses. `count=1` keeps the answer
/// payload minimal — the body is discarded, only headers matter.
async fn brave_fetch_key(key: &str) -> Result<BraveKeyQuota, String> {
    let resp = http()
        .get("https://api.search.brave.com/res/v1/web/search")
        .query(&[("q", "ping"), ("count", "1")])
        .header("Accept", "application/json")
        .header("X-Subscription-Token", key.trim())
        .header("User-Agent", "pane/brave")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Brave 请求失败: {e}"))?;
    let status = resp.status();
    // Headers survive on 429 too — a quota-exhausted key still reports.
    let limit = header_monthly(resp.headers().get("X-RateLimit-Limit").and_then(|v| v.to_str().ok()));
    let remaining =
        header_monthly(resp.headers().get("X-RateLimit-Remaining").and_then(|v| v.to_str().ok()));
    let reset_secs =
        header_monthly(resp.headers().get("X-RateLimit-Reset").and_then(|v| v.to_str().ok()));
    if matches!(status.as_u16(), 401 | 403) {
        return Err("key 无效".into());
    }
    match (limit, remaining, reset_secs) {
        (Some(limit), Some(remaining), Some(reset_secs)) => {
            Ok(BraveKeyQuota { remaining, limit, reset_secs })
        }
        _ => {
            if status.as_u16() == 429 {
                Err("月配额耗尽（429 且无配额头）".into())
            } else {
                Err(format!("HTTP {status}：响应缺少配额头"))
            }
        }
    }
}

/// Multi-key Brave: every key gets its own 12h probe-cache slot (a
/// dedicated quota cache, since the payload is numbers, not a Snapshot),
/// so the snapshot aggregates without re-spending queries on each tick.
fn brave_quota_cache() -> &'static Mutex<HashMap<String, (Instant, BraveKeyQuota)>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, (Instant, BraveKeyQuota)>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn brave_fetch(keys: &[Keyed]) -> Result<Snapshot, String> {
    let mut used = 0.0;
    let mut limit = 0.0;
    let mut soonest_reset_secs: Option<f64> = None;
    let mut per_key: Vec<Metric> = Vec::new();
    let mut first_error: Option<String> = None;
    let mut ok_count = 0usize;
    for (i, k) in keys.iter().enumerate() {
        let slot = format!("brave:{i}");
        let cached = brave_quota_cache()
            .lock()
            .ok()
            .and_then(|m| m.get(&slot).filter(|(at, _)| at.elapsed() < BRAVE_TTL).map(|(_, q)| q.clone()));
        let quota = match cached {
            Some(q) => q,
            None => match brave_fetch_key(&k.key).await {
                Ok(q) => {
                    if let Ok(mut m) = brave_quota_cache().lock() {
                        m.insert(slot, (Instant::now(), q.clone()));
                    }
                    q
                }
                Err(e) => {
                    per_key.push(Metric::text(&k.label, e.clone()));
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                    continue;
                }
            },
        };
        ok_count += 1;
        used += quota.limit - quota.remaining;
        limit += quota.limit;
        soonest_reset_secs = Some(match soonest_reset_secs {
            Some(cur) => cur.min(quota.reset_secs),
            None => quota.reset_secs,
        });
        per_key.push(Metric::text(&k.label, format!("{:.0} / {:.0} /月", quota.remaining, quota.limit)));
    }
    if ok_count == 0 {
        return Err(format!("Brave 查询失败: {}", first_error.unwrap_or_default()));
    }
    let pct = if limit > 0.0 { (used / limit * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    let reset = soonest_reset_secs
        .map(|s| chrono::Utc::now().timestamp_millis() + (s as i64) * 1000);
    let mut metrics = vec![Metric::progress(
        "Monthly",
        pct,
        Some(format!("{:.0} / {:.0} · {} keys（每次探测消耗 1 次查询）", used, limit, keys.len())),
    )
    .with_reset(reset, None)];
    metrics.extend(per_key);
    Ok(Snapshot::ok("brave", "Brave Search", None, metrics))
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

    #[test]
    fn brave_header_monthly_takes_the_last_csv_slot() {
        // Real header shapes from the 2026-10-03 probe.
        assert_eq!(header_monthly(Some("1, 2000")), Some(2000.0));
        assert_eq!(header_monthly(Some("0, 1994")), Some(1994.0));
        assert_eq!(header_monthly(Some("1, 2451530")), Some(2451530.0));
        assert_eq!(header_monthly(Some("2000")), Some(2000.0));
        assert_eq!(header_monthly(Some("")), None);
        assert_eq!(header_monthly(Some("1, abc")), None);
        assert_eq!(header_monthly(None), None);
    }

    #[test]
    fn brave_used_percent_from_probe_numbers() {
        // 1994 of 2000 remaining → 0.3% used.
        let q = BraveKeyQuota { remaining: 1994.0, limit: 2000.0, reset_secs: 2451530.0 };
        assert!((q.used_percent() - 0.3).abs() < 1e-9);
        let exhausted = BraveKeyQuota { remaining: 0.0, limit: 2000.0, reset_secs: 100.0 };
        assert!((exhausted.used_percent() - 100.0).abs() < 1e-9);
        let zero_limit = BraveKeyQuota { remaining: 0.0, limit: 0.0, reset_secs: 100.0 };
        assert_eq!(zero_limit.used_percent(), 0.0);
    }

    #[test]
    fn keyed_labels_use_vault_label_or_mask() {
        let k = Keyed { label: "Tavily key 1".into(), key: "tvly-dev-abcdefghijklmnop".into() };
        assert_eq!(k.label, "Tavily key 1");
        let masked = Keyed::masked("tvly-dev-abcdefghijklmnop");
        assert!(masked.starts_with("tvly-d"));
        assert!(masked.ends_with("mnop"));
        assert!(!masked.contains("abcdefghij"));
    }
}
