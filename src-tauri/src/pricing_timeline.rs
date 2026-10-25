//! Append-only pricing observation timeline (`pricing/schedules.json`).
//!
//! Every price Pane ever charged was observed at some moment. This module
//! records those observations and answers "what did (scope, model, tier)
//! cost at time T?" from the last observation at or before T. Entries are
//! never edited or deleted, so replaying a cached file's pricing questions
//! at its own timestamps always returns the same answer — catalog refreshes,
//! price rises and price drops are forward-only by construction.
//!
//! The file stores the observation array as a payload string plus a count and
//! an FNV-1a checksum over that exact string. A missing entry, a shortened
//! list or any byte edited outside Pane fails the check: the file is
//! quarantined next to the original (`schedules.json.corrupt-<ms>`) and the
//! timeline starts empty. Cached dollars keep their vouching until the owner
//! runs the explicit rebuild — never a silent history rewrite.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::pricing::{normalize_model_key, Price};

const VERSION: u32 = 1;

/// Set when the file was edited outside Pane. Cached dollars keep their
/// existing vouching (no silent re-price); only an explicit rebuild re-reads.
static TAMPERED: AtomicBool = AtomicBool::new(false);

pub fn tampered() -> bool {
    TAMPERED.load(Ordering::Relaxed)
}

/// Called after an explicit, user-confirmed rebuild dropped the parse cache.
pub fn acknowledge_rebuild() {
    TAMPERED.store(false, Ordering::Relaxed);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub scope: String,
    pub model: String,
    /// Pricing variant within a model: "std", "fast", "codex", "codex-fast".
    /// Fast/priority multipliers live inside `pin`, so a tier is part of the
    /// identity: a standard request and a fast request observed at the same
    /// instant are two separate price facts.
    #[serde(default)]
    pub tier: String,
    /// Epoch ms at which this price was observed (catalog ingest or first sight).
    pub observed_at: i64,
    /// None = the model was observed as unpriced at this moment.
    pub pin: Option<Price>,
    /// catalog | builtin | override | correction | manual
    pub basis: String,
}

#[derive(Serialize, Deserialize)]
struct TimelineFile {
    version: u32,
    count: usize,
    /// FNV-1a over `payload` exactly as stored.
    checksum: u64,
    /// Serialized `Vec<Observation>`; kept as a string so the checksum covers
    /// the precise bytes — a float reformatted by another serializer path can
    /// never invalidate a file this process just wrote.
    payload: String,
}

#[derive(Default)]
struct Store {
    observations: Vec<Observation>,
    /// (scope, model, tier) → indices into `observations`, ascending time.
    index: HashMap<(String, String, String), Vec<usize>>,
    loaded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub pin: Option<Price>,
    /// The event predates the first observation for this key: the pin is the
    /// earliest known price, not a price that had been observed at that time.
    pub estimated: bool,
    pub observed_at: i64,
}

fn store() -> &'static Mutex<Store> {
    static S: OnceLock<Mutex<Store>> = OnceLock::new();
    S.get_or_init(Default::default)
}

fn dir() -> PathBuf {
    crate::providers::config_dir().join("pricing")
}

fn path() -> PathBuf {
    if let Ok(custom) = std::env::var("PANE_PRICING_TIMELINE") {
        return PathBuf::from(custom);
    }
    dir().join("schedules.json")
}

/// Tests must never write the user's real `schedules.json`. Without an
/// explicit `PANE_PRICING_TIMELINE` override the store is memory-only in
/// test builds; production always persists.
fn memory_only() -> bool {
    cfg!(test) && std::env::var("PANE_PRICING_TIMELINE").is_err()
}

/// FNV-1a. Stable across runs and Rust versions (unlike DefaultHasher).
fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

fn key(scope: &str, model: &str, tier: &str) -> (String, String, String) {
    (scope.to_string(), normalize_model_key(model), tier.to_string())
}

impl Store {
    fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, obs) in self.observations.iter().enumerate() {
            let k = (obs.scope.clone(), obs.model.clone(), obs.tier.clone());
            self.index.entry(k).or_default().push(i);
        }
    }

    fn load(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if memory_only() {
            return;
        }
        let path = path();
        let Ok(raw) = std::fs::read_to_string(&path) else { return };
        let Ok(doc) = serde_json::from_str::<TimelineFile>(&raw) else {
            quarantine(&path, "unparseable");
            return;
        };
        if doc.version != VERSION || hash_bytes(0, doc.payload.as_bytes()) != doc.checksum {
            quarantine(&path, "edited outside Pane");
            return;
        }
        let Ok(observations) = serde_json::from_str::<Vec<Observation>>(&doc.payload) else {
            quarantine(&path, "unreadable payload");
            return;
        };
        if doc.count != observations.len() {
            quarantine(&path, "entry count mismatch");
            return;
        }
        self.observations = observations;
        self.rebuild_index();
    }
}

