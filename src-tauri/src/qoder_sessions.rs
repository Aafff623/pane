//! Qoder CN desktop transcripts → measured credits + derived billed input
//! tokens (GitHub #28, phase 1).
//!
//! Qoder CN writes every round-trip into
//! `<QODER_HOME>/projects/<slug>/<sessionId>.jsonl` with
//! `message.usage.{credits, original_credits, context_usage_ratio,
//! request_id}`. Its input/output token counters are always zero, so billed
//! input tokens are derived as `Σ(context_usage_ratio × contextWindow)` using
//! the window in force **for that request** (a session can switch models).
//! When the window is unknown, credits stay exact and tokens are reported as
//! unavailable — never silently baked with a 200K fallback.
//!
//! Subagent transcripts (`<sessionId>/subagents/agent-*.jsonl`) carry usage
//! but no runtime config; they inherit the parent session's window and are
//! folded into the parent session. Requests are deduplicated by
//! `request_id` across files. Credits stay out of Total Spend and the dollar
//! donut; the derived tokens join the token views as labelled estimates.

use chrono::{DateTime, Datelike, Local, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const SCAN_DAYS: i64 = 31;
const MAX_SESSIONS: usize = 50;
/// Bump when the parser's interpretation changes so an in-process cache from
/// an older build cannot keep serving events with missing windows.
const CACHE_VERSION: u32 = 3;

#[derive(Serialize, Clone, Default)]
pub struct QoderWindow {
    /// Nominal credits written to the transcript. Kept for reference even
    /// when a free/zero-rate request never charges them.
    pub credits: f64,
    /// Credits actually charged (`billable != false`). A free model writes
    /// nominal credits but leaves the platform quota untouched.
    pub charged_credits: f64,
    pub original_credits: f64,
    pub free_trips: u64,
    /// Sum of derived billed input tokens for the requests whose window is
    /// known. Trips without a window are counted separately.
    pub billed_tokens: f64,
    pub tokens_unavailable_trips: u64,
    pub trips: u64,
    pub sessions: u64,
    /// Per-model totals for this window. Tokens are the same derived
    /// estimate as `billed_tokens`; credits stay exact.
    pub models: Vec<QoderModelSpend>,
}

#[derive(Serialize, Clone)]
pub struct QoderModelSpend {
    pub model: String,
    pub label: String,
    pub credits: f64,
    pub charged_credits: f64,
    pub free_trips: u64,
    pub billed_tokens: f64,
    pub trips: u64,
    pub tokens_unavailable_trips: u64,
    /// Where the context window behind this model's token estimate came from
    /// (#37-03): "db" (a real context_window), "same_model" (carried over
    /// from another request of the same model) or "unknown". Explanation
    /// only — the token math itself never looks at this field.
    pub window_source: String,
}

#[derive(Serialize, Clone)]
pub struct QoderDaySpend {
    pub day: String,
    pub credits: f64,
    pub charged_credits: f64,
    pub original_credits: f64,
    pub free_trips: u64,
    pub billed_tokens: f64,
    pub tokens_unavailable_trips: u64,
    pub trips: u64,
    pub sessions: u64,
    pub peak_ratio: f64,
    pub models: Vec<QoderModelSpend>,
}

#[derive(Serialize, Clone)]
pub struct QoderSessionSpend {
    pub id: String,
    /// RFC 3339 UTC of the first request.
    pub started: String,
    /// Local calendar day of the first request.
    pub day: String,
    pub credits: f64,
    pub charged_credits: f64,
    pub free_trips: u64,
    pub billed_tokens: f64,
    pub tokens_unavailable_trips: u64,
    pub trips: u64,
    pub peak_ratio: f64,
    /// Credits spent by the session's subagents (already included in
    /// `credits`; this is a provenance split, not an extra).
    pub subagent_credits: f64,
    pub models: Vec<QoderModelSpend>,
}

#[derive(Serialize, Clone, Default)]
pub struct QoderSource {
    pub state: String,
    pub root: String,
    pub sessions: u64,
    pub lite_sessions: u64,
    pub tokens_unavailable_trips: u64,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct QoderRescan {
    pub spend: QoderSpend,
    pub stats_available: bool,
    pub archived_total: u64,
    pub archived_readable: u64,
    pub deleted_total: u64,
    pub deleted_readable: u64,
}

#[derive(Serialize, Clone, Default)]
pub struct QoderSpend {
    pub today: QoderWindow,
    pub yesterday: QoderWindow,
    pub d7: QoderWindow,
    pub d30: QoderWindow,
    pub all: QoderWindow,
    /// Local days, oldest first; today is the last entry when it has usage.
    pub days: Vec<QoderDaySpend>,
    /// Newest first, capped for the UI.
    pub sessions: Vec<QoderSessionSpend>,
    pub source: QoderSource,
    /// What the local data can support right now. Qoder 0.4.11 ships no exact
    /// token counts anywhere on the machine, and an app update can null the
    /// context windows that the estimate multiplies by — so a token figure is
    /// only meaningful next to this. Without it a silent basis change looks
    /// like a change in usage.
    #[serde(default)]
    pub telemetry: QoderTelemetry,
}

/// Read-only capability probe of the local Qoder CN telemetry. Never guesses:
/// when the basis is gone the token column is reported as unmeasurable rather
/// than padded with a default window (see the module docs).
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct QoderTelemetry {
    pub db_found: bool,
    pub db_path: String,
    pub schema_version: Option<i64>,
    pub models_total: u64,
    pub models_with_window: u64,
    /// True only when Qoder itself stores measured token counts. Today the
    /// official context-usage snapshot answers `tokenCountsAvailable: false`.
    pub exact_token_counts: bool,
    /// "database" | "none": which source currently supplies the window the
    /// estimate multiplies by.
    pub estimate_basis: String,
}

#[derive(Clone)]
struct Event {
    ts: DateTime<Utc>,
    model: String,
    credits: f64,
    original_credits: f64,
    ratio: Option<f64>,
    window: Option<f64>,
    /// Provenance of `window` (#37-03) — explanation only, never an input
    /// to the token math.
    window_source: WindowSource,
    request_id: String,
    /// `usage.billable`; `Some(false)` means the request was free and its
    /// nominal credits never hit the platform quota.
    billable: Option<bool>,
}

/// Where the window a request's token estimate multiplied by came from.
/// Serialized per model as `QoderModelSpend::window_source`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum WindowSource {
    /// A real `context_window`: the transcript's own `runtime-config` line
    /// for that round, or Qoder's model-preference database.
    Db,
    /// The round published none, but another request of the same model did
    /// (a null config in the same file, or a subagent's parent session).
    SameModel,
    /// Nothing anywhere for this model — tokens stay unavailable.
    #[default]
    Unknown,
}

impl WindowSource {
    fn as_str(self) -> &'static str {
        match self {
            WindowSource::Db => "db",
            WindowSource::SameModel => "same_model",
            WindowSource::Unknown => "unknown",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "db" => WindowSource::Db,
            "same_model" => WindowSource::SameModel,
            _ => WindowSource::Unknown,
        }
    }

    /// The strongest basis any request in the aggregate got: a real window
    /// outranks a carried-over one, which outranks none. Trips that could
    /// not derive tokens stay visible via `tokens_unavailable_trips`.
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (WindowSource::Db, _) | (_, WindowSource::Db) => WindowSource::Db,
            (WindowSource::SameModel, _) | (_, WindowSource::SameModel) => WindowSource::SameModel,
            _ => WindowSource::Unknown,
        }
    }
}

