//! Observed cumulative usage, kept per card, unit and independent meter.
//! Legacy credit_history.json rows migrate without changing their daily
//! samples. Unknown versions and unreadable history are never rewritten.

use crate::providers::Metric;
use crate::usage_units::{UsageReading, UsageUnit};
use chrono::{Duration, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

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
    /// Max absolute "credits used" reading recorded that day — a drain
    /// never un-happens within a day, even across fetches.
    used: f64,
    /// Latest reading of the day. A mid-day pool reset drops it below
    /// `used`; the trend then charges the post-reset segment on top of the
    /// pre-reset diff. Absent on credit files written before this field
    /// existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last: Option<f64>,
}

fn today_string() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

/// Credit variant of the day upsert: same-day keeps the max in `used` (a
/// drain never un-happens) but also remembers the latest reading, so a
/// mid-day pool reset stays visible to [`credit_trend_for`].
#[cfg(test)]
fn record_credit_sample(
    entries: &mut BTreeMap<String, Vec<DaySample>>,
    id: &str,
    day: &str,
    used: f64,
) {
    record_sample(entries.entry(id.to_string()).or_default(), day, used);
}

fn record_sample(samples: &mut Vec<DaySample>, day: &str, used: f64) {
    match samples.iter_mut().find(|s| s.day == day) {
        Some(existing) => {
            if used > existing.used {
                existing.used = used;
            }
            existing.last = Some(used);
        }
        None => {
            samples.push(DaySample {
                day: day.to_string(),
                used,
                last: Some(used),
            });
            samples.sort_by(|a, b| a.day.cmp(&b.day));
        }
    }
    prune_samples(samples, day, KEEP_DAYS);
}

/// Drop samples older than the retention window and cards left empty.
/// ISO dates compare correctly as strings.
fn prune_samples(samples: &mut Vec<DaySample>, today: &str, keep_days: i64) {
    let cutoff = parse_day(today).map(|today| {
        (today - Duration::days(keep_days))
            .format("%Y-%m-%d")
            .to_string()
    });
    let cutoff = cutoff.unwrap_or_default();
    samples.retain(|s| s.day >= cutoff);
}

fn parse_day(day: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

// ---------------------------------------------------------------------------
// Credits ledger store
// ---------------------------------------------------------------------------

const CREDIT_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Default, Clone)]
struct CreditFile {
    version: u32,
    /// card id → per-day absolute "credits used" readings, sorted by day.
    entries: BTreeMap<String, Vec<DaySample>>,
    #[serde(default)]
    meters: Vec<MeterSeries>,
}

#[derive(Serialize, Deserialize, Clone)]
struct MeterSeries {
    id: String,
    unit: UsageUnit,
    scope: String,
    #[serde(default)]
    shape: UsageShape,
    samples: Vec<DaySample>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum UsageShape {
    #[default]
    Cumulative,
    Daily,
}

struct CreditStore {
    file: Result<CreditFile, String>,
}

impl CreditStore {
    fn load(path: &std::path::Path) -> Self {
        let file = match fs::read_to_string(path) {
            Ok(raw) => (|| {
                let value: serde_json::Value =
                    serde_json::from_str(raw.trim_start_matches('\u{feff}'))
                        .map_err(|error| error.to_string())?;
                let version = value
                    .get("version")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("meter history has no version")?;
                if version > u64::from(CREDIT_VERSION) {
                    return Err(format!("unsupported meter history version {version}"));
                }
                let mut file: CreditFile =
                    serde_json::from_value(value).map_err(|error| error.to_string())?;
                for (id, samples) in std::mem::take(&mut file.entries) {
                    if !file.meters.iter().any(|meter| {
                        meter.id == id
                            && meter.unit == UsageUnit::Credits
                            && meter.scope.is_empty()
                            && meter.shape == UsageShape::Cumulative
                    }) {
                        file.meters.push(MeterSeries {
                            id,
                            unit: UsageUnit::Credits,
                            scope: String::new(),
                            shape: UsageShape::Cumulative,
                            samples,
                        });
                    }
                }
                file.version = CREDIT_VERSION;
                Ok(file)
            })(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(CreditFile {
                version: CREDIT_VERSION,
                ..CreditFile::default()
            }),
            Err(error) => Err(error.to_string()),
        };
        Self { file }
    }

    fn record(
        &mut self,
        path: &std::path::Path,
        day: &str,
        samples: &[(String, UsageReading)],
    ) -> Result<(), String> {
        let file = self.file.as_ref().map_err(Clone::clone)?;
        let mut next = file.clone();
        for (id, reading) in samples.iter().filter(|(_, reading)| reading.valid()) {
            let index = next.meters.iter().position(|meter| {
                meter.id == *id
                    && meter.unit == reading.unit
                    && meter.scope == reading.scope
                    && meter.shape == UsageShape::Cumulative
            });
            let index = index.unwrap_or_else(|| {
                next.meters.push(MeterSeries {
                    id: id.clone(),
                    unit: reading.unit,
                    scope: reading.scope.clone(),
                    shape: UsageShape::Cumulative,
                    samples: Vec::new(),
                });
                next.meters.len() - 1
            });
            record_sample(&mut next.meters[index].samples, day, reading.used);
        }
        persist_credits_at(path, &next)?;
        self.file = Ok(next);
        Ok(())
    }
}

/// Vendor daily buckets are already a flow measurement: each date owns its
/// amount, so they must not be differenced like cumulative counters.
pub fn record_daily_samples(samples: &[(String, crate::usage_units::UsageDays)]) {
    let mut store = credit_store().lock().unwrap_or_else(|e| e.into_inner());
    let Ok(file) = &store.file else {
        return;
    };
    let mut next = file.clone();
    for (id, days) in samples {
        upsert_daily_samples(&mut next, id, days);
    }
    prune_daily_meters(&mut next, &today_string());
    if let Err(error) = persist_credits_at(&credit_persist_path(), &next) {
        eprintln!("[pane] usage history: {error}");
    } else {
        store.file = Ok(next);
    }
}

/// Vendor daily buckets replace the day's value; the percent fallback is a
/// high-water mark and only ever moves up within a day.
fn upsert_daily_samples(file: &mut CreditFile, id: &str, days: &crate::usage_units::UsageDays) {
    if days.unit == UsageUnit::Percent {
        // One percent series per card: when the chosen bar is renamed, the
        // previous series stops being the quota we monitor. Keeping both
        // would leave meter_trends guessing which one is current.
        file.meters
            .retain(|meter| !(meter.id == id && meter.unit == UsageUnit::Percent && meter.scope != days.scope));
    }
    for (day, amount) in &days.days {
        if !amount.is_finite() || *amount < 0.0 {
            continue;
        }
        let index = file.meters.iter().position(|meter| {
            meter.id == id
                && meter.unit == days.unit
                && meter.scope == days.scope
                && meter.shape == UsageShape::Daily
        });
        let index = index.unwrap_or_else(|| {
            file.meters.push(MeterSeries {
                id: id.to_string(),
                unit: days.unit,
                scope: days.scope.clone(),
                shape: UsageShape::Daily,
                samples: Vec::new(),
            });
            file.meters.len() - 1
        });
        let samples = &mut file.meters[index].samples;
        match samples.iter_mut().find(|sample| sample.day == *day) {
            Some(sample) => {
                if days.unit == UsageUnit::Percent {
                    // Percent samples are a daily high-water mark, not a
                    // vendor bucket: a mid-day drop (window reset) must not
                    // erase the morning's peak.
                    sample.used = sample.used.max((*amount).clamp(0.0, 100.0));
                } else {
                    *sample = DaySample {
                        day: day.clone(),
                        used: *amount,
                        last: None,
                    };
                }
            }
            None => {
                samples.push(DaySample {
                    day: day.clone(),
                    used: if days.unit == UsageUnit::Percent {
                        (*amount).clamp(0.0, 100.0)
                    } else {
                        *amount
                    },
                    last: None,
                });
                samples.sort_by(|left, right| left.day.cmp(&right.day));
            }
        }
    }
}

fn prune_daily_meters(file: &mut CreditFile, today: &str) {
    for meter in file.meters.iter_mut().filter(|meter| meter.shape == UsageShape::Daily) {
        prune_samples(&mut meter.samples, today, KEEP_DAYS);
    }
}

fn credit_persist_path() -> PathBuf {
    crate::providers::config_dir().join("credit_history.json")
}

fn credit_store() -> &'static Mutex<CreditStore> {
    static STORE: OnceLock<Mutex<CreditStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(CreditStore::load(&credit_persist_path())))
}

