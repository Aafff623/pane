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
    /// Latest reading of the day (credit stores only). A mid-day pool reset
    /// drops it below `used`; the trend then charges the post-reset segment
    /// on top of the pre-reset diff. Absent on percent history and on credit
    /// files written before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last: Option<f64>,
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
                last: None,
            });
            samples.sort_by(|a, b| a.day.cmp(&b.day));
        }
    }
    prune_samples(samples, day, KEEP_DAYS);
}

/// Credit variant of [`record_sample`]: same-day keeps the max in `used` (a
/// drain never un-happens) but also remembers the latest reading, so a
/// mid-day pool reset stays visible to [`credit_trend_for`].
fn record_credit_sample(
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
    let cutoff = parse_day(today)
        .map(|today| (today - Duration::days(keep_days)).format("%Y-%m-%d").to_string());
    let cutoff = cutoff.unwrap_or_default();
    samples.retain(|s| s.day >= cutoff);
}

fn parse_day(day: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

/// The 30-value trend for one card, oldest first, today last. Days without
/// a sample read None — "no data" is distinct from a sampled 0.
fn trend_for(
    entries: &BTreeMap<String, Vec<DaySample>>,
    id: &str,
    today: &str,
) -> Vec<Option<f64>> {
    let Some(today) = parse_day(today) else {
        return vec![None; TREND_DAYS];
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
pub fn trend_map() -> BTreeMap<String, Vec<Option<f64>>> {
    let today = today_string();
    let file = store().lock().unwrap();
    file.entries
        .keys()
        .map(|id| (id.clone(), trend_for(&file.entries, id, &today)))
        .collect()
}

/// Move history when a multi-account family changes the identity published by
/// its bare card. Existing samples are merged instead of overwritten.
pub fn migrate_card_id(from: &str, to: &str) {
    if from == to { return; }
    let file = &mut *store().lock().unwrap();
    let Some(mut samples) = file.entries.remove(from) else { return; };
    let target = file.entries.entry(to.to_string()).or_default();
    for sample in samples.drain(..) {
        if let Some(existing) = target.iter_mut().find(|item| item.day == sample.day) {
            existing.used = existing.used.max(sample.used);
        } else {
            target.push(sample);
        }
    }
    target.sort_by(|a, b| a.day.cmp(&b.day));
    persist(file);
}

// ---------------------------------------------------------------------------
// Credits ledger — Qoder CN / Trae CN bill in credits, and their token usage
// is structurally unavailable. Same per-day sampling shape as the quota
// ledger above, but the stored value is the ABSOLUTE "credits used" reading
// and the rendered trend is the day-over-day consumption diff. Kept in its
// own file so credit numbers can never leak into percent or token series.
// ---------------------------------------------------------------------------

const CREDIT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Default)]
struct CreditFile {
    version: u32,
    /// card id → per-day absolute "credits used" readings, sorted by day.
    entries: BTreeMap<String, Vec<DaySample>>,
}

fn credit_persist_path() -> PathBuf {
    crate::providers::config_dir().join("credit_history.json")
}

fn credit_store() -> &'static Mutex<CreditFile> {
    static STORE: OnceLock<Mutex<CreditFile>> = OnceLock::new();
    STORE.get_or_init(|| {
        // The derive(Default) store seeds version 0, so every file written
        // before this was fixed carries version 0 — an exact `== 1` filter
        // silently dropped the whole ledger (credits never accumulated).
        // Accept anything up to the current version and upgrade in place.
        let mut file = fs::read_to_string(credit_persist_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<CreditFile>(&raw).ok())
            .filter(|doc| doc.version <= CREDIT_VERSION)
            .unwrap_or_default();
        file.version = CREDIT_VERSION;
        Mutex::new(file)
    })
}

fn persist_credits(file: &CreditFile) {
    let tmp = credit_persist_path().with_extension("json.tmp");
    if fs::write(&tmp, serde_json::to_string(file).unwrap_or_default()).is_ok() {
        let _ = fs::rename(&tmp, credit_persist_path());
    }
}

/// The card's absolute "credits used" reading, from its main Credits row.
/// Only credits-denominated providers (Qoder CN / Trae CN) emit that row —
/// the label and the "… credits used" detail come from their parser.
pub fn credits_used_from_metrics(metrics: &[Metric]) -> Option<f64> {
    let metric = metrics.iter().find(|m| m.label == "Credits")?;
    let detail = metric.detail.as_deref()?;
    if !detail.ends_with("credits used") {
        return None;
    }
    // "{used:.0} of {total:.0} credits used" — the producer owns this shape.
    detail.split_whitespace().next()?.parse().ok()
}