#[derive(Clone, Default)]
struct ParsedFile {
    events: Vec<Event>,
    /// Last known window per model, for subagent transcripts that carry none.
    windows: HashMap<String, f64>,
}

/// Cache entries keyed by (mtime, size): a session log is append-only, so an
/// unchanged file cannot contribute new requests.
type CacheKey = (u32, SystemTime, u64);
fn cache() -> &'static Mutex<HashMap<PathBuf, (CacheKey, ParsedFile)>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (CacheKey, ParsedFile)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn clear_cache() {
    if let Ok(mut cache) = cache().lock() {
        cache.clear();
    }
}

fn qoder_home() -> PathBuf {
    std::env::var("QODER_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".qoder-cn"))
}

fn projects_root() -> PathBuf {
    qoder_home().join("projects")
}

fn stamp(path: &Path) -> Option<CacheKey> {
    let meta = fs::metadata(path).ok()?;
    Some((CACHE_VERSION, meta.modified().ok()?, meta.len()))
}

fn cached_parse(path: &Path) -> ParsedFile {
    let Some(key) = stamp(path) else {
        return ParsedFile::default();
    };
    if let Ok(cache) = cache().lock() {
        if let Some((cached, parsed)) = cache.get(path) {
            if *cached == key {
                return parsed.clone();
            }
        }
    }
    let parsed = parse_file(path);
    if let Ok(mut cache) = cache().lock() {
        cache.insert(path.to_path_buf(), (key, parsed.clone()));
    }
    parsed
}

fn parse_file(path: &Path) -> ParsedFile {
    let Ok(file) = fs::File::open(path) else {
        return ParsedFile::default();
    };
    parse_reader(BufReader::new(file))
}

fn parse_reader(reader: impl BufRead) -> ParsedFile {
    let mut out = ParsedFile::default();
    let mut current: Option<(String, Option<f64>, WindowSource)> = None;
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        // Session logs are mostly conversation bodies. Two cheap substring
        // checks keep the JSON parser off every other line.
        if !line.contains("\"usage\"") && !line.contains("runtime-config") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) == Some("runtime-config") {
            let Some(model) = value.get("model").and_then(Value::as_str) else {
                continue;
            };
            let window = value
                .get("contextWindow")
                .and_then(Value::as_f64)
                .filter(|w| *w > 0.0);
            if let Some(w) = window {
                out.windows.insert(model.to_string(), w);
            }
            // A null config means "this round carried no window", not
            // "forget the window this model already had". Keep the model's
            // last non-null window; never borrow another model's window.
            let (effective, source) = match window {
                Some(w) => (Some(w), WindowSource::Db),
                None => match out.windows.get(model).copied() {
                    Some(w) => (Some(w), WindowSource::SameModel),
                    None => (None, WindowSource::Unknown),
                },
            };
            current = Some((model.to_string(), effective, source));
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let Some(usage) = message.get("usage").filter(|u| u.is_object()) else {
            continue;
        };
        let credits = usage.get("credits").and_then(Value::as_f64);
        let original = usage.get("original_credits").and_then(Value::as_f64);
        if credits.is_none() && original.is_none() {
            continue;
        }
        let model = message
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| current.as_ref().map(|(m, _, _)| m.clone()))
            .unwrap_or_default();
        if model.is_empty() {
            continue;
        }
        let (window, window_source) = match current.as_ref() {
            Some((current_model, window, source)) if current_model == &model => (*window, *source),
            _ => match out.windows.get(&model).copied() {
                Some(w) => (Some(w), WindowSource::SameModel),
                None => (None, WindowSource::Unknown),
            },
        };
        let request_id = usage
            .get("request_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| message.get("id").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_default();
        let Some(ts) = parse_ts(value.get("timestamp")) else {
            continue;
        };
        out.events.push(Event {
            ts,
            model,
            credits: credits.unwrap_or(0.0),
            original_credits: original.unwrap_or_else(|| credits.unwrap_or(0.0)),
            ratio: usage.get("context_usage_ratio").and_then(Value::as_f64),
            window,
            window_source,
            request_id,
            billable: usage.get("billable").and_then(Value::as_bool),
        });
    }
    out
}

fn parse_ts(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let value = value?;
    if let Some(raw) = value.as_str() {
        return DateTime::parse_from_rfc3339(raw).ok().map(|t| t.with_timezone(&Utc));
    }
    if let Some(n) = value.as_i64() {
        // Heuristic shared with spend.rs: values past ~2001-09 in ms are
        // millisecond stamps.
        let ms = if n > 1_000_000_000_000 { n } else { n * 1000 };
        return DateTime::from_timestamp_millis(ms);
    }
    None
}

/// The window a subagent request ran under: the parent's window for the same
/// model when it named one, else the parent's most recent window.
fn inherited_window(event: &Event, parent: &ParsedFile) -> Option<f64> {
    // Same-model only. A subagent for a model the parent never named must
    // fall through to the Qoder model preference (or unavailable), never
    // borrow another model's window.
    parent.windows.get(&event.model).copied()
}

fn day_key(ts: DateTime<Utc>) -> (String, i32) {
    let local = ts.with_timezone(&Local).date_naive();
    (local.format("%Y-%m-%d").to_string(), local.num_days_from_ce())
}

/// Discover main transcripts and their subagent files under
/// `<projects>/<slug>/`. Returns (session id, path, is_subagent) trips in
/// the order they should be parsed (parents first, so window inheritance
/// never depends on file order).
fn discover(root: &Path) -> (Vec<(String, PathBuf, bool)>, u64) {
    let mut out = Vec::new();
    let mut lite = 0u64;
    let cutoff = SystemTime::now() - std::time::Duration::from_secs(SCAN_DAYS as u64 * 86_400);
    let Ok(slugs) = fs::read_dir(root) else {
        return (out, lite);
    };
    for slug in slugs.flatten() {
        let slug_path = slug.path();
        if !slug_path.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&slug_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(ftype) = entry.file_type() else {
                continue;
            };
            if ftype.is_file() {
                if path.extension().map_or(true, |e| e != "jsonl") {
                    continue;
                }
                if fs::metadata(&path).ok().and_then(|m| m.modified().ok()).map_or(true, |t| t < cutoff) {
                    continue;
                }
                let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                out.push((id.to_string(), path, false));
            } else if ftype.is_dir() {
                let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                if id == "transcript" {
                    // IDE-lite layout: same session ids, no usage records.
                    if let Ok(files) = fs::read_dir(&path) {
                        for file in files.flatten() {
                            if file.path().extension().is_some_and(|e| e == "jsonl") {
                                lite += 1;
                            }
                        }
                    }
                    continue;
                }
                let subs = path.join("subagents");
                let Ok(files) = fs::read_dir(&subs) else {
                    continue;
                };
                for file in files.flatten() {
                    let sub = file.path();
                    if sub.extension().map_or(true, |e| e != "jsonl") {
                        continue;
                    }
                    if fs::metadata(&sub).ok().and_then(|m| m.modified().ok()).map_or(true, |t| t < cutoff) {
                        continue;
                    }
                    out.push((id.to_string(), sub, true));
                }
            }
        }
    }
    (out, lite)
}

