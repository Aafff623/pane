//! Opt-in reset wakeups. Claims are synced to disk BEFORE a billable request;
//! an interrupted or failed attempt is never retried for the same reset.
//! Sources (reviewed 2026-10-14): docs.z.ai/devpack/tool/others;
//! docs.bigmodel.cn/cn/coding-plan/tool/others;
//! docs.z.ai/api-reference/llm/chat-completion;
//! platform.minimax.io/docs/api-reference/text-openai-api;
//! platform.minimax.cn/docs/api-reference/text-openai-api.

use crate::providers::{self, Snapshot};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Write, path::Path};

const PERIOD_MS: i64 = 5 * 60 * 60 * 1000;
const SAFETY_MS: i64 = 2 * 60 * 1000;
const COOLDOWN_MS: i64 = 10 * 60 * 1000;
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Region {
    #[default]
    Global,
    Cn,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub zai: bool,
    pub minimax: bool,
    pub zai_region: Region,
    pub minimax_region: Region,
}

impl Settings {
    fn enabled(&self, id: &str) -> bool {
        match id {
            "zai" => self.zai,
            "minimax" => self.minimax,
            _ => false,
        }
    }
    fn region(&self, id: &str) -> Region {
        if id == "zai" {
            self.zai_region
        } else {
            self.minimax_region
        }
    }
}

/// Call for an enabling settings update. The command caller must explicitly
/// supply consent after explaining that one real request consumes quota.
pub fn validate_settings(settings: &Settings, consent: bool) -> Result<(), String> {
    if (settings.zai || settings.minimax) && !consent {
        return Err(
            "Automatic wakeups consume a real request; explicit consent is required".into(),
        );
    }
    for id in ["zai", "minimax"] {
        if settings.enabled(id) {
            endpoint(id, settings.region(id))?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WakeStatus {
    pub provider: String,
    pub scheduled_reset_at: Option<i64>,
    pub attempted_reset_at: Option<i64>,
    pub attempted_at: Option<i64>,
    /// waiting | attempted | success | failed | skipped. `attempted` after a crash means
    /// outcome unknown, not success; never automatically retry it.
    pub status: String,
    pub message: String,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Store {
    settings: Settings,
    entries: BTreeMap<String, WakeStatus>,
}

/// Pure deadline gate: no invented windows, no historical catch-up bursts.
pub fn should_trigger(
    now: i64,
    reset: Option<i64>,
    attempted: Option<i64>,
    last_at: Option<i64>,
) -> bool {
    let Some(reset) = reset.filter(|r| *r > 0) else {
        return false;
    };
    now >= reset.saturating_add(SAFETY_MS)
        && now <= reset.saturating_add(COOLDOWN_MS)
        && attempted.is_none_or(|old| reset > old)
        && last_at.is_none_or(|last| now.saturating_sub(last) >= COOLDOWN_MS)
}

/// Keep an observed reset through the idle/no-reset response after expiry.
/// Only a fresh successful 5h progress row can establish a new deadline.
fn observe(entry: &mut WakeStatus, snap: &Snapshot, now: i64) {
    if snap.status != "ok" || snap.stale || snap.boot_cache {
        return;
    }
    let reset = snap
        .metrics
        .iter()
        .filter(|m| m.kind == "progress" && m.period_ms == Some(PERIOD_MS))
        .filter_map(|m| m.resets_at)
        .filter(|r| *r > 0 && *r <= now.saturating_add(PERIOD_MS + SAFETY_MS))
        .min();
    if let Some(reset) = reset {
        // A response may already advertise the next window while our saved
        // reset is still inside its two-minute safety margin. Preserve that
        // pending deadline until claimed or too old, rather than losing it.
        let pending = entry.scheduled_reset_at.is_some_and(|old| {
            entry
                .attempted_reset_at
                .is_none_or(|attempted| old > attempted)
                && now <= old.saturating_add(COOLDOWN_MS)
        });
        if !pending && entry.scheduled_reset_at.is_none_or(|old| reset > old) {
            entry.scheduled_reset_at = Some(reset);
        }
    }
}

fn load(path: &Path) -> Result<Store, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|_| "Wakeup state is unreadable; automatic requests stopped".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Store::default()),
        Err(_) => Err("Cannot read wakeup state; automatic requests stopped".into()),
    }
}

fn save(path: &Path, store: &Store) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid wakeup state directory")?;
    std::fs::create_dir_all(parent).map_err(|_| "Cannot create wakeup state directory")?;
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec(store).map_err(|_| "Cannot encode wakeup state")?;
    let mut file =
        std::fs::File::create(&tmp).map_err(|_| "Cannot save wakeup state; request cancelled")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Cannot sync wakeup state; request cancelled")?;
    drop(file);
    std::fs::rename(tmp, path).map_err(|_| "Cannot replace wakeup state; request cancelled".into())
}

