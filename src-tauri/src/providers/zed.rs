//! Zed — local-only usage telemetry from the editor's own thread database.
//!
//! Zed keeps every assistant thread in `%LOCALAPPDATA%\Zed\threads\threads.db`
//! (SQLite; payload rows are zstd-compressed JSON, mechanism per openusage's
//! zed provider). Pane opens it read-only with the immutable flag so it
//! never competes with the running editor's lock, counts only zed.dev-hosted
//! model threads (BYOK threads bill through their own providers and are
//! skipped), and reports all-time + 30-day totals with a recent-models
//! breakdown. No credentials, no network — the card stays empty until Zed
//! has been used on this machine.

use super::{Metric, Snapshot};
use serde_json::Value;
use std::path::{Path, PathBuf};

const ID: &str = "zed";
const NAME: &str = "Zed";
/// Inflated thread payloads are capped so a corrupt or hostile record can't
/// burn memory (same cap as openusage).
const MAX_INFLATED: u64 = 32 << 20;
const HOSTED_PROVIDER: &str = "zed.dev";
const DAY_MS: i64 = 24 * 3600 * 1000;

pub async fn snapshot() -> Snapshot {
    match fetch() {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Pure local probe for the Customize gear panel (no network): Zed's thread
/// database exists on this machine.
pub fn local_credential_hint() -> Option<String> {
    thread_db_candidates().iter().any(|p| p.is_file()).then(|| "Zed local database".to_string())
}

fn fetch() -> Result<Snapshot, String> {
    let Some(db) = thread_db_candidates().into_iter().find(|p| p.is_file()) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Zed usage not found. Use Zed's assistant (zed.dev models) on this machine, then refresh.",
        ));
    };
    let conn = rusqlite::Connection::open_with_flags(
        db_uri(&db),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| format!("open Zed threads database: {e}"))?;
    let threads = read_threads(&conn)?;
    if threads.is_empty() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "No Zed assistant usage yet — hosted-model threads appear here after you use them.",
        ));
    }
    let now = chrono::Utc::now().timestamp_millis();
    Ok(Snapshot::ok(ID, NAME, None, metrics_from_threads(&threads, now)))
}

/// threads.db candidates per Zed's published data locations: Windows
/// LOCALAPPDATA first (this build's platform), then the macOS/Linux spots.
fn thread_db_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if !local.trim().is_empty() {
            out.push(Path::new(local.trim()).join("Zed").join("threads").join("threads.db"));
        }
    }
    if let Some(home) = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
    {
        out.push(home.join("AppData").join("Local").join("Zed").join("threads").join("threads.db"));
        out.push(
            home.join("Library")
                .join("Application Support")
                .join("Zed")
                .join("threads")
                .join("threads.db"),
        );
        out.push(home.join(".local").join("share").join("zed").join("threads").join("threads.db"));
    }
    out
}

/// Read-only immutable URI — sqlite then ignores the live editor's lock
/// entirely instead of fighting over the WAL.
fn db_uri(path: &Path) -> String {
    let mut uri = String::from("file:");
    for b in path.to_string_lossy().replace('\\', "/").bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'~' | b'/' | b'-' => {
                uri.push(b as char)
            }
            _ => uri.push_str(&format!("%{b:02X}")),
        }
    }
    uri.push_str("?mode=ro&immutable=1");
    uri
}

struct ThreadRow {
    model: String,
    tokens: i64,
    ts_ms: Option<i64>,
}