fn quarantine(path: &PathBuf, reason: &str) {
    TAMPERED.store(true, Ordering::Relaxed);
    let stamp = chrono::Utc::now().timestamp_millis();
    let target = path.with_extension(format!("json.corrupt-{stamp}"));
    eprintln!(
        "[pane] pricing timeline {reason} — moved to {} and starting empty (explicit rebuild required)",
        target.display()
    );
    let _ = std::fs::rename(path, target);
}

fn save(store: &Store) {
    if memory_only() {
        return;
    }
    let path = path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Snapshot first: count, checksum and payload must describe one vector.
    let observations = store.observations.clone();
    let Ok(payload) = serde_json::to_string(&observations) else { return };
    let doc = TimelineFile {
        version: VERSION,
        count: observations.len(),
        checksum: hash_bytes(0, payload.as_bytes()),
        payload,
    };
    let Ok(json) = serde_json::to_string(&doc) else { return };
    // A fixed `schedules.json.tmp` is shared by any second process (a dev
    // restart can briefly overlap the old instance). Two writers then
    // interleave inside the same file and rename a hybrid document that
    // parses but fails its own hash chain. Process-unique temp names make
    // each write complete before it replaces the target; last writer wins
    // with a self-consistent file.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = path.with_extension(format!("json.tmp-{}-{nanos}", std::process::id()));
    if std::fs::write(&tmp, json).is_err() {
        return;
    }
    // Windows can refuse the replace while a reader still holds the target
    // open; retry briefly instead of leaving the new timeline behind.
    for attempt in 0..8 {
        if std::fs::rename(&tmp, &path).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10 * (attempt + 1)));
    }
    let _ = std::fs::remove_file(&tmp);
}

/// Last observation at or before `ts_ms`; before the first observation the
/// first pin is returned with `estimated = true`. `None` means this key has
/// never been observed at all.
pub fn resolve(scope: &str, model: &str, tier: &str, ts_ms: i64) -> Option<Resolved> {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    s.load();
    let k = key(scope, model, tier);
    let indices = s.index.get(&k)?;
    let first = *indices.first()?;
    let first_obs = &s.observations[first];
    if ts_ms < first_obs.observed_at {
        return Some(Resolved {
            pin: first_obs.pin,
            estimated: true,
            observed_at: first_obs.observed_at,
        });
    }
    let idx = indices
        .iter()
        .rev()
        .find(|i| s.observations[**i].observed_at <= ts_ms)
        .copied()
        .unwrap_or(first);
    let obs = &s.observations[idx];
    Some(Resolved { pin: obs.pin, estimated: false, observed_at: obs.observed_at })
}

/// True when at least one observation exists for this key.
pub fn has_observations(scope: &str, model: &str, tier: &str) -> bool {
    resolve_last(scope, model, tier).is_some()
}

/// Last observation regardless of time.
pub fn resolve_last(scope: &str, model: &str, tier: &str) -> Option<Observation> {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    s.load();
    let k = key(scope, model, tier);
    s.index
        .get(&k)
        .and_then(|indices| indices.last())
        .map(|i| s.observations[*i].clone())
}

/// Append one observation. Identical consecutive pins are a no-op; an
/// observation older than the key's last one is rejected (system clock went
/// backwards) rather than reordering the timeline. Returns whether the file
/// changed.
pub fn append(
    scope: &str,
    model: &str,
    tier: &str,
    observed_at: i64,
    pin: Option<Price>,
    basis: &str,
) -> bool {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    s.load();
    let k = key(scope, model, tier);
    if let Some(indices) = s.index.get(&k) {
        if let Some(last_idx) = indices.last() {
            let last = &s.observations[*last_idx];
            if observed_at < last.observed_at {
                eprintln!(
                    "[pane] pricing timeline: refused backdated observation {} {} {} ({} < {})",
                    k.0, k.1, k.2, observed_at, last.observed_at
                );
                return false;
            }
            if last.pin == pin {
                return false;
            }
        }
    }
    let obs = Observation {
        scope: scope.to_string(),
        model: k.1.clone(),
        tier: k.2.clone(),
        observed_at,
        pin,
        basis: basis.to_string(),
    };
    let idx = s.observations.len();
    s.observations.push(obs);
    s.index.entry(k).or_default().push(idx);
    save(&s);
    true
}

/// Every key ever observed, for the forward-only refresh diff.
pub fn keys() -> Vec<(String, String, String)> {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    s.load();
    s.index.keys().cloned().collect()
}

/// Observation count (diagnostics / tests).
pub fn len() -> usize {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    s.load();
    s.observations.len()
}