pub async fn statuses() -> Result<Vec<WakeStatus>, String> {
    let _guard = LOCK.lock().await;
    Ok(load(&providers::config_dir().join("reset_wake.json"))?
        .entries
        .into_values()
        .collect())
}

pub async fn settings() -> Result<Settings, String> {
    let _guard = LOCK.lock().await;
    Ok(load(&providers::config_dir().join("reset_wake.json"))?.settings)
}

pub async fn set_settings(settings: Settings, consent: bool) -> Result<(), String> {
    let _guard = LOCK.lock().await;
    let path = providers::config_dir().join("reset_wake.json");
    let mut store = load(&path)?;
    validate_settings(
        &settings,
        consent || !requires_consent(&store.settings, &settings),
    )?;
    for id in ["zai", "minimax"] {
        if !settings.enabled(id) {
            if let Some(entry) = store.entries.get_mut(id) {
                entry.scheduled_reset_at = None;
            }
        }
    }
    store.settings = settings;
    save(&path, &store)
}

fn requires_consent(old: &Settings, new: &Settings) -> bool {
    ["zai", "minimax"]
        .iter()
        .any(|id| new.enabled(id) && (!old.enabled(id) || new.region(id) != old.region(id)))
}

/// Call on each live quota refresh; never pass only boot-cache snapshots.
/// Processes at most one request per enabled provider. Disabled/removed
/// providers are skipped even if a saved deadline is due.
pub async fn tick(settings: &Settings, snapshots: &[Snapshot]) -> Result<Vec<WakeStatus>, String> {
    let _guard = LOCK.lock().await;
    let path = providers::config_dir().join("reset_wake.json");
    let mut store = load(&path)?;
    if !settings.zai && !settings.minimax {
        return Ok(store.entries.into_values().collect());
    }
    let now = chrono::Utc::now().timestamp_millis();
    for id in ["zai", "minimax"] {
        // Re-read the persisted consent gate under the same lock as settings
        // updates; a stale background caller cannot re-enable a disabled job.
        if !settings.enabled(id) || !store.settings.enabled(id) || providers::provider_disabled(id)
        {
            continue;
        }
        let Some(snap) = snapshots
            .iter()
            .find(|s| s.id == id && s.status == "ok" && !s.stale && !s.boot_cache)
        else {
            continue;
        };
        let entry = store
            .entries
            .entry(id.into())
            .or_insert_with(|| WakeStatus {
                provider: id.into(),
                status: "waiting".into(),
                message: "Waiting for a known 5-hour reset".into(),
                ..Default::default()
            });
        skip_already_active(entry, snap, now);
        // Check saved deadline before recording the next one: a new snapshot
        // may already have advanced its reset into the following window.
        let due = should_trigger(
            now,
            entry.scheduled_reset_at,
            entry.attempted_reset_at,
            entry.attempted_at,
        );
        if !due {
            observe(entry, snap, now);
        }
        let due = due
            || should_trigger(
                now,
                entry.scheduled_reset_at,
                entry.attempted_reset_at,
                entry.attempted_at,
            );
        if due {
            entry.attempted_reset_at = entry.scheduled_reset_at;
            entry.attempted_at = Some(now);
            entry.status = "attempted".into();
            entry.message =
                "Request claimed; outcome may be unknown if Pane was interrupted".into();
            save(&path, &store)?;
            let result = request(id, store.settings.region(id)).await;
            let entry = store.entries.get_mut(id).expect("claimed provider exists");
            match result {
                Ok(()) => {
                    entry.status = "success".into();
                    entry.message = "Automatic request accepted".into();
                }
                Err(error) => {
                    entry.status = "failed".into();
                    entry.message = error;
                }
            }
            observe(entry, snap, now);
        }
    }
    save(&path, &store)?;
    Ok(store.entries.into_values().collect())
}