/// Every hosted-model thread row in the database (pure-ish: takes the
/// connection so tests can feed a temp db).
fn read_threads(conn: &rusqlite::Connection) -> Result<Vec<ThreadRow>, String> {
    let mut present = Vec::new();
    {
        let mut stmt = conn
            .prepare("PRAGMA table_info(threads)")
            .map_err(|e| format!("read threads schema: {e}"))?;
        let cols = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| format!("read threads schema: {e}"))?;
        for col in cols.flatten() {
            present.push(col.to_lowercase());
        }
    }
    if !present.iter().any(|c| c == "data") || !present.iter().any(|c| c == "data_type") {
        // Older schema without payloads — nothing to count.
        return Ok(vec![]);
    }
    let created = if present.iter().any(|c| c == "created_at") { "created_at" } else { "NULL" };
    let updated = if present.iter().any(|c| c == "updated_at") { "updated_at" } else { "NULL" };
    let sql = format!(
        "SELECT data_type, data, {created}, {updated} FROM threads"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("query threads: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| format!("query threads: {e}"))?;
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (data_type, data, created, updated) = row;
        let payload = match data_type.as_deref().map(str::trim).map(str::to_lowercase).as_deref() {
            None | Some("") | Some("json") => data,
            Some("zstd") => match zstd_decode(&data) {
                Some(p) => p,
                None => continue,
            },
            Some(_) => continue, // unknown encoding — be conservative
        };
        if let Some(thread) =
            parse_thread(&payload, created.as_deref().unwrap_or(""), updated.as_deref().unwrap_or(""))
        {
            out.push(thread);
        }
    }
    Ok(out)
}

/// One thread payload → row, skipping non-hosted providers and empty usage.
fn parse_thread(payload: &[u8], created: &str, updated: &str) -> Option<ThreadRow> {
    let doc: Value = serde_json::from_slice(payload).ok()?;
    let model = doc.get("model")?;
    if !model
        .get("provider")
        .and_then(Value::as_str)
        .map(|p| p.trim().eq_ignore_ascii_case(HOSTED_PROVIDER))
        .unwrap_or(false)
    {
        return None;
    }
    let name = model
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| model.get("id").and_then(Value::as_str))
        .map(str::trim)
        .filter(|n| !n.is_empty())?
        .to_string();

    let sum = |v: &Value| {
        ["input_tokens", "output_tokens", "cache_read_input_tokens", "cache_creation_input_tokens", "reasoning_tokens"]
            .iter()
            .map(|k| v.get(*k).and_then(Value::as_i64).unwrap_or(0))
            .sum::<i64>()
    };
    let tokens = if let Some(entries) = doc.get("request_token_usage").and_then(Value::as_array) {
        entries.iter().map(|e| sum(e.get("token_usage").unwrap_or(e))).sum()
    } else {
        doc.get("cumulative_token_usage").map(sum).unwrap_or(0)
    };
    if tokens == 0 {
        return None;
    }

    let ts = ["created_at", "updated_at"]
        .iter()
        .find_map(|k| doc.get(*k).and_then(Value::as_str))
        .and_then(parse_rfc3339_ms)
        .or_else(|| parse_rfc3339_ms(updated))
        .or_else(|| parse_rfc3339_ms(created));
    Some(ThreadRow { model: name, tokens, ts_ms: ts })
}

fn parse_rfc3339_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis())
}

/// zstd payload → JSON bytes, capped at MAX_INFLATED (truncated output
/// fails rather than surfacing partial JSON).
fn zstd_decode(data: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let reader = zstd::stream::read::Decoder::new(data).ok()?;
    let mut out = Vec::new();
    reader.take(MAX_INFLATED + 1).read_to_end(&mut out).ok()?;
    (out.len() as u64 <= MAX_INFLATED).then_some(out)
}

/// Threads → card rows: all-time and 30-day totals plus the top models.
fn metrics_from_threads(threads: &[ThreadRow], now_ms: i64) -> Vec<Metric> {
    let total: i64 = threads.iter().map(|t| t.tokens).sum();
    let recent: i64 = threads
        .iter()
        .filter(|t| t.ts_ms.is_some_and(|ts| ts >= now_ms - 30 * DAY_MS))
        .map(|t| t.tokens)
        .sum();
    let mut metrics = vec![Metric::text("Tokens (all time)", fmt_tokens(total as f64))];
    if recent > 0 {
        metrics.push(Metric::text("Tokens (30 days)", fmt_tokens(recent as f64)));
    }
    // Top 3 models by all-time tokens.
    let mut by_model: Vec<(String, i64)> = Vec::new();
    for t in threads {
        match by_model.iter_mut().find(|(m, _)| m == &t.model) {
            Some((_, n)) => *n += t.tokens,
            None => by_model.push((t.model.clone(), t.tokens)),
        }
    }
    by_model.sort_by(|a, b| b.1.cmp(&a.1));
    let top = by_model
        .iter()
        .take(3)
        .map(|(m, n)| format!("{m} {}", fmt_tokens(*n as f64)))
        .collect::<Vec<_>>()
        .join(" · ");
    metrics.push(Metric::text("Recent models", top));
    metrics
}