/// Drop the in-memory copy so the next call re-reads the file. Tests only.
#[cfg(test)]
pub fn reset_for_test() {
    TAMPERED.store(false, Ordering::Relaxed);
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    *s = Store::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    static ENV_LOCK: StdMutex<()> = StdMutex::new(());

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pane-timeline-{name}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("schedules.json")
    }

    fn with_temp<T>(name: &str, f: impl FnOnce(&PathBuf) -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = temp_path(name);
        std::env::set_var("PANE_PRICING_TIMELINE", &path);
        reset_for_test();
        let out = f(&path);
        reset_for_test();
        std::env::remove_var("PANE_PRICING_TIMELINE");
        out
    }

    fn price(input: f64) -> Price {
        Price::flat(input, input * 4.0, input * 0.1, input)
    }

    #[test]
    fn interval_boundaries_and_first_observation_estimate() {
        with_temp("intervals", |_| {
            assert!(append("codex", "gpt-5", "std", 1_000, Some(price(1.0)), "catalog"));
            assert!(append("codex", "gpt-5", "std", 2_000, Some(price(2.0)), "catalog"));
            let early = resolve("codex", "gpt-5", "std", 500).unwrap();
            assert_eq!(early.pin, Some(price(1.0)));
            assert!(early.estimated);
            let mid = resolve("codex", "gpt-5", "std", 1_500).unwrap();
            assert_eq!(mid.pin, Some(price(1.0)));
            assert!(!mid.estimated);
            let late = resolve("codex", "gpt-5", "std", 9_000).unwrap();
            assert_eq!(late.pin, Some(price(2.0)));
        });
    }

    #[test]
    fn identical_pins_do_not_grow_the_file_and_backdating_is_rejected() {
        with_temp("append", |path| {
            assert!(append("claude", "opus", "std", 1_000, Some(price(3.0)), "catalog"));
            assert!(!append("claude", "opus", "std", 2_000, Some(price(3.0)), "catalog"));
            assert!(!append("claude", "opus", "std", 500, Some(price(9.0)), "catalog"));
            assert_eq!(len(), 1);
            let raw = std::fs::read_to_string(path).unwrap();
            assert!(raw.contains("\"count\":1"));
        });
    }

    #[test]
    fn tampered_file_is_quarantined_and_starts_empty() {
        with_temp("tamper", |path| {
            assert!(append("codex", "gpt-5", "std", 1_000, Some(price(1.0)), "catalog"));
            let mut doc: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            let payload = doc["payload"].as_str().unwrap().replace("gpt-5", "gpt-6");
            doc["payload"] = serde_json::Value::String(payload);
            std::fs::write(path, doc.to_string()).unwrap();
            reset_for_test();
            assert_eq!(len(), 0);
            let quarantine = std::fs::read_dir(path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
            assert!(quarantine);
        });
    }

    #[test]
    fn concurrent_writers_leave_a_valid_file() {
        with_temp("concurrent", |_| {
            std::thread::scope(|scope| {
                for t in 0..8 {
                    scope.spawn(move || {
                        for i in 0..40 {
                            let model = format!("m{}", i % 20);
                            append(
                                "scope",
                                &model,
                                "std",
                                1_000 + (i / 10) as i64,
                                Some(price(1.0 + t as f64 + i as f64 / 1000.0)),
                                "catalog",
                            );
                        }
                    });
                }
            });
            let before = len();
            assert!(before > 0);
            reset_for_test();
            assert_eq!(len(), before, "a concurrently written file must reload intact");
        });
    }

    #[test]
    fn a_written_file_reloads_without_tripping_the_checksum() {
        with_temp("roundtrip", |_| {
            assert!(append("codex", "gpt-5", "std", 1_000, Some(price(1.25)), "catalog"));
            assert!(append("claude", "opus", "fast", 2_000, None, "catalog"));
            assert!(append("codex", "gpt-5", "std", 3_000, Some(price(1.5)), "override"));
            let expected = len();
            assert_eq!(expected, 3);
            reset_for_test();
            assert_eq!(len(), expected, "a self-written file must pass its own checksum");
            assert_eq!(
                resolve("codex", "gpt-5", "std", 2_500).unwrap().pin,
                Some(price(1.25))
            );
            assert_eq!(resolve("codex", "gpt-5", "std", 3_500).unwrap().pin, Some(price(1.5)));
        });
    }

    #[test]
    fn tiers_are_separate_keys() {
        with_temp("tiers", |_| {
            append("claude", "opus", "std", 1_000, Some(price(3.0)), "catalog");
            append("claude", "opus", "fast", 1_000, Some(price(6.0)), "catalog");
            assert_eq!(resolve("claude", "opus", "std", 2_000).unwrap().pin, Some(price(3.0)));
            assert_eq!(resolve("claude", "opus", "fast", 2_000).unwrap().pin, Some(price(6.0)));
        });
    }
}
