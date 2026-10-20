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
//! `request_id` across files. Nothing here is added to Total Spend or the
//! dollar donut: credits are their own unit.

use chrono::{DateTime, Datelike, Local, Utc};
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

#[derive(Serialize, Clone, Default)]
pub struct QoderWindow {
    pub credits: f64,
    pub original_credits: f64,
    /// Sum of derived billed input tokens for the requests whose window is
    /// known. Trips without a window are counted separately.
    pub billed_tokens: f64,
    pub tokens_unavailable_trips: u64,
    pub trips: u64,
    pub sessions: u64,
}

#[derive(Serialize, Clone)]
pub struct QoderModelSpend {
    pub model: String,
    pub label: String,
    pub credits: f64,
    pub billed_tokens: f64,
    pub trips: u64,
    pub tokens_unavailable_trips: u64,
}

#[derive(Serialize, Clone)]
pub struct QoderDaySpend {
    pub day: String,
    pub credits: f64,
    pub original_credits: f64,
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
}

#[derive(Clone)]
struct Event {
    ts: DateTime<Utc>,
    model: String,
    credits: f64,
    original_credits: f64,
    ratio: Option<f64>,
    window: Option<f64>,
    request_id: String,
}

#[derive(Clone, Default)]
struct ParsedFile {
    events: Vec<Event>,
    /// Last known window per model, for subagent transcripts that carry none.
    windows: HashMap<String, f64>,
    /// Last window in force regardless of model. Only used to resolve a
    /// subagent request whose model was never named in a runtime-config.
    last_window: Option<f64>,
}

/// Cache entries keyed by (mtime, size): a session log is append-only, so an
/// unchanged file cannot contribute new requests.
type CacheKey = (SystemTime, u64);
fn cache() -> &'static Mutex<HashMap<PathBuf, (CacheKey, ParsedFile)>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (CacheKey, ParsedFile)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
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
    Some((meta.modified().ok()?, meta.len()))
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
    let mut current: Option<(String, Option<f64>)> = None;
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
                out.last_window = Some(w);
            }
            current = Some((model.to_string(), window));
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
            .or_else(|| current.as_ref().map(|(m, _)| m.clone()))
            .unwrap_or_default();
        if model.is_empty() {
            continue;
        }
        let window = match current.as_ref() {
            Some((current_model, window)) if current_model == &model => *window,
            _ => out.windows.get(&model).copied(),
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
            request_id,
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
    parent
        .windows
        .get(&event.model)
        .copied()
        .or(parent.last_window)
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
    tokens: f64,
    trips: u64,
    unavailable: u64,
}

#[derive(Default)]
struct SessionAgg {
    started: Option<DateTime<Utc>>,
    credits: f64,
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
    entry.trips += 1;
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
        let (_, day_idx) = day_key(event.ts);
        let session = sessions.entry(session_id.clone()).or_default();
        if session.started.map_or(true, |s| event.ts < s) {
            session.started = Some(event.ts);
        }
        session.credits += event.credits;
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
                billed_tokens: a.tokens,
                trips: a.trips,
                tokens_unavailable_trips: a.unavailable,
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
                original_credits: a.original_credits,
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
    for (reach, from) in [today, today - 1, today - 6, today - 29, i32::MIN]
        .into_iter()
        .enumerate()
    {
        for (idx, agg) in days.iter().filter_map(|day| {
            chrono::NaiveDate::parse_from_str(&day.day, "%Y-%m-%d")
                .ok()
                .map(|d| (d.num_days_from_ce(), day))
        }) {
            if idx < from || idx > today {
                continue;
            }
            let w = &mut windows[reach];
            w.credits += agg.credits;
            w.original_credits += agg.original_credits;
            w.billed_tokens += agg.billed_tokens;
            w.tokens_unavailable_trips += agg.tokens_unavailable_trips;
            w.trips += agg.trips;
        }
    }
    // Sessions per window: a session belongs to every window that contains
    // at least one of its requests.
    for (session, indices) in &session_days {
        for (reach, slot) in window_sessions.iter_mut().enumerate() {
            let from = match reach {
                0 => today,
                1 => today - 1,
                2 => today - 6,
                3 => today - 29,
                _ => i32::MIN,
            };
            if indices.iter().any(|idx| *idx >= from && *idx <= today) {
                slot.insert(session.clone());
            }
        }
    }
    for (w, set) in windows.iter_mut().zip(window_sessions.iter()) {
        w.sessions = set.len() as u64;
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
        source: QoderSource {
            state: if root.exists() { "detected" } else { "not_detected" }.into(),
            root: root.display().to_string(),
            sessions: 0,
            lite_sessions,
            tokens_unavailable_trips: unavailable,
        },
    }
}

/// Collect all Qoder CN sessions modified in the last 31 days.
pub fn collect() -> QoderSpend {
    let root = projects_root();
    let (files, lite) = discover(&root);
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
            // subagent file itself carried no runtime-config.
            let event = if is_subagent && event.window.is_none() {
                let window = parent.and_then(|p| inherited_window(&event, p));
                Event { window, ..event }
            } else {
                event
            };
            entries.push((id.clone(), event, is_subagent));
        }
    }
    let mut report = build_report(&root, entries, lite);
    report.source.sessions = parents.len() as u64;
    report
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

    fn usage_line(ts: &str, model: &str, credits: f64, ratio: f64, request: &str) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"{ts}","message":{{"id":"m-{request}","model":"{model}","usage":{{"credits":{credits},"original_credits":{credits},"context_usage_ratio":{ratio},"request_id":"{request}"}}}}}}"#
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
    fn duplicate_request_ids_count_once_across_main_and_subagent() {
        let root = Path::new("unused");
        let event = Event {
            ts: DateTime::parse_from_rfc3339("2026-10-20T01:00:00Z").unwrap().with_timezone(&Utc),
            model: "dfmodel".into(),
            credits: 1.0,
            original_credits: 1.0,
            ratio: Some(0.1),
            window: Some(1000.0),
            request_id: "same".into(),
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
            request_id: "sub".into(),
        };
        let mut parent = ParsedFile::default();
        parent.windows.insert("dfmodel".into(), 200_000.0);
        parent.last_window = Some(200_000.0);
        let event = Event { window: inherited_window(&event, &parent), ..event };
        let report = build_report(Path::new("unused"), vec![("s1".into(), event, true)], 0);
        assert_eq!(report.all.billed_tokens, 50_000.0);
        assert_eq!(report.all.tokens_unavailable_trips, 0);
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
            request_id: request.into(),
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
            "source={} sessions={} lite={} | today: {:.3} credits, {:.0} billed tokens, {} trips, {} sessions, {} unavailable",
            report.source.state,
            report.source.sessions,
            report.source.lite_sessions,
            report.today.credits,
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
            request_id: "x".into(),
        };
        let report = build_report(root, vec![("s1".into(), base, false)], 0);
        assert_eq!(report.all.tokens_unavailable_trips, 1);
        assert_eq!(report.all.credits, 1.0);
        assert!(!report.days.is_empty());
    }
}
