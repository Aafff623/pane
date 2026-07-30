//! Hermes desktop (Nous Research) — spend source only, no provider card.
//!
//! Hermes keeps a local ledger at %LOCALAPPDATA%\hermes\state.db: the
//! `session_model_usage` table has one row per (session, model, billing
//! route) with cumulative token buckets, the app's own cost fields, and
//! first/last-seen stamps. Hermes can route the same chat through several
//! backends (MiniMax OAuth, OpenRouter, …), so each row carries the
//! billing provider — the spend scanner uses it to file tokens under the
//! provider that actually served them.

use super::minimax::{file_stamp, snapshot_db, FileStamp};

#[derive(Clone)]
pub struct HermesUsage {
    pub ts_ms: i64,
    pub model: String,
    pub billing_provider: String,
    pub input: f64,
    pub output: f64,
    pub reasoning: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    /// The app's own cost when it knows one (actual preferred over
    /// estimated); 0.0 means "price it from the catalog".
    pub cost_usd: f64,
}

fn state_db_path() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|d| d.join("hermes").join("state.db"))
}

/// Per-session-per-model usage from Hermes's local store. Cached on the
/// (db, WAL) stamps; a busy/locked db serves the last good events.
pub fn collect_usage_events() -> Vec<HermesUsage> {
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(FileStamp, FileStamp, Vec<HermesUsage>)>> = Mutex::new(None);

    let Some(db_path) = state_db_path() else { return Vec::new() };
    if !db_path.exists() {
        return Vec::new();
    }
    let db_stamp = file_stamp(&db_path);
    let wal_stamp = file_stamp(&db_path.with_extension("db-wal"));

    if let Ok(cache) = CACHE.lock() {
        if let Some((d, w, events)) = cache.as_ref() {
            if *d == db_stamp && *w == wal_stamp {
                return events.clone();
            }
        }
    }

    let tmp_base = std::env::temp_dir().join(format!("pane-hermes-{}", std::process::id()));
    let tmp_db = tmp_base.with_extension("db");
    let events = snapshot_db(&db_path, &tmp_db).and_then(|()| read_usage_events(&tmp_db));
    for suffix in ["db", "db-wal", "db-shm"] {
        let _ = std::fs::remove_file(tmp_base.with_extension(suffix));
    }

    match events {
        Ok(events) => {
            if let Ok(mut cache) = CACHE.lock() {
                *cache = Some((db_stamp, wal_stamp, events.clone()));
            }
            events
        }
        Err(_) => CACHE
            .lock()
            .ok()
            .and_then(|c| c.as_ref().map(|(_, _, e)| e.clone()))
            .unwrap_or_default(),
    }
}

fn read_usage_events(db: &std::path::Path) -> Result<Vec<HermesUsage>, String> {
    let conn = rusqlite::Connection::open(db).map_err(|e| format!("open db copy: {e}"))?;
    // last_seen/first_seen are epoch seconds as REAL; costs may be NULL.
    let mut stmt = conn
        .prepare(
            "SELECT last_seen, model, billing_provider,
                    input_tokens, output_tokens, reasoning_tokens,
                    cache_read_tokens, cache_write_tokens,
                    COALESCE(actual_cost_usd, 0.0), COALESCE(estimated_cost_usd, 0.0)
             FROM session_model_usage",
        )
        .map_err(|e| format!("query session_model_usage: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let actual: f64 = row.get(8).unwrap_or(0.0);
            let estimated: f64 = row.get(9).unwrap_or(0.0);
            Ok(HermesUsage {
                ts_ms: (row.get::<_, f64>(0).unwrap_or(0.0) * 1000.0) as i64,
                model: row.get::<_, String>(1).unwrap_or_default(),
                billing_provider: row.get::<_, String>(2).unwrap_or_default(),
                input: row.get::<_, f64>(3).unwrap_or(0.0),
                output: row.get::<_, f64>(4).unwrap_or(0.0),
                reasoning: row.get::<_, f64>(5).unwrap_or(0.0),
                cache_read: row.get::<_, f64>(6).unwrap_or(0.0),
                cache_write: row.get::<_, f64>(7).unwrap_or(0.0),
                cost_usd: if actual > 0.0 { actual } else { estimated },
            })
        })
        .map_err(|e| format!("read session_model_usage: {e}"))?;
    Ok(rows.flatten().collect())
}
