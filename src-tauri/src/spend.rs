//! Local spend computation — the "Total Spend" dashboard, per-provider
//! Today / Yesterday / Last 30 Days rows, per-model breakdowns, and the
//! 30-day Usage Trend series. Mirrors the macOS app: costs are derived
//! from the session logs each CLI already writes on this machine, so
//! nothing is sent anywhere.
//!
//! Large logs are handled with a per-file cache keyed by (mtime, size):
//! only files that changed since the last refresh are re-parsed.

use chrono::{DateTime, Datelike, Local, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use crate::pricing;
use crate::providers;

pub const TREND_DAYS: usize = 30;

#[derive(Serialize, Clone)]
pub struct ModelSpend {
    pub model: String,
    pub cost: f64,
    pub tokens: f64,
}

#[derive(Serialize, Clone, Default)]
pub struct Window {
    pub cost: f64,
    pub tokens: f64,
    pub models: Vec<ModelSpend>,
}

#[derive(Serialize, Clone)]
pub struct ProviderSpend {
    pub id: &'static str,
    pub name: &'static str,
    pub today: Window,
    pub yesterday: Window,
    pub last30: Window,
    /// Tokens per day, oldest first — trend[29] is today.
    pub trend: Vec<f64>,
    /// Events whose model no catalog prices. Their measured tokens still
    /// count in token totals/trend, but no dollars are guessed for them
    /// (a deliberate softening of the Mac's exclude-everything semantics:
    /// tokens are facts, only prices are unknown), so dollar figures
    /// under-report and the ⚠ says so.
    pub unpriced: u64,
    pub unpriced_models: Vec<String>,
}

impl ProviderSpend {
    fn has_data(&self) -> bool {
        self.last30.cost > 0.004 || self.last30.tokens > 0.0 || self.unpriced > 0
    }
}

/// (local calendar day, model) → (cost, tokens). Day = days since CE.
type DayMap = HashMap<(i32, String), (f64, f64)>;

/// Everything one file contributes: priced per-day totals plus the tally of
/// unpriced (excluded) events per model name. Cached as a unit so exclusion
/// counts survive the per-file cache.
#[derive(Default, Clone)]
struct FileData {
    days: DayMap,
    unpriced: HashMap<String, u64>,
}

struct FileEntry {
    mtime: SystemTime,
    size: u64,
    /// Pricing-catalog generation the file was priced under — a catalog
    /// refresh re-prices even files that haven't changed on disk.
    gen: u64,
    data: FileData,
}

fn cache() -> &'static Mutex<HashMap<PathBuf, FileEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, FileEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// ---------------------------------------------------------------------------
// Persistent parse cache. The per-file summaries above are tiny (a few
// day/model totals per file) but rebuilding them means re-reading every
// session log ever written — thousands of files, growing forever. Saving
// the summaries to disk makes a fresh launch re-parse only files that
// changed since the last run. The cache is only trusted when it was priced
// under the exact catalog files currently on disk (pricing::catalog_stamp);
// any mismatch, version bump, or parse error discards it wholesale — a
// stale-price cache is worse than a slow first scan.
// ---------------------------------------------------------------------------

const PERSIST_VERSION: u32 = 1;

/// Set when any file was (re)parsed this run — nothing changed, nothing saved.
static CACHE_DIRTY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Paths seen by file_days() this collect() run. Entries for paths nobody
/// scanned anymore (deleted logs, disabled providers) are dropped on save,
/// so the cache can't grow without bound.
fn touched() -> &'static Mutex<HashSet<PathBuf>> {
    static TOUCHED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    TOUCHED.get_or_init(|| Mutex::new(HashSet::new()))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistEntry {
    path: PathBuf,
    /// mtime at full filesystem precision (NTFS is 100ns) — millisecond
    /// rounding would break the equality check and re-parse everything.
    mtime_secs: u64,
    mtime_nanos: u32,
    size: u64,
    days: Vec<(i32, String, f64, f64)>,
    unpriced: Vec<(String, u64)>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistFile {
    version: u32,
    pricing_stamp: String,
    entries: Vec<PersistEntry>,
}

fn persist_path() -> PathBuf {
    providers::config_dir().join("spend_cache.json")
}

/// Loads the persisted cache into the in-memory map, once per app run.
/// Runs after pricing::ensure_fresh() so the stamp reflects any catalog
/// download that just happened — in which case it mismatches and the
/// persisted costs are correctly thrown away.
fn load_persisted_cache() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let Ok(raw) = fs::read_to_string(persist_path()) else { return };
        let Ok(doc) = serde_json::from_str::<PersistFile>(&raw) else { return };
        if doc.version != PERSIST_VERSION || doc.pricing_stamp != pricing::catalog_stamp() {
            return;
        }
        let gen = pricing::generation();
        let Ok(mut map) = cache().lock() else { return };
        for e in doc.entries {
            let mtime = SystemTime::UNIX_EPOCH
                + std::time::Duration::new(e.mtime_secs, e.mtime_nanos);
            let mut data = FileData::default();
            for (day, model, cost, tokens) in e.days {
                data.days.insert((day, model), (cost, tokens));
            }
            data.unpriced = e.unpriced.into_iter().collect();
            map.insert(e.path, FileEntry { mtime, size: e.size, gen, data });
        }
    });
}

/// Writes the cache back to disk (atomically, via temp + rename) when this
/// run parsed anything new. Only entries that are current — touched this
/// run and priced under the live catalog generation — are persisted.
fn save_persisted_cache() {
    let dirty = CACHE_DIRTY.swap(false, std::sync::atomic::Ordering::Relaxed);
    let path = persist_path();
    if !dirty && path.exists() {
        return;
    }
    let gen = pricing::generation();
    let Ok(touched_set) = touched().lock() else { return };
    let Ok(map) = cache().lock() else { return };
    let entries: Vec<PersistEntry> = map
        .iter()
        .filter(|(p, e)| e.gen == gen && touched_set.contains(*p))
        .map(|(p, e)| {
            let d = e
                .mtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            PersistEntry {
                path: p.clone(),
                mtime_secs: d.as_secs(),
                mtime_nanos: d.subsec_nanos(),
                size: e.size,
                days: e
                    .data
                    .days
                    .iter()
                    .map(|((day, model), (cost, tokens))| (*day, model.clone(), *cost, *tokens))
                    .collect(),
                unpriced: e.data.unpriced.iter().map(|(m, c)| (m.clone(), *c)).collect(),
            }
        })
        .collect();
    let doc = PersistFile {
        version: PERSIST_VERSION,
        pricing_stamp: pricing::catalog_stamp(),
        entries,
    };
    let Ok(json) = serde_json::to_string(&doc) else { return };
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

fn day_of_utc(ts: DateTime<Utc>) -> i32 {
    ts.with_timezone(&Local).date_naive().num_days_from_ce()
}

fn add_event(data: &mut FileData, ts: DateTime<Utc>, model: &str, cost: f64, tokens: f64) {
    let entry = data
        .days
        .entry((day_of_utc(ts), model.to_string()))
        .or_insert((0.0, 0.0));
    entry.0 += cost;
    entry.1 += tokens;
}

/// Tally an event no catalog can price: its tokens still count (they're
/// measured, not guessed) at zero cost, so only the dollars under-report.
fn note_unpriced(data: &mut FileData, ts: DateTime<Utc>, model: &str, tokens: f64) {
    *data.unpriced.entry(model.to_string()).or_insert(0) += 1;
    if tokens > 0.0 {
        add_event(data, ts, model, 0.0, tokens);
    }
}

fn merge_data(target: &mut FileData, source: FileData) {
    for (key, (cost, tokens)) in source.days {
        let entry = target.days.entry(key).or_insert((0.0, 0.0));
        entry.0 += cost;
        entry.1 += tokens;
    }
    for (model, count) in source.unpriced {
        *target.unpriced.entry(model).or_insert(0) += count;
    }
}

/// Ranked model list for one window: top models by cost, anything past the
/// fifth name or under a 5% share folds into "Other".
fn finalize_models(raw: HashMap<String, (f64, f64)>, window_cost: f64) -> Vec<ModelSpend> {
    let mut list: Vec<ModelSpend> = raw
        .into_iter()
        .map(|(model, (cost, tokens))| ModelSpend { model, cost, tokens })
        .collect();
    list.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    let mut named = Vec::new();
    let mut other = ModelSpend { model: "Other".into(), cost: 0.0, tokens: 0.0 };
    for (i, m) in list.into_iter().enumerate() {
        let share = if window_cost > 0.0 { m.cost / window_cost } else { 0.0 };
        if i < 5 && (share >= 0.05 || i == 0) {
            named.push(m);
        } else {
            other.cost += m.cost;
            other.tokens += m.tokens;
        }
    }
    if other.cost > 0.001 || other.tokens > 0.0 {
        named.push(other);
    }
    named
}

fn build_spend(id: &'static str, name: &'static str, data: FileData) -> ProviderSpend {
    let today = Local::now().date_naive().num_days_from_ce();
    let mut unpriced_models: Vec<String> = data.unpriced.keys().cloned().collect();
    unpriced_models.sort();
    unpriced_models.truncate(5);
    let days = data.days;
    let mut sp = ProviderSpend {
        id,
        name,
        today: Window::default(),
        yesterday: Window::default(),
        last30: Window::default(),
        trend: vec![0.0; TREND_DAYS],
        unpriced: data.unpriced.values().sum(),
        unpriced_models,
    };
    let mut models: [HashMap<String, (f64, f64)>; 3] =
        [HashMap::new(), HashMap::new(), HashMap::new()];

    for ((day, model), (cost, tokens)) in days {
        let mut bump = |idx: usize, w: &mut Window| {
            w.cost += cost;
            w.tokens += tokens;
            let entry = models[idx].entry(model.clone()).or_insert((0.0, 0.0));
            entry.0 += cost;
            entry.1 += tokens;
        };
        if day == today {
            bump(0, &mut sp.today);
        }
        if day == today - 1 {
            bump(1, &mut sp.yesterday);
        }
        if day > today - TREND_DAYS as i32 {
            bump(2, &mut sp.last30);
            let idx = (day - (today - TREND_DAYS as i32 + 1)) as usize;
            if idx < TREND_DAYS {
                sp.trend[idx] += tokens;
            }
        }
    }

    let [m0, m1, m2] = models;
    sp.today.models = finalize_models(m0, sp.today.cost);
    sp.yesterday.models = finalize_models(m1, sp.yesterday.cost);
    sp.last30.models = finalize_models(m2, sp.last30.cost);
    sp
}

/// All .jsonl files under `root` modified in the last 31 days.
/// Symlinks and junctions are followed throughout: `is_dir()` resolves
/// links when recursing, and the recency check below reads the *target*
/// file's mtime — a link's own (usually ancient) timestamp must not hide
/// logs a user relocated to another drive.
fn recent_jsonl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else { return };
    let cutoff = SystemTime::now() - Duration::from_secs(31 * 86_400);
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            recent_jsonl_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            if let Ok(meta) = fs::metadata(&path) {
                if meta.modified().map(|m| m >= cutoff).unwrap_or(true) {
                    out.push(path);
                }
            }
        }
    }
}

