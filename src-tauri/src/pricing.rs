//! Live model pricing — a port of the Mac app's ModelPricingStore.
//!
//! Three sources, most-authoritative first at lookup time:
//!   1. Robin's pricing supplement (Cursor-native models, fast multipliers,
//!      alias regexes mapping log slugs to canonical keys) — updates land
//!      without an app release.
//!   2. LiteLLM's model_prices catalog (USD per token — converted here).
//!   3. models.dev (USD per million; exact-match only — fuzzy-matching a
//!      reseller rate would fabricate dollars).
//!
//! Each source is cached at %APPDATA%\Pane\pricing\ and refreshed at
//! most every 24h (30-minute retry after a failure) with ETag revalidation.
//! `lookup()` never touches the network; `ensure_fresh()` runs on the spend
//! engine's blocking thread. The old hardcoded prices in spend.rs remain
//! the last-resort fallback when a model is missing everywhere.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

use crate::providers;

#[derive(Clone, Copy, Debug)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    /// Rates for requests whose prompt crosses 200k tokens — 1M-context
    /// models bill the whole request at a higher tier. None = no tier.
    pub input_200k: Option<f64>,
    pub output_200k: Option<f64>,
    pub cache_read_200k: Option<f64>,
    pub cache_write_200k: Option<f64>,
    /// Explicit 1-hour cache-write rates; absent, 1h writes bill at
    /// twice the (tier-selected) input rate.
    pub cache_write_1h: Option<f64>,
    pub cache_write_1h_200k: Option<f64>,
}

impl Price {
    /// A price with no long-context tier and no explicit 1h rate — what
    /// models.dev, the supplement, and the static fallbacks provide.
    pub fn flat(input: f64, output: f64, cache_read: f64, cache_write: f64) -> Self {
        Price {
            input,
            output,
            cache_read,
            cache_write,
            input_200k: None,
            output_200k: None,
            cache_read_200k: None,
            cache_write_200k: None,
            cache_write_1h: None,
            cache_write_1h_200k: None,
        }
    }
}

/// One request's token counts, cache writes split by lifetime.
pub struct Usage {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write_5m: f64,
    pub cache_write_1h: f64,
}

/// Dollar cost of one request. Vendors bill the *whole* request (output
/// included) at the >200k tier once the prompt — everything except output —
/// crosses 200k tokens; aggregated sources (Cursor's CSV) opt out because
/// their rows don't preserve request boundaries. 1-hour cache writes bill
/// at twice the tier-selected input rate unless the catalog carries an
/// explicit rate.
pub fn request_cost(p: &Price, u: &Usage, apply_long_context: bool) -> f64 {
    request_cost_at(p, u, if apply_long_context { 200_000.0 } else { f64::INFINITY })
}

/// Like `request_cost`, with an explicit long-context threshold — the tier
/// boundary is vendor-specific (Anthropic switches at 200k prompt tokens,
/// OpenAI's Codex models at 272k).
pub fn request_cost_at(p: &Price, u: &Usage, threshold: f64) -> f64 {
    let prompt = u.input + u.cache_read + u.cache_write_5m + u.cache_write_1h;
    let long = prompt > threshold;
    let pick = |base: f64, above: Option<f64>| if long { above.unwrap_or(base) } else { base };
    let input = pick(p.input, p.input_200k);
    let w1h = if long {
        p.cache_write_1h_200k
            .or(p.input_200k.map(|i| i * 2.0))
            .unwrap_or_else(|| p.cache_write_1h.unwrap_or(p.input * 2.0))
    } else {
        p.cache_write_1h.unwrap_or(p.input * 2.0)
    };
    (u.input * input
        + u.output * pick(p.output, p.output_200k)
        + u.cache_read * pick(p.cache_read, p.cache_read_200k)
        + u.cache_write_5m * pick(p.cache_write, p.cache_write_200k)
        + u.cache_write_1h * w1h)
        / 1e6
}

/// The supplement's fast multiplier for a model, 1.0 when none is
/// published — a fast-flagged request without data bills at standard
/// rates rather than a guessed premium (Mac behavior).
pub fn fast_multiplier(model: &str) -> f64 {
    let s = store().lock().unwrap();
    s.fast_multipliers.get(model).copied().unwrap_or(1.0)
}

const SOURCES: [(&str, &str); 3] = [
    (
        "litellm",
        "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json",
    ),
    ("modelsdev", "https://models.dev/api.json"),
    (
        "supplement",
        "https://robinebers.github.io/openusage/pricing_supplement.json",
    ),
];
const REFRESH_MS: i64 = 24 * 3600 * 1000;
const RETRY_MS: i64 = 30 * 60 * 1000;
/// The catalogs are third-party feeds; a compromised one must not be able
/// to fill memory (and then the disk cache) with an arbitrarily large
/// response. Largest legitimate source today is LiteLLM at ~3 MB — 32 MB
/// leaves room to grow while still bounding the damage.
const MAX_CATALOG_BYTES: usize = 32 * 1024 * 1024;