fn fmt_tokens(n: f64) -> String {
    if n >= 1e9 {
        format!("{:.1}B", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1}K", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn thread_payload(provider: &str, model: &str, input: i64, output: i64) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "model": { "provider": provider, "name": model },
            "request_token_usage": [
                { "token_usage": { "input_tokens": input, "output_tokens": output } }
            ],
            "created_at": "2026-09-01T00:00:00Z"
        }))
        .unwrap()
    }

    fn zstd_compress(data: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut encoder = zstd::stream::Encoder::new(Vec::new(), 3).unwrap();
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn hosted_threads_count_by_model_and_recent_window() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-08T00:00:00Z")
            .unwrap()
            .timestamp_millis();
        let threads = [
            ThreadRow { model: "claude-opus-4.6".into(), tokens: 1_200_000, ts_ms: Some(now - 5 * DAY_MS) },
            ThreadRow { model: "claude-opus-4.6".into(), tokens: 300_000, ts_ms: Some(now - 200 * DAY_MS) },
            ThreadRow { model: "gpt-6.1".into(), tokens: 800_000, ts_ms: None },
        ];
        let metrics = metrics_from_threads(&threads, now);
        assert_eq!(metrics[0].label, "Tokens (all time)");
        assert_eq!(metrics[0].value.as_deref(), Some("2.3M"));
        assert_eq!(metrics[1].label, "Tokens (30 days)");
        assert_eq!(metrics[1].value.as_deref(), Some("1.2M"));
        assert_eq!(metrics[2].label, "Recent models");
        assert_eq!(
            metrics[2].value.as_deref(),
            Some("claude-opus-4.6 1.5M · gpt-6.1 800.0K")
        );
    }

    #[test]
    fn thread_database_roundtrip_skips_unrelated_providers() {
        let path = std::env::temp_dir().join(format!("pane-zed-threads-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, data_type TEXT, data BLOB,
                 created_at TEXT, updated_at TEXT)",
                [],
            )
            .unwrap();
            let hosted = thread_payload("zed.dev", "claude-opus-4.6", 1_000_000, 200_000);
            conn.execute(
                "INSERT INTO threads (id, data_type, data, created_at) VALUES ('a', 'zstd', ?1, '2026-09-01T00:00:00Z')",
                [zstd_compress(&hosted)],
            )
            .unwrap();
            // BYOK thread: JSON payload, provider ollama — must be skipped.
            let byok = thread_payload("ollama", "qwen3", 500, 500);
            conn.execute(
                "INSERT INTO threads (id, data_type, data) VALUES ('b', 'json', ?1)",
                [byok],
            )
            .unwrap();
        }
        // Open through the production URI path (read-only + immutable).
        let ro = rusqlite::Connection::open_with_flags(
            db_uri(&path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .unwrap();
        let threads = read_threads(&ro).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].model, "claude-opus-4.6");
        assert_eq!(threads[0].tokens, 1_200_000);
        assert!(threads[0].ts_ms.is_some());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn db_uri_percent_encodes_specials_but_keeps_plain_paths() {
        let uri = db_uri(Path::new("C:/Users/my dir/AppData/Local/Zed/threads/threads.db"));
        assert!(uri.starts_with("file:C%3A/Users/my%20dir/"), "uri = {uri}");
        assert!(uri.ends_with("threads/threads.db?mode=ro&immutable=1"));
    }
}
