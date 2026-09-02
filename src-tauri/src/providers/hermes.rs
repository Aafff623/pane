//! Hermes desktop (Nous Research) — local ledger card + spend source.
//!
//! Hermes keeps a local ledger at %LOCALAPPDATA%\hermes\state.db: the
//! `session_model_usage` table has one row per (session, model, billing
//! route) with cumulative token buckets, the app's own cost fields, and
//! first/last-seen stamps. Hermes can route the same chat through several
//! backends (MiniMax OAuth, OpenRouter, AihubMix, a custom OpenAI-compatible
//! URL, …). The spend scanner files MiniMax/OpenRouter rows under those
//! slices; everything else — including AihubMix, whether Hermes labeled
//! it `aihubmix` or `custom` pointed at aihubmix.com — stays on the
//! Hermes card. Hermes records ZERO cost itself, so dollars come from
//! the shared pricing catalog.

use super::minimax::{file_stamp, snapshot_db, FileStamp};
use super::{Metric, Snapshot};
use std::sync::atomic::{AtomicU64, Ordering};

const ID: &str = "hermes";
const NAME: &str = "Hermes";

/// Concurrent card refresh + spend scan each copy the ledger; a per-call
/// counter keeps their temp files from colliding (same pattern as OpenCode).
static COPY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// No credential source — Pane reads Hermes' local ledger, which carries
/// usage rows rather than a sign-in.
pub fn local_credential_hint() -> Option<String> {
    None
}