#[derive(Default)]
struct Store {
    litellm: HashMap<String, Price>,
    modelsdev: HashMap<String, Price>,
    supplement: HashMap<String, Price>,
    fast_multipliers: HashMap<String, f64>,
    alias_rules: Vec<(regex::Regex, String)>,
    memo: HashMap<String, Option<Price>>,
    loaded_from_disk: bool,
}

fn store() -> &'static Mutex<Store> {
    static S: OnceLock<Mutex<Store>> = OnceLock::new();
    S.get_or_init(Default::default)
}

/// Set when the spend engine met models no catalog prices — sources are
/// then retried hourly instead of daily, so a newly shipped model (e.g. a
/// fresh Cursor slug) prices as soon as the supplement learns it.
static UNPRICED_HINT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Bumped whenever a source ingests a new document; spend's per-file cache
/// keys on it so already-parsed logs re-price under the new catalog.
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn note_unpriced() {
    UNPRICED_HINT.store(true, std::sync::atomic::Ordering::Relaxed);
}

pub fn generation() -> u64 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

/// Bump whenever the baked-in pricing behavior changes (stale-catalog
/// corrections, builtin fallbacks, long-context tables): the persisted
/// spend cache holds pre-priced totals, and only the catalog *files* are
/// fingerprinted below — an app update that reprices the same files would
/// otherwise leave history at the old dollars until upstream happens to
/// rewrite a catalog.
const CORRECTIONS_REV: u32 = 3; // 3: zero-rate catalog placeholders no longer price at $0.00

/// Stable fingerprint of the effective pricing inputs: the on-disk catalog
/// files plus this binary's corrections revision. The persistent spend
/// cache stores costs priced under a specific catalog set; when any catalog
/// file changes (a refresh rewrote it) — or an update ships changed baked
/// pricing — the stamp changes and the whole persisted cache is discarded
/// rather than served with stale prices.
pub fn catalog_stamp() -> String {
    let files = SOURCES
        .iter()
        .map(|(source, _)| {
            let path = dir().join(format!("{source}.json"));
            match std::fs::metadata(&path) {
                Ok(m) => {
                    let ms = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis())
                        .unwrap_or(0);
                    format!("{source}:{ms}:{}", m.len())
                }
                Err(_) => format!("{source}:absent"),
            }
        })
        .collect::<Vec<_>>()
        .join("|");
    format!("{files}|corrections:{CORRECTIONS_REV}")
}

fn dir() -> PathBuf {
    providers::config_dir().join("pricing")
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ---------------------------------------------------------------------------
// Parsing (each source's shape → HashMap<model, Price> per million tokens)
// ---------------------------------------------------------------------------

fn parse_litellm(doc: &Value) -> HashMap<String, Price> {
    let mut out = HashMap::new();
    let Some(obj) = doc.as_object() else { return out };
    for (model, entry) in obj {
        let per_tok = |key: &str| entry.get(key).and_then(Value::as_f64);
        let (Some(input), Some(output)) =
            (per_tok("input_cost_per_token"), per_tok("output_cost_per_token"))
        else {
            continue;
        };
        let per_m = |key: &str| per_tok(key).map(|v| v * 1e6);
        out.insert(
            model.clone(),
            Price {
                input: input * 1e6,
                output: output * 1e6,
                cache_read: per_m("cache_read_input_token_cost").unwrap_or(input * 1e6),
                cache_write: per_m("cache_creation_input_token_cost").unwrap_or(input * 1e6),
                input_200k: per_m("input_cost_per_token_above_200k_tokens"),
                output_200k: per_m("output_cost_per_token_above_200k_tokens"),
                cache_read_200k: per_m("cache_read_input_token_cost_above_200k_tokens"),
                cache_write_200k: per_m("cache_creation_input_token_cost_above_200k_tokens"),
                cache_write_1h: per_m("cache_creation_input_token_cost_above_1hr"),
                cache_write_1h_200k: per_m("cache_creation_input_token_cost_above_1hr_above_200k_tokens"),
            },
        );
    }
    out
}

fn parse_modelsdev(doc: &Value) -> HashMap<String, Price> {
    // models.dev repeats ids across resellers with varying completeness —
    // the entry documenting the most cache fields wins (ties: first seen),
    // so a reseller stub with no cache rates can't default a $0.30 cache
    // hit to the $3.00 input price.
    let mut out: HashMap<String, (Price, u8)> = HashMap::new();
    let Some(providers) = doc.as_object() else { return HashMap::new() };
    for provider in providers.values() {
        let Some(models) = provider.get("models").and_then(Value::as_object) else { continue };
        for (id, m) in models {
            let Some(cost) = m.get("cost") else { continue };
            let get = |key: &str| cost.get(key).and_then(Value::as_f64);
            let (Some(input), Some(output)) = (get("input"), get("output")) else { continue };
            let score =
                get("cache_read").is_some() as u8 + get("cache_write").is_some() as u8;
            let price = Price::flat(
                input,
                output,
                get("cache_read").unwrap_or(input),
                get("cache_write").unwrap_or(input),
            );
            match out.entry(id.clone()) {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert((price, score));
                }
                std::collections::hash_map::Entry::Occupied(mut e) if score > e.get().1 => {
                    e.insert((price, score));
                }
                _ => {}
            }
        }
    }
    out.into_iter().map(|(k, (p, _))| (k, p)).collect()
}

