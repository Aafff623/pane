//! Per-card quota history — the sampled "worst used percent" behind the
//! trend bars on cards that have no local CLI logs. spend.rs builds trends
//! from local session logs (claude/codex/kimi CLI); API-key accounts and
//! relay keys never appear there, because their quota lives on the vendor
//! side and every quota endpoint only answers with the current window.
//! So each successful fetch samples the card's most-drained progress metric
//! into a per-day bucket; the frontend renders those buckets as the trend
//! for any card the local-log spend doesn't cover.

use crate::providers::Metric;
use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const VERSION: u32 = 1;
/// Days kept per card. The trend spans 30; the slack absorbs days the app
/// stayed closed or the provider errored, so the window doesn't shrink.
const KEEP_DAYS: i64 = 35;
/// Days the rendered trend spans — trend[29] is today, like spend trends.
pub const TREND_DAYS: usize = 30;

#[derive(Serialize, Deserialize, Clone)]
struct DaySample {
    /// Local calendar day, "YYYY-MM-DD" — sortable, and the trend axis is
    /// built from the same strings.
    day: String,
    /// Max used_percent recorded that day across the card's progress rows.
    used: f64,
}

#[derive(Serialize, Deserialize, Default)]
struct HistoryFile {
    version: u32,
    /// card id → samples, sorted by day.
    entries: BTreeMap<String, Vec<DaySample>>,
}

impl HistoryFile {
    fn new() -> Self {
        Self {
            version: VERSION,
            entries: BTreeMap::new(),
        }
    }
}

fn today_string() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

/// The card's most-drained progress row this fetch. Session windows roll
/// back to 0% on reset while Weekly carries the real drain, and which row
/// is "the" quota differs per vendor — the max is the honest per-card value
/// without hardcoding metric labels.
pub fn worst_used_percent(metrics: &[Metric]) -> Option<f64> {
    metrics
        .iter()
        .filter(|m| m.kind == "progress")
        .filter_map(|m| m.used_percent)
        .fold(None, |acc: Option<f64>, v| {
            Some(match acc {
                Some(best) if best >= v => best,
                _ => v,
            })
        })
}

/// Upsert one sample: same-day samples keep the max (a drain never un-happens
/// within a day, even if a window reset made a later fetch read lower).
fn record_sample(
    entries: &mut BTreeMap<String, Vec<DaySample>>,
    id: &str,
    day: &str,
    used: f64,
) {
    let samples = entries.entry(id.to_string()).or_default();
    match samples.iter_mut().find(|s| s.day == day) {
        Some(existing) => {
            if used > existing.used {
                existing.used = used;
            }
        }
        None => {
            samples.push(DaySample {
                day: day.to_string(),
                used,
            });
            samples.sort_by(|a, b| a.day.cmp(&b.day));
        }
    }
    prune_samples(samples, day, KEEP_DAYS);
}

/// Drop samples older than the retention window and cards left empty.
/// ISO dates compare correctly as strings.
fn prune_samples(samples: &mut Vec<DaySample>, today: &str, keep_days: i64) {
    let cutoff = parse_day(today)
        .map(|today| (today - Duration::days(keep_days)).format("%Y-%m-%d").to_string());
    let cutoff = cutoff.unwrap_or_default();
    samples.retain(|s| s.day >= cutoff);
}

fn parse_day(day: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

/// The 30-value trend for one card, oldest first, today last. Days without
/// a sample read 0 — the same "nothing recorded" the token trends show.
fn trend_for(
    entries: &BTreeMap<String, Vec<DaySample>>,
    id: &str,
    today: &str,
) -> Vec<f64> {
    let Some(today) = parse_day(today) else {
        return vec![0.0; TREND_DAYS];
    };
    let samples = entries.get(id);
    (0..TREND_DAYS as i64)
        .map(|i| {
            let day = (today - Duration::days(TREND_DAYS as i64 - 1 - i))
                .format("%Y-%m-%d")
                .to_string();
            samples
                .and_then(|list| list.iter().find(|s| s.day == day))
                .map(|s| s.used)
                .unwrap_or(0.0)
        })
        .collect()
}

fn persist_path() -> PathBuf {
    crate::providers::config_dir().join("usage_history.json")
}

fn store() -> &'static Mutex<HistoryFile> {
    static STORE: OnceLock<Mutex<HistoryFile>> = OnceLock::new();
    STORE.get_or_init(|| {
        let file = fs::read_to_string(persist_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<HistoryFile>(&raw).ok())
            .filter(|doc| doc.version == VERSION)
            .unwrap_or_else(HistoryFile::new);
        Mutex::new(file)
    })
}