/// Parses one file into per-day totals, via the cache when unchanged.
fn file_days(path: &Path, parse: &mut dyn FnMut(&str, &mut FileData)) -> FileData {
    let Ok(meta) = fs::metadata(path) else { return FileData::default() };
    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let size = meta.len();

    if let Ok(mut t) = touched().lock() {
        t.insert(path.to_path_buf());
    }
    let gen = pricing::generation();
    if let Ok(map) = cache().lock() {
        if let Some(entry) = map.get(path) {
            if entry.mtime == mtime && entry.size == size && entry.gen == gen {
                return entry.data.clone();
            }
        }
    }

    let mut data = FileData::default();
    if let Ok(file) = fs::File::open(path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            parse(&line, &mut data);
        }
    }

    if let Ok(mut map) = cache().lock() {
        map.insert(path.to_path_buf(), FileEntry { mtime, size, gen, data: data.clone() });
    }
    CACHE_DIRTY.store(true, std::sync::atomic::Ordering::Relaxed);
    data
}

fn parse_ts(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let value = value?;
    if let Some(s) = value.as_str() {
        return DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc));
    }
    if let Some(n) = value.as_i64() {
        // Heuristic: values past ~2001-09 in ms are millisecond stamps.
        let ms = if n > 1_000_000_000_000 { n } else { n * 1000 };
        return DateTime::from_timestamp_millis(ms);
    }
    None
}

// ---------------------------------------------------------------------------
// Pricing (dollars per million tokens: input, output, cache read, cache write)
// ---------------------------------------------------------------------------

fn claude_price(model: &str) -> Option<(f64, f64, f64, f64)> {
    let m = model.to_lowercase();
    if m.contains("opus") {
        Some((15.0, 75.0, 1.5, 18.75))
    } else if m.contains("sonnet") {
        Some((3.0, 15.0, 0.3, 3.75))
    } else if m.contains("haiku") {
        Some((1.0, 5.0, 0.1, 1.25))
    } else {
        // Unknown model (or a new family): rely on the log's own costUSD;
        // counting tokens at a guessed price would fabricate dollars.
        None
    }
}

fn codex_price(model: &str) -> (f64, f64, f64) {
    let m = model.to_lowercase();
    if m.contains("mini") || m.contains("spark") {
        (0.25, 2.0, 0.025)
    } else {
        // gpt-5 family / codex defaults
        (1.25, 10.0, 0.125)
    }
}

