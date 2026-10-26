//! GitHub Copilot CLI local usage.
//!
//! Primary source: `~/.copilot/session-store.db` →
//! `assistant_usage_events`, one row per model call with exact token
//! buckets. `input_tokens` is the whole prompt (it includes the cache
//! buckets — verified against `token_details_json`: input 60620 =
//! fresh 10 + cache_write 60610), so tokens = prompt + output and the
//! price split uses the cache columns. `total_nano_aiu` is AI credits,
//! not USD, so it is deliberately not converted (ADR 0003).
//!
//! Fallback: `session-state/*/events.jsonl` `session.shutdown.modelMetrics`
//! for sessions the DB has no rows for (older/partial stores). The DB is
//! copied together with its WAL/SHM sidecars before reading so a running
//! Copilot process cannot hand us a torn snapshot.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use super::super::{
    add_event_est, build_spend, merge_data, note_unpriced, parse_ts, probe_price,
    recent_jsonl_files, FileData, ProviderSpend,
};
use super::{copilot_db, copilot_session_state};

static COPY_SEQ: AtomicU64 = AtomicU64::new(0);

fn cutoff() -> DateTime<Utc> {
    Utc::now() - Duration::days(31)
}

#[derive(Default)]
struct DbScan {
    data: FileData,
    sessions: HashSet<String>,
}

pub fn spend() -> ProviderSpend {
    build_spend("copilot", "GitHub Copilot", data())
}

/// Cheap `/v1/sources` probe: the DB exists but cannot be read/schema-checked.
pub fn probe_error() -> Option<String> {
    let db = copilot_db();
    if !db.exists() {
        return None;
    }
    let conn = match rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(conn) => conn,
        Err(error) => return Some(format!("open session-store.db: {error}")),
    };
    match conn.query_row("SELECT COUNT(*) FROM assistant_usage_events", [], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(_) => None,
        Err(error) => Some(format!("assistant_usage_events: {error}")),
    }
}

fn data() -> FileData {
    let db_roots = crate::wsl::roots_for(".copilot/session-store.db");
    let mut db = DbScan::default();
    for path in db_roots {
        if let Some(scan) = scan_db_at(&path) {
            merge_data(&mut db.data, scan.data);
            db.sessions.extend(scan.sessions);
        }
    }
    let event_roots = crate::wsl::roots_for(".copilot/session-state");
    merge_data(&mut db.data, scan_events_roots(&event_roots, &db.sessions));
    db.data
}

fn scan_db() -> Option<DbScan> {
    scan_db_at(&copilot_db())
}

fn scan_db_at(src: &Path) -> Option<DbScan> {
    if !src.exists() {
        return None;
    }
    with_db_copy(src, |db| {
        let conn = rusqlite::Connection::open_with_flags(
            db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT session_id, model, input_tokens, output_tokens,
                        cache_read_tokens, cache_write_tokens, created_at
                 FROM assistant_usage_events
                 WHERE created_at IS NOT NULL",
            )
            .ok()?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, i64>(2).unwrap_or(0),
                    row.get::<_, i64>(3).unwrap_or(0),
                    row.get::<_, i64>(4).unwrap_or(0),
                    row.get::<_, i64>(5).unwrap_or(0),
                    row.get::<_, String>(6)?,
                ))
            })
            .ok()?;
        let mut out = DbScan::default();
        let floor = cutoff();
        for row in rows.flatten() {
            let (session, model, input, output, cache_read, cache_write, created_at) = row;
            let Some(ts) = parse_ts(Some(&Value::String(created_at))) else { continue };
            if ts < floor {
                continue;
            }
            let model = if model.trim().is_empty() { "unknown".to_string() } else { model };
            add_usage(&mut out.data, ts, &model, input, output, cache_read, cache_write);
            if !session.is_empty() {
                out.sessions.insert(session);
            }
        }
        Some(out)
    })
}

fn scan_events(known_sessions: &HashSet<String>) -> FileData {
    scan_events_at(&copilot_session_state(), known_sessions)
}

fn scan_events_at(root: &Path, known_sessions: &HashSet<String>) -> FileData {
    scan_events_roots(std::slice::from_ref(&root.to_path_buf()), known_sessions)
}