#[derive(Default)]
struct ModelAgg {
    credits: f64,
    charged: f64,
    free: u64,
    tokens: f64,
    trips: u64,
    unavailable: u64,
    /// Strongest window basis seen for this model inside the aggregate.
    window_source: WindowSource,
}

impl ModelAgg {
    fn add_spend(&mut self, model: &QoderModelSpend) {
        self.credits += model.credits;
        self.charged += model.charged_credits;
        self.free += model.free_trips;
        self.tokens += model.billed_tokens;
        self.trips += model.trips;
        self.unavailable += model.tokens_unavailable_trips;
        self.window_source = self.window_source.merge(WindowSource::parse(&model.window_source));
    }
}

#[derive(Default)]
struct SessionAgg {
    started: Option<DateTime<Utc>>,
    credits: f64,
    charged: f64,
    free: u64,
    tokens: f64,
    unavailable: u64,
    trips: u64,
    peak_ratio: f64,
    subagent_credits: f64,
    models: HashMap<String, ModelAgg>,
}

#[derive(Default)]
struct DayAgg {
    day_idx: i32,
    credits: f64,
    charged: f64,
    free: u64,
    original_credits: f64,
    tokens: f64,
    unavailable: u64,
    trips: u64,
    peak_ratio: f64,
    sessions: HashSet<String>,
    models: HashMap<String, ModelAgg>,
}

fn add_model(agg: &mut HashMap<String, ModelAgg>, event: &Event, tokens: Option<f64>) {
    let entry = agg.entry(event.model.clone()).or_default();
    entry.credits += event.credits;
    if event.billable == Some(false) {
        entry.free += 1;
    } else {
        entry.charged += event.credits;
    }
    entry.trips += 1;
    entry.window_source = entry.window_source.merge(event.window_source);
    match tokens {
        Some(t) => entry.tokens += t,
        None => entry.unavailable += 1,
    }
}

fn derive_tokens(ratio: Option<f64>, window: Option<f64>) -> Option<f64> {
    match (ratio, window) {
        (Some(ratio), Some(window)) if window > 0.0 && ratio >= 0.0 => Some(ratio * window),
        _ => None,
    }
}

fn build_report(root: &Path, entries: Vec<(String, Event, bool)>, lite_sessions: u64) -> QoderSpend {
    let mut seen: HashSet<String> = HashSet::new();
    let mut sessions: HashMap<String, SessionAgg> = HashMap::new();
    let mut days: HashMap<i32, DayAgg> = HashMap::new();
    let mut labels: HashMap<String, String> = HashMap::new();

    for (session_id, event, is_subagent) in entries {
        let dedup = if event.request_id.is_empty() {
            format!("{session_id}|{}|{}", event.ts.timestamp_millis(), event.model)
        } else {
            event.request_id.clone()
        };
        if !seen.insert(dedup) {
            continue;
        }
        let tokens = derive_tokens(event.ratio, event.window);
        let free = event.billable == Some(false);
        let charged = if free { 0.0 } else { event.credits };
        let (_, day_idx) = day_key(event.ts);
        let session = sessions.entry(session_id.clone()).or_default();
        if session.started.map_or(true, |s| event.ts < s) {
            session.started = Some(event.ts);
        }
        session.credits += event.credits;
        session.charged += charged;
        if free {
            session.free += 1;
        }
        session.trips += 1;
        if let Some(t) = tokens {
            session.tokens += t;
        } else {
            session.unavailable += 1;
        }
        if is_subagent {
            session.subagent_credits += event.credits;
        }
        if let Some(ratio) = event.ratio {
            session.peak_ratio = session.peak_ratio.max(ratio);
        }
        add_model(&mut session.models, &event, tokens);

        let day_agg = days.entry(day_idx).or_default();
        day_agg.day_idx = day_idx;
        day_agg.credits += event.credits;
        day_agg.charged += charged;
        if free {
            day_agg.free += 1;
        }
        day_agg.original_credits += event.original_credits;
        day_agg.trips += 1;
        day_agg.sessions.insert(session_id.clone());
        if let Some(t) = tokens {
            day_agg.tokens += t;
        } else {
            day_agg.unavailable += 1;
        }
        if let Some(ratio) = event.ratio {
            day_agg.peak_ratio = day_agg.peak_ratio.max(ratio);
        }
        add_model(&mut day_agg.models, &event, tokens);
        labels.entry(event.model.clone()).or_insert_with(|| model_label(&event.model));
    }

    let label_of = |model: &str| labels.get(model).cloned().unwrap_or_else(|| model.to_string());
    let model_list = |models: HashMap<String, ModelAgg>| {
        let mut list: Vec<QoderModelSpend> = models
            .into_iter()
            .map(|(model, a)| QoderModelSpend {
                label: label_of(&model),
                model,
                credits: a.credits,
                charged_credits: a.charged,
                free_trips: a.free,
                billed_tokens: a.tokens,
                trips: a.trips,
                tokens_unavailable_trips: a.unavailable,
                window_source: a.window_source.as_str().to_string(),
            })
            .collect();
        list.sort_by(|a, b| b.credits.partial_cmp(&a.credits).unwrap_or(std::cmp::Ordering::Equal));
        list
    };

    // Session ids per day, kept before `days` is consumed into the response.
    let mut session_days: HashMap<String, Vec<i32>> = HashMap::new();
    for (idx, agg) in &days {
        for session in &agg.sessions {
            session_days.entry(session.clone()).or_default().push(*idx);
        }
    }
    let mut days: Vec<QoderDaySpend> = days
        .into_iter()
        .map(|(idx, a)| {
            let day = chrono::NaiveDate::from_num_days_from_ce_opt(idx)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            QoderDaySpend {
                day,
                credits: a.credits,
                charged_credits: a.charged,
                original_credits: a.original_credits,
                free_trips: a.free,
                billed_tokens: a.tokens,
                tokens_unavailable_trips: a.unavailable,
                trips: a.trips,
                sessions: a.sessions.len() as u64,
                peak_ratio: a.peak_ratio,
                models: model_list(a.models),
            }
        })
        .collect();
    days.sort_by(|a, b| a.day.cmp(&b.day));

    let today = Local::now().date_naive().num_days_from_ce();
    let mut windows: [QoderWindow; 5] = Default::default();
    let mut window_sessions: [HashSet<String>; 5] = Default::default();
    let mut window_models: [HashMap<String, ModelAgg>; 5] = Default::default();
    let ranges = [(today, today), (today - 1, today - 1), (today - 6, today), (today - 29, today), (i32::MIN, today)];
    for (reach, (from, to)) in ranges
        .into_iter()
        .enumerate()
    {
        for (idx, agg) in days.iter().filter_map(|day| {
            chrono::NaiveDate::parse_from_str(&day.day, "%Y-%m-%d")
                .ok()
                .map(|d| (d.num_days_from_ce(), day))
        }) {
            if idx < from || idx > to {
                continue;
            }
            let w = &mut windows[reach];
            w.credits += agg.credits;
            w.charged_credits += agg.charged_credits;
            w.free_trips += agg.free_trips;
            w.original_credits += agg.original_credits;
            w.billed_tokens += agg.billed_tokens;
            w.tokens_unavailable_trips += agg.tokens_unavailable_trips;
            w.trips += agg.trips;
            for model in &agg.models {
                window_models[reach]
                    .entry(model.model.clone())
                    .or_default()
                    .add_spend(model);
            }
        }
    }
    // Sessions per window: a session belongs to every window that contains
    // at least one of its requests.
    for (session, indices) in &session_days {
        for (reach, slot) in window_sessions.iter_mut().enumerate() {
            let (from, to) = ranges[reach];
            if indices.iter().any(|idx| *idx >= from && *idx <= to) {
                slot.insert(session.clone());
            }
        }
    }
    for (w, set) in windows.iter_mut().zip(window_sessions.iter()) {
        w.sessions = set.len() as u64;
    }
    for (w, models) in windows.iter_mut().zip(window_models.into_iter()) {
        w.models = model_list(models);
    }

    let mut sessions: Vec<QoderSessionSpend> = sessions
        .into_iter()
        .filter_map(|(id, a)| {
            let started = a.started?;
            Some(QoderSessionSpend {
                id,
                started: started.to_rfc3339(),
                day: day_key(started).0,
                credits: a.credits,
                charged_credits: a.charged,
                free_trips: a.free,
                billed_tokens: a.tokens,
                tokens_unavailable_trips: a.unavailable,
                trips: a.trips,
                peak_ratio: a.peak_ratio,
                subagent_credits: a.subagent_credits,
                models: model_list(a.models),
            })
        })
        .collect();
    sessions.sort_by(|a, b| b.started.cmp(&a.started));
    sessions.truncate(MAX_SESSIONS);

    let [today_w, yesterday, d7, d30, all] = windows;
    let unavailable: u64 = days.iter().map(|d| d.tokens_unavailable_trips).sum();
    QoderSpend {
        today: today_w,
        yesterday,
        d7,
        d30,
        all,
        days,
        sessions,
        telemetry: QoderTelemetry::default(),
        source: QoderSource {
            state: if root.exists() { "detected" } else { "not_detected" }.into(),
            root: root.display().to_string(),
            sessions: 0,
            lite_sessions,
            tokens_unavailable_trips: unavailable,
        },
    }
}