fn persist_credits_at(path: &std::path::Path, file: &CreditFile) -> Result<(), String> {
    let text = serde_json::to_string(file).map_err(|error| error.to_string())?;
    crate::accounts::write_json_atomic(path, text)
}

/// Only validated structured observations can enter the persistent ledger.
pub fn readings_from_metrics(metrics: &[Metric]) -> Vec<UsageReading> {
    metrics
        .iter()
        .filter_map(|metric| metric.usage.clone())
        .filter(UsageReading::valid)
        .collect()
}

/// Record absolute readings in one write. Same-day keeps the max: a pool
/// reset reads lower and must not drag the baseline backwards mid-day.
pub fn record_meter_samples(samples: &[(String, UsageReading)]) {
    if samples.is_empty() {
        return;
    }
    let today = today_string();
    let mut store = credit_store().lock().unwrap_or_else(|e| e.into_inner());
    if let Err(error) = store.record(&credit_persist_path(), &today, samples) {
        eprintln!("[pane] usage history: {error}");
    }
}

/// One day's credit consumption for a credits-denominated card.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CreditDay {
    /// Local calendar day, "YYYY-MM-DD".
    pub day: String,
    /// Credits consumed that day (day-over-day diff of the absolute
    /// readings, mid-day resets included).
    pub consumed: f64,
}

/// Credits consumed inside each panel window. The windows mirror the token
/// side exactly — today / yesterday are local days, 7d and 30d are rolling
/// windows that include today, all is every recorded day — so the credits
/// rows and the token rows can never disagree on the same tab.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct CreditWindows {
    pub today: f64,
    pub yesterday: f64,
    pub d7: f64,
    pub d30: f64,
    pub all: f64,
}

/// Windowed credits for credits-billed cards (Qoder CN, WorkBuddy, …).
/// Credits cannot be converted into tokens or dollars honestly, so the spend
/// panel gives them their own rows instead of blending them into the
/// token/dollar totals.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CreditExtra {
    pub id: String,
    pub unit: UsageUnit,
    pub windows: CreditWindows,
}

/// Day-over-day credit consumption for one card, oldest first. The first
/// reading is a cumulative figure with no baseline — it produces nothing
/// (showing it as a daily number would invent data).
fn credit_day_series(samples: &[DaySample]) -> Vec<CreditDay> {
    let mut days: Vec<CreditDay> = Vec::new();
    let mut prev: Option<f64> = None;
    for sample in samples {
        let end = sample.last.unwrap_or(sample.used);
        if let Some(p) = prev {
            // Day-over-day diff; a mid-day reset shows as `last < used`, so
            // the post-reset segment is charged on top.
            let consumed = (sample.used - p).max(0.0) + if end < sample.used { end } else { 0.0 };
            days.push(CreditDay {
                day: sample.day.clone(),
                consumed,
            });
        }
        prev = Some(end);
    }
    days
}