fn persist(file: &HistoryFile) {
    let tmp = persist_path().with_extension("json.tmp");
    if fs::write(&tmp, serde_json::to_string(file).unwrap_or_default()).is_ok() {
        let _ = fs::rename(&tmp, persist_path());
    }
}

/// Record already-extracted (card id, used percent) pairs in one write.
pub fn record_samples(samples: &[(String, f64)]) {
    if samples.is_empty() {
        return;
    }
    let today = today_string();
    let file = &mut *store().lock().unwrap();
    for (id, used) in samples {
        record_sample(&mut file.entries, id, &today, *used);
    }
    persist(file);
}

/// card id → 30-day trend, for the frontend's trend fallback.
pub fn trend_map() -> BTreeMap<String, Vec<f64>> {
    let today = today_string();
    let file = store().lock().unwrap();
    file.entries
        .keys()
        .map(|id| (id.clone(), trend_for(&file.entries, id, &today)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(kind: &str, used: Option<f64>) -> Metric {
        Metric {
            label: "Weekly".into(),
            kind: kind.into(),
            used_percent: used,
            detail: None,
            value: None,
            resets_at: None,
            period_ms: None,
        }
    }

    #[test]
    fn worst_used_takes_the_max_progress_row() {
        let metrics = vec![
            metric("progress", Some(12.0)),
            metric("text", Some(99.0)), // text rows never count
            metric("progress", Some(78.0)),
            metric("progress", None),
        ];
        assert_eq!(worst_used_percent(&metrics), Some(78.0));
        assert_eq!(worst_used_percent(&[]), None);
    }

    #[test]
    fn same_day_samples_keep_the_max() {
        let mut entries = BTreeMap::new();
        record_sample(&mut entries, "kimi@fp", "2026-09-01", 40.0);
        record_sample(&mut entries, "kimi@fp", "2026-09-01", 20.0); // window reset read
        record_sample(&mut entries, "kimi@fp", "2026-09-01", 65.0);
        assert_eq!(entries["kimi@fp"][0].used, 65.0);
    }

    #[test]
    fn prune_drops_samples_older_than_the_window() {
        let mut entries = BTreeMap::new();
        record_sample(&mut entries, "a", "2026-07-01", 10.0);
        record_sample(&mut entries, "a", "2026-08-20", 20.0);
        record_sample(&mut entries, "a", "2026-09-03", 30.0);
        prune_samples(entries.get_mut("a").unwrap(), "2026-09-03", KEEP_DAYS);
        let days: Vec<&str> = entries["a"].iter().map(|s| s.day.as_str()).collect();
        assert_eq!(days, ["2026-08-20", "2026-09-03"]); // 07-01 is > 35 days old
    }

    #[test]
    fn trend_spans_thirty_days_ending_today_with_gaps_as_zero() {
        let mut entries = BTreeMap::new();
        record_sample(&mut entries, "a", "2026-09-03", 55.0); // today
        record_sample(&mut entries, "a", "2026-08-05", 42.0); // 29 days back
        let trend = trend_for(&entries, "a", "2026-09-03");
        assert_eq!(trend.len(), 30);
        assert_eq!(trend[0], 42.0);
        assert_eq!(trend[1], 0.0);
        assert_eq!(trend[29], 55.0);
        assert!(trend_for(&entries, "missing", "2026-09-03").iter().all(|v| *v == 0.0));
    }
}