fn qoder_db_path() -> Option<PathBuf> {
    dirs::data_dir().and_then(|dir| qoder_db_from(&dir))
}

/// Read-only model preference windows plus the capability facts around them.
/// A missing database, a locked WAL, a missing table/column or a malformed row
/// all degrade to "nothing known"; this adapter never writes to the Qoder
/// database and never guesses a window.
fn model_windows_from(path: &Path) -> HashMap<String, f64> {
    read_window_facts(Some(path)).windows
}

#[derive(Clone, Default)]
struct WindowFacts {
    windows: HashMap<String, f64>,
    models_total: u64,
    models_with_window: u64,
    schema_version: Option<i64>,
    exact_token_counts: bool,
}

fn read_window_facts(path: Option<&Path>) -> WindowFacts {
    let mut facts = WindowFacts::default();
    let Some(path) = path else { return facts };
    let Ok(connection) = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return facts;
    };
    facts.schema_version = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row.get::<_, Option<i64>>(0))
        .unwrap_or(None);
    // Every row is counted, not just the usable ones: "3 of 12 models publish a
    // window" is the number that tells an owner an update removed the basis.
    if let Ok(mut statement) = connection.prepare(
        "SELECT model_key, context_window FROM chat_model_preferences",
    ) {
        if let Ok(rows) = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        }) {
            for (model, window) in rows.flatten() {
                facts.models_total += 1;
                if let Some(window) = window.filter(|value| *value > 0) {
                    facts.models_with_window += 1;
                    facts.windows.insert(model, window as f64);
                }
            }
        }
    }
    // The official per-session context snapshot declares whether token counts
    // exist at all; reading the newest row is enough to answer that.
    if let Ok(latest) = connection.query_row(
        "SELECT snapshot_json FROM chat_session_context_usage ORDER BY updated_at DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        if let Ok(doc) = serde_json::from_str::<Value>(&latest) {
            facts.exact_token_counts = doc
                .get("tokenCountsAvailable")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || doc.get("totalTokens").and_then(Value::as_f64).unwrap_or(0.0) > 0.0;
        }
    }
    facts
}

/// Version-agnostic discovery of the Qoder CN account database. An update moves
/// the install (`.qoder-versions/<version>`) and can rename its data container,
/// so nothing here hardcodes either: every `com.qodercn.app*` directory under
/// app data that holds a `main.sqlite` is a candidate and the most recently
/// written one wins.
fn qoder_db_from(data_dir: &Path) -> Option<PathBuf> {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return None;
    };
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("com.qodercn.app") {
            continue;
        }
        let db = entry.path().join("main.sqlite");
        if let Ok(meta) = std::fs::metadata(&db) {
            if let Ok(mtime) = meta.modified() {
                candidates.push((mtime, db));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().next().map(|(_, path)| path)
}

/// Collect all Qoder CN sessions modified in the last 31 days.
pub fn collect() -> QoderSpend {
    let db = qoder_db_path();
    collect_with_db(&projects_root(), db.as_deref())
}

fn collect_with_db(root: &Path, db: Option<&Path>) -> QoderSpend {
    let facts = read_window_facts(db);
    let db_windows = facts.windows.clone();
    let (files, lite) = discover(root);
    let mut parents: HashMap<String, ParsedFile> = HashMap::new();
    for (id, path, is_subagent) in &files {
        if !is_subagent {
            parents.insert(id.clone(), cached_parse(path));
        }
    }
    let mut entries = Vec::new();
    for (id, path, is_subagent) in files {
        let parsed = if is_subagent {
            cached_parse(&path)
        } else {
            parents.get(&id).cloned().unwrap_or_default()
        };
        let parent = if is_subagent { parents.get(&id) } else { None };
        for event in parsed.events {
            // Subagent events resolve their window from the parent when the
            // subagent file itself carried no runtime-config. A window that
            // only the parent named is a same-model carry-over, not a
            // window this transcript published.
            let event = if is_subagent && event.window.is_none() {
                let window = parent.and_then(|p| inherited_window(&event, p));
                Event {
                    window_source: if window.is_some() {
                        WindowSource::SameModel
                    } else {
                        WindowSource::Unknown
                    },
                    window,
                    ..event
                }
            } else {
                event
            };
            // Last resort: Qoder's own per-model preference window. This is
            // deliberately keyed by the event's model; no global or 200K
            // fallback and no cross-model borrowing. The preference table
            // stores a real context_window, so it counts as "db".
            let event = if event.window.is_none() {
                let window = db_windows.get(&event.model).copied();
                Event {
                    window_source: if window.is_some() {
                        WindowSource::Db
                    } else {
                        WindowSource::Unknown
                    },
                    window,
                    ..event
                }
            } else {
                event
            };
            entries.push((id.clone(), event, is_subagent));
        }
    }
    let mut report = build_report(root, entries, lite);
    report.source.sessions = parents.len() as u64;
    report.telemetry = QoderTelemetry {
        db_found: db.is_some(),
        db_path: db.map(|path| path.display().to_string()).unwrap_or_default(),
        schema_version: facts.schema_version,
        models_total: facts.models_total,
        models_with_window: facts.models_with_window,
        exact_token_counts: facts.exact_token_counts,
        estimate_basis: if facts.models_with_window > 0 { "database" } else { "none" }.into(),
    };
    report
}

/// Read-only counts for the rescan dialog. "Readable" means Pane's own
/// 31-day discovery would see a main transcript for that session id. A
/// deleted session whose file is already gone is reported, never invented.
fn archive_stats_from(root: &Path, db: Option<&Path>) -> (bool, u64, u64, u64, u64) {
    let Some(db) = db else {
        return (false, 0, 0, 0, 0);
    };
    let Ok(connection) = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return (false, 0, 0, 0, 0);
    };
    let cutoff = Utc::now().timestamp_millis() - SCAN_DAYS * 86_400_000;
    let Ok(mut statement) = connection.prepare(
        "SELECT session_id, archived, (deleted_at IS NOT NULL)
           FROM chat_sessions
          WHERE updated_at > ?1",
    ) else {
        return (false, 0, 0, 0, 0);
    };
    let Ok(rows) = statement.query_map([cutoff], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1).unwrap_or(0),
            row.get::<_, i64>(2).unwrap_or(0),
        ))
    }) else {
        return (false, 0, 0, 0, 0);
    };
    let readable: HashSet<String> = discover(root)
        .0
        .into_iter()
        .filter(|(_, _, subagent)| !subagent)
        .map(|(id, _, _)| id)
        .collect();
    let mut archived_total = 0;
    let mut archived_readable = 0;
    let mut deleted_total = 0;
    let mut deleted_readable = 0;
    for (id, archived, deleted) in rows.flatten() {
        if deleted != 0 {
            deleted_total += 1;
            if readable.contains(&id) {
                deleted_readable += 1;
            }
        } else if archived != 0 {
            archived_total += 1;
            if readable.contains(&id) {
                archived_readable += 1;
            }
        }
    }
    (
        true,
        archived_total,
        archived_readable,
        deleted_total,
        deleted_readable,
    )
}