/// Sum the recorded per-day consumption inside each panel window. `today`
/// comes from the same local-day source as `credit_trend_map`, so both
/// surfaces agree on where a window starts.
fn credit_window_totals(days: &[CreditDay], today: &str) -> CreditWindows {
    let Some(today) = parse_day(today) else {
        return CreditWindows::default();
    };
    let mut out = CreditWindows::default();
    for day in days {
        let Some(date) = parse_day(&day.day) else {
            continue;
        };
        out.all += day.consumed;
        let offset = (today - date).num_days();
        // A reading dated after "today" (clock skew, hand-edited file) counts
        // only in the whole-ledger window.
        if offset < 0 {
            continue;
        }
        if offset == 0 {
            out.today += day.consumed;
        }
        if offset == 1 {
            out.yesterday += day.consumed;
        }
        if offset < 7 {
            out.d7 += day.consumed;
        }
        if offset < TREND_DAYS as i64 {
            out.d30 += day.consumed;
        }
    }
    out
}

pub fn credit_extras() -> Vec<CreditExtra> {
    let today = today_string();
    let store = credit_store().lock().unwrap_or_else(|e| e.into_inner());
    let Ok(file) = &store.file else {
        return Vec::new();
    };
    credit_extras_for(file, &today)
}

fn credit_extras_for(file: &CreditFile, today: &str) -> Vec<CreditExtra> {
    let mut out: Vec<CreditExtra> = Vec::new();
    // Percent samples are daily peaks, not additive quantities: summing them
    // over a 7-day window would report "400% used". They belong in the card
    // trend (meter_trends), never in the measured-usage totals.
    for meter in file.meters.iter().filter(|meter| meter.unit != UsageUnit::Percent) {
        let days = if meter.shape == UsageShape::Daily {
            meter
                .samples
                .iter()
                .map(|sample| CreditDay {
                    day: sample.day.clone(),
                    consumed: sample.used,
                })
                .collect()
        } else {
            credit_day_series(&meter.samples)
        };
        let windows = credit_window_totals(&days, today);
        if let Some(existing) = out
            .iter_mut()
            .find(|row| row.id == meter.id && row.unit == meter.unit)
        {
            existing.windows.today += windows.today;
            existing.windows.yesterday += windows.yesterday;
            existing.windows.d7 += windows.d7;
            existing.windows.d30 += windows.d30;
            existing.windows.all += windows.all;
        } else {
            out.push(CreditExtra {
                id: meter.id.clone(),
                unit: meter.unit,
                windows,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// 30-day daily consumption for one card: each observed day reads its diff
/// against the previous observed day (even across gaps — consumption during
/// an unobserved stretch lands whole on the next observed day, never
/// spread). A reading drop means the pool reset: that day shows 0. The
/// first observed day has no baseline and stays None ("no data").
fn credit_trend_for(
    entries: &BTreeMap<String, Vec<DaySample>>,
    id: &str,
    today: &str,
) -> Vec<Option<f64>> {
    let Some(today) = parse_day(today) else {
        return vec![None; TREND_DAYS];
    };
    let Some(samples) = entries.get(id) else {
        return vec![None; TREND_DAYS];
    };
    let first_day = today - Duration::days(TREND_DAYS as i64 - 1);
    // Baseline may live just before the window — the first in-window day
    // still gets an honest diff.
    let mut prev: Option<f64> = samples
        .iter()
        .filter(|s| parse_day(&s.day).is_some_and(|d| d < first_day))
        .last()
        .map(|s| s.last.unwrap_or(s.used));
    (0..TREND_DAYS as i64)
        .map(|i| {
            let day = (today - Duration::days(TREND_DAYS as i64 - 1 - i))
                .format("%Y-%m-%d")
                .to_string();
            match samples.iter().find(|s| s.day == day) {
                Some(sample) => {
                    let end = sample.last.unwrap_or(sample.used);
                    // A mid-day reset shows as `last < used`: charge the
                    // pre-reset diff plus the post-reset reading (the pool
                    // restarts at zero, so `end` IS that segment).
                    let consumed = prev.map(|p| {
                        (sample.used - p).max(0.0) + if end < sample.used { end } else { 0.0 }
                    });
                    prev = Some(end);
                    consumed
                }
                None => None,
            }
        })
        .collect()
}

/// Latest local day with a positive recorded reading per card id, as Unix ms
/// at local noon. A zero-only history is unknown, not "used today" — callers
/// must render the absence instead of inventing a date.
pub fn last_used_map() -> BTreeMap<String, i64> {
    let store = credit_store().lock().unwrap_or_else(|e| e.into_inner());
    let Ok(file) = &store.file else {
        return BTreeMap::new();
    };
    last_used_map_for(file)
}

fn last_used_map_for(file: &CreditFile) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    // Percent samples are quota occupancy, not proof of activity: a stored
    // 24% weekly bar would read as "used today" every day it is refreshed.
    for meter in file.meters.iter().filter(|meter| meter.unit != UsageUnit::Percent) {
        let latest = match meter.shape {
            UsageShape::Daily => latest_used_day(&meter.samples),
            UsageShape::Cumulative => latest_cumulative_use_day(&meter.samples),
        };
        let Some(day) = latest else {
            continue;
        };
        let Some(timestamp) = day_noon_ms(day) else {
            continue;
        };
        out.entry(meter.id.clone())
            .and_modify(|current: &mut i64| *current = (*current).max(timestamp))
            .or_insert(timestamp);
    }
    out
}

fn latest_used_day(samples: &[DaySample]) -> Option<&str> {
    samples
        .iter()
        .filter(|sample| sample.used > 0.0 || sample.last.is_some_and(|value| value > 0.0))
        .map(|sample| sample.day.as_str())
        .max()
}

/// Cumulative meters store the day's max absolute reading, so a day where
/// the reading did not move is not usage. Diff consecutive days and treat a
/// drop as a reset (the new lower reading is post-reset consumption).
fn latest_cumulative_use_day(samples: &[DaySample]) -> Option<&str> {
    let mut latest = None;
    let mut previous: Option<f64> = None;
    for sample in samples {
        let consumed = match previous {
            Some(prev) if sample.used >= prev => sample.used - prev,
            Some(_) => sample.used,
            None => sample.used,
        };
        if consumed > 0.0 {
            latest = Some(sample.day.as_str());
        }
        previous = Some(sample.used);
    }
    latest
}

fn day_noon_ms(day: &str) -> Option<i64> {
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    let naive = date.and_hms_opt(12, 0, 0)?;
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|value| value.timestamp_millis())
}

/// card id → 30-day credit-consumption trend, for the frontend.
pub fn credit_trend_map() -> BTreeMap<String, Vec<Option<f64>>> {
    meter_trends()
        .into_iter()
        .filter(|trend| trend.unit == UsageUnit::Credits)
        .map(|trend| (trend.id, trend.trend))
        .collect()
}

#[derive(Serialize)]
pub struct MeterTrend {
    pub id: String,
    pub unit: UsageUnit,
    pub trend: Vec<Option<f64>>,
}

pub fn meter_trends() -> Vec<MeterTrend> {
    let today = today_string();
    let store = credit_store().lock().unwrap_or_else(|e| e.into_inner());
    let Ok(file) = &store.file else {
        return Vec::new();
    };
    meter_trends_for(file, &today)
}

fn meter_trends_for(file: &CreditFile, today: &str) -> Vec<MeterTrend> {
    let mut out: Vec<MeterTrend> = Vec::new();
    for meter in file.meters.iter().filter(|meter| meter.unit != UsageUnit::Percent) {
        let entries = BTreeMap::from([(meter.id.clone(), meter.samples.clone())]);
        let trend = if meter.shape == UsageShape::Daily {
            credit_trend_for_daily(&meter.samples, today)
        } else {
            credit_trend_for(&entries, &meter.id, today)
        };
        if let Some(existing) = out
            .iter_mut()
            .find(|row| row.id == meter.id && row.unit == meter.unit)
        {
            for (combined, value) in existing.trend.iter_mut().zip(trend) {
                *combined = match (*combined, value) {
                    (Some(left), Some(right)) => Some(left + right),
                    (Some(value), None) | (None, Some(value)) => Some(value),
                    (None, None) => None,
                };
            }
        } else {
            out.push(MeterTrend {
                id: meter.id.clone(),
                unit: meter.unit,
                trend,
            });
        }
    }
    // One percent series per card: a renamed bar starts a new scope and the
    // old one stops updating. Show the series with the newest sample (then
    // the longest one) rather than summing unrelated percentages.
    fn newest_day(meter: &MeterSeries) -> Option<&str> {
        meter.samples.last().map(|sample| sample.day.as_str())
    }
    let mut best_percent: BTreeMap<&str, &MeterSeries> = BTreeMap::new();
    for meter in file.meters.iter().filter(|meter| meter.unit == UsageUnit::Percent) {
        let replace = match best_percent.get(meter.id.as_str()) {
            None => true,
            Some(current) => {
                newest_day(meter) > newest_day(current)
                    || (newest_day(meter) == newest_day(current) && meter.samples.len() > current.samples.len())
            }
        };
        if replace {
            best_percent.insert(meter.id.as_str(), meter);
        }
    }
    for (id, meter) in best_percent {
        out.push(MeterTrend {
            id: id.to_string(),
            unit: UsageUnit::Percent,
            trend: credit_trend_for_daily(&meter.samples, today),
        });
    }
    out
}

fn credit_trend_for_daily(samples: &[DaySample], today: &str) -> Vec<Option<f64>> {
    let Some(today) = parse_day(today) else {
        return vec![None; TREND_DAYS];
    };
    (0..TREND_DAYS as i64)
        .map(|offset| {
            let day = (today - Duration::days(TREND_DAYS as i64 - 1 - offset))
                .format("%Y-%m-%d")
                .to_string();
            samples
                .iter()
                .find(|sample| sample.day == day)
                .map(|sample| sample.used)
        })
        .collect()
}

/// Move history when a multi-account family changes the identity published by
/// its bare card. Existing samples are merged instead of overwritten.
pub fn migrate_card_id(from: &str, to: &str) {
    if from == to {
        return;
    }
    let mut store = credit_store().lock().unwrap_or_else(|e| e.into_inner());
    let Ok(file) = &store.file else {
        return;
    };
    let mut next = file.clone();
    let moving: Vec<_> = next
        .meters
        .iter()
        .filter(|meter| meter.id == from)
        .cloned()
        .collect();
    next.meters.retain(|meter| meter.id != from);
    for mut meter in moving {
        if let Some(target) = next.meters.iter_mut().find(|target| {
            target.id == to && target.unit == meter.unit && target.scope == meter.scope
        }) {
            for sample in meter.samples {
                if let Some(existing) = target
                    .samples
                    .iter_mut()
                    .find(|item| item.day == sample.day)
                {
                    if sample.used > existing.used {
                        *existing = sample;
                    }
                } else {
                    target.samples.push(sample);
                }
            }
            target
                .samples
                .sort_by(|left, right| left.day.cmp(&right.day));
        } else {
            meter.id = to.to_string();
            next.meters.push(meter);
        }
    }
    if persist_credits_at(&credit_persist_path(), &next).is_ok() {
        store.file = Ok(next);
    }
}

/// Remove history for a card when deleted without keeping usage data.
pub fn remove_card_history(id: &str) {
    let mut store = credit_store().lock().unwrap_or_else(|e| e.into_inner());
    let Ok(file) = &store.file else {
        return;
    };
    let mut next = file.clone();
    next.meters.retain(|meter| meter.id != id);
    if persist_credits_at(&credit_persist_path(), &next).is_ok() {
        store.file = Ok(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_credit_reading_does_not_depend_on_display_copy() {
        let metric: Metric = serde_json::from_value(serde_json::json!({
            "label": "Renamed pool", "kind": "text", "value": "任意展示文案",
            "usage": { "unit": "credits", "used": 38.5, "total": 2000.0 }
        }))
        .unwrap();
        assert_eq!(readings_from_metrics(&[metric])[0].used, 38.5);
    }

    #[test]
    fn unknown_history_version_is_not_replaced_when_recording() {
        let directory =
            std::env::temp_dir().join(format!("pane-meter-future-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("history.json");
        let future = r#"{"version":999,"entries":{"opaque":"future representation"}}"#;
        fs::write(&path, future).unwrap();
        let mut store = CreditStore::load(&path);
        assert!(store.record(&path, "2026-10-17", &[]).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), future);
    }

    #[test]
    fn corrupt_history_is_preserved_and_failed_save_does_not_publish_samples() {
        let directory =
            std::env::temp_dir().join(format!("pane-meter-failure-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("history.json");
        fs::write(&path, "{unfinished").unwrap();
        let mut store = CreditStore::load(&path);
        assert!(store.record(&path, "2026-10-17", &[]).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{unfinished");
        let mut fresh = CreditStore {
            file: Ok(CreditFile {
                version: CREDIT_VERSION,
                ..CreditFile::default()
            }),
        };
        let impossible = directory.join("missing").join("history.json");
        assert!(fresh
            .record(
                &impossible,
                "2026-10-17",
                &[(
                    "a".into(),
                    UsageReading {
                        unit: UsageUnit::Credits,
                        used: 5.0,
                        total: None,
                        scope: String::new(),
                    }
                )]
            )
            .is_err());
        assert!(fresh.file.unwrap().meters.is_empty());
    }

    #[test]
    fn daily_meter_samples_use_the_same_retention_window_as_cumulative_meters() {
        let mut file = CreditFile {
            version: CREDIT_VERSION,
            meters: vec![MeterSeries {
                id: "giphy".into(),
                unit: UsageUnit::Requests,
                scope: "app:1".into(),
                shape: UsageShape::Daily,
                samples: vec![
                    DaySample { day: "2026-08-01".into(), used: 1.0, last: None },
                    DaySample { day: "2026-09-20".into(), used: 2.0, last: None },
                    DaySample { day: "2026-10-18".into(), used: 3.0, last: None },
                ],
            }],
            ..CreditFile::default()
        };
        prune_daily_meters(&mut file, "2026-10-18");
        assert_eq!(
            file.meters[0].samples.iter().map(|sample| sample.day.as_str()).collect::<Vec<_>>(),
            vec!["2026-09-20", "2026-10-18"]
        );
    }

    #[test]
    fn legacy_history_migrates_without_erasing_samples_and_records_units_separately() {
        let directory =
            std::env::temp_dir().join(format!("pane-meter-migrate-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("history.json");
        fs::write(
            &path,
            r#"{"version":1,"entries":{"qodercn":[{"day":"2026-10-16","used":100}]}}"#,
        )
        .unwrap();
        let mut store = CreditStore::load(&path);
        store
            .record(
                &path,
                "2026-10-17",
                &[
                    (
                        "qodercn".into(),
                        UsageReading {
                            unit: UsageUnit::Credits,
                            used: 138.0,
                            total: None,
                            scope: String::new(),
                        },
                    ),
                    (
                        "shandianshuo".into(),
                        UsageReading {
                            unit: UsageUnit::Minutes,
                            used: 5.5,
                            total: None,
                            scope: String::new(),
                        },
                    ),
                    (
                        "shandianshuo".into(),
                        UsageReading {
                            unit: UsageUnit::Credits,
                            used: 70.0,
                            total: None,
                            scope: String::new(),
                        },
                    ),
                ],
            )
            .unwrap();
        let reloaded = CreditStore::load(&path);
        let file = reloaded.file.unwrap();
        assert_eq!(file.version, CREDIT_VERSION);
        assert!(file.entries.is_empty());
        assert_eq!(file.meters.len(), 3);
        let qoder = file
            .meters
            .iter()
            .find(|meter| meter.id == "qodercn")
            .unwrap();
        assert_eq!(credit_day_series(&qoder.samples)[0].consumed, 38.0);
        assert_eq!(
            file.meters
                .iter()
                .find(|meter| meter.unit == UsageUnit::Minutes)
                .unwrap()
                .samples[0]
                .used,
            5.5
        );
    }

    #[test]
    fn renamed_key_scope_keeps_its_baseline_and_independent_keys_do_not_cross() {
        let directory =
            std::env::temp_dir().join(format!("pane-meter-scopes-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("history.json");
        let mut store = CreditStore {
            file: Ok(CreditFile {
                version: CREDIT_VERSION,
                ..CreditFile::default()
            }),
        };
        let reading = |scope: &str, used| {
            (
                "tavily".into(),
                UsageReading {
                    unit: UsageUnit::Credits,
                    used,
                    total: None,
                    scope: scope.into(),
                },
            )
        };
        store
            .record(
                &path,
                "2026-10-16",
                &[
                    reading("key-fingerprint-a", 100.0),
                    reading("key-fingerprint-b", 900.0),
                ],
            )
            .unwrap();
        store
            .record(
                &path,
                "2026-10-17",
                &[
                    reading("key-fingerprint-a", 130.0),
                    reading("key-fingerprint-b", 905.0),
                ],
            )
            .unwrap();
        let file = store.file.unwrap();
        assert_eq!(file.meters.len(), 2);
        let total: f64 = file
            .meters
            .iter()
            .map(|meter| credit_day_series(&meter.samples)[0].consumed)
            .sum();
        assert_eq!(total, 35.0);
    }

    fn metric(kind: &str, used: Option<f64>) -> Metric {
        Metric {
            label: "Weekly".into(),
            kind: kind.into(),
            used_percent: used,
            detail: None,
            value: None,
            resets_at: None,
            period_ms: None,
            available: None,
            usage: None,
            daily_usage: None,
            pace: None,
        }
    }

    fn credit_sample(day: &str, used: f64, last: Option<f64>) -> DaySample {
        DaySample {
            day: day.into(),
            used,
            last,
        }
    }

    #[test]
    #[ignore = "real-machine anchor: reads this machine's credit_history.json"]
    fn credit_extras_real_machine() {
        let extras = credit_extras();
        // A single stored reading has no baseline, so every window reads zero
        // rather than echoing the cumulative number as one day's spend. On
        // this machine Qoder CN is exactly that case — zero here is "no
        // data", not a regression.
        let qoder = extras.iter().find(|e| e.id.starts_with("qodercn"));
        assert!(qoder.is_some(), "qoder row missing entirely: {extras:?}");
        assert_eq!(
            qoder.unwrap().windows,
            CreditWindows::default(),
            "a lone reading must not produce a window total: {extras:?}"
        );
        assert!(
            extras
                .iter()
                .all(|e| e.windows.all >= 0.0 && e.windows.today >= 0.0 && e.windows.d30 >= 0.0),
            "credits windows malformed: {extras:?}"
        );
    }

    #[test]
    fn credit_day_series_needs_a_baseline() {
        // The first reading is cumulative: no baseline means no day to charge.
        let days = credit_day_series(&[credit_sample("2026-10-10", 2500.0, None)]);
        assert!(days.is_empty(), "a lone reading produced a day: {days:?}");
    }

    #[test]
    fn credit_day_series_diffs_observed_days() {
        let days = credit_day_series(&[
            credit_sample("2026-10-10", 100.0, None),
            credit_sample("2026-10-11", 130.0, None),
            credit_sample("2026-10-12", 150.0, None),
        ]);
        assert_eq!(
            days.len(),
            2,
            "one day per reading after the first: {days:?}"
        );
        assert_eq!(days[0].consumed, 30.0);
        assert_eq!(days[1].consumed, 20.0);
    }

    #[test]
    fn credit_day_series_charges_a_mid_day_reset_on_top() {
        // used jumped 100 → 150 while `last` fell to 20: the pool reset
        // mid-day, so the day consumed the pre-reset diff (50) plus the
        // post-reset reading (20).
        let days = credit_day_series(&[
            credit_sample("2026-10-10", 100.0, None),
            credit_sample("2026-10-11", 150.0, Some(20.0)),
        ]);
        assert_eq!(days[0].consumed, 70.0);
    }

    #[test]
    fn credit_window_totals_sum_each_window() {
        let days: Vec<CreditDay> = [
            ("2026-10-20", 5.0), // today
            ("2026-10-19", 3.0), // yesterday
            ("2026-10-14", 2.0), // 6 days back — inside 7d
            ("2026-10-13", 9.0), // 7 days back — outside 7d, inside 30d
            ("2026-09-21", 4.0), // 29 days back — inside 30d
            ("2026-09-20", 7.0), // 30 days back — whole ledger only
        ]
        .iter()
        .map(|(day, consumed)| CreditDay {
            day: (*day).into(),
            consumed: *consumed,
        })
        .collect();
        let w = credit_window_totals(&days, "2026-10-20");
        assert_eq!(w.today, 5.0);
        assert_eq!(w.yesterday, 3.0);
        assert_eq!(w.d7, 10.0, "today + yesterday + 6 days back");
        assert_eq!(w.d30, 23.0, "everything within 30 days of today");
        assert_eq!(w.all, 30.0, "every recorded day");
    }

    #[test]
    fn credit_window_totals_are_zero_until_a_baseline_exists() {
        // Qoder CN's real shape: one stored reading, no baseline. Every
        // window reads zero — the row disappears, and that is "no data",
        // not a regression.
        let days = credit_day_series(&[credit_sample("2026-10-10", 2500.0, None)]);
        assert_eq!(
            credit_window_totals(&days, "2026-10-14"),
            CreditWindows::default()
        );
    }

    #[test]
    fn credit_windows_stay_per_account() {
        // Two accounts of the same provider are summed apart: a flat account
        // contributes nothing while its sibling's consumption shows.
        let a = credit_day_series(&[
            credit_sample("2026-10-19", 10.0, None),
            credit_sample("2026-10-20", 25.0, None),
        ]);
        let b = credit_day_series(&[
            credit_sample("2026-10-19", 500.0, None),
            credit_sample("2026-10-20", 500.0, None),
        ]);
        let wa = credit_window_totals(&a, "2026-10-20");
        let wb = credit_window_totals(&b, "2026-10-20");
        assert_eq!(wa.today, 15.0);
        assert_eq!(wa.all, 15.0);
        assert_eq!(wb.today, 0.0, "a flat account consumes nothing");
        assert_eq!(wb.all, 0.0);
    }

    #[test]
    fn prune_drops_samples_older_than_the_window() {
        let mut entries = BTreeMap::new();
        record_credit_sample(&mut entries, "a", "2026-07-01", 10.0);
        record_credit_sample(&mut entries, "a", "2026-08-20", 20.0);
        record_credit_sample(&mut entries, "a", "2026-09-03", 30.0);
        prune_samples(entries.get_mut("a").unwrap(), "2026-09-03", KEEP_DAYS);
        let days: Vec<&str> = entries["a"].iter().map(|s| s.day.as_str()).collect();
        assert_eq!(days, ["2026-08-20", "2026-09-03"]); // 07-01 is > 35 days old
    }

    #[test]
    fn same_day_credit_samples_keep_the_max_and_latest() {
        let mut entries = BTreeMap::new();
        record_credit_sample(&mut entries, "qodercn", "2026-09-04", 900.0);
        record_credit_sample(&mut entries, "qodercn", "2026-09-04", 5.0); // pool reset mid-day
        let sample = &entries["qodercn"][0];
        assert_eq!(sample.used, 900.0, "the drain never un-happens");
        assert_eq!(
            sample.last,
            Some(5.0),
            "the latest reading survives for the diff"
        );
    }

    #[test]
    fn credits_row_uses_explicit_reading_and_rejects_display_lookalikes() {
        let credits = Metric {
            label: "Credits".into(),
            kind: "progress".into(),
            used_percent: Some(1.9),
            detail: Some("38 of 2000 credits used".into()),
            value: None,
            resets_at: None,
            period_ms: None,
            available: None,
            usage: Some(UsageReading {
                unit: UsageUnit::Credits,
                used: 38.0,
                total: Some(2000.0),
                scope: String::new(),
            }),
            daily_usage: None,
            pace: None,
        };
        assert_eq!(readings_from_metrics(&[credits])[0].used, 38.0);
        // A "Credits" label without the credits detail is not a credit row.
        let lookalike = Metric {
            label: "Credits".into(),
            kind: "progress".into(),
            used_percent: Some(40.0),
            detail: Some("40% used".into()),
            value: None,
            resets_at: None,
            period_ms: None,
            available: None,
            usage: None,
            daily_usage: None,
            pace: None,
        };
        assert!(readings_from_metrics(&[lookalike]).is_empty());
        assert!(readings_from_metrics(&[metric("progress", Some(10.0))]).is_empty());
    }

    #[test]
    fn credit_trend_diffs_against_previous_observation() {
        let mut entries = BTreeMap::new();
        record_credit_sample(&mut entries, "qodercn", "2026-09-01", 100.0);
        record_credit_sample(&mut entries, "qodercn", "2026-09-02", 138.0); // +38
                                                                            // 09-03 not observed; the gap's consumption lands whole on 09-04.
        record_credit_sample(&mut entries, "qodercn", "2026-09-04", 200.0); // +62
        let trend = credit_trend_for(&entries, "qodercn", "2026-09-04");
        assert_eq!(trend[26], None); // first observed day: no baseline
        assert_eq!(trend[27], Some(38.0));
        assert_eq!(trend[28], None); // not observed
        assert_eq!(trend[29], Some(62.0)); // gap consumption not spread
    }

    #[test]
    fn credit_trend_uses_pre_window_baseline_and_zeroes_resets() {
        let mut entries = BTreeMap::new();
        record_credit_sample(&mut entries, "qodercn", "2026-08-01", 500.0); // before window
        record_credit_sample(&mut entries, "qodercn", "2026-09-03", 550.0); // today-1: +50
        record_credit_sample(&mut entries, "qodercn", "2026-09-04", 10.0); // reset → 0
        let trend = credit_trend_for(&entries, "qodercn", "2026-09-04");
        assert_eq!(trend[28], Some(50.0));
        assert_eq!(trend[29], Some(0.0));
    }

    #[test]
    fn credit_trend_charges_both_segments_of_a_midday_reset() {
        let mut entries = BTreeMap::new();
        record_credit_sample(&mut entries, "qodercn", "2026-09-03", 890.0);
        // Same day: drain to 900, pool resets, drain 5 more.
        record_credit_sample(&mut entries, "qodercn", "2026-09-04", 900.0);
        record_credit_sample(&mut entries, "qodercn", "2026-09-04", 5.0);
        record_credit_sample(&mut entries, "qodercn", "2026-09-05", 40.0); // +35 off the 5
        let trend = credit_trend_for(&entries, "qodercn", "2026-09-05");
        assert_eq!(trend[28], Some(15.0)); // (900-890) + 5 post-reset
        assert_eq!(trend[29], Some(35.0)); // baseline is the 5, not the 900
    }

    #[test]
    fn pre_window_baseline_uses_the_latest_post_reset_reading() {
        let mut entries = BTreeMap::new();
        record_credit_sample(&mut entries, "qodercn", "2026-09-30", 900.0);
        record_credit_sample(&mut entries, "qodercn", "2026-09-30", 5.0);
        record_credit_sample(&mut entries, "qodercn", "2026-10-01", 40.0);
        assert_eq!(
            credit_trend_for(&entries, "qodercn", "2026-10-30")[0],
            Some(35.0)
        );
    }

    #[test]
    fn last_used_day_ignores_zero_only_days_and_takes_the_latest_positive() {
        let samples = vec![
            DaySample { day: "2026-10-01".into(), used: 0.0, last: Some(0.0) },
            DaySample { day: "2026-10-02".into(), used: 12.0, last: Some(12.0) },
            DaySample { day: "2026-10-03".into(), used: 0.0, last: None },
        ];
        assert_eq!(latest_used_day(&samples), Some("2026-10-02"));
        assert!(day_noon_ms("2026-10-02").is_some());
        assert_eq!(latest_used_day(&samples[..1]), None);
    }

    #[test]
    fn cumulative_last_used_ignores_flat_days_and_counts_the_reset() {
        let samples = vec![
            DaySample { day: "2026-10-01".into(), used: 100.0, last: Some(100.0) },
            DaySample { day: "2026-10-02".into(), used: 100.0, last: Some(100.0) },
            DaySample { day: "2026-10-03".into(), used: 0.0, last: Some(0.0) },
            DaySample { day: "2026-10-04".into(), used: 5.0, last: Some(5.0) },
        ];
        assert_eq!(latest_cumulative_use_day(&samples), Some("2026-10-04"));
        assert_eq!(latest_cumulative_use_day(&samples[..2]), Some("2026-10-01"));
        let flat = vec![
            DaySample { day: "2026-10-01".into(), used: 0.0, last: Some(0.0) },
            DaySample { day: "2026-10-02".into(), used: 0.0, last: Some(0.0) },
        ];
        assert_eq!(latest_cumulative_use_day(&flat), None);
    }

    #[test]
    fn vendor_daily_flow_is_kept_verbatim_and_is_not_differenced() {
        let directory =
            std::env::temp_dir().join(format!("pane-meter-flow-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("history.json");
        let mut store = CreditStore {
            file: Ok(CreditFile {
                version: CREDIT_VERSION,
                ..CreditFile::default()
            }),
        };
        let days = crate::usage_units::UsageDays {
            unit: UsageUnit::Requests,
            scope: "app:7".into(),
            days: BTreeMap::from([("2026-10-15".into(), 4.0), ("2026-10-16".into(), 9.0)]),
        };
        store.file.as_mut().unwrap().meters.push(MeterSeries {
            id: "giphy".into(),
            unit: days.unit,
            scope: days.scope.clone(),
            shape: UsageShape::Daily,
            samples: days
                .days
                .iter()
                .map(|(day, used)| DaySample {
                    day: day.clone(),
                    used: *used,
                    last: None,
                })
                .collect(),
        });
        persist_credits_at(&path, store.file.as_ref().unwrap()).unwrap();
        let reloaded = CreditStore::load(&path).file.unwrap();
        let trend = credit_trend_for_daily(&reloaded.meters[0].samples, "2026-10-16");
        assert_eq!(trend[28], Some(4.0));
        assert_eq!(trend[29], Some(9.0));
    }

    #[test]
    fn percent_samples_keep_the_day_peak_when_a_quota_resets_midday() {
        let mut file = CreditFile {
            version: CREDIT_VERSION,
            ..CreditFile::default()
        };
        let sample = |scope: &str, value: f64| crate::usage_units::UsageDays {
            unit: UsageUnit::Percent,
            scope: scope.into(),
            days: BTreeMap::from([("2026-10-24".into(), value)]),
        };
        upsert_daily_samples(&mut file, "minimax", &sample("Session", 66.0));
        upsert_daily_samples(&mut file, "minimax", &sample("Session", 18.0));
        assert_eq!(file.meters[0].samples[0].used, 66.0);
        upsert_daily_samples(&mut file, "minimax", &sample("Session", 81.0));
        assert_eq!(file.meters[0].samples[0].used, 81.0);
        assert!(matches!(file.meters[0].shape, UsageShape::Daily));
        // A renamed bar is a different quota: it replaces the old series
        // instead of leaving two percent rows for the same card.
        upsert_daily_samples(&mut file, "minimax", &sample("Weekly", 30.0));
        assert_eq!(file.meters.len(), 1);
        assert_eq!(file.meters[0].scope, "Weekly");
        assert_eq!(file.meters[0].samples[0].used, 30.0);
    }

    #[test]
    fn percent_trends_pick_the_newest_scope_instead_of_summing_bars() {
        let series = |scope: &str, days: &[(&str, f64)]| MeterSeries {
            id: "brave".into(),
            unit: UsageUnit::Percent,
            scope: scope.into(),
            shape: UsageShape::Daily,
            samples: days
                .iter()
                .map(|(day, used)| DaySample {
                    day: (*day).into(),
                    used: *used,
                    last: None,
                })
                .collect(),
        };
        let file = CreditFile {
            version: CREDIT_VERSION,
            meters: vec![
                series("Monthly", &[("2026-10-20", 10.0), ("2026-10-21", 11.0)]),
                series("Weekly", &[("2026-10-22", 40.0), ("2026-10-23", 45.0)]),
            ],
            ..CreditFile::default()
        };
        let trends = meter_trends_for(&file, "2026-10-23");
        let percent: Vec<_> = trends.iter().filter(|row| row.unit == UsageUnit::Percent).collect();
        assert_eq!(percent.len(), 1, "one percent series per card, never summed");
        assert_eq!(percent[0].trend[29], Some(45.0));
    }

    #[test]
    fn percent_samples_stay_out_of_credit_totals_and_last_used() {
        let daily = |id: &str, unit: UsageUnit, scope: &str, shape: UsageShape, samples: &[(&str, f64)]| MeterSeries {
            id: id.into(),
            unit,
            scope: scope.into(),
            shape,
            samples: samples
                .iter()
                .map(|(day, used)| DaySample {
                    day: (*day).into(),
                    used: *used,
                    last: Some(*used),
                })
                .collect(),
        };
        let file = CreditFile {
            version: CREDIT_VERSION,
            meters: vec![
                daily("minimax", UsageUnit::Percent, "period:18000000", UsageShape::Daily, &[("2026-10-24", 66.0)]),
                daily(
                    "workbuddy",
                    UsageUnit::Credits,
                    "",
                    UsageShape::Cumulative,
                    &[("2026-10-23", 0.0), ("2026-10-24", 5.0)],
                ),
            ],
            ..CreditFile::default()
        };
        let extras = credit_extras_for(&file, "2026-10-24");
        assert_eq!(extras.len(), 1, "percent must not become a measured-usage row");
        assert_eq!(extras[0].id, "workbuddy");
        assert_eq!(extras[0].windows.today, 5.0);
        let last = last_used_map_for(&file);
        assert!(!last.contains_key("minimax"), "quota occupancy is not activity");
        assert!(last.contains_key("workbuddy"));
        let trends = meter_trends_for(&file, "2026-10-24");
        assert!(trends.iter().any(|row| row.id == "minimax" && row.unit == UsageUnit::Percent));
    }
}