fn grok_price(model: &str) -> (f64, f64) {
    let m = model.to_lowercase();
    if m.contains("code") || m.contains("fast") {
        (0.2, 1.5)
    } else {
        (3.0, 15.0)
    }
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

/// Token buckets of one Claude usage object — a message's `usage` or one
/// advisor iteration inside `usage.iterations` (same field names). `None`
/// when the required input/output counts are missing, or when `speed`
/// carries a value outside the known set (an unrecognized log shape — the
/// Mac skips those lines too).
struct ClaudeTokens {
    input: f64,
    output: f64,
    cache_read: f64,
    w5m: f64,
    w1h: f64,
    fast: bool,
}

impl ClaudeTokens {
    fn total(&self) -> f64 {
        self.input + self.output + self.cache_read + self.w5m + self.w1h
    }
}

fn claude_tokens(u: &Value) -> Option<ClaudeTokens> {
    let input = u.get("input_tokens").and_then(Value::as_f64)?;
    let output = u.get("output_tokens").and_then(Value::as_f64)?;
    let speed = u.get("speed").and_then(Value::as_str);
    if let Some(s) = speed {
        if s != "fast" && s != "standard" {
            return None;
        }
    }
    let num = |k: &str| u.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let cache_write = num("cache_creation_input_tokens");
    // Cache writes split by lifetime when the breakdown is present —
    // 1-hour writes bill at twice the input rate.
    let (w5m, w1h) = match u.get("cache_creation") {
        Some(cc) => {
            let g = |k: &str| cc.get(k).and_then(Value::as_f64).unwrap_or(0.0);
            let (a, b) = (g("ephemeral_5m_input_tokens"), g("ephemeral_1h_input_tokens"));
            if a + b > 0.0 { (a, b) } else { (cache_write, 0.0) }
        }
        None => (cache_write, 0.0),
    };
    Some(ClaudeTokens {
        input,
        output,
        cache_read: num("cache_read_input_tokens"),
        w5m,
        w1h,
        fast: speed == Some("fast"),
    })
}

/// Price one Claude entry: live catalog → static family fallback → None
/// (excluded, never a guessed $0). Fast-flagged requests scale by the
/// supplement's multiplier.
fn claude_cost(model: &str, t: &ClaudeTokens) -> Option<f64> {
    let price = pricing::lookup(model).or_else(|| {
        claude_price(model).map(|(i, o, cr, cw)| pricing::Price::flat(i, o, cr, cw))
    })?;
    let u = pricing::Usage {
        input: t.input,
        output: t.output,
        cache_read: t.cache_read,
        cache_write_5m: t.w5m,
        cache_write_1h: t.w1h,
    };
    let mult = if t.fast { pricing::fast_multiplier(model) } else { 1.0 };
    Some(pricing::request_cost(&price, &u, true) * mult)
}

/// Per-file dedup state for the Claude scanner.
#[derive(Default)]
struct ClaudeFileState {
    /// (message id, request id) pairs already counted.
    seen: HashSet<String>,
    /// message id → whether its first occurrence was a sidechain line.
    seen_mids: HashMap<String, bool>,
}

/// Parse one Claude Code session-log line into spend events. Persisted
/// `claude -p` runs write the same assistant records (entrypoint "sdk-cli"),
/// so they count like interactive usage; `--no-session-persistence` runs
/// write no log at all.
fn claude_line(st: &mut ClaudeFileState, line: &str, data: &mut FileData) {
    if !line.contains("\"type\":\"assistant\"") {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(line) else { return };
    if v.get("type").and_then(Value::as_str) != Some("assistant") {
        return;
    }
    let Some(ts) = parse_ts(v.get("timestamp")) else { return };
    let usage = v.pointer("/message/usage").cloned().unwrap_or(Value::Null);
    let Some(t) = claude_tokens(&usage) else { return };

    // Resumed sessions repeat messages under the same request id, and
    // sidechain logs replay the parent's message under a *fresh* request id
    // — dedupe on both. Keep-first: the parent line precedes its sidechain
    // replay in the log. (The Mac also re-prefers a parent that arrives
    // after its sidechain copy; a streaming pass can't retract an event, so
    // that rarer order keeps the sidechain copy — still counted once.)
    let sidechain = v.get("isSidechain").and_then(Value::as_bool).unwrap_or(false);
    if let Some(mid) = v.pointer("/message/id").and_then(Value::as_str) {
        let rid = v.get("requestId").and_then(Value::as_str).unwrap_or("");
        if !st.seen.insert(format!("{mid}:{rid}")) {
            return;
        }
        if let Some(&first_was_sidechain) = st.seen_mids.get(mid) {
            if sidechain || first_was_sidechain {
                return;
            }
            // Same message id under distinct request ids with no sidechain
            // involved is a genuine retry — both count (Mac parity).
        }
        st.seen_mids.entry(mid.to_string()).or_insert(sidechain);
    }

    // `<synthetic>` is Claude Code's placeholder for tool-generated turns:
    // there is no real model to price or warn about, so only a carried
    // costUSD makes the line count (as unattributed usage).
    let model_raw = v.pointer("/message/model").and_then(Value::as_str);
    let synthetic = model_raw == Some("<synthetic>");
    let model = model_raw.unwrap_or("unknown").to_string();

    // Cost preference: the log's own costUSD → live catalog price
    // → static family fallback → excluded (never a guessed $0).
    match v.get("costUSD").and_then(Value::as_f64) {
        Some(c) => {
            let name = if synthetic { "unattributed" } else { model.as_str() };
            if t.total() > 0.0 || c > 0.0 {
                add_event(data, ts, name, c, t.total());
            }
        }
        None if synthetic => {}
        None => match claude_cost(&model, &t) {
            Some(c) => {
                if t.total() > 0.0 || c > 0.0 {
                    add_event(data, ts, &model, c, t.total());
                }
            }
            None => {
                if t.total() > 0.0 {
                    note_unpriced(data, ts, &model, t.total());
                }
            }
        },
    }

    // Fable-era logs nest advisor work in `usage.iterations`. Only
    // advisor-message iterations become extra entries, under the advisor's
    // own model — ordinary message iterations are already inside the
    // parent's usage totals, and counting them again would double-count.
    let Some(iters) = usage.get("iterations").and_then(Value::as_array) else { return };
    for it in iters {
        if it.get("type").and_then(Value::as_str) != Some("advisor_message") {
            continue;
        }
        let Some(advisor) = it
            .get("model")
            .and_then(Value::as_str)
            .filter(|m| !m.is_empty() && *m != "<synthetic>")
        else {
            continue;
        };
        let Some(at) = claude_tokens(it) else { continue };
        if at.total() <= 0.0 {
            continue;
        }
        match claude_cost(advisor, &at) {
            Some(c) => add_event(data, ts, advisor, c, at.total()),
            None => note_unpriced(data, ts, advisor, at.total()),
        }
    }
}

/// Move every event whose model starts with `prefix` out of `data` into a
/// new FileData (unpriced tallies included). Used to re-route usage that a
/// CLI logged on another vendor's behalf.
fn split_models(data: &mut FileData, prefix: &str) -> FileData {
    let mut out = FileData::default();
    data.days.retain(|(day, model), v| {
        if model.starts_with(prefix) {
            out.days.insert((*day, model.clone()), *v);
            false
        } else {
            true
        }
    });
    let moved: Vec<String> = data
        .unpriced
        .keys()
        .filter(|m| m.starts_with(prefix))
        .cloned()
        .collect();
    for m in moved {
        if let Some(c) = data.unpriced.remove(&m) {
            out.unpriced.insert(m, c);
        }
    }
    out
}

/// Claude Code writes one JSONL per session under ~/.claude/projects. Each
/// assistant line carries usage token counts and usually a precomputed
/// costUSD, which we prefer over our own pricing table.
///
/// Claude Code can also run against MiniMax's Anthropic-compatible endpoint
/// (ANTHROPIC_BASE_URL); those sessions log MiniMax models into the same
/// files. That usage is split out and returned separately — it belongs on
/// the MiniMax card, not Claude's.
fn claude() -> (ProviderSpend, FileData) {
    let root = std::env::var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".claude"))
        .join("projects");

    let mut files = Vec::new();
    recent_jsonl_files(&root, &mut files);
    let mut all = FileData::default();
    for file in files {
        let mut state = ClaudeFileState::default();
        let data = file_days(&file, &mut |line, data| claude_line(&mut state, line, data));
        merge_data(&mut all, data);
    }
    let minimax = split_models(&mut all, "MiniMax");
    (build_spend("claude", "Claude", all), minimax)
}

/// MiniMax spend: the Agent CLI's local token_usage store (its own cost_usd
/// preferred, catalog-priced otherwise) plus whatever Claude Code logged
/// while pointed at MiniMax's endpoint (passed in from the Claude scan).
fn minimax(extra: FileData) -> ProviderSpend {
    let mut data = extra;
    for ev in providers::minimax::collect_usage_events() {
        let Some(ts) = DateTime::from_timestamp_millis(ev.ts_ms) else { continue };
        let tokens = ev.input + ev.output + ev.reasoning + ev.cache_read + ev.cache_write;
        if tokens <= 0.0 && ev.cost_usd <= 0.0 {
            continue;
        }
        // Rows carry provider-prefixed slugs ("minimax/MiniMax-M3") — the
        // bare model is what the card, catalogs, and the Claude-side split
        // all use.
        let model = ev.model.strip_prefix("minimax/").unwrap_or(&ev.model).to_string();
        if ev.cost_usd > 0.0 {
            add_event(&mut data, ts, &model, ev.cost_usd, tokens);
            continue;
        }
        match pricing::lookup(&model) {
            Some(p) => {
                let u = pricing::Usage {
                    input: ev.input,
                    output: ev.output + ev.reasoning,
                    cache_read: ev.cache_read,
                    cache_write_5m: ev.cache_write,
                    cache_write_1h: 0.0,
                };
                add_event(&mut data, ts, &model, pricing::request_cost(&p, &u, true), tokens);
            }
            None => note_unpriced(&mut data, ts, &model, tokens),
        }
    }
    build_spend("minimax", "MiniMax", data)
}

/// Which spend slice a Hermes row belongs to. Hermes (Nous Research's
/// desktop agent) routes chats through whichever backend the user connected,
/// so its ledger rows are filed under the provider that actually billed
/// them; routes Pane has no slice for stay under Hermes's own name.
fn hermes_bucket(billing_provider: &str) -> (&'static str, &'static str) {
    let lower = billing_provider.to_lowercase();
    if lower.contains("minimax") {
        ("minimax", "MiniMax")
    } else if lower.contains("openrouter") {
        ("openrouter", "OpenRouter")
    } else {
        ("hermes", "Hermes")
    }
}

/// Hermes spend, grouped per target slice. Rows are cumulative per
/// (session, model, route) — the app updates them in place while a session
/// runs — so every refresh rebuilds from the full table instead of
/// accumulating deltas. The whole session lands on its last-active day.
fn hermes() -> Vec<(&'static str, &'static str, FileData)> {
    let mut buckets: Vec<(&'static str, &'static str, FileData)> = Vec::new();
    for ev in providers::hermes::collect_usage_events() {
        let Some(ts) = DateTime::from_timestamp_millis(ev.ts_ms) else { continue };
        let tokens = ev.input + ev.output + ev.reasoning + ev.cache_read + ev.cache_write;
        if tokens <= 0.0 && ev.cost_usd <= 0.0 {
            continue;
        }
        let (id, name) = hermes_bucket(&ev.billing_provider);
        let data = match buckets.iter_mut().find(|(bid, _, _)| *bid == id) {
            Some((_, _, data)) => data,
            None => {
                buckets.push((id, name, FileData::default()));
                &mut buckets.last_mut().unwrap().2
            }
        };
        if ev.cost_usd > 0.0 {
            add_event(data, ts, &ev.model, ev.cost_usd, tokens);
            continue;
        }
        match pricing::lookup(&ev.model) {
            Some(p) => {
                let u = pricing::Usage {
                    input: ev.input,
                    output: ev.output + ev.reasoning,
                    cache_read: ev.cache_read,
                    cache_write_5m: ev.cache_write,
                    cache_write_1h: 0.0,
                };
                // Rows aggregate a whole session's requests, so no single
                // request can be proven long-context — stay on base rates
                // (same reasoning as the Cursor CSV scanner).
                add_event(data, ts, &ev.model, pricing::request_cost(&p, &u, false), tokens);
            }
            None => note_unpriced(data, ts, &ev.model, tokens),
        }
    }
    buckets
}

/// One `token_count` usage object, tolerating the older field spellings
/// (`prompt_tokens`, `cache_read_input_tokens`, …) the Mac scanner accepts.
#[derive(Clone, PartialEq)]
struct CodexRaw {
    input: f64,
    cached: f64,
    output: f64,
    reasoning: f64,
    total: f64,
}