fn apply_supplement(store: &mut Store, doc: &Value) {
    store.supplement.clear();
    store.fast_multipliers.clear();
    store.alias_rules.clear();
    if let Some(pricing) = doc.get("pricing").and_then(Value::as_object) {
        for (model, entry) in pricing {
            let get = |key: &str| entry.get(key).and_then(Value::as_f64);
            let (Some(input), Some(output)) =
                (get("input_per_million"), get("output_per_million"))
            else {
                continue;
            };
            store.supplement.insert(
                model.clone(),
                Price::flat(
                    input,
                    output,
                    get("cache_read_per_million").unwrap_or(input),
                    get("cache_write_per_million").unwrap_or(input),
                ),
            );
        }
    }
    correct_stale_supplement(&mut store.supplement);
    if let Some(mults) = doc.get("fast_multipliers").and_then(Value::as_object) {
        for (model, v) in mults {
            if let Some(m) = v.as_f64() {
                store.fast_multipliers.insert(model.clone(), m);
            }
        }
    }
    // 2026-07-31: OpenAI cut gpt-5.6-terra/-luna prices and both public
    // catalogs (and the supplement, which outranks them here) still carry
    // launch pricing. Correct ONLY the exact known-stale values — a
    // self-retiring override: the moment the supplement publishes any new
    // number for these models, its data wins again untouched. Remove this
    // whole function once upstream catches up.
    fn correct_stale_supplement(supplement: &mut HashMap<String, Price>) {
        let corrections: [(&str, f64, Price); 2] = [
            ("gpt-5.6-terra", 2.5, Price::flat(2.0, 12.0, 0.2, 2.5)),
            ("gpt-5.6-luna", 1.0, Price::flat(0.2, 1.2, 0.02, 0.25)),
        ];
        for (model, stale_input, corrected) in corrections {
            if let Some(p) = supplement.get_mut(model) {
                if (p.input - stale_input).abs() < 1e-9 {
                    *p = corrected;
                }
            }
        }
    }

    // The supplement is fetched from a third-party URL, so cap what it can
    // feed us: at most 64 alias rules of at most 256 chars each, compiled
    // with a bounded size. (Rust's regex engine is linear-time by design,
    // so ReDoS-style backtracking blowups aren't possible; the caps bound
    // memory and compile cost.)
    if let Some(rules) = doc.get("alias_rules").and_then(Value::as_array) {
        for rule in rules.iter().take(64) {
            let (Some(pattern), Some(canonical)) = (
                rule.get("pattern").and_then(Value::as_str),
                rule.get("canonical").and_then(Value::as_str),
            ) else {
                continue;
            };
            if pattern.len() > 256 || canonical.len() > 128 {
                continue;
            }
            if let Ok(re) = regex::RegexBuilder::new(pattern)
                .size_limit(1 << 20)
                .build()
            {
                store.alias_rules.push((re, canonical.to_string()));
            }
        }
    }
}

fn ingest(store: &mut Store, source: &str, doc: &Value) {
    match source {
        "litellm" => store.litellm = parse_litellm(doc),
        "modelsdev" => store.modelsdev = parse_modelsdev(doc),
        "supplement" => apply_supplement(store, doc),
        _ => {}
    }
    store.memo.clear();
}

// ---------------------------------------------------------------------------
// Disk cache + refresh
// ---------------------------------------------------------------------------