#[derive(Clone)]
pub struct HermesUsage {
    pub ts_ms: i64,
    pub model: String,
    pub billing_provider: String,
    pub billing_base_url: String,
    pub session_id: String,
    pub task: String,
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

pub async fn snapshot() -> Snapshot {
    fetch()
}

fn fetch() -> Snapshot {
    let Some(db_path) = state_db_path() else {
        return Snapshot::no_credentials(
            ID,
            NAME,
            "Install the Hermes desktop app (Nous Research) — Pane reads its local ledger.",
        );
    };
    if !db_path.exists() {
        return Snapshot::no_credentials(
            ID,
            NAME,
            "Install the Hermes desktop app (Nous Research) — Pane reads its local ledger.",
        );
    }
    let events = collect_usage_events();
    Snapshot::ok(
        ID,
        NAME,
        Some("Desktop".into()),
        metrics_from_events(&events),
    )
}

/// Card face: recent user-selected models, their backends, and session count.
/// Dollar totals live on the spend rows (Today / Yesterday / Last 30 Days)
/// so these labels must not collide with those names.
fn metrics_from_events(events: &[HermesUsage]) -> Vec<Metric> {
    let mut metrics = Vec::new();
    let recent = recent_models(events);
    if recent.is_empty() {
        metrics.push(Metric::text("Recent models", "None yet".into()));
    } else {
        metrics.push(Metric::text(
            "Recent models",
            recent
                .iter()
                .map(|ev| display_model(&ev.model))
                .collect::<Vec<_>>()
                .join(" + "),
        ));
        let mut seen = std::collections::HashSet::new();
        let routes = recent
            .iter()
            .map(|ev| route_label(&ev.billing_provider, &ev.billing_base_url))
            .filter(|route| seen.insert(route.clone()))
            .collect::<Vec<_>>()
            .join(" + ");
        metrics.push(Metric::text("Via", routes));
    }
    let sessions = unique_sessions(events);
    if sessions > 0 {
        metrics.push(Metric::text("Sessions", sessions.to_string()));
    }
    metrics
}

fn recent_models(events: &[HermesUsage]) -> Vec<&HermesUsage> {
    let mut recent = events
        .iter()
        .filter(|ev| ev.task.is_empty() && !ev.model.is_empty())
        .collect::<Vec<_>>();
    recent.sort_by_key(|ev| std::cmp::Reverse(ev.ts_ms));
    let Some(latest) = recent.first().map(|ev| ev.ts_ms) else { return recent };
    let cutoff = latest.saturating_sub(24 * 60 * 60 * 1000);
    let mut seen = std::collections::HashSet::new();
    recent.retain(|ev| {
        ev.ts_ms >= cutoff && seen.insert(display_model(&ev.model).to_ascii_lowercase())
    });
    recent.truncate(2);
    recent
}

fn unique_sessions(events: &[HermesUsage]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for e in events {
        if !e.session_id.is_empty() {
            seen.insert(e.session_id.as_str());
        }
    }
    if seen.is_empty() {
        events.len()
    } else {
        seen.len()
    }
}

/// Last path segment so gateway-prefixed slugs stay readable on the card.
fn display_model(model: &str) -> &str {
    model
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(model)
}

fn route_blob(provider: &str, base_url: &str) -> String {
    format!("{} {}", provider, base_url).to_lowercase()
}

/// Human name for the backend that billed the row. A `custom` OpenAI-
/// compatible URL that points at a known gateway still shows that gateway
/// (Hermes labels AihubMix as `custom` when you paste the URL yourself).
fn route_label(provider: &str, base_url: &str) -> String {
    let blob = route_blob(provider, base_url);
    if blob.contains("aihubmix") {
        "AihubMix".into()
    } else if blob.contains("minimax") {
        "MiniMax".into()
    } else if blob.contains("openrouter") {
        "OpenRouter".into()
    } else if blob.contains("nous") {
        "Nous API".into()
    } else if provider.is_empty() || provider.eq_ignore_ascii_case("custom") {
        "Custom API".into()
    } else {
        provider.to_string()
    }
}

/// Which spend slice a Hermes row belongs to. MiniMax / OpenRouter —
/// including a custom URL pointed at those hosts — join those cards.
/// AihubMix (named or custom URL) stays on Hermes.
pub fn spend_slice(provider: &str, base_url: &str) -> (&'static str, &'static str) {
    let blob = route_blob(provider, base_url);
    if blob.contains("minimax") {
        ("minimax", "MiniMax")
    } else if blob.contains("openrouter") {
        ("openrouter", "OpenRouter")
    } else {
        ("hermes", "Hermes")
    }
}

/// Catalog key for this Hermes row. AihubMix prices can differ from the
/// model vendor or another gateway, so verified AihubMix rows use scoped
/// keys that cannot leak those rates onto other routes.
pub fn price_lookup_slug(model: &str, provider: &str, base_url: &str) -> String {
    if route_blob(provider, base_url).contains("aihubmix") {
        return match display_model(model) {
            "glm-5.3" => "coding-glm-5.3".into(),
            "hy4-preview" => "aihubmix/hy4-preview".into(),
            "qwen3.8-flash" => "aihubmix/qwen3.8-flash".into(),
            "qwen3.8-max-0902" | "qwen3.8-max-2026-09-02" => {
                "aihubmix/qwen3.8-max-2026-09-02".into()
            }
            _ => model.into(),
        };
    }
    model.into()
}

/// Per-session-per-model usage from Hermes's local store. Cached on the
/// (db, WAL) stamps; a busy/locked db serves the last good events.
pub fn collect_usage_events() -> Vec<HermesUsage> {
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(FileStamp, FileStamp, Vec<HermesUsage>)>> = Mutex::new(None);

    let Some(db_path) = state_db_path() else {
        return Vec::new();
    };
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

    let n = COPY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_base = std::env::temp_dir().join(format!("pane-hermes-{}-{n}", std::process::id()));
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
    // Optional columns (session_id, billing_base_url, task) were added as
    // Hermes grew; an older ledger must still yield tokens and cost.
    let cols = table_columns(&conn)?;
    let session_expr = if cols.iter().any(|c| c == "session_id") {
        "COALESCE(session_id, '')"
    } else {
        "''"
    };
    let url_expr = if cols.iter().any(|c| c == "billing_base_url") {
        "COALESCE(billing_base_url, '')"
    } else {
        "''"
    };
    let task_expr = if cols.iter().any(|c| c == "task") {
        "COALESCE(task, '')"
    } else {
        "''"
    };
    // last_seen/first_seen are epoch seconds as REAL; costs may be NULL.
    let sql = format!(
        "SELECT last_seen, model, billing_provider,
                input_tokens, output_tokens, reasoning_tokens,
                cache_read_tokens, cache_write_tokens,
                COALESCE(actual_cost_usd, 0.0), COALESCE(estimated_cost_usd, 0.0),
                {session_expr}, {url_expr}, {task_expr}
         FROM session_model_usage"
    );
    let mut stmt = conn
        .prepare(&sql)
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
                session_id: row.get::<_, String>(10).unwrap_or_default(),
                billing_base_url: row.get::<_, String>(11).unwrap_or_default(),
                task: row.get::<_, String>(12).unwrap_or_default(),
            })
        })
        .map_err(|e| format!("read session_model_usage: {e}"))?;
    Ok(rows.flatten().collect())
}

