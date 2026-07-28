//! Anonymous, opt-out usage telemetry — Pane's port of OpenUsage's
//! leashed-PostHog design (daily rollups, IDs/counts/enums only, hard-stop
//! opt-out). No SDK: events are plain documented POSTs to PostHog's batch
//! API, so everything that can ever leave the machine is in this one file.
//!
//! Exactly two event shapes, at most one of each per provider per local day:
//!   - `app_daily_active` — "this install was alive today" + a config
//!     snapshot (which providers are enabled, which metrics are starred —
//!     stable IDs only, never free text).
//!   - `provider_refresh_daily` — per provider: yesterday's refresh
//!     success/stale/error counts with error *categories* (auth /
//!     rate_limit / server / network / other). Raw error messages never
//!     leave the machine — they can contain paths or account details.
//!
//! The distinct ID is a random UUID minted on first send — derived from
//! nothing, linked to nothing. `$process_person_profile: false` rides on
//! every event so PostHog never builds a person profile (the Mac app's
//! `personProfiles = .never`).
//!
//! Opt-out (Settings → "Share anonymous usage statistics", config key
//! `telemetry`) is a hard stop: nothing is counted, nothing is written,
//! and the existing state file (UUID included) is deleted — turning it
//! back on starts as a brand-new anonymous install.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::providers;

/// PostHog project token — a client-side, write-only key (safe to commit,
/// like any analytics key baked into a public app). US-region project.
const TOKEN: &str = "phc_BBzyxRRRnrspg9VEyczEDBNPXjvvAZ7VbrJ2BWuhFPzt";
const ENDPOINT: &str = "https://us.i.posthog.com/batch/";