fn load_state() -> Value {
    std::fs::read_to_string(dir().join("state.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn save_state(state: &Value) {
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(dir().join("state.json"), state.to_string());
}

fn load_from_disk(store: &mut Store) {
    for (source, _) in SOURCES {
        if let Ok(raw) = std::fs::read_to_string(dir().join(format!("{source}.json"))) {
            if let Ok(doc) = serde_json::from_str::<Value>(&raw) {
                ingest(store, source, &doc);
            }
        }
    }
    store.loaded_from_disk = true;
}

/// Refreshes stale sources (blocking; called from the spend engine's
/// blocking thread). Network failures leave the cached/parsed data in place.
pub fn ensure_fresh() {
    {
        let mut s = store().lock().unwrap();
        if !s.loaded_from_disk {
            load_from_disk(&mut s);
        }
    }

    // Boot grace: a successful catalog download changes catalog_stamp(),
    // which discards the whole persisted spend cache and re-parses every
    // session log (gigabytes on long-lived installs). With autostart, the
    // daily refresh was landing exactly at Windows login — the one moment
    // the disk is already saturated. Serve yesterday's copy of any catalog
    // already on disk and refresh once the boot storm has passed; prices
    // are at most a day + grace stale. Gated per source: a catalog with no
    // file yet (first run, or a feed this machine can never reach) still
    // fetches immediately — no prices at all is worse, and one dead feed
    // must not disable the deferral for the others.
    static FIRST_CALL: OnceLock<std::time::Instant> = OnceLock::new();
    const BOOT_GRACE: std::time::Duration = std::time::Duration::from_secs(10 * 60);
    let in_grace =
        FIRST_CALL.get_or_init(std::time::Instant::now).elapsed() < BOOT_GRACE;

    let mut state = load_state();
    let now = now_ms();
    let refresh_ms = if UNPRICED_HINT.load(std::sync::atomic::Ordering::Relaxed) {
        3_600_000 // unpriced models seen: look for catalog updates hourly
    } else {
        REFRESH_MS
    };
    for (source, url) in SOURCES {
        let entry = state.get(source).cloned().unwrap_or_else(|| json!({}));
        let fetched_at = entry.get("fetchedAt").and_then(Value::as_i64).unwrap_or(0);
        let failed_at = entry.get("failedAt").and_then(Value::as_i64).unwrap_or(0);
        let due = now - fetched_at > refresh_ms && now - failed_at > RETRY_MS;
        if !due {
            continue;
        }
        if in_grace && dir().join(format!("{source}.json")).exists() {
            continue;
        }
        let etag = entry.get("etag").and_then(Value::as_str).unwrap_or("").to_string();

        let result = tauri::async_runtime::block_on(async {
            let mut req = providers::http().get(url);
            if !etag.is_empty() {
                req = req.header("If-None-Match", etag.clone());
            }
            let mut resp = req.send().await.map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            if status == 304 {
                return Ok((304u16, String::new(), String::new()));
            }
            if !(200..300).contains(&status) {
                return Err(format!("HTTP {status}"));
            }
            let new_etag = resp
                .headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            // Read with a hard byte cap instead of resp.text() — the
            // declared Content-Length check alone wouldn't bound a
            // chunked/lying response.
            if resp.content_length().is_some_and(|l| l as usize > MAX_CATALOG_BYTES) {
                return Err("catalog larger than the size cap".into());
            }
            let mut bytes: Vec<u8> = Vec::new();
            while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
                if bytes.len() + chunk.len() > MAX_CATALOG_BYTES {
                    return Err("catalog larger than the size cap".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            let body = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            Ok((status, body, new_etag))
        });

        match result {
            Ok((304, _, _)) => {
                state[source] = json!({ "etag": etag, "fetchedAt": now, "failedAt": 0 });
            }
            Ok((_, body, new_etag)) => match serde_json::from_str::<Value>(&body) {
                Ok(doc) => {
                    let _ = std::fs::create_dir_all(dir());
                    let _ = std::fs::write(dir().join(format!("{source}.json")), &body);
                    ingest(&mut store().lock().unwrap(), source, &doc);
                    state[source] = json!({ "etag": new_etag, "fetchedAt": now, "failedAt": 0 });
                    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    UNPRICED_HINT.store(false, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("[pane] pricing: refreshed {source}");
                }
                Err(e) => {
                    eprintln!("[pane] pricing: {source} parse failed ({e})");
                    state[source] = json!({ "etag": etag, "fetchedAt": fetched_at, "failedAt": now });
                }
            },
            Err(e) => {
                eprintln!("[pane] pricing: {source} fetch failed ({e})");
                state[source] = json!({ "etag": etag, "fetchedAt": fetched_at, "failedAt": now });
            }
        }
    }
    save_state(&state);
}

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

/// USD per million tokens for `model`, or None if no source prices it.
/// Memoized; disk-cache only (call ensure_fresh() beforehand to update).
pub fn lookup(model: &str) -> Option<Price> {
    let mut s = store().lock().unwrap();
    if !s.loaded_from_disk {
        load_from_disk(&mut s);
    }
    if let Some(hit) = s.memo.get(model) {
        return *hit;
    }
    let result = resolve(&s, model, 0);
    s.memo.insert(model.to_string(), result);
    result
}

/// Every rate in `p` multiplied by `m` — service tiers (fast, priority)
/// bill at a multiple of the base model's whole rate card.
fn scaled_price(p: &Price, m: f64) -> Price {
    let scale = |v: Option<f64>| v.map(|x| x * m);
    Price {
        input: p.input * m,
        output: p.output * m,
        cache_read: p.cache_read * m,
        cache_write: p.cache_write * m,
        input_200k: scale(p.input_200k),
        output_200k: scale(p.output_200k),
        cache_read_200k: scale(p.cache_read_200k),
        cache_write_200k: scale(p.cache_write_200k),
        cache_write_1h: scale(p.cache_write_1h),
        cache_write_1h_200k: scale(p.cache_write_1h_200k),
    }
}

fn resolve(s: &Store, model: &str, depth: u8) -> Option<Price> {
    // Alias rules come from a third-party URL; a crafted rule set could
    // otherwise bounce a name between an alias and the -max strip below
    // forever ("foo" → "foo-max" → "foo" → …) and overflow the stack.
    if depth >= 4 {
        return None;
    }
    let canonical = s
        .alias_rules
        .iter()
        .find(|(re, _)| re.is_match(model))
        .map(|(_, c)| c.clone())
        .unwrap_or_else(|| model.to_string());

    // A catalog row with zero input AND output rates is ambiguous: brand-new
    // slugs often land as 0/0 placeholders (qwen3.8-max), but free-tier and
    // local models are legitimately 0/0. Resolution order settles it — the
    // filtered chain below only takes entries with real rates, and 0/0
    // entries are reconsidered near the end (after the baked table), so a
    // placeholder loses to real rates anywhere while a model that is 0/0
    // in every source still prices as genuinely free, never as unpriced.
    let real = |p: &&Price| p.input > 0.0 || p.output > 0.0;
    if let Some(p) = s.supplement.get(&canonical).filter(real) {
        return Some(*p);
    }
    if let Some(p) = s.litellm.get(&canonical).filter(real) {
        return Some(*p);
    }
    // Fast tier: base price × the supplement's multiplier (default 2). The
    // base runs through the whole chain — not a bare map lookup — so
    // composed slugs like "gpt-5.6-sol-max-fast" reach the -max fallback
    // and alias/fuzzy matching too.
    if let Some(base) = canonical.strip_suffix("-fast") {
        // Multipliers are keyed by the plain model name; peel effort/mode
        // tokens off composed bases ("gpt-5.6-sol-max" → "gpt-5.6-sol") so
        // "sol-max-fast" gets sol's real multiplier, not the default.
        let mut mkey = base;
        let m = loop {
            if let Some(m) = s.fast_multipliers.get(mkey) {
                break *m;
            }
            match ["-xhigh", "-light", "-low", "-medium", "-high", "-max", "-ultra"]
                .iter()
                .find_map(|suf| mkey.strip_suffix(suf))
            {
                Some(next) => mkey = next,
                None => break 2.0,
            }
        };
        if let Some(p) = resolve(s, base, depth + 1) {
            return Some(scaled_price(&p, m));
        }
    }
    // Priority processing tier: some CLIs (Devin) bake the service tier
    // into the slug itself ("gpt-5.6-luna-xhigh-priority") instead of
    // flagging it per turn the way Codex rollouts do. OpenAI bills
    // priority at a per-model multiplier over the standard rate — keep
    // this table in sync with spend.rs's codex_priority_multiplier.
    if let Some(base) = canonical.strip_suffix("-priority") {
        let mut mkey = base;
        while let Some(next) = ["-xhigh", "-light", "-low", "-medium", "-high", "-max", "-ultra"]
            .iter()
            .find_map(|suf| mkey.strip_suffix(suf))
        {
            mkey = next;
        }
        let m = if matches!(mkey, "gpt-5.5" | "gpt-5.5-pro") { 2.5 } else { 2.0 };
        if let Some(p) = resolve(s, base, depth + 1) {
            return Some(scaled_price(&p, m));
        }
    }
    // LiteLLM fuzzy: provider-prefixed keys like "anthropic/claude-…".
    // Prefer an exact segment match; never fuzzy-match models.dev.
    if let Some(p) = s
        .litellm
        .iter()
        .find(|(k, p)| k.rsplit('/').next() == Some(canonical.as_str()) && real(&p))
        .map(|(_, p)| *p)
    {
        return Some(p);
    }
    if let Some(p) = s.modelsdev.get(&canonical).filter(real) {
        return Some(*p);
    }
    // Vendor-documented rates for models the live catalogs haven't learned
    // yet — consulted after every online source so a real catalog entry
    // always wins the moment one ships. Keep this list tiny and sourced.
    if let Some(p) = builtin_price(&canonical) {
        return Some(p);
    }
    // Zero-rate entries, reconsidered: nothing anywhere carries real rates
    // for this slug, so a 0/0 catalog row means the model is genuinely
    // free — take it, keeping free models at $0.00 without an unpriced ⚠.
    if let Some(p) = s.supplement.get(&canonical) {
        return Some(*p);
    }
    if let Some(p) = s.litellm.get(&canonical) {
        return Some(*p);
    }
    if let Some(p) = s
        .litellm
        .iter()
        .find(|(k, _)| k.rsplit('/').next() == Some(canonical.as_str()))
        .map(|(_, p)| *p)
    {
        return Some(p);
    }
    if let Some(p) = s.modelsdev.get(&canonical) {
        return Some(*p);
    }
    // Slug tails no catalog carries under their own name, billed at the
    // base model's per-token rates: reasoning-effort tiers (they change how
    // many tokens burn, not the unit price) and Cursor's Max/Ultra modes
    // (token-based at model rates). Only when the whole chain above misses
    // does one trailing token get peeled and the rest rerun — compositions
    // unwind right to left ("…-max-xhigh" → "…-max" → base), the depth cap
    // bounds it, and a real entry for any tail in any source always wins.
    for suffix in ["-xhigh", "-light", "-low", "-medium", "-high", "-max", "-ultra"] {
        if let Some(base) = canonical.strip_suffix(suffix) {
            return resolve(s, base, depth + 1);
        }
    }
    None
}

/// Kimi K3 — platform.kimi.ai/docs/pricing/chat-k3 (USD/MTok): input $3,
/// cache hit $0.30, output $15; no published cache-write rate, so writes
/// bill at the input rate. The `-code` spelling follows the K2.7 pattern
/// (the supplement priced k2.7 and k2.7-code identically); "moonshot/" and
/// "moonshot-ai/" prefixed spellings match how the CLIs log it.
fn builtin_price(canonical: &str) -> Option<Price> {
    let bare = canonical
        .strip_prefix("moonshot/")
        .or_else(|| canonical.strip_prefix("moonshot-ai/"))
        .or_else(|| canonical.strip_prefix("xai/"))
        .unwrap_or(canonical);
    match bare {
        "kimi-k3" | "kimi-k3-code" => Some(Price::flat(3.0, 15.0, 0.3, 3.0)),
        // Alibaba Model Studio, GA'd 2026-08-03 (USD/MTok): input $2,
        // output $6, implicit cache read $0.25, explicit cache write $2.50.
        // Public catalogs still carry 0/0 placeholders for these slugs.
        "qwen3.8-max" | "qwen3.8-max-preview" => Some(Price::flat(2.0, 6.0, 0.25, 2.5)),
        // Grok 4.6, released 2026-08-12 — docs.x.ai/docs/pricing (USD/MTok):
        // $2 in / $0.50 cached / $6 out; prompts ≥200k bill $4 / $1 / $12
        // for the WHOLE request (xAI's long-context rule matches
        // request_cost's tiering, and Grok spend passes the default 200k
        // threshold). xAI bills no separate cache-write rate — writes are
        // plain input. The announced 2x "-fast" variant needs no entry:
        // the -fast resolution path applies the default 2x multiplier to
        // this rate card. Public catalogs don't carry 4.6 yet.
        "grok-4.6" | "grok-4-6" => Some(Price {
            input_200k: Some(4.0),
            output_200k: Some(12.0),
            cache_read_200k: Some(1.0),
            cache_write_200k: Some(4.0),
            ..Price::flat(2.0, 6.0, 0.5, 2.0)
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{request_cost, Price, Usage};

    fn usage(input: f64, output: f64, cache_read: f64, w5m: f64, w1h: f64) -> Usage {
        Usage { input, output, cache_read, cache_write_5m: w5m, cache_write_1h: w1h }
    }

    #[test]
    fn modelsdev_prefers_the_most_complete_reseller_entry() {
        // A stub reseller listing kimi-k3 without cache rates must not
        // shadow a complete entry (alphabetical order made "aaa" win before,
        // silently pricing $0.30 cache hits at the $3.00 input rate).
        let doc = serde_json::json!({
            "aaa-stub": { "models": { "kimi-k3": { "cost": { "input": 3.0, "output": 15.0 } } } },
            "moonshotai": { "models": { "kimi-k3": {
                "cost": { "input": 3.0, "output": 15.0, "cache_read": 0.3, "cache_write": 3.0 }
            } } },
        });
        let map = super::parse_modelsdev(&doc);
        let p = map.get("kimi-k3").expect("kimi-k3 parsed");
        assert_eq!((p.input, p.output, p.cache_read, p.cache_write), (3.0, 15.0, 0.3, 3.0));
    }

    #[test]
    fn zero_rate_catalog_placeholders_are_skipped() {
        let mut store = super::Store::default();
        store.litellm.insert("qwen-test".into(), super::Price::flat(0.0, 0.0, 0.0, 0.0));
        store.modelsdev.insert("qwen-test".into(), super::Price::flat(1.0, 5.0, 0.1, 1.25));
        // The 0/0 litellm placeholder must not shadow models.dev's real price.
        let p = super::resolve(&store, "qwen-test", 0).unwrap();
        assert!((p.input - 1.0).abs() < 1e-9);

        // Zero in EVERY source → the model is genuinely free: price $0.00
        // (no unpriced ⚠), like ":free" gateway variants and local models.
        store.modelsdev.insert("qwen-test".into(), super::Price::flat(0.0, 0.0, 0.0, 0.0));
        let free = super::resolve(&store, "qwen-test", 0).unwrap();
        assert_eq!((free.input, free.output), (0.0, 0.0));

        // The baked table outranks 0/0 placeholders: a catalog that lists
        // qwen3.8-max as 0/0 must not shadow Alibaba's documented rates.
        store.litellm.insert("qwen3.8-max".into(), super::Price::flat(0.0, 0.0, 0.0, 0.0));
        let baked = super::resolve(&store, "qwen3.8-max", 0).unwrap();
        assert!((baked.input - 2.0).abs() < 1e-9);
    }

    #[test]
    fn priority_slugs_price_at_base_times_priority_multiplier() {
        let mut store = super::Store::default();
        store
            .supplement
            .insert("gpt-5.6-luna".into(), super::Price::flat(0.2, 1.2, 0.02, 0.25));
        // Devin logs the service tier inside the slug; effort tokens between
        // the base and -priority must not break resolution.
        let p = super::resolve(&store, "gpt-5.6-luna-xhigh-priority", 0).unwrap();
        assert!((p.input - 0.4).abs() < 1e-9);
        assert!((p.output - 2.4).abs() < 1e-9);
        assert!((p.cache_read - 0.04).abs() < 1e-9);

        // gpt-5.5's priority tier is 2.5x, not the default 2x.
        store.supplement.insert("gpt-5.5".into(), super::Price::flat(10.0, 45.0, 1.0, 1.25));
        let p55 = super::resolve(&store, "gpt-5.5-priority", 0).unwrap();
        assert!((p55.input - 25.0).abs() < 1e-9);
    }

    #[test]
    fn stale_gpt56_supplement_prices_are_corrected_until_upstream_updates() {
        let stale = serde_json::json!({ "pricing": {
            "gpt-5.6-terra": { "input_per_million": 2.5, "output_per_million": 15.0,
                               "cache_read_per_million": 0.25, "cache_write_per_million": 3.125 },
            "gpt-5.6-luna":  { "input_per_million": 1.0, "output_per_million": 6.0,
                               "cache_read_per_million": 0.1, "cache_write_per_million": 1.25 },
        }});
        let mut store = super::Store::default();
        super::apply_supplement(&mut store, &stale);
        let terra = store.supplement.get("gpt-5.6-terra").unwrap();
        assert_eq!((terra.input, terra.output, terra.cache_read), (2.0, 12.0, 0.2));
        let luna = store.supplement.get("gpt-5.6-luna").unwrap();
        assert_eq!((luna.input, luna.output, luna.cache_read), (0.2, 1.2, 0.02));

        // Self-retiring: any NEW upstream number passes through untouched.
        let updated = serde_json::json!({ "pricing": {
            "gpt-5.6-terra": { "input_per_million": 1.75, "output_per_million": 11.0 },
        }});
        super::apply_supplement(&mut store, &updated);
        let terra = store.supplement.get("gpt-5.6-terra").unwrap();
        assert_eq!((terra.input, terra.output), (1.75, 11.0));
    }

    #[test]
    fn grok_46_builtin_prices_with_long_context_tier() {
        // Vendor rates (docs.x.ai/docs/pricing): $2/$0.50/$6, doubling for
        // ≥200k prompts — resolvable in every spelling before the public
        // catalogs learn the model. Empty store = builtin only.
        let store = super::Store::default();
        for slug in ["grok-4.6", "grok-4-6", "xai/grok-4.6", "grok-4.6-high"] {
            let p = super::resolve(&store, slug, 0)
                .unwrap_or_else(|| panic!("{slug} did not price"));
            assert_eq!((p.input, p.output, p.cache_read, p.cache_write), (2.0, 6.0, 0.5, 2.0), "{slug}");
            assert_eq!(
                (p.input_200k, p.output_200k, p.cache_read_200k),
                (Some(4.0), Some(12.0), Some(1.0)),
                "{slug}"
            );
        }
        // The fast variant is "twice the price" (launch post): the -fast
        // path scales the whole rate card, long-context tier included.
        let fast = super::resolve(&store, "grok-4.6-fast", 0).unwrap();
        assert_eq!((fast.input, fast.output, fast.cache_read), (4.0, 12.0, 1.0));
        assert_eq!(fast.input_200k, Some(8.0));
    }

    #[test]
    fn kimi_k3_builtin_prices_every_spelling() {
        // Vendor-documented rates (platform.kimi.ai): $3 in, $15 out, $0.30
        // cache hit — resolvable however each tool spells the slug.
        for slug in [
            "kimi-k3",                    // Cursor / Devin bare
            "kimi-k3-code",               // Kimi Code CLI variant
            "moonshot/kimi-k3",           // catalog-style prefix
            "moonshot-ai/kimi-k3-code",   // Kimi CLI's own prefix
            "kimi-k3-high",               // effort tier → peels to base
            "kimi-k3-max",                // mode → peels to base
        ] {
            let p = super::lookup(slug).unwrap_or_else(|| panic!("{slug} did not price"));
            assert_eq!((p.input, p.output, p.cache_read), (3.0, 15.0, 0.3), "{slug}");
        }
    }

    #[test]
    fn long_context_reprices_the_whole_request() {
        let mut p = Price::flat(3.0, 15.0, 0.3, 3.75);
        p.input_200k = Some(6.0);
        p.output_200k = Some(22.5);
        p.cache_read_200k = Some(0.6);

        // Under the threshold: base rates.
        let small = usage(150_000.0, 10_000.0, 0.0, 0.0, 0.0);
        let expect = (150_000.0 * 3.0 + 10_000.0 * 15.0) / 1e6;
        assert!((request_cost(&p, &small, true) - expect).abs() < 1e-9);

        // Prompt over 200k: every component reprices, output included.
        let big = usage(250_000.0, 10_000.0, 0.0, 0.0, 0.0);
        let expect = (250_000.0 * 6.0 + 10_000.0 * 22.5) / 1e6;
        assert!((request_cost(&p, &big, true) - expect).abs() < 1e-9);

        // Aggregated sources opt out and stay on base rates.
        let expect = (250_000.0 * 3.0 + 10_000.0 * 15.0) / 1e6;
        assert!((request_cost(&p, &big, false) - expect).abs() < 1e-9);

        // Cache reads count toward the threshold even with tiny input.
        let cached = usage(1_000.0, 0.0, 240_000.0, 0.0, 0.0);
        let expect = (1_000.0 * 6.0 + 240_000.0 * 0.6) / 1e6;
        assert!((request_cost(&p, &cached, true) - expect).abs() < 1e-9);
    }

    #[test]
    fn one_hour_cache_writes_bill_twice_input() {
        let p = Price::flat(4.0, 20.0, 0.4, 5.0);
        let u = usage(0.0, 0.0, 0.0, 0.0, 1_000_000.0);
        assert!((request_cost(&p, &u, true) - 8.0).abs() < 1e-9);

        // An explicit catalog rate wins over the ×2 convention.
        let mut p = p;
        p.cache_write_1h = Some(9.0);
        assert!((request_cost(&p, &u, true) - 9.0).abs() < 1e-9);
    }

    /// Live probe: fetches the three catalogs and resolves a few real slugs.
    /// Run via `cargo test --lib pricing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_probe() {
        super::ensure_fresh();
        let mut matrix: Vec<String> = vec![
            "claude-opus-4-8".into(),
            "gpt-5.1-codex-max-xhigh".into(),
            "composer-2.5".into(),
            "claude-4.5-haiku-thinking".into(),
            "gpt-5".into(),
            "some-unknown-model-xyz".into(),
        ];
        // The full GPT-5.6 family surface Cursor/Devin can emit: every
        // effort tier, Max/Ultra modes, fast tier, and their compositions.
        for base in ["gpt-5.6-luna", "gpt-5.6-terra", "gpt-5.6-sol"] {
            matrix.push(base.to_string());
            for suffix in [
                "-light", "-low", "-medium", "-high", "-xhigh", "-max", "-ultra", "-fast",
                "-max-xhigh", "-ultra-high", "-max-fast", "-fast-high",
                "-light-fast", "-xhigh-fast", "-ultra-fast", "-max-fast-xhigh",
            ] {
                matrix.push(format!("{base}{suffix}"));
            }
        }
        for model in &matrix {
            match super::lookup(model) {
                Some(p) => eprintln!(
                    "{model}: in=${:.2} out=${:.2} cr=${:.3} cw=${:.2} (per 1M)",
                    p.input, p.output, p.cache_read, p.cache_write
                ),
                None => eprintln!("{model}: UNPRICED"),
            }
        }
    }
}
