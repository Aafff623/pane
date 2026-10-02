//! Permanent per-day spend rollups.
//!
//! `spend.rs` rescans the recent log window. This module keeps those daily
//! facts so older spend remains queryable after a log ages out or an account
//! is archived. The merge is replacement based and therefore idempotent.

use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
struct ModelDay {
    cost: f64,
    tokens: f64,
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
struct ProviderDay {
    cost: f64,
    tokens: f64,
    models: BTreeMap<String, ModelDay>,
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
struct HistoryFile {
    version: u32,
    /// YYYY-MM-DD -> provider id -> daily rollup.
    days: BTreeMap<String, BTreeMap<String, ProviderDay>>,
}

static STORE: OnceLock<Mutex<Result<HistoryFile, String>>> = OnceLock::new();

fn persist_path() -> std::path::PathBuf {
    crate::providers::config_dir().join("spend_history.json")
}

fn load_file() -> Result<HistoryFile, String> {
    let raw = match fs::read_to_string(persist_path()) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(HistoryFile { version: VERSION, ..HistoryFile::default() });
        }
        Err(err) => return Err(format!("read spend history: {err}")),
    };
    let file: HistoryFile = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|err| format!("parse spend history: {err}"))?;
    if file.version != VERSION { return Err("unsupported spend history version".into()); }
    Ok(file)
}

fn store() -> &'static Mutex<Result<HistoryFile, String>> {
    STORE.get_or_init(|| Mutex::new(load_file()))
}

fn persist(file: &HistoryFile) -> Result<(), String> {
    let path = persist_path();
    let parent = path.parent().ok_or("missing spend history directory")?;
    fs::create_dir_all(parent).map_err(|err| format!("create spend history directory: {err}"))?;
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(file).map_err(|err| format!("encode spend history: {err}"))?;
    fs::write(&tmp, bytes).map_err(|err| format!("write spend history: {err}"))?;
    fs::rename(tmp, path).map_err(|err| format!("replace spend history: {err}"))
}

fn day_key(day_ce: i32) -> Option<String> {
    NaiveDate::from_num_days_from_ce_opt(day_ce).map(|d| d.format("%Y-%m-%d").to_string())
}

fn merge_into(file: &mut HistoryFile, daily: &[crate::spend::ProviderDays]) {
    let mut grouped: BTreeMap<String, BTreeMap<String, BTreeMap<String, ModelDay>>> = BTreeMap::new();
    for provider in daily {
        for ((day_ce, model), (cost, tokens)) in &provider.days {
            let Some(day) = day_key(*day_ce) else { continue };
            grouped
                .entry(day)
                .or_default()
                .entry(provider.id.clone())
                .or_default()
                .insert(model.clone(), ModelDay { cost: *cost, tokens: *tokens });
        }
    }

    for (day, providers) in grouped {
        let target = file.days.entry(day).or_default();
        for (provider, models) in providers {
            let cell = target.entry(provider).or_default();
            cell.models = models;
            cell.cost = cell.models.values().map(|m| m.cost).sum();
            cell.tokens = cell.models.values().map(|m| m.tokens).sum();
        }
    }
    file.version = VERSION;
}

/// Replace the cells present in this scan, while preserving providers/days
/// absent from it (for example a disabled provider or a failed Cursor fetch).
pub fn merge_daily(daily: &[crate::spend::ProviderDays]) -> Result<(), String> {
    if daily.is_empty() { return Ok(()); }
    let mut state = store().lock().map_err(|_| "spend history lock poisoned")?;
    let file = state.as_mut().map_err(|err| err.clone())?;
    let mut next = file.clone();
    merge_into(&mut next, daily);
    persist(&next)?;
    *file = next;
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct RangeSpend {
    pub id: String,
    pub cost: f64,
    pub tokens: f64,
    pub active_days: u32,
    pub models: Vec<crate::spend::ModelSpend>,
}

fn range_from(file: &HistoryFile, range_days: Option<u32>, today: NaiveDate) -> Vec<RangeSpend> {
    let cutoff = range_days.map(|days| today - Duration::days(days.saturating_sub(1) as i64));
    let mut by_provider: BTreeMap<String, (f64, f64, u32, BTreeMap<String, (f64, f64)>)> = BTreeMap::new();
    for (day_key, providers) in &file.days {
        let Ok(day) = NaiveDate::parse_from_str(day_key, "%Y-%m-%d") else { continue };
        if cutoff.is_some_and(|start| day < start || day > today) { continue; }
        for (id, cell) in providers {
            let entry = by_provider.entry(id.clone()).or_default();
            entry.0 += cell.cost;
            entry.1 += cell.tokens;
            if cell.tokens > 0.0 || cell.cost > 0.004 { entry.2 += 1; }
            for (model, usage) in &cell.models {
                let model_total = entry.3.entry(model.clone()).or_default();
                model_total.0 += usage.cost;
                model_total.1 += usage.tokens;
            }
        }
    }
    by_provider
        .into_iter()
        .filter(|(_, (cost, tokens, _, _))| *cost > 0.004 || *tokens > 0.0)
        .map(|(id, (cost, tokens, active_days, models))| {
            let mut models: Vec<_> = models
                .into_iter()
                .map(|(model, (cost, tokens))| crate::spend::ModelSpend { model, cost, tokens })
                .collect();
            models.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));
            models.truncate(20);
            RangeSpend { id, cost, tokens, active_days, models }
        })
        .collect()
}