fn scan_events_roots(roots: &[PathBuf], known_sessions: &HashSet<String>) -> FileData {
    let mut files = Vec::new();
    for root in roots {
        recent_jsonl_files(root, &mut files);
    }
    let mut out = FileData::default();
    let floor = cutoff();
    // A resumed session writes one shutdown per process, each carrying the
    // session's TOTAL modelMetrics. Keeping only the first (the old `seen`
    // rule) silently dropped every post-resume increment. Collect all
    // cumulative snapshots, then add per (session, model) deltas in time
    // order; a shrinking counter is an upstream reset, so its full value is
    // the new baseline.
    let mut snapshots: Vec<(String, DateTime<Utc>, Vec<(String, [i64; 4])>)> = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        for line in text.lines() {
            if !line.contains("session.shutdown") {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
            if v.get("type").and_then(Value::as_str) != Some("session.shutdown") {
                continue;
            }
            let Some(ts) = parse_ts(v.get("timestamp")) else { continue };
            if ts < floor {
                continue;
            }
            let data = v.get("data").cloned().unwrap_or(Value::Null);
            let session = data
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| path.parent().and_then(Path::file_name).map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_default();
            if known_sessions.contains(&session) {
                continue;
            }
            let Some(metrics) = data.get("modelMetrics").and_then(Value::as_object) else { continue };
            let mut models = Vec::new();
            for (model, metric) in metrics {
                let Some(usage) = metric.get("usage") else { continue };
                let num = |key: &str| usage.get(key).and_then(Value::as_f64).unwrap_or(0.0);
                models.push((
                    if model.trim().is_empty() { "unknown".to_string() } else { model.clone() },
                    [
                        num("inputTokens") as i64,
                        num("outputTokens") as i64,
                        num("cacheReadTokens") as i64,
                        num("cacheWriteTokens") as i64,
                    ],
                ));
            }
            if !models.is_empty() {
                snapshots.push((session, ts, models));
            }
        }
    }
    snapshots.sort_by_key(|(_, ts, _)| *ts);
    let mut last: std::collections::HashMap<(String, String), [i64; 4]> =
        std::collections::HashMap::new();
    for (session, ts, models) in snapshots {
        for (model, current) in models {
            let key = (session.clone(), model.clone());
            let previous = last.get(&key).copied().unwrap_or([0; 4]);
            let reset = current.iter().zip(previous.iter()).any(|(cur, prev)| cur < prev);
            let delta = if reset {
                current
            } else {
                [
                    current[0] - previous[0],
                    current[1] - previous[1],
                    current[2] - previous[2],
                    current[3] - previous[3],
                ]
            };
            last.insert(key, current);
            add_usage(&mut out, ts, &model, delta[0], delta[1], delta[2], delta[3]);
        }
    }
    out
}

