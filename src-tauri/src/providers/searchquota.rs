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
//! A key is a credential, not proof of an independent quota pool. Team/account
//! balances are shown separately; only explicitly key-scoped counters aggregate.
//!
//! Quota endpoints are read-only and free, but the refresh loop ticks
//! every minute — a 45-minute TTL keeps real traffic at ~32 calls/day per
//! provider. Brave is the exception: it has NO quota endpoint, the only
//! signal is rate-limit headers on real search responses, so one real
//! search (1 query off the actual monthly allowance) is spent per probe and cached
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
use sha2::{Digest, Sha256};
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

/// Only the matching credential/label/lock-state fingerprint can hit this TTL.
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
            if let Some((service, _)) = id.split_once(':') {
                let prefix = format!("{service}:");
                map.retain(|key, _| !key.starts_with(&prefix));
            }
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
}

/// Drop every cached snapshot. The vault's lock state changes which keys the
/// fetchers can read, so unlocking or locking must rebuild the cards rather
/// than serve rows that describe the previous state. The per-key Brave quota
/// cache is left alone: its numbers stay valid, and re-probing costs a query.
pub fn invalidate_all() {
    if let Ok(mut map) = cache().lock() {
        map.clear();
    }
}

// Hash length-delimited input; neither cache keys nor diagnostics contain secrets.
fn credential_slot(service: &str, key: &str) -> String {
    format!("{service}:{:x}", Sha256::digest(key.trim().as_bytes()))
}

fn snapshot_slot(service: &str, keys: &[Keyed], locked: &[Metric]) -> String {
    let mut hash = Sha256::new();
    for field in keys
        .iter()
        .flat_map(|k| [k.key.as_str(), k.label.as_str()])
        .chain(
            locked
                .iter()
                .flat_map(|m| [m.label.as_str(), m.value.as_deref().unwrap_or("")]),
        )
    {
        hash.update((field.len() as u64).to_le_bytes());
        hash.update(field.as_bytes());
    }
    format!("{service}:{hash:x}", hash = hash.finalize())
}

fn env_key(vars: &[&str]) -> Option<String> {
    vars.iter().find_map(|v| {
        std::env::var(v)
            .ok()
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
    })
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

/// The value shown on a key row the vault holds but cannot read (locked).
const LOCKED_ROW: &str = "🔒 保险箱已锁定（解锁后查询）";

/// Vault keys (with their labels) first, then legacy keys (masked label),
/// deduped by key value. The second half of the pair is one row per vault
/// entry the LOCKED vault could not hand over — the card then still lists
/// every key the user stored, marked unqueried, instead of quietly showing
/// fewer keys than the vault does. Masks already covered by a readable key
/// are skipped (a vault copy of an env key is not a second pool).
fn collect_keys(service: &str, legacy: Vec<String>) -> (Vec<Keyed>, Vec<Metric>) {
    let mut out: Vec<Keyed> = Vec::new();
    for (label, key) in crate::keyvault::keys_for_service(service) {
        let key = key.trim().to_string();
        if key.is_empty() || out.iter().any(|k| k.key == key) {
            continue;
        }
        let label = if label.trim().is_empty() {
            Keyed::masked(&key)
        } else {
            label
        };
        out.push(Keyed { label, key });
    }
    for key in legacy {
        let key = key.trim().to_string();
        if key.is_empty() || out.iter().any(|k| k.key == key) {
            continue;
        }
        out.push(Keyed {
            label: Keyed::masked(&key),
            key,
        });
    }
    let seen: Vec<String> = out.iter().map(|k| Keyed::masked(&k.key)).collect();
    let locked = crate::keyvault::locked_service_entries(service)
        .into_iter()
        .filter(|(_, masked)| !seen.contains(masked))
        .map(|(label, masked)| {
            let name = if label.trim().is_empty() {
                masked
            } else {
                label
            };
            Metric::text(&name, LOCKED_ROW.into())
        })
        .collect();
    (out, locked)
}

/// The active Firecrawl legacy key: the ZCode CLI's MCP server config
/// carries the literal current key; env vars may still hold the old one.
fn firecrawl_legacy_key() -> Option<String> {
    let cfg = dirs::home_dir()?
        .join(".zcode")
        .join("cli")
        .join("config.json");
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
            for k in v
                .get("keys")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
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
    http()
        .get(url)
        .bearer_auth(key.trim())
        .header("Accept", "application/json")
        .header("User-Agent", format!("pane/{what}"))
}

/// Append the locked-vault rows to a snapshot, widening the headline's
/// "· N keys" into "· N/M keys" so the count matches the rows below instead of
/// reading as the whole story. A longer suffix would just be ellipsized away.
fn with_locked(mut snap: Snapshot, locked: Vec<Metric>) -> Snapshot {
    if !locked.is_empty() {
        if let Some(head) = snap.metrics.first_mut().filter(|m| m.kind == "text") {
            if let Some(value) = head.value.as_mut() {
                value.push_str(&format!(" · {} keys 待解锁", locked.len()));
            }
        }
        if let Some(head) = snap.metrics.iter_mut().find(|m| m.kind == "progress") {
            if let Some(detail) = head.detail.as_mut() {
                *detail = widen_key_count(detail, locked.len());
            }
        }
        snap.metrics.extend(locked);
    }
    snap
}

/// `… · 2 keys` → `… · 2/5 keys` (3 locked). Any other shape is returned
/// unchanged: the headline either carries no key count or already fits.
fn widen_key_count(detail: &str, locked: usize) -> String {
    let Some(at) = detail.find(" keys") else {
        return detail.to_string();
    };
    let Some(sep) = detail[..at].rfind("· ") else {
        return detail.to_string();
    };
    let count = detail[sep + "· ".len()..at].trim();
    if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
        return detail.to_string();
    }
    let total = count.parse::<usize>().unwrap_or(0) + locked;
    format!("{}· {count}/{total}{}", &detail[..sep], &detail[at..])
}