fn table_columns(conn: &rusqlite::Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(session_model_usage)")
        .map_err(|e| format!("pragma table_info: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("pragma table_info rows: {e}"))?;
    Ok(rows.flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: i64, model: &str, provider: &str, url: &str, session: &str) -> HermesUsage {
        HermesUsage {
            ts_ms: ts,
            model: model.into(),
            billing_provider: provider.into(),
            billing_base_url: url.into(),
            session_id: session.into(),
            task: String::new(),
            input: 1.0,
            output: 1.0,
            reasoning: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            cost_usd: 0.0,
        }
    }

    #[test]
    fn empty_ledger_still_has_a_recent_models_row() {
        let m = metrics_from_events(&[]);
        assert_eq!(m[0].label, "Recent models");
        assert_eq!(m[0].value.as_deref(), Some("None yet"));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn card_shows_recent_user_models_gateway_and_session_count() {
        let mut internal = ev(101_000_000, "glm-5", "", "", "s1");
        internal.task = "title_generation".into();
        let events = [
            ev(1, "glm-5.3", "aihubmix", "https://aihubmix.com/v1", "s3"),
            ev(
                100_000_000,
                "qwen3.8-flash",
                "custom",
                "https://aihubmix.com/v1/",
                "s1",
            ),
            ev(
                99_000_000,
                "hy4-preview",
                "aihubmix",
                "https://aihubmix.com/v1",
                "s2",
            ),
            internal,
        ];
        let m = metrics_from_events(&events);
        assert_eq!(m[0].label, "Recent models");
        assert_eq!(m[0].value.as_deref(), Some("qwen3.8-flash + hy4-preview"));
        assert_eq!(m[1].label, "Via");
        assert_eq!(m[1].value.as_deref(), Some("AihubMix"));
        assert_eq!(m[2].label, "Sessions");
        assert_eq!(m[2].value.as_deref(), Some("3"));
    }

    #[test]
    fn custom_url_without_a_known_host_stays_custom() {
        assert_eq!(
            route_label("custom", "https://example.com/v1"),
            "Custom API"
        );
        assert_eq!(route_label("aihubmix", ""), "AihubMix");
        assert_eq!(route_label("minimax-oauth", ""), "MiniMax");
        assert_eq!(route_label("nous-api", ""), "Nous API");
    }

    #[test]
    fn spend_slice_follows_custom_minimax_and_openrouter_urls() {
        assert_eq!(spend_slice("minimax-oauth", "").0, "minimax");
        assert_eq!(spend_slice("openrouter", "").0, "openrouter");
        assert_eq!(spend_slice("aihubmix", "").0, "hermes");
        assert_eq!(spend_slice("custom", "https://aihubmix.com/v1").0, "hermes");
        assert_eq!(
            spend_slice("custom", "https://api.minimax.io/v1").0,
            "minimax"
        );
        assert_eq!(
            spend_slice("custom", "https://openrouter.ai/api/v1").0,
            "openrouter"
        );
        assert_eq!(spend_slice("custom", "https://example.com/v1").0, "hermes");
        assert_eq!(spend_slice("nous-api", "").0, "hermes");
        assert_eq!(spend_slice("", "").0, "hermes");
    }

    #[test]
    fn aihubmix_glm53_looks_up_the_gateway_sku() {
        assert_eq!(
            price_lookup_slug("glm-5.3", "aihubmix", ""),
            "coding-glm-5.3"
        );
        assert_eq!(
            price_lookup_slug("glm-5.3", "custom", "https://aihubmix.com/v1"),
            "coding-glm-5.3"
        );
        // Other routes keep the generic name (unpriced until a catalog
        // learns it) so Z.ai / OpenRouter dollars aren't guessed.
        assert_eq!(price_lookup_slug("glm-5.3", "nous-api", ""), "glm-5.3");
        assert_eq!(
            price_lookup_slug("coding-glm-5.3", "aihubmix", ""),
            "coding-glm-5.3"
        );
    }

    #[test]
    fn aihubmix_launch_models_use_gateway_specific_skus() {
        assert_eq!(
            price_lookup_slug("hy4-preview", "aihubmix", "https://aihubmix.com/v1"),
            "aihubmix/hy4-preview"
        );
        assert_eq!(
            price_lookup_slug("qwen3.8-flash", "custom", "https://aihubmix.com/v1"),
            "aihubmix/qwen3.8-flash"
        );
        assert_eq!(
            price_lookup_slug("tencent/hy4-preview", "openrouter", ""),
            "tencent/hy4-preview"
        );
        assert_eq!(
            price_lookup_slug("qwen3.8-flash", "custom", "https://example.com/v1"),
            "qwen3.8-flash"
        );
        assert_eq!(
            price_lookup_slug("qwen3.8-max-0902", "custom", "https://aihubmix.com/v1/"),
            "aihubmix/qwen3.8-max-2026-09-02"
        );
        assert_eq!(
            price_lookup_slug(
                "qwen3.8-max-2026-09-02",
                "custom",
                "https://aihubmix.com/v1"
            ),
            "aihubmix/qwen3.8-max-2026-09-02"
        );
        assert_eq!(
            price_lookup_slug("qwen3.8-max-0902", "nous-api", ""),
            "qwen3.8-max-0902"
        );
    }

    #[test]
    fn display_model_peels_gateway_prefixes() {
        assert_eq!(display_model("coding-glm-5.3"), "coding-glm-5.3");
        assert_eq!(
            display_model("accounts/fireworks/models/deepseek-v4-pro-0813"),
            "deepseek-v4-pro-0813"
        );
    }

    #[test]
    fn older_ledger_without_optional_columns_still_reads_tokens() {
        let path =
            std::env::temp_dir().join(format!("pane-hermes-narrow-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE session_model_usage (
                last_seen REAL, model TEXT, billing_provider TEXT,
                input_tokens REAL, output_tokens REAL, reasoning_tokens REAL,
                cache_read_tokens REAL, cache_write_tokens REAL,
                actual_cost_usd REAL, estimated_cost_usd REAL
             );
             INSERT INTO session_model_usage VALUES
                (1.0, 'glm-5.3', 'aihubmix', 10, 5, 0, 0, 0, 0, 0);",
        )
        .unwrap();
        drop(conn);
        let events = read_usage_events(&path).expect("narrow schema should still parse");
        let _ = std::fs::remove_file(&path);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model, "glm-5.3");
        assert_eq!(events[0].input, 10.0);
        assert!(events[0].session_id.is_empty());
        assert!(events[0].billing_base_url.is_empty());
        assert!(events[0].task.is_empty());
    }
}