fn codex_raw(v: &Value) -> CodexRaw {
    let num = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| v.get(*k).and_then(Value::as_f64))
            .unwrap_or(0.0)
    };
    let input = num(&["input_tokens", "prompt_tokens", "input"]);
    let cached = num(&["cached_input_tokens", "cache_read_input_tokens", "cached_tokens"]);
    let output = num(&["output_tokens", "completion_tokens", "output"]);
    let reasoning = num(&["reasoning_output_tokens", "reasoning_tokens"]);
    let reported = num(&["total_tokens"]);
    let recomputed = input + output + reasoning;
    let total = if reported > 0.0 || recomputed == 0.0 { reported } else { recomputed };
    CodexRaw { input, cached, output, reasoning, total }
}

impl CodexRaw {
    fn any_tokens(&self) -> bool {
        self.input > 0.0 || self.cached > 0.0 || self.output > 0.0 || self.reasoning > 0.0
    }

    /// Recover a turn delta from cumulative totals (when `last_token_usage`
    /// is absent).
    fn minus(&self, prev: Option<&CodexRaw>) -> CodexRaw {
        let p = |f: fn(&CodexRaw) -> f64| prev.map(f).unwrap_or(0.0);
        CodexRaw {
            input: (self.input - p(|r| r.input)).max(0.0),
            cached: (self.cached - p(|r| r.cached)).max(0.0),
            output: (self.output - p(|r| r.output)).max(0.0),
            reasoning: (self.reasoning - p(|r| r.reasoning)).max(0.0),
            total: (self.total - p(|r| r.total)).max(0.0),
        }
    }
}

/// A session_meta payload marking the file as a child session (subagent
/// spawn or fork) whose leading `token_count` lines replay the parent's
/// history. JSON `null` and blank strings count as absent — a root session
/// declaring `forked_from_id: null` must not be misclassified as a child.
fn codex_child_meta(payload: &Value) -> bool {
    let set = |k: &str| {
        payload.get(k).is_some_and(|v| match v {
            Value::Null => false,
            Value::String(s) => !s.trim().is_empty(),
            _ => true,
        })
    };
    set("forked_from_id")
        || set("parent_thread_id")
        || payload.get("thread_source").and_then(Value::as_str) == Some("subagent")
        || payload.pointer("/source/subagent").is_some_and(|v| !v.is_null())
}

/// How a child session's replayed parent history is gated until its first
/// live turn.
enum CodexReplayGate {
    /// Clear when `task_started.started_at` is at/after the child's creation
    /// epoch (replayed task_started lines carry the parent's older one).
    UntilStartedAt(f64),
    /// The child's session_meta had no parseable creation timestamp: clear
    /// when `started_at` is at/after that task_started line's own wall-clock
    /// second.
    SelfTimed,
}

/// Per-file parse state for one Codex rollout.
#[derive(Default)]
struct CodexFileState {
    model: String,
    saw_meta: bool,
    gate: Option<CodexReplayGate>,
    fast_tier: bool,
    prev_totals: Option<CodexRaw>,
}

/// Date-stamped snapshots ("gpt-5.6-sol-2026-06-01" / "-20260601") map to
/// their base slug for the provider tables below.
fn codex_dated_base(model: &str) -> String {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"-(\d{4}-\d{2}-\d{2}|\d{8})$").unwrap());
    re.replace(model, "").into_owned()
}

/// Codex priority/fast service-tier multipliers are provider-specific and
/// intentionally not Cursor's `-fast` supplement multipliers. Unknown models
/// use the supplement's multiplier when one exists, else 2x.
fn codex_priority_multiplier(dated: &str, rate_model: &str) -> f64 {
    match dated {
        "gpt-5.5" | "gpt-5.5-pro" => 2.5,
        "gpt-5.4" | "gpt-5.4-pro" | "gpt-5.6-sol" | "gpt-5.6-terra" | "gpt-5.6-luna" => 2.0,
        _ => {
            let m = pricing::fast_multiplier(rate_model);
            if m == 1.0 { 2.0 } else { m }
        }
    }
}

/// OpenAI's published long-context rates (input, output, cache read $/MTok)
/// for Codex models — the whole request switches tiers above 272k prompt
/// tokens, not the 200k Anthropic uses.
fn codex_long_context(dated: &str) -> Option<(f64, f64, f64)> {
    match dated {
        "gpt-5.4" | "gpt-5.6-terra" => Some((5.0, 22.5, 0.5)),
        "gpt-5.4-pro" | "gpt-5.5-pro" => Some((60.0, 270.0, 60.0)),
        "gpt-5.5" | "gpt-5.6-sol" => Some((10.0, 45.0, 1.0)),
        "gpt-5.6-luna" => Some((2.0, 9.0, 0.2)),
        _ => None,
    }
}

/// OpenAI publishes no cached-input discount for these Pro models: cached
/// input bills at the full input rate.
fn codex_no_cache_discount(dated: &str) -> bool {
    matches!(dated, "gpt-5.4-pro" | "gpt-5.5-pro")
}

/// Parse one Codex rollout line. Tracks the current model (turn_context),
/// the fast/priority service tier (thread_settings_applied — config.toml is
/// deliberately not consulted, toggling it must not reprice history), and a
/// child session's replay gate; normalizes each token_count into a delta
/// event.
fn codex_line(st: &mut CodexFileState, line: &str, data: &mut FileData) {
    if !(line.contains("token_count")
        || line.contains("turn_context")
        || line.contains("session_meta")
        || line.contains("task_started")
        || line.contains("thread_settings_applied"))
    {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(line) else { return };

    // Only the file's own (first) session_meta counts — a child file replays
    // the parent's session_meta lines right after its own.
    if v.get("type").and_then(Value::as_str) == Some("session_meta") {
        if !st.saw_meta {
            st.saw_meta = true;
            if let Some(p) = v.get("payload") {
                if codex_child_meta(p) {
                    st.gate = Some(match parse_ts(v.get("timestamp")) {
                        Some(ts) => CodexReplayGate::UntilStartedAt(ts.timestamp() as f64),
                        None => CodexReplayGate::SelfTimed,
                    });
                }
                if let Some(m) = p.get("model").and_then(Value::as_str) {
                    st.model = m.to_string();
                }
            }
        }
        return;
    }

    match v.pointer("/payload/type").and_then(Value::as_str) {
        Some("thread_settings_applied") => {
            let tier = v
                .pointer("/payload/thread_settings/service_tier")
                .or_else(|| v.pointer("/payload/service_tier"))
                .and_then(Value::as_str);
            if let Some(t) = tier {
                st.fast_tier = t == "fast" || t == "priority";
            }
            return;
        }
        Some("task_started") => {
            // The first live task_started ends a child's replayed history —
            // replayed ones carry the parent's original, older started_at.
            if let Some(gate) = &st.gate {
                if let Some(started) = v.pointer("/payload/started_at").and_then(Value::as_f64) {
                    let cleared = match gate {
                        CodexReplayGate::UntilStartedAt(t) => started >= *t,
                        CodexReplayGate::SelfTimed => parse_ts(v.get("timestamp"))
                            .is_some_and(|ts| started >= ts.timestamp() as f64),
                    };
                    if cleared {
                        st.gate = None;
                    }
                }
            }
            return;
        }
        Some("token_count") => {}
        _ => {
            // turn_context (or older shapes): update the session's model.
            if let Some(m) = v.pointer("/payload/model").and_then(Value::as_str) {
                st.model = m.to_string();
            }
            return;
        }
    }

    // token_count from here on. A model on the line itself wins.
    if let Some(m) = v
        .pointer("/payload/model")
        .and_then(Value::as_str)
        .or_else(|| v.pointer("/payload/info/model").and_then(Value::as_str))
    {
        st.model = m.to_string();
    }
    let Some(ts) = parse_ts(v.get("timestamp")) else { return };
    let totals = v.pointer("/payload/info/total_token_usage").map(codex_raw);

    // Replayed parent history: seed the delta baseline, never count it —
    // a large parent history takes several seconds to replay, which is why
    // this is a log marker and not a time window (the Mac's old one-second
    // window leaked replays and inflated spend ~20x).
    if st.gate.is_some() {
        if let Some(t) = totals {
            st.prev_totals = Some(t);
        }
        return;
    }
    // Unchanged cumulative totals mean a re-emitted stale snapshot, not new
    // usage — even when the line repeats a last_token_usage.
    if let (Some(t), Some(p)) = (&totals, &st.prev_totals) {
        if t == p {
            return;
        }
    }
    let usage = match v.pointer("/payload/info/last_token_usage") {
        Some(l) => codex_raw(l),
        None => match &totals {
            Some(t) => t.minus(st.prev_totals.as_ref()),
            None => return,
        },
    };
    if let Some(t) = totals {
        st.prev_totals = Some(t);
    }
    if !usage.any_tokens() {
        return;
    }

    let model = if st.model.is_empty() { "gpt-5".to_string() } else { st.model.clone() };
    let tokens = usage.total;

    // Codex speed is a provider tier, not Cursor's `-fast` price variant: a
    // `-fast` slug resolves through its unscaled base rates and the Codex
    // multiplier applies exactly once. A fast-only third-party slug with no
    // base entry keeps its already-scaled rate, no second multiplier.
    let (rate_model, alias_fast) = match model.strip_suffix("-fast") {
        Some(base) if !base.is_empty() => (base.to_string(), true),
        _ => (model.clone(), false),
    };
    let lower = model.to_lowercase();
    let base_price = pricing::lookup(&rate_model);
    let price = base_price
        .or_else(|| if alias_fast { pricing::lookup(&model) } else { None })
        .or_else(|| {
            // The static gpt-5 table only for recognizably Codex-family
            // models; anything else is excluded.
            if lower.contains("gpt") || lower.contains("codex") {
                let (i, o, cr) = codex_price(&rate_model);
                Some(pricing::Price::flat(i, o, cr, i))
            } else {
                None
            }
        });
    let Some(mut p) = price else {
        note_unpriced(data, ts, &model, tokens);
        return;
    };

    let dated = codex_dated_base(&rate_model.to_lowercase());
    let mut threshold = 200_000.0;
    if let Some((i, o, cr)) = codex_long_context(&dated) {
        p.input_200k = Some(i);
        p.output_200k = Some(o);
        p.cache_read_200k = Some(cr);
        threshold = 272_000.0;
    }
    if codex_no_cache_discount(&dated) {
        p.cache_read = p.input;
        p.cache_read_200k = p.input_200k;
    }
    let is_fast = if alias_fast { base_price.is_some() } else { st.fast_tier };
    let mult = if is_fast { codex_priority_multiplier(&dated, &rate_model) } else { 1.0 };

    let cached = usage.cached.min(usage.input);
    let u = pricing::Usage {
        input: usage.input - cached,
        output: usage.output,
        cache_read: cached,
        cache_write_5m: 0.0,
        cache_write_1h: 0.0,
    };
    add_event(data, ts, &model, pricing::request_cost_at(&p, &u, threshold) * mult, tokens);
}

/// Codex rollout files log a token_count event per turn; the model rides in
/// the surrounding turn_context/session_meta lines. Child sessions (subagent
/// spawns and forks) replay the parent's entire history at spawn — those
/// lines are skipped via a replay gate (see `codex_line`).
fn codex() -> ProviderSpend {
    let home = std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".codex"));

    // An archived session is often a byte-for-byte copy of one still in
    // sessions/ — count each relative path once, sessions/ winning.
    let sessions_root = home.join("sessions");
    let archived_root = home.join("archived_sessions");
    let mut files = Vec::new();
    recent_jsonl_files(&sessions_root, &mut files);
    let live_rel: HashSet<PathBuf> = files
        .iter()
        .filter_map(|f| f.strip_prefix(&sessions_root).ok().map(Path::to_path_buf))
        .collect();
    let mut archived = Vec::new();
    recent_jsonl_files(&archived_root, &mut archived);
    files.extend(archived.into_iter().filter(|f| {
        f.strip_prefix(&archived_root)
            .map(|rel| !live_rel.contains(rel))
            .unwrap_or(true)
    }));

    let mut all = FileData::default();
    for file in files {
        let mut state = CodexFileState::default();
        let data = file_days(&file, &mut |line, data| codex_line(&mut state, line, data));
        merge_data(&mut all, data);
    }
    build_spend("codex", "Codex", all)
}

