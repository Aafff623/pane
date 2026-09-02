use super::{http, Metric, Snapshot};
use serde_json::{json, Value};
use std::path::PathBuf;

const ID: &str = "devin";
const NAME: &str = "Devin";
// Mirrors the Mac app's DevinUsageClient — the server expects an IDE-shaped
// client identity and the Connect RPC protocol header.
const COMPAT_VERSION: &str = "1.108.2";

fn credentials_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        paths.push(PathBuf::from(appdata).join("devin").join("credentials.toml"));
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local").join("share").join("devin").join("credentials.toml"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(local).join("devin").join("credentials.toml"));
    }
    paths
}

/// Pure local probe for the Customize gear panel (no network): a Devin CLI
/// credentials.toml exists at any of the known locations.
pub fn local_credential_hint() -> Option<String> {
    credentials_paths()
        .iter()
        .any(|p| p.exists())
        .then(|| "Devin CLI sign-in (credentials.toml)".to_string())
}

/// Devin sends some numbers as JSON strings ("5000000") — accept both.
fn as_num(v: Option<&Value>) -> Option<f64> {
    let v = v?;
    v.as_f64().or_else(|| v.as_str()?.trim().parse().ok())
}

/// A quota percent that proto3 omitted (zero value) reads as 0% remaining
/// when its sibling reset timestamp proves the quota exists.
fn zero_when_omitted(remaining: Option<f64>, reset: Option<f64>) -> Option<f64> {
    remaining.or_else(|| reset.is_some().then_some(0.0))
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_quota_percent_with_reset_means_exhausted() {
        // Exhausted week: percent omitted, reset present → 0% remaining.
        assert_eq!(zero_when_omitted(None, Some(1_786_262_400.0)), Some(0.0));
        // Percent present → passes through untouched.
        assert_eq!(zero_when_omitted(Some(37.0), Some(1.0)), Some(37.0));
        // Neither field → genuinely no such quota window.
        assert_eq!(zero_when_omitted(None, None), None);
    }

    /// Live diagnostic (ignored): prints the raw planStatus so quota
    /// misreports can be debugged against real account states (never
    /// prints the key). Run:
    ///   cargo test devin_status_live_probe -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn devin_status_live_probe() {
        let Some(path) = credentials_paths().into_iter().find(|p| p.exists()) else {
            println!("no credentials.toml");
            return;
        };
        let raw = std::fs::read_to_string(&path).unwrap();
        let doc: toml::Value = toml::from_str(&raw).unwrap();
        let api_key = doc.get("windsurf_api_key").and_then(toml::Value::as_str).unwrap();
        let server = doc
            .get("api_server_url")
            .and_then(toml::Value::as_str)
            .unwrap_or("https://server.codeium.com")
            .trim_end_matches('/');
        let resp = http()
            .post(format!("{server}/exa.seat_management_pb.SeatManagementService/GetUserStatus"))
            .header("Content-Type", "application/json")
            .header("Connect-Protocol-Version", "1")
            .json(&json!({ "metadata": { "apiKey": api_key, "ideName": "devin",
                "ideVersion": COMPAT_VERSION, "extensionName": "devin",
                "extensionVersion": COMPAT_VERSION, "locale": "en" } }))
            .send()
            .await
            .unwrap();
        println!("status: {}", resp.status());
        let body: Value = resp.json().await.unwrap();
        println!(
            "planStatus: {}",
            serde_json::to_string_pretty(
                body.pointer("/userStatus/planStatus").unwrap_or(&Value::Null)
            )
            .unwrap()
        );
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(path) = credentials_paths().into_iter().find(|p| p.exists()) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Devin CLI sign-in not found (credentials.toml).",
        ));
    };

    let raw = std::fs::read_to_string(&path).map_err(|e| format!("read credentials.toml: {e}"))?;
    let doc: toml::Value = toml::from_str(&raw).map_err(|e| format!("parse credentials.toml: {e}"))?;
    let api_key = doc
        .get("windsurf_api_key")
        .and_then(toml::Value::as_str)
        .ok_or("credentials.toml has no windsurf_api_key")?
        .to_string();
    let server = doc
        .get("api_server_url")
        .and_then(toml::Value::as_str)
        .unwrap_or("https://server.codeium.com")
        .trim_end_matches('/')
        .to_string();

    let resp = http()
        .post(format!(
            "{server}/exa.seat_management_pb.SeatManagementService/GetUserStatus"
        ))
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .json(&json!({
            "metadata": {
                "apiKey": api_key,
                "ideName": "devin",
                "ideVersion": COMPAT_VERSION,
                "extensionName": "devin",
                "extensionVersion": COMPAT_VERSION,
                "locale": "en",
            }
        }))
        .send()
        .await
        .map_err(|e| format!("status request: {e}"))?;
    if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
        return Err("Devin credentials were rejected — sign in with the Devin CLI again".into());
    }
    if !resp.status().is_success() {
        return Err(format!("status endpoint: HTTP {}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| format!("status parse: {e}"))?;

    let plan_status = body
        .pointer("/userStatus/planStatus")
        .ok_or("response has no userStatus.planStatus")?;
    let plan_info = plan_status.get("planInfo").cloned().unwrap_or(Value::Null);

    let plan = plan_info
        .get("planName")
        .and_then(Value::as_str)
        .map(str::to_string);
    let hide_daily = plan_info
        .get("hideDailyQuota")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let daily_reset = as_num(plan_status.get("dailyQuotaResetAtUnix"));
    let weekly_reset = as_num(plan_status.get("weeklyQuotaResetAtUnix"));
    // proto3 JSON drops zero-valued fields: an exhausted quota loses its
    // RemainingPercent entirely while its reset timestamp stays. A missing
    // percent alongside a present reset therefore means 0% left — not "no
    // quota" — else a fully spent week rendered as a fresh 0%-used bar
    // (the same omitted-field trick Grok pulls with creditUsagePercent).
    let daily_remaining =
        zero_when_omitted(as_num(plan_status.get("dailyQuotaRemainingPercent")), daily_reset);
    let weekly_remaining =
        zero_when_omitted(as_num(plan_status.get("weeklyQuotaRemainingPercent")), weekly_reset);

    const DAY: i64 = 86_400_000;
    let to_ms = |unix: Option<f64>| unix.map(|s| (s * 1000.0) as i64);

    // Devin reports percent *remaining*; the meter shows percent *used*.
    let mut metrics = Vec::new();
    if !hide_daily {
        if let Some(remaining) = daily_remaining {
            metrics.push(
                Metric::progress("Daily", (100.0 - remaining).clamp(0.0, 100.0), None)
                    .with_reset(to_ms(daily_reset), Some(DAY)),
            );
        }
    }
    match (weekly_remaining, hide_daily, daily_remaining) {
        (Some(remaining), _, _) => {
            metrics.push(
                Metric::progress("Weekly", (100.0 - remaining).clamp(0.0, 100.0), None)
                    .with_reset(to_ms(weekly_reset), Some(7 * DAY)),
            );
        }
        // No weekly quota reported: surface the hidden daily quota in the
        // Weekly row so the card stays meaningful (same as the Mac app).
        (None, true, Some(remaining)) => {
            metrics.push(
                Metric::progress("Weekly", (100.0 - remaining).clamp(0.0, 100.0), None)
                    .with_reset(to_ms(weekly_reset), Some(7 * DAY)),
            );
        }
        _ => {}
    }
    if let Some(micros) = as_num(plan_status.get("overageBalanceMicros")) {
        let dollars = micros.max(0.0) / 1_000_000.0;
        // A funded balance meters like a plan window (against the highest
        // balance seen — a top-up raises it); an empty one stays a plain row.
        let meter = (dollars > 0.0)
            .then(|| super::credit_meter_labeled("devin-extra", "$", dollars, "Extra balance", ""))
            .flatten();
        match meter {
            Some(m) => metrics.push(m),
            None => metrics.push(Metric::text("Extra balance", format!("${dollars:.2}"))),
        }
    }

    if metrics.is_empty() {
        return Err("no quota data in response".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

// ---------------------------------------------------------------------------
// Local spend events — the Devin CLI's sessions.db
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct UsageEvent {
    pub ts_ms: i64,
    pub model: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

fn sessions_db_path() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(appdata).join("devin").join("cli").join("sessions.db"))
}

/// (mtime, size) of one file; a fixed sentinel when it doesn't exist.
type FileStamp = (std::time::SystemTime, u64);

fn file_stamp(path: &std::path::Path) -> FileStamp {
    std::fs::metadata(path)
        .map(|m| (m.modified().unwrap_or(std::time::UNIX_EPOCH), m.len()))
        .unwrap_or((std::time::UNIX_EPOCH, 0))
}

/// Per-request token metrics from the Devin CLI's local session store.
/// Assistant messages carry a metrics object (input/output/cache tokens);
/// the store keeps one row per message per branch of the session's message
/// forest, so rows dedupe by (session, message id). The model is tracked
/// per session. Cloud Devin sessions bill ACUs and never land in this db —
/// only CLI usage shows up. Parsed results are cached so the 37 MB file
/// isn't copied on every refresh; the cache keys on the main db AND the
/// WAL sidecar, because SQLite appends new rows to the -wal without
/// touching the main file until a checkpoint runs.
pub fn collect_usage_events() -> Vec<UsageEvent> {
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(FileStamp, FileStamp, Vec<UsageEvent>)>> = Mutex::new(None);

    let Some(db_path) = sessions_db_path() else { return Vec::new() };
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

    // Snapshot through SQLite's backup API rather than copying files: the
    // Devin app writes new rows to the WAL continuously, and a raw copy of
    // db + WAL taken at slightly different instants makes SQLite silently
    // discard the WAL — recent sessions would just vanish from the totals.
    let tmp_base = std::env::temp_dir().join(format!("pane-devin-{}", std::process::id()));
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
        // Busy/locked db: keep showing the last good events instead of a
        // sudden empty Devin row; the next refresh retries.
        Err(_) => CACHE
            .lock()
            .ok()
            .and_then(|c| c.as_ref().map(|(_, _, e)| e.clone()))
            .unwrap_or_default(),
    }
}

/// Consistent point-in-time copy of the live db (reads through the WAL
/// with proper locks, retrying briefly while the writer is busy).
fn snapshot_db(src_path: &std::path::Path, dst_path: &std::path::Path) -> Result<(), String> {
    let _ = std::fs::remove_file(dst_path);
    let src = rusqlite::Connection::open_with_flags(
        src_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("open live db: {e}"))?;
    let mut dst =
        rusqlite::Connection::open(dst_path).map_err(|e| format!("open snapshot: {e}"))?;
    let backup = rusqlite::backup::Backup::new(&src, &mut dst)
        .map_err(|e| format!("backup init: {e}"))?;
    backup
        .run_to_completion(256, std::time::Duration::from_millis(10), None)
        .map_err(|e| format!("backup run: {e}"))
}

fn read_usage_events(db: &std::path::Path) -> Result<Vec<UsageEvent>, String> {
    let conn = rusqlite::Connection::open(db).map_err(|e| format!("open db copy: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT m.session_id, m.chat_message, m.created_at, s.model
             FROM message_nodes m JOIN sessions s ON s.id = m.session_id",
        )
        .map_err(|e| format!("query messages: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("read messages: {e}"))?;

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (session_id, chat_message, node_created_s, model) = row;
        let Ok(msg) = serde_json::from_str::<Value>(&chat_message) else { continue };
        if msg.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let md = msg.get("metadata").cloned().unwrap_or(Value::Null);
        let Some(metrics) = md.get("metrics").filter(|m| m.is_object()) else { continue };
        // One message can appear on several branches of the session forest.
        if let Some(mid) = msg
            .get("message_id")
            .and_then(Value::as_str)
            .or_else(|| md.get("request_id").and_then(Value::as_str))
        {
            if !seen.insert((session_id.clone(), mid.to_string())) {
                continue;
            }
        }
        let num = |k: &str| metrics.get(k).and_then(Value::as_f64).unwrap_or(0.0);
        let ts_ms = md
            .get("created_at")
            .and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(node_created_s * 1000);
        // The message's own generation_model is the truth: the session-level
        // model is rewritten in place on every switch, which retroactively
        // relabels (and misprices) everything the session ran before. Older
        // records without the field fall back to the session model.
        let event_model = md
            .get("generation_model")
            .and_then(Value::as_str)
            .filter(|m| !m.trim().is_empty())
            .unwrap_or(&model)
            .to_string();
        out.push(UsageEvent {
            ts_ms,
            model: event_model,
            input: num("input_tokens"),
            output: num("output_tokens"),
            cache_read: num("cache_read_tokens"),
            cache_write: num("cache_creation_tokens"),
        });
    }
    Ok(out)
}