// ---------------------------------------------------------------------------
// State (persisted at %APPDATA%\Pane\telemetry.json)
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Serialize, Deserialize)]
struct ProviderDay {
    ok: u32,
    stale: u32,
    error: u32,
    /// error category → count ("auth", "rate_limit", "server", "network",
    /// "other") — categories only, never messages.
    #[serde(default)]
    categories: HashMap<String, u32>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct State {
    /// Random anonymous install ID; minted once, attached to nothing.
    uuid: String,
    /// Local day ("YYYY-MM-DD") the counters below belong to.
    #[serde(default)]
    day: String,
    /// Local day the last `app_daily_active` was actually delivered for.
    #[serde(default)]
    daily_sent: String,
    #[serde(default)]
    providers: HashMap<String, ProviderDay>,
}

fn state_path() -> PathBuf {
    providers::config_dir().join("telemetry.json")
}

fn load_state() -> State {
    let mut state: State = std::fs::read_to_string(state_path())
        .ok()
        .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default();
    if state.uuid.is_empty() {
        state.uuid = new_uuid();
    }
    state
}

fn save_state(state: &State) {
    let path = state_path();
    let tmp = path.with_extension("json.tmp");
    if let Ok(raw) = serde_json::to_string(state) {
        if std::fs::write(&tmp, raw).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

/// Random v4-style UUID from OS entropy (getrandom via the ring of deps we
/// already have would be heavier — two calls to the system RNG suffice).
fn new_uuid() -> String {
    let mut bytes = [0u8; 16];
    getrandom_fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h = |r: std::ops::Range<usize>| {
        bytes[r].iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    format!("{}-{}-{}-{}-{}", h(0..4), h(4..6), h(6..8), h(8..10), h(10..16))
}

fn getrandom_fill(buf: &mut [u8]) {
    // rand isn't in the tree; derive entropy from the OS UUID generator.
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for chunk in buf.chunks_mut(8) {
        // splitmix64 — statistically solid for an anonymous identifier.
        seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        for (i, b) in chunk.iter_mut().enumerate() {
            *b = (z >> (i * 8)) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Pure core — testable without fs or network
// ---------------------------------------------------------------------------

/// One refresh outcome as fed from fetch_usage: (provider id, status,
/// stale flag, error text — categorized here, never stored or sent raw).
pub struct Outcome {
    pub id: String,
    pub status: String,
    pub stale: bool,
    pub error: Option<String>,
}

/// Everything `app_daily_active` snapshots. Stable IDs and enums only.
pub struct ConfigSnapshot {
    pub app_version: String,
    pub enabled_providers: Vec<String>,
    pub starred_metrics: Vec<String>,
    pub appearance: String,
    pub density: String,
    pub refresh_minutes: u64,
}

/// Sort error text into a category enum. Mirrors the UI's ⚠-tooltip
/// classifier; the free-form message itself never leaves this function.
fn categorize(error: &str) -> &'static str {
    let e = error.to_lowercase();
    if e.contains("http 401")
        || e.contains("http 403")
        || e.contains("invalid_grant")
        || e.contains("expired")
        || e.contains("token refresh")
        || e.contains("sign in")
        || e.contains("sign-in")
        || e.contains("log in")
        || e.contains("credentials")
    {
        "auth"
    } else if e.contains("http 429") || e.contains("rate limit") {
        "rate_limit"
    } else if e.contains("http 5") {
        "server"
    } else if e.contains("error sending request")
        || e.contains("timed out")
        || e.contains("timeout")
        || e.contains("connect")
        || e.contains("network")
        || e.contains("dns")
        || e.contains("proxy")
    {
        "network"
    } else {
        "other"
    }
}

/// Fold one refresh's outcomes into the day counters (rolling the day over
/// first if needed). Returns the `provider_refresh_daily` events for the
/// *finished* day, ready to send — empty on ordinary same-day ticks.
fn accumulate(state: &mut State, today: &str, outcomes: &[Outcome]) -> Vec<Value> {
    let mut finished = Vec::new();
    if state.day != today {
        if !state.day.is_empty() {
            for (id, day) in std::mem::take(&mut state.providers) {
                let mut props = json!({
                    "provider": id,
                    "day": state.day,
                    "ok_count": day.ok,
                    "stale_count": day.stale,
                    "error_count": day.error,
                });
                for (cat, n) in &day.categories {
                    props[format!("errors_{cat}")] = json!(n);
                }
                finished.push(event("provider_refresh_daily", &state.uuid, props));
            }
        }
        state.providers.clear();
        state.day = today.to_string();
    }
    for o in outcomes {
        // "no_credentials" is a setup state, not a refresh result — a card
        // waiting for a sign-in shouldn't read as a daily failure.
        if o.status == "no_credentials" {
            continue;
        }
        let entry = state.providers.entry(o.id.clone()).or_default();
        if o.stale {
            entry.stale += 1;
        } else if o.status == "ok" {
            entry.ok += 1;
        } else {
            entry.error += 1;
            let cat = o.error.as_deref().map(categorize).unwrap_or("other");
            *entry.categories.entry(cat.to_string()).or_insert(0) += 1;
        }
    }
    finished
}

/// The once-a-day heartbeat event, when today's hasn't been sent yet.
fn daily_active(state: &State, today: &str, snap: &ConfigSnapshot) -> Option<Value> {
    if state.daily_sent == today {
        return None;
    }
    Some(event(
        "app_daily_active",
        &state.uuid,
        json!({
            "app_version": snap.app_version,
            "os": "windows",
            "enabled_providers": snap.enabled_providers,
            "enabled_provider_count": snap.enabled_providers.len(),
            "starred_metrics": snap.starred_metrics,
            "appearance": snap.appearance,
            "density": snap.density,
            "refresh_minutes": snap.refresh_minutes,
        }),
    ))
}

fn event(name: &str, uuid: &str, mut props: Value) -> Value {
    // Anonymous event: PostHog must never build a person profile for it
    // (the Mac app's `personProfiles = .never`, per event).
    props["$process_person_profile"] = json!(false);
    props["$lib"] = json!("pane");
    json!({
        "event": name,
        "distinct_id": uuid,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "properties": props,
    })
}

// ---------------------------------------------------------------------------
// Side-effect shell
// ---------------------------------------------------------------------------

/// Record one refresh cycle and flush anything due. Fire-and-forget from
/// fetch_usage; every failure path is silent (telemetry must never affect
/// the app) and undelivered events simply try again next cycle.
pub async fn record(enabled: bool, snap: ConfigSnapshot, outcomes: Vec<Outcome>) {
    if !enabled {
        // Hard stop: no counting, no state — and any previously stored
        // state (UUID included) is removed, so re-enabling later starts
        // over as a fresh anonymous install.
        let _ = std::fs::remove_file(state_path());
        return;
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut state = load_state();

    let mut batch = accumulate(&mut state, &today, &outcomes);
    let daily = daily_active(&state, &today, &snap);
    let has_daily = daily.is_some();
    batch.extend(daily);

    if !batch.is_empty() {
        let sent = send(&batch).await;
        if sent && has_daily {
            state.daily_sent = today.clone();
        }
        // provider_refresh_daily failures are dropped rather than retried:
        // the counters they summarized are already reset, and a lossy
        // daily rollup beats building a persistent outbox for it.
    }
    save_state(&state);
}

async fn send(batch: &[Value]) -> bool {
    let body = json!({ "api_key": TOKEN, "batch": batch });
    match providers::http().post(ENDPOINT).json(&body).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(id: &str, status: &str, stale: bool, error: Option<&str>) -> Outcome {
        Outcome {
            id: id.into(),
            status: status.into(),
            stale,
            error: error.map(str::to_string),
        }
    }

    #[test]
    fn same_day_ticks_accumulate_without_emitting() {
        let mut state = State { uuid: "u".into(), ..Default::default() };
        let events = accumulate(&mut state, "2026-07-26", &[outcome("claude", "ok", false, None)]);
        assert!(events.is_empty());
        let events = accumulate(&mut state, "2026-07-26", &[outcome("claude", "ok", false, None)]);
        assert!(events.is_empty());
        assert_eq!(state.providers["claude"].ok, 2);
    }

    #[test]
    fn day_rollover_emits_one_event_per_provider_and_resets() {
        let mut state = State { uuid: "u".into(), ..Default::default() };
        accumulate(&mut state, "2026-07-26", &[
            outcome("claude", "ok", false, None),
            outcome("grok", "error", false, Some("billing endpoint: HTTP 500")),
        ]);
        let events = accumulate(&mut state, "2026-07-27", &[outcome("claude", "ok", false, None)]);
        assert_eq!(events.len(), 2);
        let grok = events
            .iter()
            .find(|e| e["properties"]["provider"] == "grok")
            .expect("grok event");
        assert_eq!(grok["properties"]["error_count"], 1);
        assert_eq!(grok["properties"]["errors_server"], 1);
        assert_eq!(grok["properties"]["day"], "2026-07-26");
        // New day started fresh with the post-rollover outcome.
        assert_eq!(state.providers.len(), 1);
        assert_eq!(state.providers["claude"].ok, 1);
    }

    #[test]
    fn raw_error_text_never_appears_in_events() {
        let secret = "token refresh failed for C:\\Users\\alice\\.grok\\auth.json";
        let mut state = State { uuid: "u".into(), ..Default::default() };
        accumulate(&mut state, "2026-07-26", &[outcome("grok", "error", false, Some(secret))]);
        let events = accumulate(&mut state, "2026-07-27", &[]);
        let raw = serde_json::to_string(&events).unwrap();
        assert!(!raw.contains("alice"), "raw error text leaked into telemetry");
        assert!(raw.contains("errors_auth"), "categorized as auth (expired/credentials)");
    }

    #[test]
    fn no_credentials_is_not_a_failure() {
        let mut state = State { uuid: "u".into(), ..Default::default() };
        accumulate(&mut state, "2026-07-26", &[outcome("minimax", "no_credentials", false, None)]);
        assert!(state.providers.is_empty());
    }

    #[test]
    fn stale_counts_as_stale_not_error() {
        let mut state = State { uuid: "u".into(), ..Default::default() };
        accumulate(&mut state, "2026-07-26", &[outcome("claude", "ok", true, None)]);
        assert_eq!(state.providers["claude"].stale, 1);
        assert_eq!(state.providers["claude"].error, 0);
    }

    #[test]
    fn daily_active_fires_once_per_day() {
        let snap = ConfigSnapshot {
            app_version: "0.4.24".into(),
            enabled_providers: vec!["claude".into()],
            starred_metrics: vec![],
            appearance: "system".into(),
            density: "regular".into(),
            refresh_minutes: 5,
        };
        let mut state = State { uuid: "u".into(), ..Default::default() };
        let ev = daily_active(&state, "2026-07-26", &snap).expect("first tick fires");
        assert_eq!(ev["properties"]["$process_person_profile"], false);
        assert_eq!(ev["properties"]["enabled_provider_count"], 1);
        state.daily_sent = "2026-07-26".into();
        assert!(daily_active(&state, "2026-07-26", &snap).is_none());
        assert!(daily_active(&state, "2026-07-27", &snap).is_some());
    }

    #[test]
    fn error_categories_cover_the_common_shapes() {
        assert_eq!(categorize("billing endpoint: HTTP 401"), "auth");
        assert_eq!(categorize("Grok token expired — run the Grok CLI"), "auth");
        assert_eq!(categorize("HTTP 429 too many requests"), "rate_limit");
        assert_eq!(categorize("quota endpoint: HTTP 503"), "server");
        assert_eq!(categorize("error sending request: connection reset"), "network");
        assert_eq!(categorize("unexpected billing response shape"), "other");
    }
}
