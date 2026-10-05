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

const VERSION: u32 = 3;

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
    migrate(file)
}

/// Version ladder. Each step drops only the cells whose provider's
/// accounting changed; the next scan re-merges corrected numbers for every
/// day the source still covers, and older days of that provider stay gone.
/// Other providers are untouched.
fn migrate(mut file: HistoryFile) -> Result<HistoryFile, String> {
    if file.version == 1 {
        // v1→v2: the zcode scanner stopped double-counting cache tokens
        // (its input column already contains them, ~1.96x inflation).
        for providers in file.days.values_mut() {
            providers.remove("zcode");
        }
        file.version = 2;
    }
    if file.version == 2 {
        // v2→v3: Claude's source switched to cc-switch's ledger (its proxy
        // records real tokens even where a relay zeroed the CLI logs —
        // the self-scan undercounted ~7-24x). Every stored claude cell is
        // replaced by the real numbers the next scan merges. New
        // cc-switch-only providers (mcode, …) need no clearing: their rows
        // are pure additions.
        for providers in file.days.values_mut() {
            providers.remove("claude");
            // Extra account cards (`claude@<fnv1a>`) stay: cc-switch's
            // session import only reads the default ~/.claude/projects, so
            // their self-scan accounting did not change — clearing them
            // would erase history nothing re-merges.
        }
        file.version = 3;
    }
    file.days.retain(|_, providers| !providers.is_empty());
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

#[derive(Serialize, Clone)]
pub struct DailyModelSpend {
    pub model: String,
    pub cost: f64,
    pub tokens: f64,
}

#[derive(Serialize, Clone)]
pub struct DailySpendRow {
    pub day: String,
    pub id: String,
    pub cost: f64,
    pub tokens: f64,
    pub models: Vec<DailyModelSpend>,
}

/// Day/provider/model grain for the spend-detail heatmap. Read-only, served
/// from the same cached store the merger keeps fresh.
pub fn daily_spend(range_days: Option<u32>) -> Result<Vec<DailySpendRow>, String> {
    let state = store().lock().map_err(|_| "spend history lock poisoned")?;
    let file = state.as_ref().map_err(|err| err.clone())?;
    Ok(daily_rows_from(file, range_days, Local::now().date_naive()))
}

fn daily_rows_from(file: &HistoryFile, range_days: Option<u32>, today: NaiveDate) -> Vec<DailySpendRow> {
    let cutoff = range_days.map(|days| today - Duration::days(days.saturating_sub(1) as i64));
    let mut rows = Vec::new();
    for (day, providers) in &file.days {
        let Ok(date) = NaiveDate::parse_from_str(day, "%Y-%m-%d") else { continue };
        if cutoff.is_some_and(|start| date < start || date > today) { continue; }
        for (id, cell) in providers {
            let mut models: Vec<DailyModelSpend> = cell
                .models
                .iter()
                .map(|(model, usage)| DailyModelSpend {
                    model: model.clone(),
                    cost: usage.cost,
                    tokens: usage.tokens,
                })
                .filter(|usage| usage.cost > 0.0 || usage.tokens > 0.0)
                .collect();
            models.sort_by(|a, b| b.tokens.total_cmp(&a.tokens));
            if cell.cost <= 0.0 && cell.tokens <= 0.0 && models.is_empty() {
                continue;
            }
            rows.push(DailySpendRow {
                day: day.clone(),
                id: id.clone(),
                cost: cell.cost,
                tokens: cell.tokens,
                models,
            });
        }
    }
    rows.sort_by(|a, b| a.day.cmp(&b.day).then_with(|| b.tokens.total_cmp(&a.tokens)));
    rows
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

    #[test]
    fn daily_rows_expose_models_and_respect_cutoff() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let mut file = HistoryFile::default();
        merge_into(&mut file, &[
            days(today, "zcode", "GLM-5.3", 1.0, 30.0),
            days(today, "zcode", "glm-5.3-flash", 0.5, 10.0),
            days(today - Duration::days(40), "old", "m", 9.0, 90.0),
        ]);
        // Same day+provider collapses into one row; models sort by tokens.
        let rows30 = daily_rows_from(&file, Some(30), today);
        assert_eq!(rows30.len(), 1);
        assert_eq!(rows30[0].id, "zcode");
        assert_eq!(rows30[0].models.len(), 2);
        assert_eq!(rows30[0].models[0].model, "GLM-5.3");
        // No cutoff keeps the aged day, day-ascending.
        let all = daily_rows_from(&file, None, today);
        assert_eq!(all.len(), 2);
        assert!(all[0].day < all[1].day);
    }

    #[test]
    fn daily_rows_skip_all_zero_cells() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let mut file = HistoryFile::default();
        merge_into(&mut file, &[days(today, "ghost", "m", 0.0, 0.0)]);
        assert!(daily_rows_from(&file, None, today).is_empty());
    }

    // ---- Version migrations -----------------------------------------------

    fn cell(cost: f64, tokens: f64) -> ProviderDay {
        ProviderDay { cost, tokens, models: BTreeMap::new() }
    }

    #[test]
    fn v2_to_v3_clears_only_claude() {
        let mut file = HistoryFile { version: 2, ..HistoryFile::default() };
        file.days.insert("2026-10-01".to_string(), BTreeMap::from([
            ("claude".to_string(), cell(1.0, 10.0)),
            ("codex".to_string(), cell(2.0, 20.0)),
        ]));
        // A day holding only claude cells collapses with them.
        file.days.insert("2026-09-30".to_string(), BTreeMap::from([
            ("claude".to_string(), cell(3.0, 30.0)),
        ]));
        let file = migrate(file).unwrap();
        assert_eq!(file.version, VERSION);
        let day = &file.days["2026-10-01"];
        assert!(!day.contains_key("claude"));
        assert_eq!(day["codex"].cost, 2.0);
        assert!(!file.days.contains_key("2026-09-30"));
    }

    #[test]
    fn v1_migration_cascades_through_v3() {
        let mut file = HistoryFile { version: 1, ..HistoryFile::default() };
        file.days.insert("2026-10-01".to_string(), BTreeMap::from([
            ("zcode".to_string(), cell(1.0, 10.0)),
            ("claude".to_string(), cell(2.0, 20.0)),
            ("kimi".to_string(), cell(3.0, 30.0)),
        ]));
        let file = migrate(file).unwrap();
        assert_eq!(file.version, VERSION);
        let day = &file.days["2026-10-01"];
        assert!(!day.contains_key("zcode"));
        assert!(!day.contains_key("claude"));
        assert_eq!(day["kimi"].cost, 3.0);
    }

    #[test]
    fn current_version_loads_untouched_future_version_errors() {
        let mut file = HistoryFile { version: VERSION, ..HistoryFile::default() };
        file.days.insert("2026-10-01".to_string(), BTreeMap::from([
            ("claude".to_string(), cell(1.0, 10.0)),
        ]));
        let file = migrate(file).unwrap();
        assert!(file.days["2026-10-01"].contains_key("claude"));
        let future = HistoryFile { version: VERSION + 1, ..HistoryFile::default() };
        assert!(migrate(future).is_err());
    }
}