/// The "no credentials" snapshot for a card whose only keys live in a locked
/// vault: the note names how many are waiting instead of telling the user to
/// add a key they already stored.
fn no_keys(id: &str, name: &str, add_hint: &str, locked: &[Metric]) -> Snapshot {
    if locked.is_empty() {
        Snapshot::no_credentials(id, name, add_hint)
    } else {
        Snapshot::no_credentials(
            id,
            name,
            &format!(
                "密钥保险箱已锁定，{} 把 {} key 暂不可用（解锁保险箱后自动查询）。",
                locked.len(),
                name
            ),
        )
    }
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
    let (keys, locked) = collect_keys("bocha", env_key(&["BOCHA_API_KEY"]).into_iter().collect());
    let slot = snapshot_slot("bocha", &keys, &locked);
    if let Some(snap) = fresh_or(&slot, TTL) {
        return snap;
    }
    if keys.is_empty() {
        return no_keys(
            "bocha",
            "BochaAI",
            "在密钥保险箱添加 BochaAI key（或设置环境变量 BOCHA_API_KEY）后可查询余额。",
            &locked,
        );
    }
    match bocha_fetch(&keys).await {
        Ok(snap) => {
            let snap = with_locked(snap, locked);
            remember_ok(&slot, &snap);
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
    let (keys, locked) = collect_keys("tavily", tavily_legacy_keys());
    let slot = snapshot_slot("tavily", &keys, &locked);
    if let Some(snap) = fresh_or(&slot, TTL) {
        return snap;
    }
    if keys.is_empty() {
        return no_keys(
            "tavily",
            "Tavily",
            "在密钥保险箱添加 Tavily key（或设置环境变量 TAVILY_API_KEY）后可查询额度。",
            &locked,
        );
    }
    match tavily_fetch(&keys).await {
        Ok(snap) => {
            let snap = with_locked(snap, locked);
            remember_ok(&slot, &snap);
            snap
        }
        Err(e) => Snapshot::error("tavily", "Tavily", e),
    }
}

struct TavilyKeyUsage {
    used: f64,
    limit: Option<f64>,
    key_used: f64,
    key_limit: Option<f64>,
    key_unlimited: bool,
    paygo: Option<f64>,
    plan: Option<String>,
}

fn quota_number(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
        .filter(|n| n.is_finite() && *n >= 0.0)
}

fn parse_tavily_usage(body: &Value) -> Result<TavilyKeyUsage, String> {
    let account = body.get("account").ok_or("返回中没有 account")?;
    let key = body.get("key").ok_or("返回中没有 key")?;
    Ok(TavilyKeyUsage {
        used: quota_number(account.get("plan_usage")).ok_or("缺少有效 account.plan_usage")?,
        limit: quota_number(account.get("plan_limit")),
        key_used: quota_number(key.get("usage")).ok_or("缺少有效 key.usage")?,
        key_limit: quota_number(key.get("limit")),
        key_unlimited: key.get("limit").is_some_and(Value::is_null),
        paygo: quota_number(account.get("paygo_usage")),
        plan: account
            .get("current_plan")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

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
    parse_tavily_usage(&json_body(resp, 64 * 1024, "Tavily").await?)
}

async fn tavily_fetch(keys: &[Keyed]) -> Result<Snapshot, String> {
    let mut results = Vec::with_capacity(keys.len());
    for k in keys {
        results.push((k.label.clone(), tavily_fetch_key(&k.key).await));
    }
    tavily_snapshot_from_results(results)
}

fn tavily_snapshot_from_results(
    results: Vec<(String, Result<TavilyKeyUsage, String>)>,
) -> Result<Snapshot, String> {
    let count = results.len();
    let mut metrics = Vec::new();
    let mut plans = Vec::new();
    let mut ok_count = 0;
    let mut first_error = None;
    for (label, result) in results {
        match result {
            Ok(u) => {
                ok_count += 1;
                if let Some(p) = &u.plan {
                    if !plans.contains(p) {
                        plans.push(p.clone());
                    }
                }
                let account = match u.limit {
                    Some(l) => format!("账户套餐已用 {:.0} / {l:.0} credits", u.used),
                    None => format!("账户套餐已用 {:.0} credits · 上限未返回", u.used),
                };
                let key = match u.key_limit {
                    Some(l) => format!("Key 已用 {:.0} / {l:.0} credits", u.key_used),
                    None if u.key_unlimited => {
                        format!("Key 已用 {:.0} credits · 未设 Key 上限", u.key_used)
                    }
                    None => format!("Key 已用 {:.0} credits · Key 上限未返回", u.key_used),
                };
                let paygo = u
                    .paygo
                    .map(|v| format!(" · 按量已用 {v:.0} credits"))
                    .unwrap_or_default();
                if count == 1 {
                    if let Some(limit) = u.limit.filter(|l| *l > 0.0) {
                        metrics.push(Metric::progress(
                            "Credits",
                            u.used / limit * 100.0,
                            Some(account.clone()),
                        ));
                    }
                }
                metrics.push(Metric::text(&label, format!("{key} · {account}{paygo}")));
            }
            Err(e) => {
                metrics.push(Metric::text(&label, "查询失败".into()));
                first_error.get_or_insert(e);
            }
        }
    }
    if ok_count == 0 {
        return Err(format!(
            "Tavily 查询失败: {}",
            first_error.unwrap_or_default()
        ));
    }
    if count > 1 {
        metrics.insert(
            0,
            Metric::text(
                "Credits",
                format!("{ok_count}/{count} keys · 账户额度分别显示（可能共享）"),
            ),
        );
    }
    Ok(Snapshot::ok(
        "tavily",
        "Tavily",
        (!plans.is_empty()).then(|| plans.join(" / ")),
        metrics,
    ))
}

// ── Firecrawl ───────────────────────────────────────────────────────────────

pub async fn firecrawl_snapshot() -> Snapshot {
    let (keys, locked) = collect_keys(
        "firecrawl",
        firecrawl_legacy_key()
            .into_iter()
            .chain(env_key(&[
                "FIRECRAWL_FIRECRAWL_API_KEY",
                "FIRECRAWL_API_KEY",
            ]))
            .collect(),
    );
    let slot = snapshot_slot("firecrawl", &keys, &locked);
    if let Some(snap) = fresh_or(&slot, TTL) {
        return snap;
    }
    if keys.is_empty() {
        return no_keys(
            "firecrawl",
            "Firecrawl",
            "在密钥保险箱添加 Firecrawl key（或设置环境变量 FIRECRAWL_API_KEY）后可查询额度。",
            &locked,
        );
    }
    match firecrawl_fetch(&keys).await {
        Ok(snap) => {
            let snap = with_locked(snap, locked);
            remember_ok(&slot, &snap);
            snap
        }
        Err(e) => Snapshot::error("firecrawl", "Firecrawl", e),
    }
}

struct FirecrawlKeyUsage {
    plan: f64,
    remaining: f64,
    /// billing_period_end in epoch ms.
    reset: Option<i64>,
}

/// The balance includes grants/top-ups, while plan_credits is only the plan.
/// Even a balance below the plan cannot prove actual consumed plan credits.
fn parse_firecrawl_usage(data: &Value) -> Result<FirecrawlKeyUsage, String> {
    let remaining = data
        .get("remaining_credits")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("返回中缺少有效 remaining_credits")?;
    let plan = data
        .get("plan_credits")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("返回中缺少有效 plan_credits")?;
    let reset = data
        .get("billing_period_end")
        .and_then(Value::as_str)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_millis());
    Ok(FirecrawlKeyUsage {
        plan,
        remaining,
        reset,
    })
}

fn firecrawl_key_metric(label: &str, usage: &FirecrawlKeyUsage) -> Metric {
    let reset = usage
        .reset
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map(|d| format!(" · {} 重置", d.format("%m-%d")))
        .unwrap_or_default();
    let extra = if usage.remaining > usage.plan {
        "；含额外额度"
    } else {
        ""
    };
    let value = format!(
        "剩余 {:.0} credits（套餐 {:.0}{extra}）{reset}",
        usage.remaining, usage.plan
    );
    Metric::text(label, value).with_reset(usage.reset, None)
}

/// `GET /v1/team/credit-usage` preserves the total balance and billing end.
async fn firecrawl_fetch_key(key: &str) -> Result<FirecrawlKeyUsage, String> {
    let resp = bearer(
        "https://api.firecrawl.dev/v1/team/credit-usage",
        key,
        "firecrawl",
    )
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
    parse_firecrawl_usage(data)
}

/// The endpoint reports team balance, not key usage. Multiple keys may share a team.
async fn firecrawl_fetch(keys: &[Keyed]) -> Result<Snapshot, String> {
    let mut results = Vec::with_capacity(keys.len());
    for k in keys {
        results.push((k.label.clone(), firecrawl_fetch_key(&k.key).await));
    }
    firecrawl_snapshot_from_results(results)
}

fn firecrawl_snapshot_from_results(
    results: Vec<(String, Result<FirecrawlKeyUsage, String>)>,
) -> Result<Snapshot, String> {
    let count = results.len();
    let mut rows = Vec::new();
    let mut first_error = None;
    let mut ok_count = 0;
    for (label, result) in results {
        match result {
            Ok(u) => {
                ok_count += 1;
                rows.push(firecrawl_key_metric(&label, &u));
            }
            Err(e) => {
                rows.push(Metric::text(&label, "查询失败".into()));
                first_error.get_or_insert(e);
            }
        }
    }
    if ok_count == 0 {
        return Err(format!(
            "Firecrawl 查询失败: {}",
            first_error.unwrap_or_default()
        ));
    }
    let headline = if count == 1 {
        rows[0].clone()
    } else {
        Metric::text(
            "Credits",
            format!("{ok_count}/{count} keys · 团队余额分别显示（可能共享）"),
        )
    };
    let mut metrics = vec![headline];
    metrics.extend(rows);
    Ok(Snapshot::ok("firecrawl", "Firecrawl", None, metrics))
}

// ── Brave ───────────────────────────────────────────────────────────────────

pub async fn brave_snapshot() -> Snapshot {
    let (keys, locked) = collect_keys(
        "brave",
        env_key(&["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"])
            .into_iter()
            .collect(),
    );
    if keys.is_empty() {
        return no_keys(
            "brave",
            "Brave Search",
            "在密钥保险箱添加 Brave key（或设置环境变量 BRAVE_API_KEY）后可查询月配额。",
            &locked,
        );
    }
    match brave_fetch(&keys).await {
        Ok(snap) => with_locked(snap, locked),
        Err(e) => Snapshot::error("brave", "Brave Search", e),
    }
}

/// The monthly figure of a comma-joined header — `X-RateLimit-Remaining:
/// "0, 1994"` is [per-second, per-month], the monthly slot is last.
fn header_monthly(value: Option<&str>) -> Option<f64> {
    let v = value?.trim();
    let last = v.rsplit(',').next()?.trim();
    last.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
}

#[derive(Clone)]
struct BraveKeyQuota {
    remaining: f64,
    limit: f64,
    /// Seconds until the monthly window resets.
    reset_secs: f64,
}

fn cached_brave_quota(q: &BraveKeyQuota, elapsed: Duration) -> Option<BraveKeyQuota> {
    // Once the monthly window rolls, the old remaining/limit pair is stale.
    if elapsed >= BRAVE_TTL || elapsed.as_secs_f64() >= q.reset_secs {
        return None;
    }
    let mut q = q.clone();
    q.reset_secs -= elapsed.as_secs_f64();
    Some(q)
}

fn brave_key_metric(label: &str, quota: &BraveKeyQuota) -> Metric {
    let value = if quota.limit == 0.0 {
        "月额度不限".into()
    } else {
        format!(
            "已用 {:.0} / {:.0} credits",
            (quota.limit - quota.remaining).max(0.0),
            quota.limit
        )
    };
    Metric::text(label, value)
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
    let limit = header_monthly(
        resp.headers()
            .get("X-RateLimit-Limit")
            .and_then(|v| v.to_str().ok()),
    );
    let remaining = header_monthly(
        resp.headers()
            .get("X-RateLimit-Remaining")
            .and_then(|v| v.to_str().ok()),
    );
    let reset_secs = header_monthly(
        resp.headers()
            .get("X-RateLimit-Reset")
            .and_then(|v| v.to_str().ok()),
    );
    if matches!(status.as_u16(), 401 | 403) {
        return Err("key 无效".into());
    }
    if !status.is_success() && status.as_u16() != 429 {
        return Err(format!("Brave HTTP {status}"));
    }
    match (limit, remaining, reset_secs) {
        (Some(limit), Some(remaining), Some(reset_secs)) => Ok(BraveKeyQuota {
            remaining,
            limit,
            reset_secs,
        }),
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
    let mut unlimited_count = 0;
    let mut per_key: Vec<Metric> = Vec::new();
    let mut first_error: Option<String> = None;
    let mut ok_count = 0usize;
    for k in keys {
        let slot = credential_slot("brave", &k.key);
        let cached = brave_quota_cache().lock().ok().and_then(|m| {
            m.get(&slot)
                .and_then(|(at, q)| cached_brave_quota(q, at.elapsed()))
        });
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
        if quota.limit == 0.0 {
            unlimited_count += 1;
        } else {
            used += (quota.limit - quota.remaining).max(0.0);
            limit += quota.limit;
        }
        soonest_reset_secs = Some(match soonest_reset_secs {
            Some(cur) => cur.min(quota.reset_secs),
            None => quota.reset_secs,
        });
        per_key.push(brave_key_metric(&k.label, &quota));
    }
    if ok_count == 0 {
        return Err(format!(
            "Brave 查询失败: {}",
            first_error.unwrap_or_default()
        ));
    }
    let pct = if limit > 0.0 {
        (used / limit * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let reset =
        soonest_reset_secs.map(|s| chrono::Utc::now().timestamp_millis() + (s as i64) * 1000);
    let headline = if unlimited_count > 0 {
        Metric::text("Monthly", format!("{unlimited_count} keys 月额度不限 · 有限 Key 已用 {used:.0} / {limit:.0} credits（每次探测消耗 1 次查询）"))
    } else {
        Metric::progress(
            "Monthly",
            pct,
            Some(format!(
                "已用 {:.0} / {:.0} credits · {} keys（每次探测消耗 1 次查询）",
                used,
                limit,
                keys.len()
            )),
        )
    };
    let mut metrics = vec![headline.with_reset(reset, None)];
    metrics.extend(per_key);
    Ok(Snapshot::ok("brave", "Brave Search", None, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tavily_key_scope_paygo_and_paid_plan_are_distinct() {
        let doc = json!({"key":{"usage":150,"limit":1000},"account":{"current_plan":"Bootstrap","plan_usage":500,"plan_limit":15000,"paygo_usage":25}});
        let snap = tavily_snapshot_from_results(vec![(
            "key".into(),
            Ok(parse_tavily_usage(&doc).unwrap()),
        )])
        .unwrap();
        assert_eq!(snap.plan.as_deref(), Some("Bootstrap"));
        assert!((snap.metrics[0].used_percent.unwrap() - 500.0 / 15000.0 * 100.0).abs() < 1e-9);
        let row = snap.metrics[1].value.as_deref().unwrap();
        assert!(row.contains("Key 已用 150 / 1000"));
        assert!(row.contains("账户套餐已用 500 / 15000"));
        assert!(row.contains("按量已用 25"));
        let multi = tavily_snapshot_from_results(vec![
            ("one".into(), Ok(parse_tavily_usage(&doc).unwrap())),
            ("two".into(), Ok(parse_tavily_usage(&doc).unwrap())),
        ])
        .unwrap();
        assert_eq!(multi.metrics[0].used_percent, None);
        assert!(multi.metrics[0]
            .value
            .as_deref()
            .unwrap()
            .contains("可能共享"));
        assert!(
            parse_tavily_usage(&json!({"key":{"usage":0},"account":{"plan_limit":15000}})).is_err()
        );
    }

    #[test]
    fn credential_cache_identity_follows_keys_and_labels_not_positions() {
        assert_ne!(
            credential_slot("brave", "first"),
            credential_slot("brave", "second")
        );
        let first = vec![Keyed {
            key: "first".into(),
            label: "one".into(),
        }];
        let second = vec![Keyed {
            key: "second".into(),
            label: "one".into(),
        }];
        assert_ne!(
            snapshot_slot("tavily", &first, &[]),
            snapshot_slot("tavily", &second, &[])
        );
        let renamed = vec![Keyed {
            key: "first".into(),
            label: "renamed".into(),
        }];
        assert_ne!(
            snapshot_slot("tavily", &first, &[]),
            snapshot_slot("tavily", &renamed, &[])
        );
        assert_ne!(
            snapshot_slot("tavily", &first, &[]),
            snapshot_slot(
                "tavily",
                &first,
                &[Metric::text("locked", "解锁后查询".into())]
            )
        );
    }

    #[test]
    fn brave_unlimited_and_invalid_headers_do_not_become_free_quota() {
        let q = BraveKeyQuota {
            remaining: 0.0,
            limit: 0.0,
            reset_secs: 100.0,
        };
        let m = brave_key_metric("key", &q);
        assert_eq!(m.used_percent, None);
        assert_eq!(m.value.as_deref(), Some("月额度不限"));
        assert_eq!(header_monthly(Some("1, NaN")), None);
        assert_eq!(header_monthly(Some("1, -10")), None);
    }

    use serde_json::json;

    #[test]
    fn bocha_balance_parses_and_zero_is_a_normal_state() {
        let body = json!({
            "success": true, "code": "200",
            "data": {"remaining": 0.00},
            "timestamp": 1790935073011i64
        });
        let remaining = body
            .pointer("/data/remaining")
            .and_then(Value::as_f64)
            .unwrap();
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
        let usage = parse_tavily_usage(&body).unwrap();
        assert_eq!(usage.used, 0.0);
        assert_eq!(usage.limit, Some(1000.0));
        assert_eq!(usage.key_limit, None);
        assert!(usage.key_unlimited);
        let missing_limit = json!({"key":{"usage":0},"account":{"plan_usage":3.0}});
        assert!(!parse_tavily_usage(&missing_limit).unwrap().key_unlimited);
    }

    #[test]
    fn locked_rows_widen_the_headline_key_count_only_when_the_shape_fits() {
        assert_eq!(
            widen_key_count("已用 1000 / 2000 credits · 2 keys", 3),
            "已用 1000 / 2000 credits · 2/5 keys"
        );
        assert_eq!(
            widen_key_count(
                "已用 193 / 2000 credits · 1 keys（每次探测消耗 1 次查询）",
                1
            ),
            "已用 193 / 2000 credits · 1/2 keys（每次探测消耗 1 次查询）"
        );
        // No "· N keys" tail: the headline is left exactly as it was.
        assert_eq!(widen_key_count("¥12.34", 2), "¥12.34");
        assert_eq!(
            widen_key_count("已用 1000 / 2000 credits", 2),
            "已用 1000 / 2000 credits"
        );
        assert_eq!(widen_key_count("used · many keys", 2), "used · many keys");
    }

    #[test]
    fn firecrawl_balance_and_reset_parse() {
        let body = json!({
            "success": true,
            "data": {
                "remaining_credits": 967.0,
                "plan_credits": 1000.0,
                "billing_period_start": "2026-09-30T01:34:41.381Z",
                "billing_period_end": "2026-10-30T01:34:41.381Z"
            }
        });
        let usage = parse_firecrawl_usage(body.pointer("/data").unwrap()).unwrap();
        assert_eq!(usage.remaining, 967.0);
        assert_eq!(usage.plan, 1000.0);
        assert!(usage.reset.is_some());
    }

    #[test]
    fn firecrawl_extra_credits_preserve_balance_and_do_not_fabricate_a_ratio() {
        let usage = parse_firecrawl_usage(&json!({
            "remaining_credits": 10890,
            "plan_credits": 1000,
            "billing_period_end": "2026-10-28T11:08:14.197Z"
        }))
        .unwrap();
        let reset = usage.reset;
        let exhausted =
            parse_firecrawl_usage(&json!({"remaining_credits": 0, "plan_credits": 1000})).unwrap();
        let snap = firecrawl_snapshot_from_results(vec![
            ("new key".into(), Ok(usage)),
            ("old key".into(), Ok(exhausted)),
        ])
        .unwrap();
        assert_eq!(snap.metrics[0].kind, "text");
        assert_eq!(snap.metrics[0].used_percent, None);
        assert!(snap.metrics[0]
            .value
            .as_deref()
            .unwrap()
            .contains("可能共享"));
        let row = &snap.metrics[1];
        assert!(row.value.as_deref().unwrap().contains("10890 credits"));
        assert!(row.value.as_deref().unwrap().contains("10-28"));
        assert_eq!(row.resets_at, reset);
        assert_eq!(
            snap.metrics[2].value.as_deref(),
            Some("剩余 0 credits（套餐 1000）")
        );
    }

    #[test]
    fn firecrawl_balance_below_plan_does_not_prove_consumption() {
        let usage = parse_firecrawl_usage(&json!({"remaining_credits": 967, "plan_credits": 1000}))
            .unwrap();
        let snap = firecrawl_snapshot_from_results(vec![("key".into(), Ok(usage))]).unwrap();
        assert_eq!(snap.metrics[0].kind, "text");
        assert_eq!(snap.metrics[0].used_percent, None);
        assert_eq!(
            snap.metrics[1].value.as_deref(),
            Some("剩余 967 credits（套餐 1000）")
        );
        assert!(parse_firecrawl_usage(&json!({"plan_credits": 1000})).is_err());
        assert!(
            firecrawl_snapshot_from_results(vec![("key".into(), Err("key 无效".into()))]).is_err()
        );
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
    fn brave_cache_countdown_and_rollover_are_not_extended_by_refresh() {
        let q = BraveKeyQuota {
            remaining: 1994.0,
            limit: 2000.0,
            reset_secs: 100.0,
        };
        assert_eq!(
            cached_brave_quota(&q, Duration::from_secs(25))
                .unwrap()
                .reset_secs,
            75.0
        );
        assert!(cached_brave_quota(&q, Duration::from_secs(100)).is_none());
        assert!(cached_brave_quota(&q, BRAVE_TTL).is_none());
        assert_eq!(
            brave_key_metric("key", &q).value.as_deref(),
            Some("已用 6 / 2000 credits")
        );
    }

    #[test]
    fn keyed_labels_use_vault_label_or_mask() {
        let k = Keyed {
            label: "Tavily key 1".into(),
            key: "tvly-dev-abcdefghijklmnop".into(),
        };
        assert_eq!(k.label, "Tavily key 1");
        let masked = Keyed::masked("tvly-dev-abcdefghijklmnop");
        assert!(masked.starts_with("tvly-d"));
        assert!(masked.ends_with("mnop"));
        assert!(!masked.contains("abcdefghij"));
    }
}