/// Force a fresh Qoder scan (cache clear + reparse) and return both the
/// recomputed credits/tokens and the archived/deleted transcript coverage.
pub fn rescan() -> QoderRescan {
    clear_cache();
    let root = projects_root();
    let db = qoder_db_path();
    let spend = collect_with_db(&root, db.as_deref());
    let (stats_available, archived_total, archived_readable, deleted_total, deleted_readable) =
        archive_stats_from(&root, db.as_deref());
    QoderRescan {
        spend,
        stats_available,
        archived_total,
        archived_readable,
        deleted_total,
        deleted_readable,
    }
}

fn model_label(model: &str) -> String {
    static LABELS: OnceLock<HashMap<String, String>> = OnceLock::new();
    let labels = LABELS.get_or_init(|| {
        let Some(dir) = dirs::data_dir() else {
            return HashMap::new();
        };
        let path = dir
            .join("com.qodercn.app.stable")
            .join("dynamic-text")
            .join("qoder.v1.json");
        let Ok(raw) = fs::read_to_string(path) else {
            return HashMap::new();
        };
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            return HashMap::new();
        };
        let Some(data) = value.get("data").and_then(Value::as_object) else {
            return HashMap::new();
        };
        let mut out = HashMap::new();
        for locale in ["en", "zh"] {
            let Some(texts) = data.get(locale).and_then(Value::as_object) else {
                continue;
            };
            for (key, label) in texts {
                let Some(model) = key.strip_prefix("model.").and_then(|k| k.strip_suffix(".label")) else {
                    continue;
                };
                if let Some(label) = label.as_str() {
                    out.entry(model.to_string()).or_insert_with(|| label.to_string());
                }
            }
        }
        out
    });
    labels.get(model).cloned().unwrap_or_else(|| model.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// An update moves the install and can rename the data container, so the
    /// durable rule is "newest `com.qodercn.app*/main.sqlite`" — never a
    /// hardcoded version or container name that breaks the next bump.
    #[test]
    fn db_discovery_follows_the_newest_container() {
        let base = std::env::temp_dir().join(format!("pane-qoder-discovery-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("Pane")).unwrap();
        std::fs::create_dir_all(base.join("com.qodercn.app.other")).unwrap();
        std::fs::create_dir_all(base.join("com.qodercn.app.stable")).unwrap();
        std::fs::create_dir_all(base.join("com.qodercn.app.stable2")).unwrap();
        let older = base.join("com.qodercn.app.stable").join("main.sqlite");
        let newer = base.join("com.qodercn.app.stable2").join("main.sqlite");
        std::fs::write(&older, "x").unwrap();
        std::fs::write(&newer, "x").unwrap();
        let future = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
        std::fs::OpenOptions::new().write(true).open(&newer).unwrap().set_modified(future).unwrap();

        assert_eq!(qoder_db_from(&base), Some(newer.clone()));
        // A container without a database is not a candidate.
        std::fs::remove_file(&newer).unwrap();
        assert_eq!(qoder_db_from(&base), Some(older));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The probe must be able to tell "no usage today" apart from "the update
    /// took the windows away", so null windows are counted, not filtered out
    /// before the tally, and a missing table degrades to "nothing known".
    #[test]
    fn window_facts_count_null_windows_and_report_the_official_token_flag() {
        let base = std::env::temp_dir().join(format!("pane-qoder-facts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let path = base.join("main.sqlite");
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER);
                 CREATE TABLE chat_model_preferences (model_key TEXT, context_window INTEGER);
                 CREATE TABLE chat_session_context_usage (session_id TEXT, snapshot_json TEXT, updated_at INTEGER);
                 INSERT INTO schema_migrations VALUES (83), (84);
                 INSERT INTO chat_model_preferences VALUES
                     ('dfmodel', 1000000), ('gmodel', NULL), ('kmodel', 0);
                 INSERT INTO chat_session_context_usage VALUES
                     ('old', '{\"tokenCountsAvailable\":true,\"totalTokens\":50000}', 1),
                     ('new', '{\"tokenCountsAvailable\":false,\"totalTokens\":0}', 2);",
            ).unwrap();
        }
        let facts = read_window_facts(Some(&path));
        assert_eq!(facts.schema_version, Some(84));
        assert_eq!(facts.models_total, 3);
        assert_eq!(facts.models_with_window, 1);
        assert!(!facts.exact_token_counts, "the newest snapshot declares no token counts");
        assert_eq!(model_windows_from(&path).get("dfmodel"), Some(&1_000_000.0));

        // A database without those tables is "nothing known", never a guess.
        let bare = base.join("bare.sqlite");
        Connection::open(&bare).unwrap();
        let facts = read_window_facts(Some(&bare));
        assert_eq!(facts.models_total, 0);
        assert_eq!(facts.models_with_window, 0);
        assert_eq!(facts.schema_version, None);
        assert!(read_window_facts(None).windows.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    fn usage_line(ts: &str, model: &str, credits: f64, ratio: f64, request: &str) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"{ts}","message":{{"id":"m-{request}","model":"{model}","usage":{{"credits":{credits},"original_credits":{credits},"context_usage_ratio":{ratio},"request_id":"{request}","billable":true}}}}}}"#
        )
    }

    fn config_line(ts_ms: i64, model: &str, window: Option<u64>) -> String {
        let window = window.map(|w| w.to_string()).unwrap_or_else(|| "null".into());
        format!(
            r#"{{"type":"runtime-config","model":"{model}","contextWindow":{window},"timestamp":{ts_ms}}}"#
        )
    }

    fn parse(lines: &[String]) -> ParsedFile {
        parse_reader(lines.iter().map(|l| format!("{l}\n")).collect::<String>().as_bytes())
    }

    #[test]
    fn window_is_the_one_in_force_at_request_time() {
        let lines = vec![
            config_line(1, "qmodel", Some(1_000_000)),
            usage_line("2026-10-20T01:00:00Z", "qmodel", 2.0, 0.1, "a"),
            config_line(2, "qmodel", Some(200_000)),
            usage_line("2026-10-20T02:00:00Z", "qmodel", 1.0, 0.5, "b"),
        ];
        let parsed = parse(&lines);
        let events = &parsed.events;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].window, Some(1_000_000.0));
        assert_eq!(events[1].window, Some(200_000.0));
        assert_eq!(derive_tokens(events[0].ratio, events[0].window), Some(100_000.0));
        assert_eq!(derive_tokens(events[1].ratio, events[1].window), Some(100_000.0));
    }

    #[test]
    fn null_config_keeps_same_model_window_and_never_crosses_models() {
        let lines = vec![
            config_line(1, "dfmodel", Some(1_000_000)),
            usage_line("2026-10-20T01:00:00Z", "dfmodel", 1.0, 0.1, "a"),
            config_line(2, "dfmodel", None),
            usage_line("2026-10-20T02:00:00Z", "dfmodel", 1.0, 0.2, "b"),
            config_line(3, "qfmodel", None),
            usage_line("2026-10-20T03:00:00Z", "qfmodel", 1.0, 0.3, "c"),
        ];
        let parsed = parse(&lines);
        assert_eq!(parsed.events[0].window, Some(1_000_000.0));
        assert_eq!(parsed.events[1].window, Some(1_000_000.0), "null must not clear the same model's window");
        assert_eq!(parsed.events[2].window, None, "a new model must not borrow dfmodel's window");
        let report = build_report(
            Path::new("unused"),
            parsed.events.into_iter().map(|event| ("s1".into(), event, false)).collect(),
            0,
        );
        assert_eq!(report.all.billed_tokens, 300_000.0);
        assert_eq!(report.all.tokens_unavailable_trips, 1);
    }

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("qoder-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("proj")).unwrap();
        root
    }

    fn write_session(root: &Path, name: &str, lines: &[String]) {
        fs::write(root.join("proj").join(name), lines.join("\n") + "\n").unwrap();
    }

    fn make_preferences(path: &Path, rows: &[(&str, Option<i64>)]) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE chat_model_preferences (
                    model_key TEXT PRIMARY KEY,
                    visible INTEGER,
                    context_window INTEGER,
                    reasoning_effort TEXT,
                    updated_at INTEGER
                );",
            )
            .unwrap();
        for (model, window) in rows {
            connection
                .execute(
                    "INSERT INTO chat_model_preferences (model_key, context_window) VALUES (?1, ?2)",
                    rusqlite::params![model, window],
                )
                .unwrap();
        }
    }

    #[test]
    fn model_preferences_fill_a_null_only_new_session_and_stay_per_model() {
        let root = temp_root("db-window");
        let db = root.join("main.sqlite");
        make_preferences(&db, &[("dfmodel", Some(1_000_000)), ("qfmodel", None)]);
        write_session(
            &root,
            "s1.jsonl",
            &[
                config_line(1, "dfmodel", None),
                usage_line("2026-10-20T01:00:00Z", "dfmodel", 1.0, 0.25, "a"),
                usage_line("2026-10-20T01:01:00Z", "qfmodel", 1.0, 0.5, "b"),
            ],
        );
        let report = collect_with_db(&root, Some(&db));
        assert_eq!(report.all.billed_tokens, 250_000.0);
        assert_eq!(report.all.tokens_unavailable_trips, 1);
        assert_eq!(report.all.credits, 2.0);
        let df = report.all.models.iter().find(|model| model.model == "dfmodel").unwrap();
        assert_eq!(df.billed_tokens, 250_000.0);
        assert_eq!(df.window_source, "db", "the preference table stores a real context_window");
        let qf = report.all.models.iter().find(|model| model.model == "qfmodel").unwrap();
        assert_eq!(qf.billed_tokens, 0.0);
        assert_eq!(qf.tokens_unavailable_trips, 1);
        assert_eq!(qf.window_source, "unknown");
        let _ = fs::remove_dir_all(&root);
    }

    /// #37-03: the report explains where each model's window came from —
    /// a real context_window ("db"), a same-model carry-over ("same_model")
    /// or nothing ("unknown") — without touching the token math.
    #[test]
    fn window_source_explains_where_each_model_window_came_from() {
        let lines = vec![
            config_line(1, "dfmodel", Some(1_000_000)),
            usage_line("2026-10-20T01:00:00Z", "dfmodel", 1.0, 0.1, "a"),
            config_line(2, "dfmodel", None),
            usage_line("2026-10-20T02:00:00Z", "dfmodel", 1.0, 0.2, "b"),
            config_line(3, "qfmodel", None),
            usage_line("2026-10-20T03:00:00Z", "qfmodel", 1.0, 0.3, "c"),
        ];
        let parsed = parse(&lines);
        assert_eq!(parsed.events[0].window_source, WindowSource::Db);
        assert_eq!(
            parsed.events[1].window_source,
            WindowSource::SameModel,
            "a null-config round carries the same model's window over"
        );
        assert_eq!(parsed.events[2].window_source, WindowSource::Unknown);
        let report = build_report(
            Path::new("unused"),
            parsed.events.into_iter().map(|event| ("s1".into(), event, false)).collect(),
            0,
        );
        // The strongest basis any request got wins per model.
        let df = report.all.models.iter().find(|m| m.model == "dfmodel").unwrap();
        assert_eq!(df.window_source, "db");
        let qf = report.all.models.iter().find(|m| m.model == "qfmodel").unwrap();
        assert_eq!(qf.window_source, "unknown");
        // Token math unchanged.
        assert_eq!(report.all.billed_tokens, 300_000.0);
        assert_eq!(report.all.tokens_unavailable_trips, 1);
    }

    #[test]
    fn subagent_only_model_reports_same_model_window_source() {
        let root = temp_root("window-source");
        // The parent names the window but spends nothing itself; the subagent
        // spends under an inherited window — that is a carry-over, not a db
        // reading inside the subagent transcript.
        write_session(&root, "s1.jsonl", &[config_line(1, "dfmodel", Some(1_000_000))]);
        fs::create_dir_all(root.join("proj").join("s1").join("subagents")).unwrap();
        fs::write(
            root.join("proj").join("s1").join("subagents").join("agent-1.jsonl"),
            usage_line("2026-10-20T01:00:00Z", "dfmodel", 0.5, 0.25, "sub") + "\n",
        )
        .unwrap();
        let report = collect_with_db(&root, None);
        let df = report.all.models.iter().find(|m| m.model == "dfmodel").unwrap();
        assert_eq!(df.window_source, "same_model");
        assert_eq!(df.billed_tokens, 250_000.0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_database_or_table_degrades_to_unavailable() {
        let root = temp_root("db-missing");
        write_session(
            &root,
            "s1.jsonl",
            &[
                config_line(1, "dfmodel", None),
                usage_line("2026-10-20T01:00:00Z", "dfmodel", 1.0, 0.25, "a"),
            ],
        );
        let missing = collect_with_db(&root, Some(&root.join("missing.sqlite")));
        assert_eq!(missing.all.billed_tokens, 0.0);
        assert_eq!(missing.all.tokens_unavailable_trips, 1);

        let empty_db = root.join("empty.sqlite");
        let _ = Connection::open(&empty_db).unwrap();
        let empty = collect_with_db(&root, Some(&empty_db));
        assert_eq!(empty.all.billed_tokens, 0.0);
        assert_eq!(empty.all.tokens_unavailable_trips, 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn archive_stats_separate_readable_archived_and_deleted_sessions() {
        let root = temp_root("archive-stats");
        write_session(
            &root,
            "s1.jsonl",
            &[usage_line("2026-10-20T01:00:00Z", "dfmodel", 1.0, 0.1, "a")],
        );
        write_session(
            &root,
            "s2.jsonl",
            &[usage_line("2026-10-20T01:01:00Z", "dfmodel", 1.0, 0.1, "b")],
        );
        let db = root.join("main.sqlite");
        let connection = Connection::open(&db).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE chat_sessions (
                    session_id TEXT,
                    archived INTEGER,
                    deleted_at TEXT,
                    updated_at INTEGER
                );",
            )
            .unwrap();
        let now = Utc::now().timestamp_millis();
        for (id, archived, deleted) in [
            ("s1", 1, None),
            ("s2", 0, Some("2026-10-22T00:00:00Z")),
            ("s3", 1, None),
            ("s4", 0, Some("2026-10-22T00:00:00Z")),
        ] {
            connection
                .execute(
                    "INSERT INTO chat_sessions (session_id, archived, deleted_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![id, archived, deleted, now],
                )
                .unwrap();
        }
        drop(connection);
        assert_eq!(
            archive_stats_from(&root, Some(&db)),
            (true, 2, 1, 2, 1),
            "s1 archived+readable; s3 archived+missing; s2 deleted+readable; s4 deleted+missing"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_window_leaves_tokens_unavailable_but_credits_stand() {
        let lines = vec![
            config_line(1, "dfmodel", None),
            usage_line("2026-10-20T01:00:00Z", "dfmodel", 0.82, 0.18672, "a"),
        ];
        let parsed = parse(&lines);
        assert_eq!(parsed.events[0].window, None);
        assert_eq!(derive_tokens(parsed.events[0].ratio, parsed.events[0].window), None);
        assert_eq!(parsed.events[0].credits, 0.82);
    }

    #[test]
    fn free_requests_keep_nominal_credits_but_charge_nothing() {
        let line = r#"{"type":"assistant","timestamp":"2026-10-20T01:00:00Z","message":{"id":"m-a","model":"dfmodel","usage":{"credits":2.5,"original_credits":2.5,"context_usage_ratio":0.1,"request_id":"a","billable":false}}}"#.to_string();
        let parsed = parse(&[line]);
        assert_eq!(parsed.events[0].billable, Some(false));
        let report = build_report(
            Path::new("unused"),
            vec![("s1".into(), parsed.events[0].clone(), false)],
            0,
        );
        assert_eq!(report.all.credits, 2.5);
        assert_eq!(report.all.charged_credits, 0.0);
        assert_eq!(report.all.free_trips, 1);
        assert_eq!(report.days[0].charged_credits, 0.0);
        assert_eq!(report.days[0].models[0].charged_credits, 0.0);
        assert_eq!(report.sessions[0].charged_credits, 0.0);
        assert_eq!(report.sessions[0].free_trips, 1);
    }

    #[test]
    fn duplicate_request_ids_count_once_across_main_and_subagent() {
        let root = Path::new("unused");
        let event = Event {
            ts: DateTime::parse_from_rfc3339("2026-10-20T01:00:00Z").unwrap().with_timezone(&Utc),
            model: "dfmodel".into(),
            credits: 1.0,
            original_credits: 1.0,
            ratio: Some(0.1),
            window: Some(1000.0),
            window_source: WindowSource::Db,
            request_id: "same".into(),
            billable: Some(true),
        };
        let report = build_report(
            root,
            vec![
                ("s1".into(), event.clone(), false),
                ("s1".into(), event, true),
            ],
            0,
        );
        assert_eq!(report.all.trips, 1);
        assert_eq!(report.all.credits, 1.0);
        // The duplicate was dropped, and it never counted as subagent spend.
        assert_eq!(report.sessions[0].subagent_credits, 0.0);
    }

    #[test]
    fn subagent_inherits_parent_window() {
        let event = Event {
            ts: DateTime::parse_from_rfc3339("2026-10-20T02:00:00Z").unwrap().with_timezone(&Utc),
            model: "dfmodel".into(),
            credits: 0.5,
            original_credits: 0.5,
            ratio: Some(0.25),
            window: None,
            window_source: WindowSource::Unknown,
            request_id: "sub".into(),
            billable: Some(true),
        };
        let mut parent = ParsedFile::default();
        parent.windows.insert("dfmodel".into(), 200_000.0);
        let window = inherited_window(&event, &parent);
        let event = Event {
            window_source: if window.is_some() { WindowSource::SameModel } else { WindowSource::Unknown },
            window,
            ..event
        };
        let report = build_report(Path::new("unused"), vec![("s1".into(), event, true)], 0);
        assert_eq!(report.all.billed_tokens, 50_000.0);
        assert_eq!(report.all.tokens_unavailable_trips, 0);
        assert_eq!(report.all.models[0].window_source, "same_model");
    }

    #[test]
    fn subagent_does_not_borrow_another_models_window() {
        let event = Event {
            ts: DateTime::parse_from_rfc3339("2026-10-20T02:00:00Z").unwrap().with_timezone(&Utc),
            model: "qfmodel".into(),
            credits: 0.5,
            original_credits: 0.5,
            ratio: Some(0.25),
            window: None,
            window_source: WindowSource::Unknown,
            request_id: "sub-other".into(),
            billable: Some(true),
        };
        let mut parent = ParsedFile::default();
        parent.windows.insert("dfmodel".into(), 1_000_000.0);
        assert_eq!(inherited_window(&event, &parent), None);
    }

    #[test]
    fn windows_carry_per_model_breakdown() {
        let ts = Local::now().with_timezone(&Utc);
        let event = |model: &str, credits: f64, ratio: f64, request: &str| Event {
            ts,
            model: model.into(),
            credits,
            original_credits: credits,
            ratio: Some(ratio),
            window: Some(100_000.0),
            window_source: WindowSource::Db,
            request_id: request.into(),
            billable: Some(true),
        };
        let report = build_report(
            Path::new("unused"),
            vec![
                ("s1".into(), event("alpha", 1.0, 0.1, "a"), false),
                ("s1".into(), event("beta", 2.0, 0.5, "b"), false),
            ],
            0,
        );
        assert_eq!(report.today.models.len(), 2);
        let alpha = report.today.models.iter().find(|m| m.model == "alpha").unwrap();
        assert_eq!(alpha.credits, 1.0);
        assert_eq!(alpha.billed_tokens, 10_000.0);
        let beta = report.today.models.iter().find(|m| m.model == "beta").unwrap();
        assert_eq!(beta.billed_tokens, 50_000.0);
        assert_eq!(report.all.models.len(), 2);
    }

    #[test]
    fn events_split_across_local_midnight() {
        use chrono::TimeZone;
        let anchor = Local::now().date_naive() - chrono::Duration::days(1);
        let midnight = anchor
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .earliest()
            .unwrap();
        let event = |ts: DateTime<Local>, request: &str| Event {
            ts: ts.with_timezone(&Utc),
            model: "dfmodel".into(),
            credits: 1.0,
            original_credits: 1.0,
            ratio: Some(0.1),
            window: Some(100_000.0),
            window_source: WindowSource::Db,
            request_id: request.into(),
            billable: Some(true),
        };
        let report = build_report(
            Path::new("unused"),
            vec![
                ("s1".into(), event(midnight - chrono::Duration::minutes(1), "before"), false),
                ("s1".into(), event(midnight + chrono::Duration::minutes(1), "after"), false),
            ],
            0,
        );
        assert_eq!(report.days.len(), 2);
        assert!(report.days.iter().all(|day| day.trips == 1 && day.credits == 1.0));
        assert_eq!(report.all.trips, 2);
    }

    #[test]
    fn lite_layout_is_counted_but_rich_layout_wins() {
        let root = std::env::temp_dir().join(format!("qoder-discover-{}", std::process::id()));
        let slug = root.join("proj");
        fs::create_dir_all(slug.join("transcript")).unwrap();
        fs::create_dir_all(slug.join("s1").join("subagents")).unwrap();
        fs::write(
            slug.join("s1.jsonl"),
            format!("{}\n", usage_line("2026-10-20T01:00:00Z", "dfmodel", 1.0, 0.1, "a")),
        )
        .unwrap();
        fs::write(slug.join("transcript").join("s1.jsonl"), "{}\n").unwrap();
        fs::write(slug.join("transcript").join("s2.jsonl"), "{}\n").unwrap();
        fs::write(
            slug.join("s1").join("subagents").join("agent-1.jsonl"),
            format!("{}\n", usage_line("2026-10-20T01:01:00Z", "dfmodel", 0.5, 0.1, "b")),
        )
        .unwrap();
        let (files, lite) = discover(&root);
        assert_eq!(lite, 2);
        let mains: Vec<_> = files.iter().filter(|(_, _, sub)| !sub).collect();
        assert_eq!(mains.len(), 1);
        assert_eq!(mains[0].0, "s1");
        assert_eq!(files.iter().filter(|(_, _, sub)| *sub).count(), 1);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    #[ignore = "real-machine anchor: reads this machine's Qoder CN transcripts"]
    fn real_machine_snapshot() {
        let report = collect();
        eprintln!(
            "source={} sessions={} lite={} | today: nominal {:.3} / charged {:.3} credits ({} free trips), {:.0} billed tokens, {} trips, {} sessions, {} unavailable",
            report.source.state,
            report.source.sessions,
            report.source.lite_sessions,
            report.today.credits,
            report.today.charged_credits,
            report.today.free_trips,
            report.today.billed_tokens,
            report.today.trips,
            report.today.sessions,
            report.today.tokens_unavailable_trips,
        );
        for session in &report.sessions {
            if session.day == report.days.last().map(|d| d.day.clone()).unwrap_or_default() {
                eprintln!(
                    "  session {} : {:.3} credits, {} trips, subagent {:.3}",
                    &session.id[..8.min(session.id.len())],
                    session.credits,
                    session.trips,
                    session.subagent_credits
                );
            }
        }
        for day in report.days.iter().rev().take(3) {
            eprintln!(
                "  {}: {:.3} credits, {:.0} tokens, {} trips, models {:?}",
                day.day,
                day.credits,
                day.billed_tokens,
                day.trips,
                day.models.iter().map(|m| (&m.model, &m.label, m.trips)).collect::<Vec<_>>()
            );
        }
        assert_eq!(report.source.state, "detected");
    }

    #[test]
    fn yesterday_is_one_local_calendar_day_not_yesterday_plus_today() {
        let today=Local::now().date_naive();
        let yesterday=today.pred_opt().unwrap();
        let ts=|day: chrono::NaiveDate, h, m, sec| day.and_hms_opt(h,m,sec).unwrap()
            .and_local_timezone(Local).single().unwrap().with_timezone(&Utc);
        let event=|ts, id: &str, ratio| Event {ts,model:"dfmodel".into(),credits:1.0,original_credits:2.0,
            ratio:Some(ratio),window:Some(1_000_000.0),window_source:WindowSource::Db,request_id:id.into(),billable:Some(true)};
        let entries=vec![
            ("yesterday-start".into(),event(ts(yesterday,0,0,0),"ys",0.1),false),
            ("yesterday-end".into(),event(ts(yesterday,23,59,59),"ye",0.2),false),
            ("today".into(),event(ts(today,0,0,0),"t",0.8),false),
            ("older".into(),event(ts(yesterday.pred_opt().unwrap(),23,59,59),"o",0.9),false),
        ];
        let report=build_report(Path::new("unused"),entries,0);
        assert_eq!(report.yesterday.billed_tokens,300_000.0);
        assert_eq!(report.yesterday.trips,2);
        assert_eq!(report.yesterday.sessions,2);
        assert_eq!(report.yesterday.credits,2.0);
        assert_eq!(report.yesterday.original_credits,4.0);
        assert_eq!(report.yesterday.models[0].billed_tokens,300_000.0);
        assert_eq!(report.today.billed_tokens,800_000.0);
        assert_eq!(report.d7.billed_tokens,2_000_000.0);
    }

    #[test]
    #[ignore = "read-only owner-authorized audit of local Qoder usage metadata"]
    fn real_machine_yesterday_audit() {
        let report=collect();
        eprintln!("window today tokens={:.0} trips={} yesterday tokens={:.0} trips={}",
            report.today.billed_tokens,report.today.trips,report.yesterday.billed_tokens,report.yesterday.trips);
        for day in report.days.iter().rev().take(3) {
            eprintln!("day={} tokens={:.0} credits={:.3} trips={}",day.day,day.billed_tokens,day.credits,day.trips);
            for model in &day.models {
                eprintln!("  model={} tokens={:.0} trips={} unavailable={}",model.label,model.billed_tokens,model.trips,model.tokens_unavailable_trips);
            }
        }
        for model in &report.yesterday.models {
            eprintln!("yesterday-model={} tokens={:.0} trips={}",model.label,model.billed_tokens,model.trips);
        }
    }

    #[test]
    fn day_windows_and_unavailable_trips_are_tallied() {
        let root = Path::new("unused");
        let ts = DateTime::parse_from_rfc3339("2026-10-20T01:00:00Z").unwrap().with_timezone(&Utc);
        let base = Event {
            ts,
            model: "dfmodel".into(),
            credits: 1.0,
            original_credits: 1.0,
            ratio: Some(0.1),
            window: None,
            window_source: WindowSource::Unknown,
            request_id: "x".into(),
            billable: Some(true),
        };
        let report = build_report(root, vec![("s1".into(), base, false)], 0);
        assert_eq!(report.all.tokens_unavailable_trips, 1);
        assert_eq!(report.all.credits, 1.0);
        assert_eq!(report.all.models[0].window_source, "unknown");
        assert!(!report.days.is_empty());
    }
}