fn add_usage(
    data: &mut FileData,
    ts: DateTime<Utc>,
    model: &str,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
) {
    let input = input.max(0) as f64;
    let output = output.max(0) as f64;
    let cache_read = cache_read.max(0) as f64;
    let cache_write = cache_write.max(0) as f64;
    // `input_tokens` includes the cache buckets; tokens are prompt + output.
    let prompt = input.max(cache_read + cache_write);
    let tokens = prompt + output;
    if tokens <= 0.0 {
        return;
    }
    let fresh = (prompt - cache_read - cache_write).max(0.0);
    let usage = crate::pricing::Usage {
        input: fresh,
        output,
        cache_read,
        cache_write_5m: cache_write,
        cache_write_1h: 0.0,
    };
    let priced = probe_price("copilot", model, "std", ts.timestamp_millis());
    match priced.price {
        Some(price) => add_event_est(
            data,
            ts,
            model,
            crate::pricing::request_cost_at(&price, &usage, 200_000.0),
            tokens,
            priced.estimated,
        ),
        None => note_unpriced(data, ts, model, tokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "pane-copilot-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn db_rows_use_input_as_whole_prompt_and_keep_cache_buckets_separate() {
        let dir = root("db");
        let db = dir.join("session-store.db");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE assistant_usage_events (
                session_id TEXT, model TEXT, input_tokens INTEGER,
                output_tokens INTEGER, cache_read_tokens INTEGER,
                cache_write_tokens INTEGER, created_at TEXT);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO assistant_usage_events VALUES
             ('s1','gpt-5.6-terra',60620,135,0,60610,'2026-10-20T10:00:00Z'),
             ('s1','pane-copilot-unpriced',100,20,0,0,'2026-10-20T10:00:01Z')",
            [],
        )
        .unwrap();
        drop(conn);
        let scan = scan_db_at(&db).expect("db scan");
        let tokens: f64 = scan.data.days.values().map(|(_, t)| t).sum();
        assert_eq!(tokens, 60_755.0 + 120.0);
        assert!(scan.sessions.contains("s1"));
        assert!(scan.data.unpriced.contains_key("pane-copilot-unpriced"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn events_shutdown_is_the_fallback_for_sessions_the_db_lacks() {
        let dir = root("events");
        let session = dir.join("session-1");
        std::fs::create_dir_all(&session).unwrap();
        std::fs::write(
            session.join("events.jsonl"),
            r#"{"type":"session.shutdown","timestamp":"2026-10-20T10:00:00Z","data":{"sessionId":"sess-1","modelMetrics":{"gpt-5.6-terra":{"usage":{"inputTokens":1000,"outputTokens":100,"cacheReadTokens":200,"cacheWriteTokens":300}}}}}"#,
        )
        .unwrap();
        let data = scan_events_at(&dir, &HashSet::new());
        let tokens: f64 = data.days.values().map(|(_, t)| t).sum();
        assert_eq!(tokens, 1_100.0);
        let known: HashSet<String> = ["sess-1".to_string()].into_iter().collect();
        assert!(scan_events_at(&dir, &known).days.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resumed_session_counts_only_the_cumulative_increment() {
        let dir = root("resume");
        let session = dir.join("session-1");
        std::fs::create_dir_all(&session).unwrap();
        std::fs::write(
            session.join("events.jsonl"),
            r#"{"type":"session.shutdown","timestamp":"2026-10-20T10:00:00Z","data":{"sessionId":"sess-r","modelMetrics":{"gpt-5.6-terra":{"usage":{"inputTokens":1000,"outputTokens":100,"cacheReadTokens":200,"cacheWriteTokens":300}}}}}
{"type":"session.shutdown","timestamp":"2026-10-20T11:00:00Z","data":{"sessionId":"sess-r","modelMetrics":{"gpt-5.6-terra":{"usage":{"inputTokens":2000,"outputTokens":200,"cacheReadTokens":200,"cacheWriteTokens":300}}}}}
{"type":"session.shutdown","timestamp":"2026-10-20T12:00:00Z","data":{"sessionId":"sess-r","modelMetrics":{"gpt-5.6-terra":{"usage":{"inputTokens":500,"outputTokens":50,"cacheReadTokens":0,"cacheWriteTokens":0}}}}}"#,
        )
        .unwrap();
        let data = scan_events_at(&dir, &HashSet::new());
        let tokens: f64 = data.days.values().map(|(_, t)| t).sum();
        // 1100 first + 1100 increment + reset to 550 = 2750, never double-counted.
        assert_eq!(tokens, 2_750.0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn with_db_copy<T>(src: &Path, f: impl FnOnce(&Path) -> Option<T>) -> Option<T> {
    let seq = COPY_SEQ.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let base: PathBuf = std::env::temp_dir().join(format!(
        "pane-copilot-{}-{seq}-{stamp:x}",
        std::process::id()
    ));
    let tmp = base.with_extension("db");
    if std::fs::copy(src, &tmp).is_err() {
        return None;
    }
    for suffix in ["db-wal", "db-shm"] {
        let side = src.with_extension(suffix);
        if side.exists() {
            let _ = std::fs::copy(&side, base.with_extension(suffix));
        }
    }
    let result = f(&tmp);
    for suffix in ["db", "db-wal", "db-shm"] {
        let _ = std::fs::remove_file(base.with_extension(suffix));
    }
    result
}
