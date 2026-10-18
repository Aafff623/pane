//! Snapshot acceptance and fallback policy, shared by boot and live refresh.

use crate::providers;
use crate::providers::Snapshot;
use serde_json::Value;
use std::collections::HashMap;

/// Last-good snapshots older than this are too misleading to show or to
/// use as a stand-in for a live Moonshot card.
pub(crate) const SNAPSHOT_CACHE_MS: i64 = 24 * 60 * 60 * 1000;
/// One failed cycle isn't "Outdated": vendors hiccup routinely.
pub(crate) const STALE_GRACE_MS: i64 = 3 * 60 * 1000;

pub(crate) struct FailState {
    pub(crate) until_ms: i64,
    pub(crate) note: String,
    pub(crate) rate_limited: bool,
}

pub(crate) fn retain_rate_limited_benches(map: &mut HashMap<String, FailState>) {
    map.retain(|_, f| f.rate_limited);
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct CachedSnap {
    pub(crate) at: i64,
    pub(crate) snap: providers::Snapshot,
}

pub(crate) fn cached_kimi_ok_from(doc: &Value, now_ms: i64) -> bool {
    if doc.pointer("/kimi/snap/status").and_then(Value::as_str) != Some("ok") {
        return false;
    }
    let at = doc.pointer("/kimi/at").and_then(Value::as_i64).unwrap_or(0);
    at > 0 && now_ms.saturating_sub(at) <= SNAPSHOT_CACHE_MS
}

pub(crate) fn fold_moonshot_into_kimi(all: &mut Vec<providers::Snapshot>) {
    let Some(kimi) = all.iter().find(|s| s.id == "kimi" && s.status == "ok") else {
        return;
    };
    // Don't throw away a freshly fetched wallet just because the plan
    // card loaded. Fold only when Kimi already carries those rows, or
    // when Moonshot has nothing to show (plan-only / no_credentials).
    let kimi_has_wallet = kimi.metrics.iter().any(|m| is_kimi_wallet_label(&m.label));
    let moonshot_has_rows = all
        .iter()
        .any(|s| s.id == "moonshot" && !s.metrics.is_empty());
    if kimi_has_wallet || !moonshot_has_rows {
        all.retain(|s| s.id != "moonshot");
    }
}

pub(crate) fn is_kimi_wallet_label(label: &str) -> bool {
    matches!(
        label,
        "API" | "Credits used" | "Balance" | "Vouchers" | "Cash"
    )
}

pub(crate) fn restore_kimi_wallet_rows(
    current: &mut providers::Snapshot,
    previous: &providers::Snapshot,
) {
    if current.metrics.iter().any(|m| m.label == "API") {
        return;
    }
    for m in &previous.metrics {
        if is_kimi_wallet_label(&m.label) && !current.metrics.iter().any(|x| x.label == m.label) {
            current.metrics.push(m.clone());
        }
    }
}

pub(crate) fn restore_last_success_after_error(
    current: &mut providers::Snapshot,
    previous: &providers::Snapshot,
    age_ms: i64,
    consecutive_failures: u32,
) -> bool {
    if current.status != "error" || age_ms > SNAPSHOT_CACHE_MS {
        return false;
    }
    let warning = current.error.clone();
    *current = previous.clone();
    if providers::should_mark_stale(age_ms > STALE_GRACE_MS, consecutive_failures) {
        current.stale = true;
        current.warning = warning;
    }
    true
}

pub(crate) fn identity_changed(stored: &Value, current: &Value, family: &str) -> bool {
    matches!(
        (stored.get(family).and_then(Value::as_str), current.get(family).and_then(Value::as_str)),
        (Some(old), Some(new)) if old != new
    )
}

/// Save the pruned cache before advancing its identity stamp. A failed cache
/// write leaves the old stamp intact so the next launch rechecks the swap.
pub(crate) fn reconcile_identities(
    cache: &mut HashMap<String, CachedSnap>,
    stored: &Value,
    current: &Value,
    persist_cache: impl FnOnce(&HashMap<String, CachedSnap>) -> Result<(), String>,
    persist_stamp: impl FnOnce(&Value) -> Result<(), String>,
) -> Result<(), String> {
    let mut stamp = serde_json::Map::new();
    let mut swapped = false;
    for family in ["claude", "codex"] {
        if identity_changed(stored, current, family) {
            cache.remove(family);
            swapped = true;
        }
        let old = stored.get(family).cloned().unwrap_or(Value::Null);
        let new = current.get(family).cloned().unwrap_or(Value::Null);
        stamp.insert(family.to_string(), if new.is_null() { old } else { new });
    }
    if swapped {
        persist_cache(cache)?;
    }
    let stamp = Value::Object(stamp);
    if &stamp != stored {
        persist_stamp(&stamp)?;
    }
    Ok(())
}

/// Returns whether the cache changed and valid observed meter readings.
/// A restored wallet never refreshes its original timestamp.
pub(crate) fn merge_snapshot(
    snapshot: &mut Snapshot,
    cache: &mut HashMap<String, CachedSnap>,
    failures: &mut HashMap<String, u32>,
    now_ms: i64,
) -> (
    bool,
    Vec<crate::usage_units::UsageReading>,
    Vec<crate::usage_units::UsageDays>,
) {
    let mut restored_wallet = false;
    if snapshot.id == "kimi" && snapshot.status == "ok" && snapshot.warning.is_some() {
        if let Some(previous) = cache.get("kimi") {
            let age = now_ms.saturating_sub(previous.at);
            if age <= SNAPSHOT_CACHE_MS {
                let rows = snapshot.metrics.len();
                restore_kimi_wallet_rows(snapshot, &previous.snap);
                restored_wallet = snapshot.metrics.len() > rows;
                if restored_wallet && age > STALE_GRACE_MS {
                    snapshot.stale = true;
                }
            }
        }
    }
    if snapshot.status == "ok" && !restored_wallet {
        failures.remove(&snapshot.id);
        cache.insert(
            snapshot.id.clone(),
            CachedSnap {
                at: now_ms,
                snap: snapshot.clone(),
            },
        );
        return (
            true,
            crate::usage_history::readings_from_metrics(&snapshot.metrics),
            snapshot
                .metrics
                .iter()
                .filter_map(|metric| metric.daily_usage.clone())
                .collect(),
        );
    }
    if snapshot.status == "error" {
        let streak = failures.entry(snapshot.id.clone()).or_default();
        *streak = streak.saturating_add(1);
        if let Some(previous) = cache.get(&snapshot.id) {
            restore_last_success_after_error(
                snapshot,
                &previous.snap,
                now_ms.saturating_sub(previous.at),
                *streak,
            );
        }
    }
    (false, Vec::new(), Vec::new())
}

pub(crate) fn bench_after_error(error: &str, now_ms: i64, interval_ms: i64) -> FailState {
    let rate_limited = error.contains("429");
    let retry_after_ms = error
        .split("retry_after_s=")
        .nth(1)
        .and_then(|rest| {
            rest.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<i64>()
                .ok()
        })
        .map(|seconds| seconds.saturating_mul(1000).min(3_600_000));
    let bench_ms = retry_after_ms
        .unwrap_or_else(|| interval_ms.max(if rate_limited { 300_000 } else { 60_000 }));
    FailState {
        until_ms: now_ms.saturating_add(bench_ms),
        rate_limited,
        note: match retry_after_ms {
            Some(ms) => format!(
                "rate limited — the vendor asked to wait ~{}m",
                (ms / 60_000).max(1)
            ),
            None if rate_limited => {
                format!("rate limited — cooling down for a few minutes ({error})")
            }
            None => error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Metric;
    use serde_json::json;

    #[test]
    fn live_merge_keeps_last_success_through_failure_then_replaces_it_on_recovery() {
        let mut cache = HashMap::new();
        let mut failures = HashMap::new();
        let mut success = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        assert!(merge_snapshot(&mut success, &mut cache, &mut failures, 1_000).0);
        for (attempt, warned) in [(1, false), (2, true)] {
            let mut error = Snapshot::error("codex", "Codex", "timeout".into());
            assert!(
                !merge_snapshot(
                    &mut error,
                    &mut cache,
                    &mut failures,
                    STALE_GRACE_MS + 2_000
                )
                .0
            );
            assert_eq!(error.status, "ok");
            assert_eq!(error.stale, warned, "failure {attempt}");
            assert_eq!(
                cache["codex"].at, 1_000,
                "fallback must not refresh the timestamp"
            );
        }
        let mut recovered = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 40.0, None)],
        );
        merge_snapshot(&mut recovered, &mut cache, &mut failures, 500_000);
        assert!(failures.is_empty());
        assert_eq!(cache["codex"].snap.metrics[0].used_percent, Some(40.0));
    }

    #[test]
    fn wallet_fallback_does_not_extend_the_old_balance_lifetime() {
        let mut cache = HashMap::new();
        let mut failures = HashMap::new();
        let mut wallet = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![Metric::text("Balance", "$10".into())],
        );
        merge_snapshot(&mut wallet, &mut cache, &mut failures, 1_000);
        let mut plan = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![Metric::progress("Session", 20.0, None)],
        );
        plan.warning = Some("wallet timeout".into());
        assert!(!merge_snapshot(&mut plan, &mut cache, &mut failures, STALE_GRACE_MS + 2_000).0);
        assert_eq!(plan.metrics.len(), 2);
        assert!(plan.stale);
        assert_eq!(cache["kimi"].at, 1_000);
    }

    #[test]
    fn daily_flow_survives_the_snapshot_policy_without_becoming_a_cumulative_meter() {
        let mut cache = HashMap::new();
        let mut failures = HashMap::new();
        let metric =
            Metric::text("app", "3 calls".into()).with_daily_usage(crate::usage_units::UsageDays {
                unit: crate::usage_units::UsageUnit::Requests,
                scope: "app:7".into(),
                days: std::collections::BTreeMap::from([("2026-10-16".into(), 3.0)]),
            });
        let mut snapshot = Snapshot::ok("giphy", "GIPHY", None, vec![metric]);
        let (changed, readings, daily) =
            merge_snapshot(&mut snapshot, &mut cache, &mut failures, 1_000);
        assert!(changed);
        assert!(
            readings.is_empty(),
            "daily flow is not a cumulative reading"
        );
        assert_eq!(daily[0].days["2026-10-16"], 3.0);
    }

    #[test]
    fn identity_swaps_do_not_invalidate_unknown_or_unrelated_accounts() {
        let stored = json!({"claude": "old", "codex": "same"});
        let current = json!({"claude": "new", "codex": "same"});
        assert!(identity_changed(&stored, &current, "claude"));
        assert!(!identity_changed(&stored, &current, "codex"));
        assert!(!identity_changed(
            &stored,
            &json!({"claude": null}),
            "claude"
        ));
        assert!(!identity_changed(&json!({}), &current, "claude"));
        assert!(!identity_changed(&stored, &current, "claude@extra"));
    }

    #[test]
    fn failed_cache_prune_never_advances_the_identity_stamp_even_on_retry() {
        use std::cell::Cell;
        let mut cache = HashMap::from([(
            "claude".into(),
            CachedSnap {
                at: 1,
                snap: Snapshot::ok("claude", "Claude", None, vec![]),
            },
        )]);
        let stored = json!({"claude": "old", "codex": null});
        let current = json!({"claude": "new", "codex": null});
        let stamp_writes = Cell::new(0);
        for _ in 0..2 {
            assert!(reconcile_identities(
                &mut cache,
                &stored,
                &current,
                |_| Err("disk full".into()),
                |_| {
                    stamp_writes.set(stamp_writes.get() + 1);
                    Ok(())
                },
            )
            .is_err());
        }
        assert_eq!(stamp_writes.get(), 0);
        assert!(!cache.contains_key("claude"));
    }

    #[test]
    fn identity_reconciliation_preserves_known_stamps_and_orders_effects() {
        use std::cell::RefCell;
        let mut cache = HashMap::from([(
            "claude".into(),
            CachedSnap {
                at: 1,
                snap: Snapshot::ok("claude", "Claude", None, vec![]),
            },
        )]);
        let effects = RefCell::new(vec![]);
        reconcile_identities(
            &mut cache,
            &json!({"claude": "old", "codex": "keep"}),
            &json!({"claude": "new", "codex": null}),
            |pruned| {
                assert!(!pruned.contains_key("claude"));
                effects.borrow_mut().push("cache");
                Ok(())
            },
            |stamp| {
                assert_eq!(stamp["codex"], "keep");
                effects.borrow_mut().push("stamp");
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(*effects.borrow(), ["cache", "stamp"]);
    }

    #[test]
    fn cooldown_obeys_retry_after_and_manual_refresh_keeps_vendor_rate_limits() {
        let mut benches = HashMap::from([
            (
                "rate".into(),
                bench_after_error("HTTP 429 retry_after_s=7200", 10_000, 60_000),
            ),
            (
                "network".into(),
                bench_after_error("timeout", 10_000, 300_000),
            ),
        ]);
        assert_eq!(benches["rate"].until_ms, 3_610_000);
        assert_eq!(benches["network"].until_ms, 310_000);
        retain_rate_limited_benches(&mut benches);
        assert!(benches.contains_key("rate"));
        assert!(!benches.contains_key("network"));
    }
    #[test]
    fn kimi_wallet_labels() {
        assert!(is_kimi_wallet_label("API"));
        assert!(is_kimi_wallet_label("Balance"));
        assert!(!is_kimi_wallet_label("Session"));
        assert!(!is_kimi_wallet_label("Weekly"));
    }
    #[test]
    fn restore_wallet_rows_when_api_missing() {
        let mut current = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![Metric::progress("Session", 0.0, None)],
        );
        current.warning = Some("Moonshot API wallet couldn't refresh".into());
        let previous = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![
                Metric::progress("Session", 10.0, None),
                Metric::progress("API", 24.0, None),
                Metric::text("Balance", "$152.00".into()),
            ],
        );
        restore_kimi_wallet_rows(&mut current, &previous);
        let labels: Vec<_> = current.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, ["Session", "API", "Balance"]);
    }
    #[test]
    fn restore_wallet_rows_skips_when_api_present() {
        let mut current = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![
                Metric::progress("Session", 0.0, None),
                Metric::progress("API", 1.0, None),
            ],
        );
        let previous = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![Metric::progress("API", 99.0, None)],
        );
        restore_kimi_wallet_rows(&mut current, &previous);
        let api = current.metrics.iter().find(|m| m.label == "API").unwrap();
        assert!((api.used_percent.unwrap() - 1.0).abs() < 0.01);
    }
    #[test]
    fn cached_kimi_ok_ignores_stale_or_missing_entries() {
        let now = 1_800_000_000_000i64;
        let fresh = json!({"kimi": {"at": now - 60_000, "snap": {"status": "ok"}}});
        assert!(cached_kimi_ok_from(&fresh, now));
        let old = json!({"kimi": {"at": now - SNAPSHOT_CACHE_MS - 1, "snap": {"status": "ok"}}});
        assert!(!cached_kimi_ok_from(&old, now));
        let err = json!({"kimi": {"at": now, "snap": {"status": "error"}}});
        assert!(!cached_kimi_ok_from(&err, now));
        assert!(!cached_kimi_ok_from(&json!({}), now));
    }
    #[test]
    fn recent_error_fallback_within_grace_is_not_marked_stale() {
        let previous = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        let mut current = Snapshot::error("codex", "Codex", "timeout".into());

        assert!(restore_last_success_after_error(
            &mut current,
            &previous,
            1_000,
            1,
        ));
        assert_eq!(current.status, "ok");
        assert!(!current.stale);
        assert_eq!(current.warning, None);
        assert_eq!(current.metrics[0].used_percent, Some(25.0));
    }
    #[test]
    fn recent_error_fallback_after_grace_is_marked_stale() {
        let previous = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        let mut current = Snapshot::error("codex", "Codex", "timeout".into());

        assert!(restore_last_success_after_error(
            &mut current,
            &previous,
            STALE_GRACE_MS + 1,
            2,
        ));
        assert_eq!(current.status, "ok");
        assert!(current.stale);
        assert_eq!(current.warning.as_deref(), Some("timeout"));
        assert_eq!(current.metrics[0].used_percent, Some(25.0));
    }
    #[test]
    fn one_consecutive_failure_past_the_grace_stays_silent() {
        let previous = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        let mut current = Snapshot::error("codex", "Codex", "timeout".into());

        assert!(restore_last_success_after_error(
            &mut current,
            &previous,
            STALE_GRACE_MS + 1,
            1,
        ));
        assert_eq!(current.status, "ok");
        assert!(!current.stale, "the first failure never warns");
        assert_eq!(current.warning, None);
    }
    #[test]
    fn expired_error_fallback_is_not_restored() {
        let previous = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        let mut current = Snapshot::error("codex", "Codex", "timeout".into());

        assert!(!restore_last_success_after_error(
            &mut current,
            &previous,
            SNAPSHOT_CACHE_MS + 1,
            5,
        ));
        assert_eq!(current.status, "error");
        assert!(!current.stale);
    }
    #[test]
    fn fold_keeps_moonshot_when_kimi_has_no_wallet() {
        let mut all = vec![
            Snapshot::ok(
                "kimi",
                "Kimi Code",
                None,
                vec![Metric::progress("Session", 0.0, None)],
            ),
            Snapshot::ok(
                "moonshot",
                "Kimi API",
                None,
                vec![Metric::progress("Credits used", 24.0, None)],
            ),
        ];
        fold_moonshot_into_kimi(&mut all);
        assert!(all.iter().any(|s| s.id == "moonshot"));
    }
    #[test]
    fn fold_hides_moonshot_when_kimi_has_wallet_or_moonshot_is_empty() {
        let mut with_api = vec![
            Snapshot::ok(
                "kimi",
                "Kimi Code",
                None,
                vec![
                    Metric::progress("Session", 0.0, None),
                    Metric::progress("API", 24.0, None),
                ],
            ),
            Snapshot::ok(
                "moonshot",
                "Kimi API",
                None,
                vec![Metric::progress("Credits used", 24.0, None)],
            ),
        ];
        fold_moonshot_into_kimi(&mut with_api);
        assert!(!with_api.iter().any(|s| s.id == "moonshot"));

        let mut empty_moon = vec![
            Snapshot::ok(
                "kimi",
                "Kimi Code",
                None,
                vec![Metric::progress("Session", 0.0, None)],
            ),
            Snapshot::no_credentials("moonshot", "Kimi API", "paste a key"),
        ];
        fold_moonshot_into_kimi(&mut empty_moon);
        assert!(!empty_moon.iter().any(|s| s.id == "moonshot"));
    }
    #[test]
    fn clear_soft_benches_keeps_only_rate_limits() {
        let mut map = HashMap::new();
        map.insert(
            "claude@bench-soft".to_string(),
            FailState {
                until_ms: i64::MAX,
                note: "connection refused".into(),
                rate_limited: false,
            },
        );
        map.insert(
            "codex@bench-hard".to_string(),
            FailState {
                until_ms: i64::MAX,
                note: "rate limited — cooling down".into(),
                rate_limited: true,
            },
        );
        retain_rate_limited_benches(&mut map);
        assert!(
            !map.contains_key("claude@bench-soft"),
            "ordinary bench must drop"
        );
        assert!(
            map.contains_key("codex@bench-hard"),
            "429 bench must survive"
        );
    }
}
