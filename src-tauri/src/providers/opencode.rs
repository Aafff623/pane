use super::{Metric, Snapshot};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const ID: &str = "opencode";
const NAME: &str = "OpenCode";

// OpenCode Go plan limits from https://opencode.ai/docs/go/ (dollars).
// There is no public usage API yet (anomalyco/opencode#10448), so we compute
// spend locally from opencode's own message database — the same data
// `opencode stats` uses. Swap to the official API once it ships.
const SESSION_LIMIT: f64 = 12.0; // rolling 5 hours
const WEEKLY_LIMIT: f64 = 30.0; // rolling 7 days
const MONTHLY_LIMIT: f64 = 60.0; // per month

static COPY_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".local")
        .join("share")
        .join("opencode")
}

/// Reads an entry like {"opencode-go": {"type": "api", "key": "..."}} from
/// OpenCode's auth.json. Also used by the OpenRouter provider.
pub fn auth_entry_key(entry: &str) -> Option<String> {
    let raw = std::fs::read_to_string(data_dir().join("auth.json")).ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    doc.get(entry)?
        .get("key")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Runs `f` against a private copy of opencode.db (plus its write-ahead-log
/// files) so we never touch the live copy a running OpenCode holds open.
/// The unique counter keeps concurrent readers (usage + spend) apart.
fn with_db_copy<T>(f: impl FnOnce(&Path) -> Result<T, String>) -> Result<T, String> {
    let db_path = data_dir().join("opencode.db");
    if !db_path.exists() {
        return Err("opencode.db not found — has OpenCode been used on this PC?".into());
    }
    let n = COPY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_base = std::env::temp_dir().join(format!("openusage-oc-{}-{n}", std::process::id()));
    let tmp_db = tmp_base.with_extension("db");
    std::fs::copy(&db_path, &tmp_db).map_err(|e| format!("copy opencode.db: {e}"))?;
    for suffix in ["db-wal", "db-shm"] {
        let side = db_path.with_extension(suffix);
        if side.exists() {
            let _ = std::fs::copy(&side, tmp_base.with_extension(suffix));
        }
    }

    let result = f(&tmp_db);

    for suffix in ["db", "db-wal", "db-shm"] {
        let _ = std::fs::remove_file(tmp_base.with_extension(suffix));
    }
    result
}

pub async fn snapshot() -> Snapshot {
    match fetch() {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

fn fetch() -> Result<Snapshot, String> {
    let auth_path = data_dir().join("auth.json");
    if !auth_path.exists() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "OpenCode sign-in not found. Run `opencode` and log in.",
        ));
    }
    if auth_entry_key("opencode-go").is_none() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "No OpenCode Go subscription found in auth.json.",
        ));
    }

    let (session, weekly, monthly) = with_db_copy(sum_windows)?;
    let metrics = vec![
        Metric::progress(
            "Session",
            session / SESSION_LIMIT * 100.0,
            Some(format!("${session:.2} of ${SESSION_LIMIT:.0} · this PC only")),
        ),
        Metric::progress(
            "Weekly",
            weekly / WEEKLY_LIMIT * 100.0,
            Some(format!("${weekly:.2} of ${WEEKLY_LIMIT:.0} · this PC only")),
        ),
        Metric::progress(
            "Monthly",
            monthly / MONTHLY_LIMIT * 100.0,
            Some(format!("${monthly:.2} of ${MONTHLY_LIMIT:.0} · this PC only")),
        ),
    ];
    Ok(Snapshot::ok(ID, NAME, Some("Go".into()), metrics))
}

/// Sums the cost of Go-plan assistant messages in the last 5h / 7d / 30d.
fn sum_windows(db: &Path) -> Result<(f64, f64, f64), String> {
    let now_ms = chrono::Utc::now().timestamp_millis() as f64;
    let (mut session, mut weekly, mut monthly) = (0.0, 0.0, 0.0);
    for row in read_messages(db)? {
        if row.provider != "opencode-go" || row.cost <= 0.0 {
            continue;
        }
        let (cost, age_ms) = (row.cost, now_ms - row.ts);
        if age_ms <= 5.0 * 3600e3 {
            session += cost;
        }
        if age_ms <= 7.0 * 86400e3 {
            weekly += cost;
        }
        if age_ms <= 30.0 * 86400e3 {
            monthly += cost;
        }
    }
    Ok((session, weekly, monthly))
}

/// (timestamp ms, cost $, tokens, model) of every priced message, any
/// provider — this is money spent through OpenCode, used by Total Spend.
pub fn collect_cost_events() -> Vec<(f64, f64, f64, String)> {
    with_db_copy(|db| {
        Ok(read_messages(db)?
            .into_iter()
            // Free models record cost 0 with real token counts — the
            // tokens are usage facts and count at their true $0 price.
            // Rows with neither cost nor tokens (aborted turns) drop.
            .filter(|r| r.cost > 0.0 || r.tokens > 0.0)
            .map(|r| (r.ts, r.cost, r.tokens, r.model))
            .collect())
    })
    .unwrap_or_default()
}

pub struct MessageRow {
    pub ts: f64,
    pub cost: f64,
    pub tokens: f64,
    pub provider: String,
    pub model: String,
}

/// Raw assistant-message rows from opencode.db.
fn read_messages(db: &Path) -> Result<Vec<MessageRow>, String> {
    let conn = rusqlite::Connection::open(db).map_err(|e| format!("open db copy: {e}"))?;
    let mut stmt = conn
        .prepare("SELECT time_created, data FROM message")
        .map_err(|e| format!("query messages: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("read messages: {e}"))?;

    let mut out = Vec::new();
    for row in rows.flatten() {
        let (time_created, data) = row;
        let Ok(msg) = serde_json::from_str::<Value>(&data) else { continue };
        if msg.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let provider = msg
            .get("providerID")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let model = msg
            .get("modelID")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let cost = msg.get("cost").and_then(Value::as_f64).unwrap_or(0.0);
        let ts = msg
            .pointer("/time/completed")
            .or_else(|| msg.pointer("/time/created"))
            .and_then(Value::as_f64)
            .unwrap_or(time_created as f64);
        let tokens = ["/tokens/input", "/tokens/output", "/tokens/reasoning"]
            .iter()
            .filter_map(|p| msg.pointer(p).and_then(Value::as_f64))
            .sum::<f64>();
        out.push(MessageRow { ts, cost, tokens, provider, model });
    }
    Ok(out)
}