/// Closed allowlist; no URL can originate in settings or provider key files.
fn endpoint(id: &str, region: Region) -> Result<&'static str, String> {
    match (id, region) {
        ("zai", Region::Global) => Ok("https://api.z.ai/api/coding/paas/v4/chat/completions"),
        ("zai", Region::Cn) => Ok("https://open.bigmodel.cn/api/coding/paas/v4/chat/completions"),
        ("minimax", Region::Global) => Ok("https://api.minimax.io/v1/chat/completions"),
        ("minimax", Region::Cn) => Ok("https://api.minimax.cn/v1/chat/completions"),
        _ => Err("Provider does not support automatic wakeups".into()),
    }
}

async fn request(id: &str, region: Region) -> Result<(), String> {
    let url = endpoint(id, region)?;
    let (key, model) = match id {
        "zai" => (
            providers::zai::find_key(),
            if region == Region::Cn {
                "glm-5.2"
            } else {
                "glm-4.7"
            },
        ),
        "minimax" => (providers::minimax::find_api_key(), "MiniMax-M2.7"),
        _ => return Err("Unsupported automatic wakeup provider".into()),
    };
    let key = key.ok_or("No credential available for automatic wakeup")?;
    let body = serde_json::json!({"model":model,"messages":[{"role":"user","content":"hi"}],"max_tokens":1,"stream":false});
    let response = providers::http_no_redirect()
        .post(url)
        .bearer_auth(&key)
        .timeout(std::time::Duration::from_secs(30))
        .json(&body)
        .send()
        .await
        .map_err(|_| {
            "Automatic request failed (network or timeout); no automatic retry".to_string()
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "Automatic request failed (HTTP {}); no automatic retry",
            response.status().as_u16()
        ));
    }
    let doc = providers::json_body(response, 64 * 1024, "wakeup")
        .await
        .map_err(|_| "Automatic request returned an unreadable response".to_string())?;
    if accepted(&doc) {
        Ok(())
    } else {
        Err("Automatic request was not accepted by provider; no automatic retry".into())
    }
}

fn accepted(doc: &serde_json::Value) -> bool {
    doc.get("error").is_none()
        && doc.get("success").and_then(|v| v.as_bool()) != Some(false)
        && doc
            .pointer("/base_resp/status_code")
            .and_then(|v| v.as_i64())
            .is_none_or(|code| code == 0)
        && doc
            .get("choices")
            .and_then(|v| v.as_array())
            .is_some_and(|choices| !choices.is_empty())
}