/// Record absolute readings in one write. Same-day keeps the max: a pool
/// reset reads lower and must not drag the baseline backwards mid-day.
pub fn record_credit_samples(samples: &[(String, f64)]) {
    if samples.is_empty() {
        return;
    }
    let today = today_string();
    let file = &mut *credit_store().lock().unwrap();
    for (id, used) in samples {
        record_credit_sample(&mut file.entries, id, &today, *used);
    }
    persist_credits(file);
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
            days.push(CreditDay { day: sample.day.clone(), consumed });
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
    let file = credit_store().lock().unwrap();
    let mut out: Vec<CreditExtra> = file
        .entries
        .iter()
        .filter(|(_, samples)| !samples.is_empty())
        .map(|(id, samples)| CreditExtra {
            id: id.clone(),
            windows: credit_window_totals(&credit_day_series(samples), &today),
        })
        .collect();
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
        .map(|s| s.used);
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
                    let consumed = prev
                        .map(|p| (sample.used - p).max(0.0) + if end < sample.used { end } else { 0.0 });
                    prev = Some(end);
                    consumed
                }
                None => None,
            }
        })
        .collect()
}

/// card id → 30-day credit-consumption trend, for the frontend.
pub fn credit_trend_map() -> BTreeMap<String, Vec<Option<f64>>> {
    let today = today_string();
    let file = credit_store().lock().unwrap();
    file.entries
        .keys()
        .map(|id| (id.clone(), credit_trend_for(&file.entries, id, &today)))
        .collect()
}

/// Remove history for a card when deleted without keeping usage data.
pub fn remove_card_history(id: &str) {
    {
        let mut file = store().lock().unwrap();
        if file.entries.remove(id).is_some() {
            persist(&file);
        }
    }
    {
        let mut file = credit_store().lock().unwrap();
        if file.entries.remove(id).is_some() {
            persist_credits(&file);
        }
    }
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

    fn credit_sample(day: &str, used: f64, last: Option<f64>) -> DaySample {
        DaySample { day: day.into(), used, last }
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
        assert_eq!(days.len(), 2, "one day per reading after the first: {days:?}");
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
        .map(|(day, consumed)| CreditDay { day: (*day).into(), consumed: *consumed })
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
        assert_eq!(credit_window_totals(&days, "2026-10-14"), CreditWindows::default());
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
    fn trend_spans_thirty_days_ending_today_with_gaps_as_none() {
        let mut entries = BTreeMap::new();
        record_sample(&mut entries, "a", "2026-09-03", 55.0); // today
        record_sample(&mut entries, "a", "2026-08-05", 42.0); // 29 days back
        let trend = trend_for(&entries, "a", "2026-09-03");
        assert_eq!(trend.len(), 30);
        assert_eq!(trend[0], Some(42.0));
        assert_eq!(trend[1], None);
        assert_eq!(trend[29], Some(55.0));
        assert!(trend_for(&entries, "missing", "2026-09-03").iter().all(|v| v.is_none()));
    }

    #[test]
    fn credits_row_parses_only_the_real_shape() {
        let credits = Metric {
            label: "Credits".into(),
            kind: "progress".into(),
            used_percent: Some(1.9),
            detail: Some("38 of 2000 credits used".into()),
            value: None,
            resets_at: None,
            period_ms: None,
        };
        assert_eq!(credits_used_from_metrics(&[credits]), Some(38.0));
        // A "Credits" label without the credits detail is not a credit row.
        let lookalike = Metric {
            label: "Credits".into(),
            kind: "progress".into(),
            used_percent: Some(40.0),
            detail: Some("40% used".into()),
            value: None,
            resets_at: None,
            period_ms: None,
        };
        assert_eq!(credits_used_from_metrics(&[lookalike]), None);
        assert_eq!(credits_used_from_metrics(&[metric("progress", Some(10.0))]), None);
    }

    #[test]
    fn credit_trend_diffs_against_previous_observation() {
        let mut entries = BTreeMap::new();
        record_sample(&mut entries, "qodercn", "2026-09-01", 100.0);
        record_sample(&mut entries, "qodercn", "2026-09-02", 138.0); // +38
        // 09-03 not observed; the gap's consumption lands whole on 09-04.
        record_sample(&mut entries, "qodercn", "2026-09-04", 200.0); // +62
        let trend = credit_trend_for(&entries, "qodercn", "2026-09-04");
        assert_eq!(trend[26], None); // first observed day: no baseline
        assert_eq!(trend[27], Some(38.0));
        assert_eq!(trend[28], None); // not observed
        assert_eq!(trend[29], Some(62.0)); // gap consumption not spread
    }

    #[test]
    fn credit_trend_uses_pre_window_baseline_and_zeroes_resets() {
        let mut entries = BTreeMap::new();
        record_sample(&mut entries, "qodercn", "2026-08-01", 500.0); // before window
        record_sample(&mut entries, "qodercn", "2026-09-03", 550.0); // today-1: +50
        record_sample(&mut entries, "qodercn", "2026-09-04", 10.0); // reset → 0
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
}