/// Grok CLI appends one global log at ~/.grok/logs/unified.jsonl (or under
/// $GROK_HOME). Token counts ride on `shell.turn.inference_done` lines
/// (prompt/completion/reasoning/cached_prompt); those rows carry no model
/// id, so the active model is tracked per CLI process from the model-change
/// events the CLI also logs — the same scheme the Mac scanner uses.
fn grok() -> ProviderSpend {
    let root = std::env::var("GROK_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".grok"));
    let path = root.join("logs").join("unified.jsonl");
    let mut all = FileData::default();
    if path.exists() {
        let mut model_by_pid: HashMap<i64, String> = HashMap::new();
        let data = file_days(&path, &mut |line, data| {
            if !line.contains("inference_done") && !line.contains("model") {
                return;
            }
            let Ok(v) = serde_json::from_str::<Value>(line) else { return };
            let Some(msg) = v.get("msg").and_then(Value::as_str) else { return };
            let ctx = v.get("ctx").cloned().unwrap_or(Value::Null);
            let pid = v.get("pid").and_then(Value::as_i64);
            let model_field = match msg {
                "model changed" => ctx.get("model"),
                "model catalog: notifying clients" => ctx.get("current_model_id"),
                "backend_search: model switch" => ctx
                    .get("model")
                    .or_else(|| ctx.get("current_model_id"))
                    .or_else(|| ctx.get("model_id")),
                "subagent model resolved" => ctx.get("model_id").or_else(|| ctx.get("model")),
                _ => None,
            };
            if let Some(m) = model_field.and_then(Value::as_str) {
                let m = m.trim();
                if !m.is_empty() {
                    if let Some(pid) = pid {
                        model_by_pid.insert(pid, m.to_string());
                    }
                }
                return;
            }
            if msg != "shell.turn.inference_done" {
                return;
            }
            let num = |k: &str| ctx.get(k).and_then(Value::as_f64);
            let Some(prompt) = num("prompt_tokens") else { return };
            let Some(ts) = parse_ts(v.get("ts")) else { return };
            let output = num("completion_tokens").unwrap_or(0.0) + num("reasoning_tokens").unwrap_or(0.0);
            // cached_prompt_tokens is a subset of prompt_tokens, so the total
            // counts the prompt once.
            let cached = num("cached_prompt_tokens").unwrap_or(0.0).min(prompt);
            let tokens = prompt + output;
            if tokens <= 0.0 {
                return;
            }
            // Token rows carry no model id — attribute via the row's process;
            // rows with no attributable model are excluded, like the Mac.
            let Some(model) = pid.and_then(|p| model_by_pid.get(&p)).cloned() else { return };
            // Static backstop only for recognizably Grok-family models
            // (catalog down); it has no cache rate, so cached tokens are
            // conservatively priced as fresh input there.
            let price = pricing::lookup(&model).or_else(|| {
                if model.to_lowercase().contains("grok") {
                    let (i, o) = grok_price(&model);
                    Some(pricing::Price::flat(i, o, i, i))
                } else {
                    None
                }
            });
            let Some(p) = price else {
                note_unpriced(data, ts, &model, tokens);
                return;
            };
            let u = pricing::Usage {
                input: prompt - cached,
                output,
                cache_read: cached,
                cache_write_5m: 0.0,
                cache_write_1h: 0.0,
            };
            add_event(data, ts, &model, pricing::request_cost(&p, &u, true), tokens);
        });
        merge_data(&mut all, data);
    }
    build_spend("grok", "Grok", all)
}

/// OpenCode stores real per-message costs in its database — no pricing
/// table needed.
fn opencode() -> ProviderSpend {
    let mut data = FileData::default();
    for (ts_ms, cost, tokens, model) in providers::opencode::collect_cost_events() {
        if let Some(ts) = DateTime::from_timestamp_millis(ts_ms as i64) {
            add_event(&mut data, ts, &model, cost, tokens);
        }
    }
    build_spend("opencode", "OpenCode", data)
}

/// Devin CLI keeps per-request token metrics in its local sessions.db
/// (cloud Devin sessions bill ACUs and write no local logs, so only CLI
/// usage appears). Events carry the session's model with Windsurf-style
/// reasoning-effort suffixes stripped for pricing.
fn devin() -> ProviderSpend {
    let mut data = FileData::default();
    for ev in providers::devin::collect_usage_events() {
        let Some(ts) = DateTime::from_timestamp_millis(ev.ts_ms) else { continue };
        let tokens = ev.input + ev.output + ev.cache_read + ev.cache_write;
        if tokens <= 0.0 {
            continue;
        }
        let model = devin_model(&ev.model);
        match pricing::lookup(&model) {
            Some(p) => {
                let u = pricing::Usage {
                    input: ev.input,
                    output: ev.output,
                    cache_read: ev.cache_read,
                    cache_write_5m: ev.cache_write,
                    cache_write_1h: 0.0,
                };
                add_event(&mut data, ts, &model, pricing::request_cost(&p, &u, true), tokens);
            }
            None => note_unpriced(&mut data, ts, &model, tokens),
        }
    }
    build_spend("devin", "Devin", data)
}

/// Windsurf-style slugs append a reasoning effort ("claude-opus-4-8-medium")
/// that no catalog knows; price and display the base model. Some slugs also
/// spell the model differently than the catalogs: version dots become
/// dashes ("gpt-5-6-sol-max" is GPT-5.6 Sol Max) and Fable's parts are
/// reordered.
fn devin_model(raw: &str) -> String {
    let mut base = raw;
    // Effort tiers and Max/Ultra modes bill at the base model's rates.
    for suffix in ["-xhigh", "-light", "-low", "-medium", "-high", "-max", "-ultra"] {
        if let Some(b) = raw.strip_suffix(suffix) {
            base = b;
            break;
        }
    }
    if base == "claude-5-fable" {
        return "claude-fable-5".into(); // LiteLLM's slug order
    }
    if let Some(rest) = base.strip_prefix("gpt-") {
        let parts: Vec<&str> = rest.splitn(3, '-').collect();
        // Version components are 1–2 digits ("5-6" is 5.6); OpenAI's
        // date-stamped snapshots ("4-0125-preview") use 4-digit segments
        // and must pass through untouched.
        let is_ver = |s: &str| {
            !s.is_empty() && s.len() <= 2 && s.chars().all(|c| c.is_ascii_digit())
        };
        if parts.len() >= 2 && is_ver(parts[0]) && is_ver(parts[1]) {
            let tail = parts.get(2).map(|t| format!("-{t}")).unwrap_or_default();
            return format!("gpt-{}.{}{}", parts[0], parts[1], tail);
        }
    }
    base.to_string()
}

/// One Kimi Code CLI wire.jsonl line → spend event. usage.record rows are
/// self-contained: model, token buckets, epoch-ms time. Only the "turn"
/// scope counts — other scopes would double-report the same tokens.
fn moonshot_line(line: &str, data: &mut FileData) {
    if !line.contains("\"usage.record\"") {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(line) else { return };
    if v.get("type").and_then(Value::as_str) != Some("usage.record") {
        return;
    }
    if v.get("usageScope").and_then(Value::as_str) != Some("turn") {
        return;
    }
    let Some(ts) = parse_ts(v.get("time")) else { return };
    let model_raw = v.get("model").and_then(Value::as_str).unwrap_or("unknown");
    let model = model_raw
        .strip_prefix("moonshot-ai/")
        .unwrap_or(model_raw)
        .to_string();
    let u = v.get("usage").cloned().unwrap_or(Value::Null);
    let num = |k: &str| u.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let (input, output) = (num("inputOther"), num("output"));
    let (cache_read, cache_write) = (num("inputCacheRead"), num("inputCacheCreation"));
    let tokens = input + output + cache_read + cache_write;
    if tokens <= 0.0 {
        return;
    }
    // Catalogs key Kimi models as "moonshot/<slug>"; try the bare slug
    // first (alias/fuzzy chain), then the prefixed spelling.
    let price = pricing::lookup(&model).or_else(|| pricing::lookup(&format!("moonshot/{model}")));
    match price {
        Some(p) => {
            let usage = pricing::Usage {
                input,
                output,
                cache_read,
                cache_write_5m: cache_write,
                cache_write_1h: 0.0,
            };
            add_event(data, ts, &model, pricing::request_cost(&p, &usage, true), tokens);
        }
        None => note_unpriced(data, ts, &model, tokens),
    }
}

/// Moonshot spend: Kimi Code CLI sessions under ~/.kimi-code/sessions —
/// each agent's wire.jsonl logs one usage.record per turn.
fn moonshot() -> ProviderSpend {
    let root = dirs::home_dir()
        .unwrap_or_default()
        .join(".kimi-code")
        .join("sessions");
    let mut files = Vec::new();
    recent_jsonl_files(&root, &mut files);
    let mut all = FileData::default();
    for file in files {
        let data = file_days(&file, &mut |line, data| moonshot_line(line, data));
        merge_data(&mut all, data);
    }
    build_spend("moonshot", "Moonshot", all)
}

/// Cursor spend from the dashboard's usage-events CSV export (fetched by the
/// async caller — this stays a pure parser). Column layout is discovered
/// from the header row; rows with an explicit cost win, token-only rows are
/// priced via the live catalog (the supplement carries Cursor-native models).
pub fn cursor_from_csv(csv: &str) -> ProviderSpend {
    let mut data = FileData::default();
    let mut lines = csv.lines();
    let Some(header) = lines.next() else {
        return build_spend("cursor", "Cursor", data);
    };
    let cols: Vec<String> = split_csv_row(header)
        .into_iter()
        .map(|c| c.trim().to_lowercase())
        .collect();
    let find = |names: &[&str]| {
        cols.iter().position(|c| names.iter().any(|n| c.contains(n)))
    };
    let date_col = find(&["date", "time"]);
    let model_col = find(&["model"]);
    let cost_col = find(&["cost", "amount", "price"]);
    // "Input (w/ Cache Write)" is write-inclusive; the w/o column is the
    // plain input. Their difference gets the cache-write rate.
    let input_wo_col = cols.iter().position(|c| c.contains("input") && c.contains("w/o"));
    let input_with_col = cols
        .iter()
        .position(|c| c.contains("input") && !c.contains("w/o"));
    let output_col = find(&["output"]);
    let cache_read_col = find(&["cache read", "cache_read", "cacheread"]);
    let total_col = find(&["total tokens", "total_tokens"]);
    let (Some(date_col), Some(model_col)) = (date_col, model_col) else {
        return build_spend("cursor", "Cursor", data);
    };

    for line in lines {
        let row = split_csv_row(line);
        let get = |i: Option<usize>| i.and_then(|i| row.get(i)).map(|s| s.trim()).unwrap_or("");
        let Some(ts) = parse_csv_date(get(Some(date_col))) else { continue };
        let model = {
            let m = get(Some(model_col));
            if m.is_empty() { "Unattributed".to_string() } else { m.to_string() }
        };
        let num = |i: Option<usize>| {
            get(i).replace(['$', ','], "").parse::<f64>().unwrap_or(0.0)
        };
        let input_with = num(input_with_col);
        let input_wo = if input_wo_col.is_some() { num(input_wo_col) } else { input_with };
        let cache_write = (input_with - input_wo).max(0.0);
        let output = num(output_col);
        let cache_read = num(cache_read_col);
        let tokens = {
            let t = num(total_col);
            if t > 0.0 { t } else { input_with + output + cache_read }
        };
        let explicit_cost = num(cost_col);

        if explicit_cost > 0.0 {
            add_event(&mut data, ts, &model, explicit_cost, tokens);
        } else if tokens > 0.0 {
            match pricing::lookup(&model) {
                Some(p) => {
                    let u = pricing::Usage {
                        input: input_wo,
                        output,
                        cache_read,
                        cache_write_5m: cache_write,
                        cache_write_1h: 0.0,
                    };
                    // CSV rows aggregate requests, so no single-request
                    // long-context call can be proven — stay on base rates.
                    add_event(&mut data, ts, &model, pricing::request_cost(&p, &u, false), tokens);
                }
                None => note_unpriced(&mut data, ts, &model, tokens),
            }
        }
    }
    build_spend("cursor", "Cursor", data)
}

/// Cursor CSV dates arrive in several shapes depending on export era:
/// RFC3339, "YYYY-MM-DD HH:MM:SS", bare "YYYY-MM-DD", or epoch (s/ms).
fn parse_csv_date(s: &str) -> Option<DateTime<Utc>> {
    if s.is_empty() {
        return None;
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f", "%m/%d/%Y %H:%M:%S", "%b %d, %Y, %I:%M %p", "%b %d, %Y"] {
        if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Some(d.and_utc());
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
            return d.and_hms_opt(12, 0, 0).map(|dt| dt.and_utc());
        }
    }
    if let Ok(n) = s.parse::<i64>() {
        return DateTime::from_timestamp_millis(if n > 1_000_000_000_000 { n } else { n * 1000 });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tokens_sum(d: &FileData) -> f64 {
        d.days.values().map(|v| v.1).sum()
    }

    fn cost_sum(d: &FileData) -> f64 {
        d.days.values().map(|v| v.0).sum()
    }

    // ---- Persistent cache: serialization roundtrip -----------------------

    #[test]
    fn persist_file_roundtrips_losslessly() {
        let doc = PersistFile {
            version: PERSIST_VERSION,
            pricing_stamp: "litellm:1:2|modelsdev:3:4|supplement:5:6".into(),
            entries: vec![PersistEntry {
                path: PathBuf::from(r"C:\logs\session.jsonl"),
                mtime_secs: 1_784_600_000,
                mtime_nanos: 123_456_700, // NTFS 100ns precision must survive
                size: 4096,
                days: vec![(739_000, "claude-fable-5".into(), 1.25, 40_000.0)],
                unpriced: vec![("mystery-model".into(), 3)],
            }],
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back: PersistFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, doc.version);
        assert_eq!(back.pricing_stamp, doc.pricing_stamp);
        let (a, b) = (&back.entries[0], &doc.entries[0]);
        assert_eq!(a.path, b.path);
        assert_eq!((a.mtime_secs, a.mtime_nanos, a.size), (b.mtime_secs, b.mtime_nanos, b.size));
        assert_eq!(a.days, b.days);
        assert_eq!(a.unpriced, b.unpriced);
    }

    #[test]
    fn corrupt_persist_file_is_rejected_not_panicked() {
        assert!(serde_json::from_str::<PersistFile>("{not json").is_err());
        assert!(serde_json::from_str::<PersistFile>(r#"{"version":1}"#).is_err());
    }

    // ---- Hermes: billing-route buckets -----------------------------------

    #[test]
    fn hermes_routes_land_in_the_right_slice() {
        assert_eq!(hermes_bucket("minimax-oauth").0, "minimax");
        assert_eq!(hermes_bucket("MiniMax").0, "minimax");
        assert_eq!(hermes_bucket("openrouter").0, "openrouter");
        assert_eq!(hermes_bucket("nous-api").0, "hermes");
        assert_eq!(hermes_bucket("").0, "hermes");
    }

    // ---- Codex: child-session replay gate --------------------------------

    #[test]
    fn codex_child_meta_rules() {
        // JSON null / blank strings are absent — a root session declaring
        // `forked_from_id: null` is not a child.
        assert!(!codex_child_meta(&json!({"forked_from_id": null, "parent_thread_id": null})));
        assert!(!codex_child_meta(&json!({"forked_from_id": "  "})));
        assert!(!codex_child_meta(&json!({"session_id": "root"})));
        assert!(codex_child_meta(&json!({"forked_from_id": "abc"})));
        assert!(codex_child_meta(&json!({"parent_thread_id": "abc"})));
        assert!(codex_child_meta(&json!({"thread_source": "subagent"})));
        assert!(codex_child_meta(&json!({"source": {"subagent": {"thread_spawn": {}}}})));
        assert!(!codex_child_meta(&json!({"source": {"subagent": null}})));
    }

    fn codex_run(lines: &[String]) -> FileData {
        let mut st = CodexFileState::default();
        let mut data = FileData::default();
        for line in lines {
            codex_line(&mut st, line, &mut data);
        }
        data
    }

    fn token_count_line(ts: &str, last: Option<(f64, f64)>, total: (f64, f64)) -> String {
        let mut info = json!({
            "total_token_usage": {"input_tokens": total.0, "output_tokens": total.1,
                                  "total_tokens": total.0 + total.1}
        });
        if let Some((i, o)) = last {
            info["last_token_usage"] =
                json!({"input_tokens": i, "output_tokens": o, "total_tokens": i + o});
        }
        json!({"timestamp": ts, "type": "event_msg",
               "payload": {"type": "token_count", "info": info}})
        .to_string()
    }

    #[test]
    fn codex_replay_gate_skips_child_history() {
        let spawn_epoch = chrono::DateTime::parse_from_rfc3339("2026-07-10T10:00:00Z")
            .unwrap()
            .timestamp();
        let lines = vec![
            // The child's own session_meta, then the replayed parent history:
            // token_counts with rewritten (fresh) timestamps and a replayed
            // task_started still carrying the parent's old started_at.
            json!({"timestamp": "2026-07-10T10:00:00Z", "type": "session_meta",
                   "payload": {"parent_thread_id": "abc", "thread_source": "subagent"}})
            .to_string(),
            json!({"timestamp": "2026-07-10T10:00:00Z", "type": "turn_context",
                   "payload": {"model": "gpt-5.6-terra"}})
            .to_string(),
            token_count_line("2026-07-10T10:00:01Z", Some((50_000.0, 5_000.0)), (50_000.0, 5_000.0)),
            json!({"timestamp": "2026-07-10T10:00:02Z", "type": "event_msg",
                   "payload": {"type": "task_started", "started_at": spawn_epoch - 3600}})
            .to_string(),
            token_count_line("2026-07-10T10:00:03Z", Some((30_000.0, 3_000.0)), (80_000.0, 8_000.0)),
            // First live turn: started_at at/after the child's creation.
            json!({"timestamp": "2026-07-10T10:00:05Z", "type": "event_msg",
                   "payload": {"type": "task_started", "started_at": spawn_epoch + 5}})
            .to_string(),
            token_count_line("2026-07-10T10:00:09Z", Some((1_000.0, 100.0)), (81_000.0, 8_100.0)),
        ];
        let data = codex_run(&lines);
        // Only the live turn counts — 88k replayed tokens stay out.
        assert_eq!(tokens_sum(&data), 1_100.0);
        assert!(data.unpriced.is_empty());
    }

    #[test]
    fn codex_root_session_with_null_parent_counts_normally() {
        let lines = vec![
            json!({"timestamp": "2026-07-10T10:00:00Z", "type": "session_meta",
                   "payload": {"forked_from_id": null, "parent_thread_id": null}})
            .to_string(),
            json!({"timestamp": "2026-07-10T10:00:00Z", "type": "turn_context",
                   "payload": {"model": "gpt-5.6-terra"}})
            .to_string(),
            token_count_line("2026-07-10T10:00:01Z", Some((1_000.0, 100.0)), (1_000.0, 100.0)),
        ];
        assert_eq!(tokens_sum(&codex_run(&lines)), 1_100.0);
    }

    #[test]
    fn codex_stale_snapshot_reemission_skipped() {
        let lines = vec![
            json!({"timestamp": "2026-07-10T10:00:00Z", "type": "turn_context",
                   "payload": {"model": "gpt-5.6-terra"}})
            .to_string(),
            token_count_line("2026-07-10T10:00:01Z", Some((1_000.0, 100.0)), (1_000.0, 100.0)),
            // Same cumulative totals re-emitted (Codex does this) — not new
            // usage even though it repeats a last_token_usage.
            token_count_line("2026-07-10T10:00:02Z", Some((1_000.0, 100.0)), (1_000.0, 100.0)),
        ];
        assert_eq!(tokens_sum(&codex_run(&lines)), 1_100.0);
    }

    #[test]
    fn codex_totals_delta_when_last_usage_absent() {
        let lines = vec![
            json!({"timestamp": "2026-07-10T10:00:00Z", "type": "turn_context",
                   "payload": {"model": "gpt-5.6-terra"}})
            .to_string(),
            token_count_line("2026-07-10T10:00:01Z", None, (1_000.0, 100.0)),
            token_count_line("2026-07-10T10:00:02Z", None, (3_000.0, 300.0)),
        ];
        // 1100 from the first cumulative snapshot, 2200 recovered as a delta.
        assert_eq!(tokens_sum(&codex_run(&lines)), 3_300.0);
    }

    #[test]
    fn codex_fast_tier_applies_provider_multiplier() {
        let turn = json!({"timestamp": "2026-07-10T10:00:00Z", "type": "turn_context",
                          "payload": {"model": "gpt-5.6-terra"}})
        .to_string();
        let usage = token_count_line("2026-07-10T10:00:01Z", Some((1_000.0, 100.0)), (1_000.0, 100.0));
        let standard = codex_run(&[turn.clone(), usage.clone()]);
        let fast = codex_run(&[
            turn,
            json!({"timestamp": "2026-07-10T10:00:00Z", "type": "event_msg",
                   "payload": {"type": "thread_settings_applied",
                               "thread_settings": {"service_tier": "fast"}}})
            .to_string(),
            usage,
        ]);
        // gpt-5.6-terra's Codex priority multiplier is exactly 2x, whatever
        // catalog resolved the base rates.
        assert!(cost_sum(&standard) > 0.0);
        assert!((cost_sum(&fast) / cost_sum(&standard) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn codex_dated_base_strips_snapshot_stamps() {
        assert_eq!(codex_dated_base("gpt-5.6-sol-2026-06-01"), "gpt-5.6-sol");
        assert_eq!(codex_dated_base("gpt-5.6-sol-20260601"), "gpt-5.6-sol");
        assert_eq!(codex_dated_base("gpt-5.6-sol"), "gpt-5.6-sol");
        assert_eq!(codex_dated_base("gpt-4-0125-preview"), "gpt-4-0125-preview");
    }

    // ---- Claude: advisor iterations, sidechain dedup, synthetic ----------

    fn claude_run(lines: &[String]) -> FileData {
        let mut st = ClaudeFileState::default();
        let mut data = FileData::default();
        for line in lines {
            claude_line(&mut st, line, &mut data);
        }
        data
    }

    #[test]
    fn claude_advisor_iterations_expand_once() {
        // Two ordinary message iterations (already inside the parent totals)
        // and one advisor_message that must become its own entry.
        let line = json!({"type": "assistant", "timestamp": "2026-07-10T10:00:00Z",
            "requestId": "req_1",
            "message": {"id": "msg_1", "model": "claude-fable-5-20260115",
                "usage": {"input_tokens": 2.0, "output_tokens": 491.0,
                    "cache_read_input_tokens": 1000.0,
                    "iterations": [
                        {"type": "message", "input_tokens": 1.0, "output_tokens": 200.0},
                        {"type": "advisor_message", "model": "claude-haiku-4-5",
                         "input_tokens": 10.0, "output_tokens": 2.0,
                         "cache_read_input_tokens": 4.0},
                        {"type": "message", "input_tokens": 1.0, "output_tokens": 291.0}
                    ]}}})
        .to_string();
        let once = claude_run(&[line.clone()]);
        let models: HashSet<&str> = once.days.keys().map(|(_, m)| m.as_str()).collect();
        assert!(models.iter().any(|m| m.contains("fable")));
        assert!(models.iter().any(|m| m.contains("haiku")));
        // Parent 1493 + advisor 16; the plain message iterations add nothing.
        assert_eq!(tokens_sum(&once), 1_509.0);
        // A replayed copy of the same line (same message + request id) is
        // dropped, advisors included.
        let twice = claude_run(&[line.clone(), line]);
        assert_eq!(tokens_sum(&twice), 1_509.0);
    }

    #[test]
    fn claude_sidechain_replay_is_deduped() {
        let parent = json!({"type": "assistant", "timestamp": "2026-07-10T10:00:00Z",
            "requestId": "req_1",
            "message": {"id": "msg_1", "model": "claude-haiku-4-5",
                        "usage": {"input_tokens": 100.0, "output_tokens": 10.0}}})
        .to_string();
        // Sidechain log replays the same message under a fresh request id.
        let replay = json!({"type": "assistant", "timestamp": "2026-07-10T10:00:01Z",
            "requestId": "req_2", "isSidechain": true,
            "message": {"id": "msg_1", "model": "claude-haiku-4-5",
                        "usage": {"input_tokens": 100.0, "output_tokens": 10.0}}})
        .to_string();
        assert_eq!(tokens_sum(&claude_run(&[parent.clone(), replay.clone()])), 110.0);
        // Reverse arrival order still counts the message exactly once.
        assert_eq!(tokens_sum(&claude_run(&[replay, parent.clone()])), 110.0);
        // A genuine retry (no sidechain involved) keeps both.
        let retry = json!({"type": "assistant", "timestamp": "2026-07-10T10:00:02Z",
            "requestId": "req_3",
            "message": {"id": "msg_1", "model": "claude-haiku-4-5",
                        "usage": {"input_tokens": 100.0, "output_tokens": 10.0}}})
        .to_string();
        assert_eq!(tokens_sum(&claude_run(&[parent, retry])), 220.0);
    }

    #[test]
    fn claude_synthetic_model_never_priced() {
        let bare = json!({"type": "assistant", "timestamp": "2026-07-10T10:00:00Z",
            "requestId": "req_1",
            "message": {"id": "msg_1", "model": "<synthetic>",
                        "usage": {"input_tokens": 5.0, "output_tokens": 5.0}}})
        .to_string();
        let data = claude_run(&[bare]);
        assert!(data.days.is_empty());
        assert!(data.unpriced.is_empty()); // a placeholder, not an unknown model

        let carried = json!({"type": "assistant", "timestamp": "2026-07-10T10:00:00Z",
            "requestId": "req_2", "costUSD": 0.5,
            "message": {"id": "msg_2", "model": "<synthetic>",
                        "usage": {"input_tokens": 5.0, "output_tokens": 5.0}}})
        .to_string();
        let data = claude_run(&[carried]);
        assert_eq!(cost_sum(&data), 0.5);
        assert!(data.days.keys().all(|(_, m)| m == "unattributed"));
    }

    #[test]
    fn devin_model_normalizes_fable_and_modes() {
        assert_eq!(devin_model("claude-5-fable-medium"), "claude-fable-5");
        assert_eq!(devin_model("claude-5-fable-max"), "claude-fable-5");
        assert_eq!(devin_model("claude-5-fable-high"), "claude-fable-5");
        assert_eq!(devin_model("gpt-5-6-sol-max"), "gpt-5.6-sol");
        assert_eq!(devin_model("claude-opus-4-8-medium"), "claude-opus-4-8");
        assert_eq!(devin_model("gpt-4-0125-preview"), "gpt-4-0125-preview");
    }

    #[test]
    fn moonshot_counts_turn_records_only() {
        let mut data = FileData::default();
        let turn = json!({"type": "usage.record", "model": "moonshot-ai/kimi-test-model",
            "usage": {"inputOther": 400.0, "output": 200.0, "inputCacheRead": 300.0,
                      "inputCacheCreation": 100.0},
            "usageScope": "turn", "time": 1784208630652i64})
        .to_string();
        let session_scope = turn.replace("\"turn\"", "\"session\"");
        moonshot_line(&turn, &mut data);
        moonshot_line(&session_scope, &mut data);
        // One event; unknown model → tokens counted, dollars honest zero.
        assert_eq!(data.days.values().map(|v| v.1).sum::<f64>(), 1_000.0);
        assert_eq!(data.unpriced.get("kimi-test-model"), Some(&1));
        assert!(data.days.keys().all(|(_, m)| m == "kimi-test-model"));
    }

    #[test]
    fn split_models_reroutes_minimax_usage() {
        let mut data = FileData::default();
        data.days.insert((1000, "claude-fable-5".into()), (5.0, 100.0));
        data.days.insert((1000, "MiniMax-M3".into()), (0.5, 50.0));
        data.days.insert((1001, "MiniMax-M2.7".into()), (0.2, 20.0));
        data.unpriced.insert("MiniMax-Unknown".into(), 3);
        data.unpriced.insert("mystery-model".into(), 1);

        let mm = split_models(&mut data, "MiniMax");
        assert_eq!(data.days.len(), 1);
        assert_eq!(data.unpriced.len(), 1);
        assert_eq!(mm.days.len(), 2);
        assert_eq!(mm.days[&(1000, "MiniMax-M3".to_string())], (0.5, 50.0));
        assert_eq!(mm.unpriced.get("MiniMax-Unknown"), Some(&3));
    }

    #[test]
    fn claude_unknown_speed_marks_foreign_log_shape() {
        let line = json!({"type": "assistant", "timestamp": "2026-07-10T10:00:00Z",
            "requestId": "req_1",
            "message": {"id": "msg_1", "model": "claude-haiku-4-5",
                        "usage": {"input_tokens": 100.0, "output_tokens": 10.0,
                                  "speed": "turbo"}}})
        .to_string();
        assert!(claude_run(&[line]).days.is_empty());
    }

    /// Live probe over this machine's real logs + Cursor export. Prints
    /// aggregates and the CSV header only. Run via
    /// `cargo test --lib spend -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_probe() {
        let csv = tauri::async_runtime::block_on(crate::providers::cursor::fetch_usage_csv());
        eprintln!("cursor csv: {} bytes", csv.as_deref().map(str::len).unwrap_or(0));
        if let Some(c) = &csv {
            eprintln!("csv header: {}", c.lines().next().unwrap_or(""));
            for row in c.lines().skip(1).take(3) {
                let cells = super::split_csv_row(row);
                eprintln!(
                    "row: date={:?} parsed={} model={:?} in={:?} out={:?} total={:?} cost={:?}",
                    cells.first(),
                    cells.first().map(|d| super::parse_csv_date(d).is_some()).unwrap_or(false),
                    cells.get(4),
                    cells.get(6),
                    cells.get(9),
                    cells.get(10),
                    cells.get(11),
                );
            }
        }
        for sp in super::collect(csv) {
            eprintln!(
                "{}: today=${:.2} 30d=${:.2} tokens30={:.0} unpriced={} {:?}",
                sp.id, sp.today.cost, sp.last30.cost, sp.last30.tokens, sp.unpriced, sp.unpriced_models
            );
        }
    }
}

/// Minimal CSV field splitter with quoted-field support.
fn split_csv_row(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => out.push(std::mem::take(&mut field)),
            _ => field.push(c),
        }
    }
    out.push(field);
    out
}

pub fn collect(cursor_csv: Option<String>) -> Vec<ProviderSpend> {
    pricing::ensure_fresh();
    load_persisted_cache();
    if let Ok(mut t) = touched().lock() {
        t.clear();
    }
    let (claude_sp, mut minimax_extra) = claude();
    // Hermes rows going to an existing slice merge into it (MiniMax via the
    // extra-data path); the rest become their own spend entries.
    let mut hermes_rest = Vec::new();
    for (id, name, data) in hermes() {
        if id == "minimax" {
            merge_data(&mut minimax_extra, data);
        } else {
            hermes_rest.push(build_spend(id, name, data));
        }
    }
    let mut list = vec![
        claude_sp,
        codex(),
        grok(),
        opencode(),
        devin(),
        minimax(minimax_extra),
        moonshot(),
    ];
    list.extend(hermes_rest);
    if let Some(csv) = cursor_csv {
        list.push(cursor_from_csv(&csv));
    }
    // Models nothing prices yet (new slugs ship often): flag the catalog
    // to look for updates hourly instead of daily.
    if list.iter().any(|sp| sp.unpriced > 0) {
        pricing::note_unpriced();
    }
    save_persisted_cache();
    list.into_iter().filter(ProviderSpend::has_data).collect()
}