/// A later 5h window with actual usage is evidence that the user's own
/// request already activated it. Consume the old deadline without a poke.
fn skip_already_active(entry: &mut WakeStatus, snap: &Snapshot, now: i64) {
    let Some(old) = entry.scheduled_reset_at else {
        return;
    };
    if now < old || entry.attempted_reset_at.is_some_and(|last| old <= last) {
        return;
    }
    let activated = snap.metrics.iter().any(|m| {
        m.kind == "progress"
            && m.period_ms == Some(PERIOD_MS)
            && m.resets_at.is_some_and(|next| next > old)
            && m.used_percent
                .is_some_and(|used| used.is_finite() && used > 0.0)
    });
    if activated {
        entry.attempted_reset_at = Some(old);
        entry.status = "skipped".into();
        entry.message = "New window already has usage; no automatic request needed".into();
        observe(entry, snap, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RESET: i64 = 1_790_000_000_000;
    #[test]
    fn explicit_opt_in_and_closed_regions() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.zai && !settings.minimax);
        assert!(validate_settings(&settings, false).is_ok());
        let enabled = Settings {
            zai: true,
            ..Default::default()
        };
        assert!(validate_settings(&enabled, false).is_err());
        assert!(validate_settings(&enabled, true).is_ok());
        assert!(validate_settings(
            &Settings {
                zai_region: Region::Cn,
                ..enabled
            },
            true
        )
        .is_ok());
        assert!(
            serde_json::from_str::<Settings>(r#"{"endpoint":"https://evil.example"}"#).is_err()
        );
        assert!(endpoint("openai", Region::Global).is_err());
    }
    #[test]
    fn disabling_one_provider_does_not_require_consent_for_unchanged_other() {
        let both = Settings {
            zai: true,
            minimax: true,
            ..Default::default()
        };
        let only_minimax = Settings {
            zai: false,
            ..both.clone()
        };
        assert!(!requires_consent(&both, &only_minimax));
        assert!(!requires_consent(&both, &Settings::default()));
        assert!(requires_consent(&only_minimax, &both));
        assert!(requires_consent(
            &both,
            &Settings {
                minimax_region: Region::Cn,
                ..both.clone()
            }
        ));
    }
    #[test]
    fn safety_boundary_cooldown_and_expired_deadlines() {
        assert!(!should_trigger(
            RESET + SAFETY_MS - 1,
            Some(RESET),
            None,
            None
        ));
        assert!(should_trigger(RESET + SAFETY_MS, Some(RESET), None, None));
        assert!(!should_trigger(
            RESET + COOLDOWN_MS + 1,
            Some(RESET),
            None,
            None
        ));
        assert!(!should_trigger(RESET + SAFETY_MS, None, None, None));
        assert!(!should_trigger(RESET, Some(i64::MAX), None, None));
        assert!(!should_trigger(
            RESET + SAFETY_MS,
            Some(RESET),
            None,
            Some(RESET)
        ));
    }
    #[test]
    fn persisted_claim_prevents_retry_after_failure_restart_or_clock_rollback() {
        let status = WakeStatus {
            attempted_reset_at: Some(RESET),
            attempted_at: Some(RESET + SAFETY_MS),
            status: "attempted".into(),
            ..Default::default()
        };
        let restored: WakeStatus =
            serde_json::from_str(&serde_json::to_string(&status).unwrap()).unwrap();
        assert!(!should_trigger(
            RESET + COOLDOWN_MS,
            Some(RESET),
            restored.attempted_reset_at,
            restored.attempted_at
        ));
        assert!(!should_trigger(
            RESET,
            Some(RESET - 1),
            restored.attempted_reset_at,
            restored.attempted_at
        ));
        assert!(should_trigger(
            RESET + PERIOD_MS + SAFETY_MS,
            Some(RESET + PERIOD_MS),
            restored.attempted_reset_at,
            restored.attempted_at
        ));
    }
    #[test]
    fn only_fresh_five_hour_progress_observations_schedule() {
        let mut entry = WakeStatus::default();
        let mut snap = Snapshot::ok(
            "zai",
            "Z.ai",
            None,
            vec![providers::Metric::progress("Weekly", 5.0, None)
                .with_reset(Some(RESET), Some(7 * 86_400_000))],
        );
        observe(&mut entry, &snap, RESET);
        assert_eq!(entry.scheduled_reset_at, None);
        snap.metrics[0].period_ms = Some(PERIOD_MS);
        snap.boot_cache = true;
        observe(&mut entry, &snap, RESET);
        assert_eq!(entry.scheduled_reset_at, None);
        snap.boot_cache = false;
        observe(&mut entry, &snap, RESET);
        assert_eq!(entry.scheduled_reset_at, Some(RESET));
        snap.metrics[0].resets_at = None;
        observe(&mut entry, &snap, RESET + SAFETY_MS);
        assert_eq!(entry.scheduled_reset_at, Some(RESET));
    }
    #[test]
    fn next_window_does_not_erase_pending_reset_during_safety_margin() {
        let mut entry = WakeStatus {
            scheduled_reset_at: Some(RESET),
            ..Default::default()
        };
        let snap = Snapshot::ok(
            "zai",
            "Z.ai",
            None,
            vec![providers::Metric::progress("Session", 0.0, None)
                .with_reset(Some(RESET + PERIOD_MS), Some(PERIOD_MS))],
        );
        observe(&mut entry, &snap, RESET + SAFETY_MS - 1);
        assert_eq!(entry.scheduled_reset_at, Some(RESET));
        assert!(should_trigger(
            RESET + SAFETY_MS,
            entry.scheduled_reset_at,
            None,
            None
        ));
        entry.attempted_reset_at = Some(RESET);
        observe(&mut entry, &snap, RESET + SAFETY_MS);
        assert_eq!(entry.scheduled_reset_at, Some(RESET + PERIOD_MS));
    }
    #[test]
    fn a_window_already_activated_by_usage_is_not_poked_again() {
        let mut entry = WakeStatus {
            scheduled_reset_at: Some(RESET),
            ..Default::default()
        };
        let mut snap = Snapshot::ok(
            "zai",
            "Z.ai",
            None,
            vec![providers::Metric::progress("Session", 0.0, None)
                .with_reset(Some(RESET + PERIOD_MS), Some(PERIOD_MS))],
        );
        skip_already_active(&mut entry, &snap, RESET + SAFETY_MS);
        assert_eq!(entry.attempted_reset_at, None);
        snap.metrics[0].used_percent = Some(0.1);
        skip_already_active(&mut entry, &snap, RESET + SAFETY_MS);
        assert_eq!(entry.status, "skipped");
        assert_eq!(entry.attempted_reset_at, Some(RESET));
        assert_eq!(entry.attempted_at, None);
        assert!(!should_trigger(
            RESET + SAFETY_MS,
            entry.scheduled_reset_at,
            entry.attempted_reset_at,
            entry.attempted_at
        ));
    }
    #[test]
    fn business_errors_cannot_masquerade_as_success() {
        assert!(accepted(
            &serde_json::json!({"choices":[{"finish_reason":"length"}]})
        ));
        assert!(!accepted(
            &serde_json::json!({"error":{"message":"secret must not be forwarded"}})
        ));
        assert!(!accepted(
            &serde_json::json!({"choices":[{}],"base_resp":{"status_code":1004}})
        ));
        assert!(!accepted(&serde_json::json!({"choices":[]})));
        assert!(!accepted(
            &serde_json::json!({"choices":[{}],"success":false})
        ));
    }
    #[test]
    fn durable_claim_survives_real_disk_round_trip_and_corruption_fails_closed() {
        let path = std::env::temp_dir().join(format!(
            "pane-wake-test-{}-{}.json",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let mut store = Store::default();
        store.settings.zai = true;
        store.entries.insert(
            "zai".into(),
            WakeStatus {
                provider: "zai".into(),
                scheduled_reset_at: Some(RESET),
                attempted_reset_at: Some(RESET),
                attempted_at: Some(RESET + SAFETY_MS),
                status: "attempted".into(),
                message: "outcome unknown".into(),
            },
        );
        save(&path, &store).unwrap();
        let restored = load(&path).unwrap();
        assert!(restored.settings.zai);
        let entry = &restored.entries["zai"];
        assert!(!should_trigger(
            RESET + SAFETY_MS + 1,
            entry.scheduled_reset_at,
            entry.attempted_reset_at,
            entry.attempted_at
        ));
        // Exercise replacement of an existing state file on Windows as well.
        save(&path, &restored).unwrap();
        std::fs::write(&path, b"not json").unwrap();
        assert!(load(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