/// `None` returns all retained history; `Some(n)` includes today and the
/// preceding n-1 calendar days.
pub fn range_spend(range_days: Option<u32>) -> Result<Vec<RangeSpend>, String> {
    let state = store().lock().map_err(|_| "spend history lock poisoned")?;
    let file = state.as_ref().map_err(|err| err.clone())?;
    Ok(range_from(file, range_days, Local::now().date_naive()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use std::collections::HashMap;

    fn days(day: NaiveDate, id: &str, model: &str, cost: f64, tokens: f64) -> crate::spend::ProviderDays {
        let mut data = HashMap::new();
        data.insert((day.num_days_from_ce(), model.to_string()), (cost, tokens));
        crate::spend::ProviderDays { id: id.to_string(), name: id.to_string(), days: data }
    }

    #[test]
    fn merge_is_idempotent() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let input = vec![days(today, "a", "m", 1.0, 10.0)];
        let mut file = HistoryFile { version: VERSION, ..HistoryFile::default() };
        merge_into(&mut file, &input);
        let once = file.clone();
        merge_into(&mut file, &input);
        assert_eq!(file, once);
    }

    #[test]
    fn merge_replaces_cell_keeps_others() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let mut file = HistoryFile { version: VERSION, ..HistoryFile::default() };
        merge_into(&mut file, &[days(today, "a", "m", 1.0, 10.0), days(today, "b", "m", 2.0, 20.0)]);
        merge_into(&mut file, &[days(today, "a", "m", 3.0, 30.0)]);
        let row = &file.days["2026-10-01"];
        assert_eq!(row["a"].cost, 3.0);
        assert_eq!(row["b"].cost, 2.0);
    }

    #[test]
    fn replacing_a_day_removes_old_models_and_retains_older_days() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let old = today - Duration::days(80);
        let mut file = HistoryFile::default();
        let mut input = days(today, "a", "old", 1.0, 10.0);
        input.days.insert((today.num_days_from_ce(), "second".into()), (2.0, 20.0));
        merge_into(&mut file, &[input, days(old, "a", "old", 4.0, 40.0)]);
        assert_eq!(file.days["2026-10-01"]["a"].models.len(), 2);
        merge_into(&mut file, &[days(today, "a", "new", 3.0, 30.0)]);
        let current = &file.days["2026-10-01"]["a"];
        assert_eq!(current.models.len(), 1);
        assert!(current.models.contains_key("new"));
        assert_eq!(range_from(&file, None, today)[0].cost, 7.0);
    }

    #[test]
    fn empty_or_missing_scan_does_not_erase_history() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let mut file = HistoryFile::default();
        merge_into(&mut file, &[days(today, "archived@id", "m", 0.0, 100.0)]);
        let before = file.clone();
        merge_into(&mut file, &[]);
        assert_eq!(file, before);
        let result = range_from(&file, None, today);
        assert_eq!(result[0].tokens, 100.0);
        assert_eq!(result[0].active_days, 1);
    }

    #[test]
    fn range_spend_filters_and_counts_active_days() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let mut file = HistoryFile { version: VERSION, ..HistoryFile::default() };
        merge_into(&mut file, &[
            days(today - Duration::days(2), "a", "m", 1.0, 10.0),
            days(today - Duration::days(1), "a", "m", 2.0, 20.0),
            days(today, "a", "m", 3.0, 30.0),
        ]);
        let recent = range_from(&file, Some(2), today);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].cost, 5.0);
        assert_eq!(recent[0].tokens, 50.0);
        assert_eq!(recent[0].active_days, 2);
        assert_eq!(range_from(&file, None, today)[0].cost, 6.0);
    }
}
